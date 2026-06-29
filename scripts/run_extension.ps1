<#
.SYNOPSIS
    Rebuild and run the Tex2Doc browser extension together with doc-server.

.EXAMPLE
    .\scripts\run_extension.ps1

.EXAMPLE
    .\scripts\run_extension.ps1 -Browser firefox -ServerPort 2625 -DatabaseUrl "postgres://postgres:postgres@127.0.0.1:5432/docdb"
#>

param(
    [ValidateSet("chrome", "edge", "firefox", "safari")]
    [string]$Browser = "chrome",
    [int]$ServerPort = 2624,
    [string]$DatabaseUrl = $env:DATABASE_URL,
    [switch]$SkipInstall,
    [switch]$KeepExistingServer,
    [switch]$KeepServer
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$Root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$ExtensionRoot = Join-Path $Root "apps\browser-extension"
$LogRoot = Join-Path $Root "var\run-extension"
$ServerHost = "127.0.0.1"
$ApiBaseUrl = "http://${ServerHost}:${ServerPort}"
$HealthUrl = "${ApiBaseUrl}/api/v1/health"
$serverProcess = $null
$startedServer = $false

function Write-Info([string]$Message) { Write-Host "[run-extension] $Message" -ForegroundColor Cyan }
function Write-Ok([string]$Message) { Write-Host "[run-extension] OK: $Message" -ForegroundColor Green }
function Write-WarnMsg([string]$Message) { Write-Host "[run-extension] WARN: $Message" -ForegroundColor Yellow }

function Resolve-Executable([string]$Name) {
    $command = Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $command) { throw "$Name was not found in PATH." }
    return $command.Source
}

function Get-OptionalExecutable([string]$Name) {
    $command = Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $command) { return $null }
    return $command.Source
}

function Invoke-WslCommand([string]$Command) {
    $wslExe = Get-OptionalExecutable "wsl.exe"
    if (-not $wslExe) { return $false }
    & $wslExe sh -lc $Command | Out-Null
    return $LASTEXITCODE -eq 0
}

function Test-PostgresUrl([string]$Url) {
    $psqlExe = Get-OptionalExecutable "psql.exe"
    if (-not $psqlExe) {
        return $null
    }

    & $psqlExe $Url -Atc "select 1" *> $null
    return $LASTEXITCODE -eq 0
}

function Wait-Database([string]$Url, [int]$Attempts = 30) {
    $canUsePsql = $null -ne (Get-OptionalExecutable "psql.exe")
    if (-not $canUsePsql) {
        Write-WarnMsg "psql.exe was not found; skipping DATABASE_URL preflight."
        return
    }

    for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
        if (Test-PostgresUrl $Url) {
            Write-Ok "database reachable"
            return
        }
        Start-Sleep -Milliseconds 700
    }

    throw "Database is not reachable via DATABASE_URL. Start PostgreSQL in WSL or pass -DatabaseUrl explicitly. Tried: $Url"
}

function Resolve-DatabaseUrl([string]$CandidateUrl) {
    if (-not [string]::IsNullOrWhiteSpace($CandidateUrl)) {
        return $CandidateUrl
    }

    $wslExe = Get-OptionalExecutable "wsl.exe"
    if ($wslExe) {
        Write-Info "Checking WSL PostgreSQL for docdb..."
        $ready = Invoke-WslCommand "pg_isready -h 127.0.0.1 -p 5432 -d docdb >/dev/null 2>&1"
        if (-not $ready) {
            Write-WarnMsg "WSL PostgreSQL is not ready; trying to start postgresql service."
            Invoke-WslCommand "sudo -n service postgresql start >/dev/null 2>&1 || service postgresql start >/dev/null 2>&1" | Out-Null
        }

        $wslIps = @(& $wslExe sh -lc "hostname -I" 2>$null)
        foreach ($ip in ($wslIps -join " ").Split(" ", [System.StringSplitOptions]::RemoveEmptyEntries)) {
            $wslUrl = "postgres://postgres:postgres@${ip}:5432/docdb"
            if (Test-PostgresUrl $wslUrl) {
                Write-Ok "using WSL PostgreSQL at ${ip}:5432"
                return $wslUrl
            }
        }
    }

    return "postgres://postgres:postgres@127.0.0.1:5432/docdb"
}

function Get-ListenerProcessIds([int]$Port) {
    try {
        return @(Get-NetTCPConnection -LocalAddress $ServerHost -LocalPort $Port -State Listen -ErrorAction Stop |
            Select-Object -ExpandProperty OwningProcess -Unique)
    } catch {
        return @()
    }
}

function Stop-ProcessTree([int]$ProcessId) {
    $children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId=$ProcessId" -ErrorAction SilentlyContinue)
    foreach ($child in $children) {
        Stop-ProcessTree $child.ProcessId
    }
    Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
}

function Wait-Server([string]$Url, [int]$Attempts = 90) {
    for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
        if ($serverProcess -and $serverProcess.HasExited) {
            throw "doc-server exited with code $($serverProcess.ExitCode). See logs under $LogRoot."
        }
        try {
            $response = Invoke-RestMethod -Uri $Url -TimeoutSec 2
            if ($response.status -eq "ok") {
                Write-Ok "doc-server healthy at $Url"
                return
            }
        } catch {
            Start-Sleep -Milliseconds 700
        }
    }
    throw "doc-server did not become healthy at $Url. See logs under $LogRoot."
}

if (-not (Test-Path -LiteralPath $ExtensionRoot -PathType Container)) {
    throw "Browser extension directory not found: $ExtensionRoot"
}

$cargoExe = Resolve-Executable "cargo.exe"
$npmExe = Resolve-Executable "npm.cmd"
$ResolvedDatabaseUrl = Resolve-DatabaseUrl $DatabaseUrl
New-Item -ItemType Directory -Path $LogRoot -Force | Out-Null

$buildScript = if ($Browser -eq "edge") { "build:edge" } else { "build:$Browser" }
$devScript = if ($Browser -eq "edge") { "dev:edge" } else { "dev:$Browser" }

try {
    if (-not $SkipInstall) {
        Write-Info "Installing extension dependencies with npm ci..."
        & $npmExe ci --prefix $ExtensionRoot
        if ($LASTEXITCODE -ne 0) { throw "npm ci failed with exit code $LASTEXITCODE." }
    }

    Write-Info "Compiling doc-server..."
    & $cargoExe build -p doc-server --manifest-path (Join-Path $Root "Cargo.toml")
    if ($LASTEXITCODE -ne 0) { throw "doc-server compilation failed with exit code $LASTEXITCODE." }

    Write-Info "Rebuilding the $Browser extension against $ApiBaseUrl..."
    $oldApiBaseUrl = $env:VITE_API_BASE_URL
    try {
        $env:VITE_API_BASE_URL = $ApiBaseUrl
        & $npmExe run $buildScript --prefix $ExtensionRoot
        if ($LASTEXITCODE -ne 0) { throw "Extension build failed with exit code $LASTEXITCODE." }
    } finally {
        if ($null -eq $oldApiBaseUrl) { Remove-Item Env:\VITE_API_BASE_URL -ErrorAction SilentlyContinue }
        else { $env:VITE_API_BASE_URL = $oldApiBaseUrl }
    }

    $listeners = @(Get-ListenerProcessIds $ServerPort)
    if ($listeners.Count -gt 0) {
        if (-not $KeepExistingServer) {
            throw "Port ${ServerHost}:${ServerPort} is already in use by PID(s): $($listeners -join ', '). Use -KeepExistingServer to reuse it."
        }
        Write-WarnMsg "Reusing doc-server listener PID(s): $($listeners -join ', ')"
    } else {
        $serverOut = Join-Path $LogRoot "doc-server.out.log"
        $serverErr = Join-Path $LogRoot "doc-server.err.log"
        $oldServerAddr = $env:DOC_SERVER_ADDR
        $oldDatabaseUrl = $env:DATABASE_URL
        try {
            $env:DOC_SERVER_ADDR = "${ServerHost}:${ServerPort}"
            $env:DATABASE_URL = $ResolvedDatabaseUrl
            Wait-Database $ResolvedDatabaseUrl
            Write-Info "Starting doc-server on ${ServerHost}:${ServerPort}..."
            $serverProcess = Start-Process -FilePath $cargoExe `
                -ArgumentList @("run", "-p", "doc-server") `
                -WorkingDirectory $Root `
                -RedirectStandardOutput $serverOut `
                -RedirectStandardError $serverErr `
                -WindowStyle Hidden `
                -PassThru
            $startedServer = $true
        } finally {
            if ($null -eq $oldServerAddr) { Remove-Item Env:\DOC_SERVER_ADDR -ErrorAction SilentlyContinue }
            else { $env:DOC_SERVER_ADDR = $oldServerAddr }
            if ($null -eq $oldDatabaseUrl) { Remove-Item Env:\DATABASE_URL -ErrorAction SilentlyContinue }
            else { $env:DATABASE_URL = $oldDatabaseUrl }
        }
        Write-Info "doc-server PID $($serverProcess.Id); logs: $serverOut / $serverErr"
    }

    Wait-Server $HealthUrl
    Write-Ok "Build complete. Starting WXT $Browser development mode."
    Write-Host "  API:       $ApiBaseUrl"
    Write-Host "  Extension: $ExtensionRoot"
    Write-Host "  Stop both processes with Ctrl+C."

    $oldApiBaseUrl = $env:VITE_API_BASE_URL
    try {
        $env:VITE_API_BASE_URL = $ApiBaseUrl
        Push-Location $ExtensionRoot
        try {
            & $npmExe run $devScript
            if ($LASTEXITCODE -ne 0) { throw "Extension dev process exited with code $LASTEXITCODE." }
        } finally {
            Pop-Location
        }
    } finally {
        if ($null -eq $oldApiBaseUrl) { Remove-Item Env:\VITE_API_BASE_URL -ErrorAction SilentlyContinue }
        else { $env:VITE_API_BASE_URL = $oldApiBaseUrl }
    }
} finally {
    if ($startedServer -and -not $KeepServer -and $serverProcess -and -not $serverProcess.HasExited) {
        Write-Info "Stopping doc-server PID $($serverProcess.Id)..."
        Stop-ProcessTree $serverProcess.Id
    } elseif ($startedServer -and $KeepServer -and $serverProcess -and -not $serverProcess.HasExited) {
        Write-WarnMsg "Leaving doc-server running at $ApiBaseUrl (PID $($serverProcess.Id))."
    }
}
