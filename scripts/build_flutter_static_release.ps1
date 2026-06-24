param(
    [string]$FlutterAppPath = "flutter_app",
    [string]$StaticPath = "apps/rust-service/static",
    [string]$FlutterCommand = "flutter",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$Root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$FlutterRoot = Join-Path $Root $FlutterAppPath
$StaticRoot = Join-Path $Root $StaticPath

if (-not (Test-Path -LiteralPath $FlutterRoot -PathType Container)) {
    throw "Flutter app path not found: $FlutterRoot"
}

function Copy-WebBuild {
    param(
        [string]$Target,
        [string]$DestinationName
    )

    $destination = Join-Path $StaticRoot $DestinationName
    $buildOutput = Join-Path $FlutterRoot "build\web"

    if (-not $SkipBuild) {
        Push-Location -LiteralPath $FlutterRoot
        try {
            & $FlutterCommand build web --release --target $Target
            if ($LASTEXITCODE -ne 0) {
                throw "flutter build web failed for $Target with exit code $LASTEXITCODE"
            }
        } finally {
            Pop-Location
        }
    }

    if (-not (Test-Path -LiteralPath $buildOutput -PathType Container)) {
        throw "Flutter web output not found: $buildOutput"
    }

    if (Test-Path -LiteralPath $destination) {
        Remove-Item -LiteralPath $destination -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    Copy-Item -Path (Join-Path $buildOutput "*") -Destination $destination -Recurse -Force
    Write-Host "Copied $Target to $destination"
}

New-Item -ItemType Directory -Force -Path $StaticRoot | Out-Null

Copy-WebBuild -Target "lib/main.dart" -DestinationName "home"
Copy-WebBuild -Target "lib/main_user.dart" -DestinationName "user"
Copy-WebBuild -Target "lib/main_admin.dart" -DestinationName "admin"

Write-Host "Flutter static release assets are ready under $StaticRoot"
