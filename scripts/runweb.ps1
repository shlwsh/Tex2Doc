<#
.SYNOPSIS
Start the Tex2Doc Flutter Web app.

.DESCRIPTION
Launches flutter_app with Flutter's web target. The default web-server device
prints a local URL that can be opened from any browser.

.PARAMETER Port
Web server port. Default: 2625.

.PARAMETER HostAddress
Host address for the web-server target. Default: 127.0.0.1.

.PARAMETER Device
Flutter web device: web-server, chrome, or edge. Default: web-server.

.PARAMETER Release
Run the app in release mode.

.EXAMPLE
.\scripts\runweb.ps1
.EXAMPLE
.\scripts\runweb.ps1 -Device chrome -Port 2625
#>

param(
    [int]$Port = 2625,
    [string]$HostAddress = "127.0.0.1",
    [ValidateSet("web-server", "chrome", "edge")]
    [string]$Device = "web-server",
    [switch]$Release
)

$ErrorActionPreference = "Stop"

function Write-Info($Message) { Write-Host "[runweb] $Message" -ForegroundColor Cyan }
function Write-Err($Message) { Write-Host "[runweb] ERROR: $Message" -ForegroundColor Red }

function Assert-FlutterInstalled() {
    $flutter = Get-Command flutter -ErrorAction SilentlyContinue
    if (-not $flutter) {
        Write-Err "'flutter' not found in PATH. Install Flutter SDK and retry."
        exit 1
    }
    Write-Info "Flutter: $($flutter.Source)"
}

$repoRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::Combine($PSScriptRoot, ".."))
$appDir = Join-Path $repoRoot "flutter_app"
$pubspec = Join-Path $appDir "pubspec.yaml"

if (-not (Test-Path $pubspec)) {
    Write-Err "Flutter app not found at: $appDir"
    exit 1
}

Assert-FlutterInstalled

$flutterArgs = @("run", "-d", $Device)
if ($Device -eq "web-server") {
    $flutterArgs += @("--web-hostname", $HostAddress, "--web-port", $Port)
} else {
    $flutterArgs += @("--web-port", $Port)
}
if ($Release) {
    $flutterArgs += "--release"
}

Write-Info "Working directory: $appDir"
Write-Info "Starting: flutter $($flutterArgs -join ' ')"
if ($Device -eq "web-server") {
    Write-Info "URL: http://$HostAddress`:$Port/"
}
Write-Info "Press Ctrl+C to stop. Inside flutter run: r=hot reload, R=hot restart, q=quit."
Write-Info ""

Push-Location $appDir
try {
    & flutter @flutterArgs
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
