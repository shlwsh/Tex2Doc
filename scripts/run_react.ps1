<#
.SYNOPSIS
    Start Tex2Doc React Web and the Rust doc-server together.

.DESCRIPTION
    Starts the Rust API service on 127.0.0.1:2624 and the Vite React frontend
    on 127.0.0.1:5174. The React Vite config proxies /v1, /api and /admin/v1
    to the Rust service, so the browser can use same-origin API URLs.

.PARAMETER ReactPort
    Vite dev server port. Default: 2630.

.PARAMETER ServerPort
    Rust doc-server port. Default: 2624.

.PARAMETER KeepExisting
    Reuse existing listeners on the target ports instead of stopping them.

.PARAMETER NoBrowser
    Do not open the React URL after both services are healthy.

.PARAMETER DatabaseUrl
    PostgreSQL connection string for doc-server. Defaults to DATABASE_URL
    from the current environment; if unset, doc-server uses its built-in
    default postgres://postgres:postgres@127.0.0.1:5432/docdb.

.EXAMPLE
    .\scripts\run_react.ps1

.EXAMPLE
    .\scripts\run_react.ps1 -DatabaseUrl "postgres://postgres:postgres@127.0.0.1:5432/docdb"
#>

param(
    [int]$ReactPort = 2630,
    [int]$ServerPort = 2624,
    [string]$DatabaseUrl = $env:DATABASE_URL,
    [switch]$KeepExisting,
    [switch]$NoBrowser
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$Root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$ReactRoot = Join-Path $Root "apps\react-web"
$LogRoot = Join-Path $Root "var\run-react"
$ServerHost = "127.0.0.1"
$ServerHealthUrl = "http://${ServerHost}:${ServerPort}/api/v1/health"
$ReactUrl = "http://${ServerHost}:${ReactPort}/react"
$ProxyHealthUrl = "http://${ServerHost}:${ReactPort}/api/v1/health"
$DocServerProcess = $null

function Write-Info($Message) { Write-Host "[run-react] $Message" -ForegroundColor Cyan }
function Write-Ok($Message) { Write-Host "[run-react] OK: $Message" -ForegroundColor Green }
function Write-WarnMsg($Message) { Write-Host "[run-react] WARN: $Message" -ForegroundColor Yellow }

function Resolve-CommandPath($Name) {
    $cmd = Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $cmd) {
        throw "$Name was not found in PATH."
    }
    return $cmd.Source
}

function Get-PortProcessIds([int]$Port) {
    try {
        $connections = Get-NetTCPConnection -LocalAddress $ServerHost -LocalPort $Port -State Listen -ErrorAction Stop
        return @($connections | Select-Object -ExpandProperty OwningProcess -Unique)
    } catch {
        return @()
    }
}

function Test-PortOpen([int]$Port) {
    try {
        $client = [System.Net.Sockets.TcpClient]::new()
        $connect = $client.BeginConnect($ServerHost, $Port, $null, $null)
        $success = $connect.AsyncWaitHandle.WaitOne(500)
        if ($success) {
            $client.EndConnect($connect)
        }
        $client.Close()
        return $success
    } catch {
        return $false
    }
}

function Clear-Port([int]$Port) {
    $processIds = @(Get-PortProcessIds $Port)
    if ($processIds.Count -eq 0) {
        return
    }

    if ($KeepExisting) {
        Write-WarnMsg "Reusing existing listener on ${ServerHost}:${Port}, PID(s): $($processIds -join ', ')"
        return
    }

    Write-WarnMsg "Stopping existing listener on ${ServerHost}:${Port}, PID(s): $($processIds -join ', ')"
    foreach ($processId in $processIds) {
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
    }

    for ($attempt = 0; $attempt -lt 20; $attempt++) {
        if (-not (Test-PortOpen $Port)) {
            return
        }
        Start-Sleep -Milliseconds 250
    }

    throw "Port ${ServerHost}:${Port} is still occupied."
}

function Stop-ReactWorkspaceProcesses {
    if ($KeepExisting) {
        return
    }

    $normalizedReactRoot = $ReactRoot.ToLowerInvariant()
    $candidates = Get-CimInstance Win32_Process |
        Where-Object {
            $rawCommandLine = $_.CommandLine
            if ($null -eq $rawCommandLine) {
                $rawCommandLine = ""
            }
            $cmd = $rawCommandLine.ToLowerInvariant()
            ($cmd.Contains($normalizedReactRoot) -or
             ($cmd.Contains("vite") -and $cmd.Contains("react-web"))) -and
            $_.ProcessId -ne $PID
        }

    foreach ($candidate in $candidates) {
        Write-WarnMsg "Stopping previous React dev process PID $($candidate.ProcessId)"
        Stop-Process -Id $candidate.ProcessId -Force -ErrorAction SilentlyContinue
    }
}

function Wait-JsonHealth([string]$Url, [string]$Name, [int]$Attempts = 60) {
    for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
        if ($script:DocServerProcess -and $script:DocServerProcess.HasExited) {
            throw "doc-server exited with code $($script:DocServerProcess.ExitCode). See logs under $LogRoot."
        }
        try {
            $response = Invoke-RestMethod -Uri $Url -TimeoutSec 2
            if ($response.status -eq "ok") {
                Write-Ok "$Name healthy at $Url"
                return
            }
        } catch {
            Start-Sleep -Milliseconds 800
        }
    }
    throw "$Name did not become healthy at $Url"
}

function Wait-HttpOk([string]$Url, [string]$Name, [int]$Attempts = 45) {
    for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
        try {
            $response = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 2
            if ($response.StatusCode -ge 200 -and $response.StatusCode -lt 400) {
                Write-Ok "$Name available at $Url"
                return
            }
        } catch {
            Start-Sleep -Milliseconds 700
        }
    }
    throw "$Name did not become available at $Url"
}

if (-not (Test-Path -LiteralPath $ReactRoot -PathType Container)) {
    throw "React app not found: $ReactRoot"
}

New-Item -ItemType Directory -Path $LogRoot -Force | Out-Null

Stop-ReactWorkspaceProcesses
Clear-Port $ServerPort
Clear-Port $ReactPort

$cargoExe = Resolve-CommandPath "cargo"
$npmExe = Resolve-CommandPath "npm.cmd"

if (-not (Test-PortOpen $ServerPort)) {
    Write-Info "Starting doc-server on ${ServerHost}:${ServerPort}..."
    $serverOut = Join-Path $LogRoot "doc-server.out.log"
    $serverErr = Join-Path $LogRoot "doc-server.err.log"
    $oldDocServerAddr = $env:DOC_SERVER_ADDR
    $oldDatabaseUrl = $env:DATABASE_URL
    $oldAdminEmail = $env:TEX2DOC_BOOTSTRAP_ADMIN_EMAIL
    $oldAdminPassword = $env:TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD
    try {
        $env:DOC_SERVER_ADDR = "${ServerHost}:${ServerPort}"
        if (-not [string]::IsNullOrWhiteSpace($DatabaseUrl)) { $env:DATABASE_URL = $DatabaseUrl }
        $env:TEX2DOC_BOOTSTRAP_ADMIN_EMAIL = "demo@example.com"
        $env:TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD = "demo"
        $DocServerProcess = Start-Process -FilePath $cargoExe `
            -ArgumentList @("run", "-p", "doc-server") `
            -WorkingDirectory $Root `
            -RedirectStandardOutput $serverOut `
            -RedirectStandardError $serverErr `
            -WindowStyle Hidden `
            -PassThru
    } finally {
        if ($null -eq $oldDocServerAddr) { Remove-Item Env:\DOC_SERVER_ADDR -ErrorAction SilentlyContinue } else { $env:DOC_SERVER_ADDR = $oldDocServerAddr }
        if ($null -eq $oldDatabaseUrl) { Remove-Item Env:\DATABASE_URL -ErrorAction SilentlyContinue } else { $env:DATABASE_URL = $oldDatabaseUrl }
        if ($null -eq $oldAdminEmail) { Remove-Item Env:\TEX2DOC_BOOTSTRAP_ADMIN_EMAIL -ErrorAction SilentlyContinue } else { $env:TEX2DOC_BOOTSTRAP_ADMIN_EMAIL = $oldAdminEmail }
        if ($null -eq $oldAdminPassword) { Remove-Item Env:\TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD -ErrorAction SilentlyContinue } else { $env:TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD = $oldAdminPassword }
    }
    Write-Info "doc-server PID $($DocServerProcess.Id), logs: $serverOut / $serverErr"
}

Wait-JsonHealth $ServerHealthUrl "doc-server"

if (-not (Test-PortOpen $ReactPort)) {
    Write-Info "Starting React Vite dev server on ${ServerHost}:${ReactPort}..."
    $reactOut = Join-Path $LogRoot "react.out.log"
    $reactErr = Join-Path $LogRoot "react.err.log"
    $oldReactTarget = $env:TEX2DOC_REACT_API_TARGET
    try {
        $env:TEX2DOC_REACT_API_TARGET = "http://${ServerHost}:${ServerPort}"
        $reactProcess = Start-Process -FilePath $npmExe `
            -ArgumentList @("run", "dev", "--", "--host", $ServerHost, "--port", "$ReactPort") `
            -WorkingDirectory $ReactRoot `
            -RedirectStandardOutput $reactOut `
            -RedirectStandardError $reactErr `
            -WindowStyle Hidden `
            -PassThru
    } finally {
        if ($null -eq $oldReactTarget) { Remove-Item Env:\TEX2DOC_REACT_API_TARGET -ErrorAction SilentlyContinue } else { $env:TEX2DOC_REACT_API_TARGET = $oldReactTarget }
    }
    Write-Info "React PID $($reactProcess.Id), logs: $reactOut / $reactErr"
}

Wait-HttpOk $ReactUrl "React frontend"
Wait-JsonHealth $ProxyHealthUrl "Vite API proxy"

Write-Ok "React + Rust service are ready."
Write-Host "  React:      $ReactUrl"
Write-Host "  Admin:      http://${ServerHost}:${ReactPort}/admin-react"
Write-Host "  Rust API:   $ServerHealthUrl"
Write-Host "  Proxy API:  $ProxyHealthUrl"

if (-not $NoBrowser) {
    Start-Process $ReactUrl | Out-Null
}
