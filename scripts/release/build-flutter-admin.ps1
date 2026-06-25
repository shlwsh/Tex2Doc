Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Join-Path $PSScriptRoot "../../flutter_app")
try {
  flutter build web --target lib/main_admin.dart
} finally {
  Pop-Location
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "../..")
$source = Join-Path $repoRoot "flutter_app/build/web"
$target = Join-Path $repoRoot "apps/rust-service/static/admin"
New-Item -ItemType Directory -Force -Path $target | Out-Null
Copy-Item -Path (Join-Path $source "*") -Destination $target -Recurse -Force
