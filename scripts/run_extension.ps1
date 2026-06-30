<#
.SYNOPSIS
    Rebuild and run the Tex2Doc browser extension together with doc-server and React Web.

.EXAMPLE
    .\scripts\run_extension.ps1

.EXAMPLE
    .\scripts\run_extension.ps1 -Browser firefox -ServerPort 2625 -DatabaseUrl "postgres://postgres:postgres@127.0.0.1:5432/docdb"
#>

param(
    [ValidateSet("chrome", "edge", "firefox", "safari")]
    [string]$Browser = "chrome",
    [int]$ServerPort = 2624,
    [int]$ReactPort = 2630,
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
$ReactRoot = Join-Path $Root "apps\react-web"
$ReactUrl = "http://${ServerHost}:${ReactPort}/react"
$ProxyHealthUrl = "http://${ServerHost}:${ReactPort}/api/v1/health"
$serverProcess = $null
$startedServer = $false
$reactProcess = $null
$startedReact = $false
$wslKeepAliveProcess = $null

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

function Resolve-NpmCliPath([string]$NpmCommandPath) {
    $npmCli = Join-Path (Split-Path -Parent $NpmCommandPath) "node_modules\npm\bin\npm-cli.js"
    if (-not (Test-Path -LiteralPath $npmCli -PathType Leaf)) {
        throw "npm CLI script was not found: $npmCli"
    }
    return $npmCli
}

function Invoke-WslCommand([string]$Command) {
    $wslExe = Get-OptionalExecutable "wsl.exe"
    if (-not $wslExe) { return $false }
    & $wslExe sh -lc $Command | Out-Null
    return $LASTEXITCODE -eq 0
}

function Start-WslKeepAlive {
    if ($script:wslKeepAliveProcess -and -not $script:wslKeepAliveProcess.HasExited) {
        return
    }

    $wslExe = Get-OptionalExecutable "wsl.exe"
    if (-not $wslExe) { return }

    $script:wslKeepAliveProcess = Start-Process -FilePath $wslExe `
        -ArgumentList @("sh", "-lc", "while true; do sleep 3600; done") `
        -WindowStyle Hidden `
        -PassThru
    Write-Info "Keeping WSL alive for PostgreSQL (PID $($script:wslKeepAliveProcess.Id))."
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
        Start-WslKeepAlive

        $localhostUrl = "postgres://postgres:postgres@127.0.0.1:5432/docdb"
        if (Test-PostgresUrl $localhostUrl) {
            Write-Ok "using WSL PostgreSQL via localhost:5432"
            return $localhostUrl
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

function Stop-ProcessTree([int]$ProcessId) {
    $children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId=$ProcessId" -ErrorAction SilentlyContinue)
    foreach ($child in $children) {
        Stop-ProcessTree $child.ProcessId
    }
    Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
}

function Stop-ExtensionWorkspaceProcesses {
    $normalizedExtensionRoot = $ExtensionRoot.ToLowerInvariant()
    $normalizedExtensionNodeModules = (Join-Path $ExtensionRoot "node_modules").ToLowerInvariant()
    $candidates = @(Get-CimInstance Win32_Process |
        Where-Object {
            $rawCommandLine = $_.CommandLine
            if ($null -eq $rawCommandLine) {
                $rawCommandLine = ""
            }
            $cmd = $rawCommandLine.ToLowerInvariant()
            $isNodeTool = $_.Name -match '^(node|npm|npm.cmd|npx|npx.cmd|wxt)(\.exe)?$'
            $isExtensionProcess = $cmd.Contains($normalizedExtensionRoot) -or
                $cmd.Contains($normalizedExtensionNodeModules) -or
                ($cmd.Contains("wxt") -and $cmd.Contains("browser-extension"))
            $isNodeTool -and $isExtensionProcess -and $_.ProcessId -ne $PID
        })

    foreach ($candidate in $candidates) {
        Write-WarnMsg "Stopping previous browser-extension dev process PID $($candidate.ProcessId)"
        Stop-ProcessTree $candidate.ProcessId
    }
}

function Clear-Port([int]$Port, [string]$Name) {
    $listeners = @(Get-ListenerProcessIds $Port)
    if ($listeners.Count -eq 0) {
        return $false
    }

    if ($KeepExistingServer) {
        Write-WarnMsg "Reusing existing $Name listener on ${ServerHost}:${Port}, PID(s): $($listeners -join ', ')"
        return $true
    }

    Write-WarnMsg "Stopping existing $Name listener on ${ServerHost}:${Port}, PID(s): $($listeners -join ', ')"
    foreach ($processId in $listeners) {
        Stop-ProcessTree $processId
    }

    for ($attempt = 1; $attempt -le 20; $attempt++) {
        if (-not (Test-PortOpen $Port)) {
            return $false
        }
        Start-Sleep -Milliseconds 250
    }

    throw "Port ${ServerHost}:${Port} is still occupied after stopping PID(s): $($listeners -join ', ')."
}

function ConvertTo-ProcessArguments([string[]]$Arguments) {
    return ($Arguments | ForEach-Object {
        if ($null -eq $_) {
            '""'
        } elseif ($_ -match '[\s"]') {
            $escaped = ($_ -replace '(\\*)"', '$1$1\"') -replace '(\\+)$', '$1$1'
            '"' + $escaped + '"'
        } else {
            $_
        }
    }) -join " "
}

function Start-LoggedProcess(
    [string]$Name,
    [string]$FilePath,
    [string[]]$ArgumentList,
    [string]$WorkingDirectory,
    [string]$StdoutLog,
    [string]$StderrLog,
    [hashtable]$Environment = @{}
) {
    New-Item -ItemType Directory -Path (Split-Path -Parent $StdoutLog) -Force | Out-Null
    "" | Set-Content -LiteralPath $StdoutLog
    "" | Set-Content -LiteralPath $StderrLog

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FilePath
    if ($startInfo.PSObject.Properties.Name -contains "ArgumentList") {
        foreach ($arg in $ArgumentList) {
            [void]$startInfo.ArgumentList.Add($arg)
        }
    } else {
        $startInfo.Arguments = ConvertTo-ProcessArguments $ArgumentList
    }
    $startInfo.WorkingDirectory = $WorkingDirectory
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.CreateNoWindow = $true
    foreach ($key in $Environment.Keys) {
        $startInfo.Environment[$key] = [string]$Environment[$key]
    }

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    $process.EnableRaisingEvents = $true

    $stdoutHandler = [System.Diagnostics.DataReceivedEventHandler]{
        param($sender, $eventArgs)
        if ($null -ne $eventArgs.Data) {
            Add-Content -LiteralPath $StdoutLog -Value $eventArgs.Data
            Write-Host "[$Name] $($eventArgs.Data)"
        }
    }.GetNewClosure()
    $stderrHandler = [System.Diagnostics.DataReceivedEventHandler]{
        param($sender, $eventArgs)
        if ($null -ne $eventArgs.Data) {
            Add-Content -LiteralPath $StderrLog -Value $eventArgs.Data
            Write-Host "[${Name}:err] $($eventArgs.Data)" -ForegroundColor DarkYellow
        }
    }.GetNewClosure()

    $process.add_OutputDataReceived($stdoutHandler)
    $process.add_ErrorDataReceived($stderrHandler)
    if (-not $process.Start()) {
        throw "Failed to start $Name."
    }
    $process.BeginOutputReadLine()
    $process.BeginErrorReadLine()
    return $process
}

function Wait-Server([string]$Url, [string]$Name = "doc-server", [int]$Attempts = 90) {
    for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
        if ($serverProcess -and $serverProcess.HasExited) {
            throw "doc-server exited with code $($serverProcess.ExitCode). See logs under $LogRoot."
        }
        if ($reactProcess -and $reactProcess.HasExited) {
            throw "React Vite exited with code $($reactProcess.ExitCode). See logs under $LogRoot."
        }
        try {
            $response = Invoke-RestMethod -Uri $Url -TimeoutSec 2
            if ($response.status -eq "ok") {
                Write-Ok "$Name healthy at $Url"
                return
            }
        } catch {
            Start-Sleep -Milliseconds 700
        }
    }
    throw "$Name did not become healthy at $Url. See logs under $LogRoot."
}

function Wait-HttpOk([string]$Url, [string]$Name, [int]$Attempts = 45) {
    for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
        if ($reactProcess -and $reactProcess.HasExited) {
            throw "$Name exited with code $($reactProcess.ExitCode). See logs under $LogRoot."
        }
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
    throw "$Name did not become available at $Url. See logs under $LogRoot."
}

if (-not (Test-Path -LiteralPath $ExtensionRoot -PathType Container)) {
    throw "Browser extension directory not found: $ExtensionRoot"
}
if (-not (Test-Path -LiteralPath $ReactRoot -PathType Container)) {
    throw "React app directory not found: $ReactRoot"
}

$cargoExe = Resolve-Executable "cargo.exe"
$npmExe = Resolve-Executable "npm.cmd"
$nodeExe = Resolve-Executable "node.exe"
$npmCli = Resolve-NpmCliPath $npmExe
$ResolvedDatabaseUrl = Resolve-DatabaseUrl $DatabaseUrl
New-Item -ItemType Directory -Path $LogRoot -Force | Out-Null

$buildScript = if ($Browser -eq "edge") { "build:edge" } else { "build:$Browser" }
$devScript = if ($Browser -eq "edge") { "dev:edge" } else { "dev:$Browser" }

try {
    Stop-ExtensionWorkspaceProcesses

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

    $reuseServer = Clear-Port $ServerPort "doc-server"
    $reuseReact = Clear-Port $ReactPort "React Vite"

    if (-not $reuseServer) {
        $serverOut = Join-Path $LogRoot "doc-server.out.log"
        $serverErr = Join-Path $LogRoot "doc-server.err.log"
        Wait-Database $ResolvedDatabaseUrl
        Write-Info "Starting doc-server on ${ServerHost}:${ServerPort}..."
        $serverProcess = Start-LoggedProcess `
            -Name "doc-server" `
            -FilePath $cargoExe `
            -ArgumentList @("run", "-p", "doc-server") `
            -WorkingDirectory $Root `
            -StdoutLog $serverOut `
            -StderrLog $serverErr `
            -Environment @{
                DOC_SERVER_ADDR = "${ServerHost}:${ServerPort}"
                DATABASE_URL = $ResolvedDatabaseUrl
                TEX2DOC_LOG_TO_STDOUT = "true"
            }
        $startedServer = $true
        Write-Info "doc-server PID $($serverProcess.Id); console logs enabled; files: $serverOut / $serverErr"
    }

    Wait-Server $HealthUrl "doc-server"

    if (-not $reuseReact) {
        $reactOut = Join-Path $LogRoot "react.out.log"
        $reactErr = Join-Path $LogRoot "react.err.log"
        Write-Info "Starting React Vite dev server on ${ServerHost}:${ReactPort}..."
        $reactProcess = Start-LoggedProcess `
            -Name "react-web" `
            -FilePath $nodeExe `
            -ArgumentList @($npmCli, "run", "dev", "--", "--host", $ServerHost, "--port", "$ReactPort") `
            -WorkingDirectory $ReactRoot `
            -StdoutLog $reactOut `
            -StderrLog $reactErr `
            -Environment @{
                TEX2DOC_REACT_API_TARGET = $ApiBaseUrl
            }
        $startedReact = $true
        Write-Info "React Vite PID $($reactProcess.Id); console logs enabled; files: $reactOut / $reactErr"
    }

    Wait-HttpOk $ReactUrl "React frontend"
    Wait-Server $ProxyHealthUrl "Vite API proxy"

    Write-Ok "Build complete. Starting WXT $Browser development mode."
    Write-Host "  API:       $ApiBaseUrl"
    Write-Host "  React:     $ReactUrl"
    Write-Host "  React API: $ProxyHealthUrl"
    Write-Host "  Extension: $ExtensionRoot"
    Write-Host "  Stop all processes with Ctrl+C."

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
    if ($startedReact -and $reactProcess -and -not $reactProcess.HasExited) {
        Write-Info "Stopping React Vite PID $($reactProcess.Id)..."
        Stop-ProcessTree $reactProcess.Id
    }
    if ($startedServer -and -not $KeepServer -and $serverProcess -and -not $serverProcess.HasExited) {
        Write-Info "Stopping doc-server PID $($serverProcess.Id)..."
        Stop-ProcessTree $serverProcess.Id
    } elseif ($startedServer -and $KeepServer -and $serverProcess -and -not $serverProcess.HasExited) {
        Write-WarnMsg "Leaving doc-server running at $ApiBaseUrl (PID $($serverProcess.Id))."
    }
    if ($wslKeepAliveProcess -and -not $wslKeepAliveProcess.HasExited) {
        Write-Info "Stopping WSL keepalive PID $($wslKeepAliveProcess.Id)..."
        Stop-ProcessTree $wslKeepAliveProcess.Id
    }
}
