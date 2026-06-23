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

$targetDir = Join-Path $Root "target"
$profileDir = if ($Profile -eq "release") {
    Join-Path $targetDir "release"
} else {
    Join-Path $targetDir "debug"
}

$exeName = if ($env:OS -eq "Windows_NT") { "$packageName.exe" } else { $packageName }
$exePath = Join-Path $profileDir $exeName

if ($NoBuild) {
    if (-not (Test-Path -LiteralPath $exePath)) {
        throw "Binary not found (run without -NoBuild first): $exePath"
    }
    Write-Host "[runSlint] launching $exePath ..."
    & $exePath
    return
}

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

Write-Host "[runSlint] launching $exePath ..."
& $exePath
