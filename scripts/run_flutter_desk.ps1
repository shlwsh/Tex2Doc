<#
.SYNOPSIS
    Build (if needed) and launch the Flutter Windows desktop application.

.DESCRIPTION
    Ensures the Rust doc-native crate is compiled, then builds the Flutter
    release for Windows and launches the resulting exe. If the build is
    up-to-date it skips recompilation and just runs the existing binary.

.PARAMETER SkipBuild
    Skip the build step entirely and run the existing build artefact.

.PARAMETER NoServer
    Skip starting the local doc-server backend before launching the app.

.PARAMETER ServerPort
    Local doc-server port. Default: 2624.

.EXAMPLE
    .\scripts\run_flutter_desk.ps1
.EXAMPLE
    .\scripts\run_flutter_desk.ps1 -SkipBuild
#>

param(
    [switch]$SkipBuild,

    [switch]$NoServer,

    [int]$ServerPort = 2624
)

$ErrorActionPreference = 'Stop'

$PROJECT_ROOT = [System.IO.Path]::GetFullPath([System.IO.Path]::Combine($PSScriptRoot, '..'))
$FLUTTER_APP  = [System.IO.Path]::GetFullPath([System.IO.Path]::Combine($PROJECT_ROOT, 'flutter_app'))
$EXE_DIR      = [System.IO.Path]::GetFullPath(
    [System.IO.Path]::Combine($FLUTTER_APP, 'build', 'windows', 'x64', 'runner', 'Release')
)
$EXE_PATH     = [System.IO.Path]::Combine($EXE_DIR, 'doc_engine.exe')
$SERVER_HOST  = '127.0.0.1'
$SERVER_PORT  = $ServerPort
$SERVER_HEALTH_URL = "http://${SERVER_HOST}:${SERVER_PORT}/api/v1/health"

function Write-Info($msg) { Write-Host "[run-flutter-desk] $msg" -ForegroundColor Cyan }
function Write-Warn($msg) { Write-Host "[run-flutter-desk] WARN: $msg" -ForegroundColor Yellow }
function Write-Err($msg)  { Write-Host "[run-flutter-desk] ERROR: $msg" -ForegroundColor Red }
function Write-Succ($msg) { Write-Host "[run-flutter-desk] OK:   $msg" -ForegroundColor Green }

function Assert-FlutterInstalled() {
    $flutter = Get-Command flutter -ErrorAction SilentlyContinue
    if (-not $flutter) {
        Write-Err "'flutter' not found in PATH. Install Flutter SDK and retry."
        exit 1
    }
    Write-Info "Flutter: $($flutter.Source)"
}

function Assert-CargoInstalled() {
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargo) {
        $defaultCargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
        if (Test-Path $defaultCargo) {
            $cargoBin = Split-Path $defaultCargo -Parent
            $env:Path = "$cargoBin;$env:Path"
            $cargo = Get-Command cargo -ErrorAction SilentlyContinue
        }
    }

    if (-not $cargo) {
        Write-Err "'cargo' not found in PATH. Install Rust toolchain or add '$env:USERPROFILE\.cargo\bin' to PATH, then retry."
        exit 1
    }
    Write-Info "Cargo:  $($cargo.Source)"
}

function Get-CargoCommand() {
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargo) {
        Write-Err "'cargo' not found in PATH. Install Rust toolchain or add '$env:USERPROFILE\.cargo\bin' to PATH, then retry."
        exit 1
    }
    return $cargo.Source
}

function Test-ServerPortOpen() {
    try {
        $client = [System.Net.Sockets.TcpClient]::new()
        $connect = $client.BeginConnect($SERVER_HOST, $SERVER_PORT, $null, $null)
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

function Get-PortProcessIds() {
    try {
        $connections = Get-NetTCPConnection -LocalAddress $SERVER_HOST -LocalPort $SERVER_PORT -State Listen -ErrorAction Stop
        return @($connections | Select-Object -ExpandProperty OwningProcess -Unique)
    } catch {
        return @()
    }
}

function Clear-ServerPort() {
    $processIds = Get-PortProcessIds
    if (-not $processIds -or $processIds.Count -eq 0) {
        return
    }

    Write-Warn "Clearing ${SERVER_HOST}:${SERVER_PORT}, stopping PID(s): $($processIds -join ', ')"
    foreach ($processId in $processIds) {
        try {
            Stop-Process -Id $processId -Force -ErrorAction Stop
        } catch {
            Write-Warn "Failed to stop PID ${processId}: $($_.Exception.Message)"
        }
    }

    for ($attempt = 0; $attempt -lt 20; $attempt++) {
        if (-not (Test-ServerPortOpen)) {
            return
        }
        Start-Sleep -Milliseconds 250
    }

    Write-Err "Port ${SERVER_HOST}:${SERVER_PORT} is still occupied after cleanup."
    exit 1
}

function Start-LocalServer() {
    Clear-ServerPort

    $cargoExe = Get-CargoCommand
    Write-Info "Starting doc-server on ${SERVER_HOST}:${SERVER_PORT}..."

    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $cargoExe
    $psi.Arguments = 'run -p doc-server'
    $psi.WorkingDirectory = $PROJECT_ROOT
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.Environment['DOC_SERVER_ADDR'] = "${SERVER_HOST}:${SERVER_PORT}"
    $process = [System.Diagnostics.Process]::Start($psi)

    for ($attempt = 0; $attempt -lt 45; $attempt++) {
        if ($process.HasExited) {
            Write-Err "doc-server exited immediately with code $($process.ExitCode)."
            exit 1
        }
        try {
            $response = Invoke-RestMethod -Uri $SERVER_HEALTH_URL -TimeoutSec 1
            if ($response.status -eq 'ok') {
                Write-Succ "doc-server ready (PID $($process.Id))"
                return
            }
        } catch {
            Start-Sleep -Milliseconds 700
        }
    }

    Write-Err "doc-server did not become healthy at $SERVER_HEALTH_URL."
    exit 1
}

function Build-RustCrate() {
    Write-Info "Building Rust crate 'doc-native'..."
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    Push-Location $PROJECT_ROOT
    try {
        # Use 6>&1 to merge stderr into stdout so everything arrives as
        # plain strings, avoiding PowerShell's ErrorRecord "RemoteException"
        # wrapper on stderr lines.
        $output = & cargo build -p doc-native 6>&1
        foreach ($line in $output) { Write-Host "        $line" }
        if ($LASTEXITCODE -ne 0) {
            Write-Err "'cargo build -p doc-native' failed."
            exit 1
        }
    } finally {
        Pop-Location
    }
    $sw.Stop()
    Write-Succ "Rust crate built in $($sw.Elapsed.TotalSeconds)s"
}

function Build-FlutterDesktop() {
    Write-Info "Building Flutter Windows desktop app (release)..."
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    Push-Location $FLUTTER_APP
    try {
        # CMake custom target inside the Windows build will trigger cargo
        # automatically; we run it explicitly above so the output is visible.
        $output = & flutter build windows --release 6>&1
        foreach ($line in $output) { Write-Host "        $line" }
        if ($LASTEXITCODE -ne 0) {
            Write-Err "'flutter build windows --release' failed."
            exit 1
        }
    } finally {
        Pop-Location
    }
    $sw.Stop()
    Write-Succ "Flutter build complete in $($sw.Elapsed.TotalSeconds)s"
    Write-Info "Output: $EXE_DIR"
}

function Launch-App() {
    if (-not (Test-Path $EXE_PATH)) {
        Write-Err "Executable not found: $EXE_PATH"
        Write-Info "Run without -SkipBuild to build first."
        exit 1
    }

    # Kill any existing instance so we get a clean slate
    $existing = Get-Process -Name 'doc_engine' -ErrorAction SilentlyContinue
    if ($existing) {
        Write-Warn "Existing doc_engine process detected (PID $($existing.Id)). Terminating..."
        Stop-Process -Name 'doc_engine' -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 500
    }

    Write-Info "Launching: $EXE_PATH"
    Start-Process -FilePath $EXE_PATH -WorkingDirectory $EXE_DIR

    # Poll for 5s to confirm it didn't crash immediately
    Start-Sleep -Seconds 5
    $live = Get-Process -Name 'doc_engine' -ErrorAction SilentlyContinue
    if (-not $live) {
        Write-Err "No doc_engine process found after launch. The app may have crashed on startup."
        exit 1
    }
    Write-Succ "App is running (WS: $([math]::Round($live.WorkingSet64 / 1MB, 1)) MB)"
    Write-Info "Window: '$($live.MainWindowTitle)'  Responding: $($live.Responding)"
}

# ---------- Main ----------
Write-Info "Flutter Windows Desktop launcher"
Write-Info "Project root: $PROJECT_ROOT"
Write-Info "Flutter app:  $FLUTTER_APP"

if (-not $SkipBuild) {
    Assert-FlutterInstalled
    Assert-CargoInstalled
    Build-RustCrate
    Build-FlutterDesktop
} else {
    Write-Info "-SkipBuild specified; skipping all build steps."
    if (-not $NoServer) {
        Assert-CargoInstalled
    }
}

if (-not $NoServer) {
    Start-LocalServer
}

Launch-App
