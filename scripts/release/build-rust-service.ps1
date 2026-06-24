Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Join-Path $PSScriptRoot "../..")
try {
  cargo build -p doc-server --release
} finally {
  Pop-Location
}
