Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Join-Path $PSScriptRoot "../..")
try {
  cargo build -p doc-desktop-slint --release
} finally {
  Pop-Location
}
