<#
.SYNOPSIS
Start Flutter Web dev server with hot reload, auto-clearing port conflicts.

.DESCRIPTION
Checks whether the target port (default 4173) is already in use. If so, force-kills
the occupying process and retries until the port is free, then launches:
    flutter run -d chrome --web-port <port>

.PARAMETER Port
Web server port. Default: 4173.

.EXAMPLE
.\scripts\run_flutter_webservice.ps1
.EXAMPLE
.\scripts\run_flutter_webservice.ps1 -Port 8080
#>

param(
    [int]$Port = 4173,
    [int]$RetrySeconds = 10,
    [int]$RetryIntervalSec = 1
)

$ErrorActionPreference = 'Stop'

function Write-Info($msg) { Write-Host "[run-flutter-web] $msg" -ForegroundColor Cyan }
function Write-Warn($msg) { Write-Host "[run-flutter-web] WARN: $msg" -ForegroundColor Yellow }
function Write-Err($msg)  { Write-Host "[run-flutter-web] ERROR: $msg" -ForegroundColor Red }

function Get-PortPid([int]$port) {
    $lines = netstat -ano 2>&1 | Select-String -Pattern ":${port}\s+.*LISTENING"
    $pids = @()
    foreach ($line in $lines) {
        if ($line -match '\s+(\d+)\s*$') {
            $pids += [long]$matches[1]
        }
    }
    return ($pids | Sort-Object -Unique)
}

function Clear-Port([int]$port) {
    $pids = Get-PortPid $port
    if (-not $pids) { return $true }

    foreach ($targetPid in $pids) {
        try {
            $procName = (Get-Process -Id $targetPid -ErrorAction SilentlyContinue).ProcessName
            Write-Warn "Port $port occupied by PID $targetPid ($procName). Force-killing..."
            taskkill /F /PID $targetPid | Out-Null
        } catch {
            Write-Warn "taskkill /F /PID $targetPid failed: $_"
        }
    }

    Start-Sleep -Milliseconds 500
    $pids = Get-PortPid $port
    if ($pids) {
        Write-Err "Port $port still occupied after kill: $($pids -join ', ')"
        return $false
    }

    Write-Info "Port $port cleared."
    return $true
}

function Assert-FlutterInstalled() {
    $flutter = Get-Command flutter -ErrorAction SilentlyContinue
    if (-not $flutter) {
        Write-Err "'flutter' not found in PATH. Install Flutter SDK and retry."
        exit 1
    }
    Write-Info "Flutter: $($flutter.Source)"
}

function Assert-FlutterAppDir() {
    $appDir = [System.IO.Path]::GetFullPath([System.IO.Path]::Combine($PSScriptRoot, '..', 'flutter_app'))
    if (-not (Test-Path $appDir)) {
        Write-Err "flutter_app directory not found at: $appDir"
        exit 1
    }
    return $appDir
}

# ---------- Main ----------
Write-Info "Flutter Web dev service starter"
Write-Info "Target port: $Port"

$pids = Get-PortPid $Port
if ($pids) {
    Write-Warn "Port $Port is in use by PID(s): $($pids -join ', ')"
    if (-not (Clear-Port $Port)) {
        Write-Err "Failed to free port $Port. Exiting."
        exit 1
    }
} else {
    Write-Info "Port $Port is free."
}

Assert-FlutterInstalled | Out-Null
$appDir = Assert-FlutterAppDir
Set-Location $appDir

Write-Info "Working directory: $appDir"
Write-Info "Starting: flutter run -d chrome --web-port $Port"
Write-Info "Press Ctrl+C to stop. Inside flutter run: r=hot reload, R=hot restart, q=quit."
Write-Info ""

flutter run -d chrome --web-port $Port
