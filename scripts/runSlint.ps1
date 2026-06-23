<#
.SYNOPSIS
    Run the Tex2Doc Slint desktop application.

.DESCRIPTION
    Builds (if needed) and launches the doc-desktop-slint crate. Supports
    both debug (default) and release profiles.

.PARAMETER Profile
    Build profile: "dev" (default, nooptimizations + debug info) or "release".

.PARAMETER NoBuild
    Skip the cargo build step and only run the already-built binary.

.PARAMETER BuildOnly
    Build the binary and skip launching it.

.PARAMETER NoServer
    Skip starting the local doc-server backend before launching the desktop app.

.PARAMETER CargoPath
    Optional path to cargo.exe/cargo. When omitted, the script checks PATH and
    common rustup install locations.

.EXAMPLE
    .\scripts\runSlint.ps1
.EXAMPLE
    .\scripts\runSlint.ps1 -Profile release
#>

param(
    [ValidateSet("dev", "release")]
    [string]$Profile = "dev",

    [switch]$NoBuild,

    [switch]$BuildOnly,

    [switch]$NoServer,

    [string]$CargoPath
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if ($NoBuild -and $BuildOnly) {
    throw "-NoBuild and -BuildOnly cannot be used together."
}

$Root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$CargoToml = Join-Path $Root "crates\desktop-slint\Cargo.toml"

if (-not (Test-Path -LiteralPath $CargoToml)) {
    throw "desktop-slint Cargo.toml not found at: $CargoToml"
}

function Resolve-CargoPath {
    param(
        [string]$RequestedPath
    )

    if ($RequestedPath) {
        $resolved = (Resolve-Path -LiteralPath $RequestedPath -ErrorAction Stop).Path
        if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
            throw "CargoPath is not a file: $resolved"
        }
        return $resolved
    }

    $fromPath = Get-Command cargo -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($fromPath) {
        return $fromPath.Source
    }

    $candidatePaths = @(
        (Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"),
        (Join-Path $env:USERPROFILE ".cargo\bin\cargo.cmd"),
        (Join-Path $env:USERPROFILE ".rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\cargo.exe")
    )

    foreach ($candidatePath in $candidatePaths) {
        if ($candidatePath -and (Test-Path -LiteralPath $candidatePath -PathType Leaf)) {
            return $candidatePath
        }
    }

    throw "Cargo was not found. Install Rust from https://rustup.rs/ or pass -CargoPath <path-to-cargo.exe>."
}

$packageName = "doc-desktop-slint"
$serverPackageName = "doc-server"
$serverHost = "127.0.0.1"
$serverPort = 8080
$serverHealthUrl = "http://${serverHost}:${serverPort}/api/v1/health"

$targetDir = Join-Path $Root "target"
$profileDir = if ($Profile -eq "release") {
    Join-Path $targetDir "release"
} else {
    Join-Path $targetDir "debug"
}

$exeName = if ($env:OS -eq "Windows_NT") { "$packageName.exe" } else { $packageName }
$exePath = Join-Path $profileDir $exeName
$processName = [System.IO.Path]::GetFileNameWithoutExtension($exeName)

function Stop-ExistingSlint {
    $existing = Get-Process -Name $processName -ErrorAction SilentlyContinue
    if ($existing) {
        $ids = ($existing | Select-Object -ExpandProperty Id) -join ", "
        Write-Host "[runSlint] stopping existing $processName process(es): $ids"
        $existing | Stop-Process -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 500
    }
}

function Start-Slint {
    if (-not (Test-Path -LiteralPath $exePath)) {
        throw "Binary not found (run without -NoBuild first): $exePath"
    }

    Stop-ExistingSlint
    Write-Host "[runSlint] launching $exePath ..."
    $process = Start-Process -FilePath $exePath -WorkingDirectory $profileDir -PassThru
    Start-Sleep -Milliseconds 800
    if ($process.HasExited) {
        throw "Slint app exited immediately with code $($process.ExitCode)."
    }
    Write-Host "[runSlint] started PID $($process.Id)"
}

function Test-ServerPortOpen {
    try {
        $client = [System.Net.Sockets.TcpClient]::new()
        $connect = $client.BeginConnect($serverHost, $serverPort, $null, $null)
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

function Start-LocalServer {
    param(
        [string]$CargoExe
    )

    if (Test-ServerPortOpen) {
        Write-Host "[runSlint] doc-server already listening on ${serverHost}:${serverPort}"
        return
    }

    Write-Host "[runSlint] starting local $serverPackageName on ${serverHost}:${serverPort} ..."
    $serverArgs = @("run", "-p", $serverPackageName)
    $envBlock = @{
        "DOC_SERVER_ADDR" = "127.0.0.1:8080"
    }

    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $CargoExe
    $psi.Arguments = ($serverArgs -join " ")
    $psi.WorkingDirectory = $Root
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    foreach ($key in $envBlock.Keys) {
        $psi.Environment[$key] = $envBlock[$key]
    }
    $process = [System.Diagnostics.Process]::Start($psi)

    for ($attempt = 0; $attempt -lt 45; $attempt++) {
        if ($process.HasExited) {
            throw "doc-server exited immediately with code $($process.ExitCode)."
        }
        try {
            $response = Invoke-RestMethod -Uri $serverHealthUrl -TimeoutSec 1
            if ($response.status -eq "ok") {
                Write-Host "[runSlint] doc-server ready (PID $($process.Id))"
                return
            }
        } catch {
            Start-Sleep -Milliseconds 700
        }
    }

    throw "doc-server did not become healthy at $serverHealthUrl."
}

if ($NoBuild) {
    if (-not $NoServer) {
        $cargoExe = Resolve-CargoPath -RequestedPath $CargoPath
        Start-LocalServer -CargoExe $cargoExe
    }
    Start-Slint
    return
}

Stop-ExistingSlint

$displayProfile = if ($Profile -eq "release") { "release" } else { "dev" }
Write-Host "[runSlint] building ($displayProfile) $packageName ..."
$cargoExe = Resolve-CargoPath -RequestedPath $CargoPath
Write-Host "[runSlint] using cargo: $cargoExe"
[string[]]$cargoArgs = if ($Profile -eq "release") {
    @("build", "--profile=release", "-p", $packageName)
} else {
    @("build", "-p", $packageName)
}

Push-Location -LiteralPath $Root
try {
    & $cargoExe @cargoArgs
    $exitCode = $LASTEXITCODE
} finally {
    Pop-Location
}

if ($exitCode -ne 0) {
    throw "cargo build failed with exit code $exitCode"
}

if (-not (Test-Path -LiteralPath $exePath)) {
    throw "build succeeded but binary not found at: $exePath"
}

if ($BuildOnly) {
    Write-Host "[runSlint] build completed: $exePath"
    return
}

if (-not $NoServer) {
    Start-LocalServer -CargoExe $cargoExe
}

Start-Slint
