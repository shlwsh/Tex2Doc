Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

& (Join-Path $PSScriptRoot "build-flutter-home.ps1")
& (Join-Path $PSScriptRoot "build-flutter-user.ps1")
& (Join-Path $PSScriptRoot "build-flutter-admin.ps1")
