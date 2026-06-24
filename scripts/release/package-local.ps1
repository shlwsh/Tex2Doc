Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "../..")
$distRoot = Join-Path $repoRoot "dist"
$rid = switch ($true) {
  $IsWindows { "windows-x64"; break }
  $IsLinux { "linux-x64"; break }
  $IsMacOS {
    $arch = (uname -m)
    if ($arch -eq "arm64") { "macos-arm" } else { "macos-intel" }
    break
  }
  default { "unknown" }
}

$stage = Join-Path $distRoot "tex2doc-$rid"
Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path (Join-Path $stage "server") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stage "client") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stage "static") | Out-Null

Push-Location $repoRoot
try {
  cargo build -p doc-server --release
  cargo build -p doc-desktop-slint --release

  $exe = if ($IsWindows) { ".exe" } else { "" }
  Copy-Item "target/release/doc-server$exe" (Join-Path $stage "server")
  Copy-Item "target/release/doc-desktop-slint$exe" (Join-Path $stage "client")

  if (Get-Command flutter -ErrorAction SilentlyContinue) {
    & (Join-Path $PSScriptRoot "package-web-static.ps1")
    Copy-Item "apps/rust-service/static/*" (Join-Path $stage "static") -Recurse -Force
  } else {
    Write-Warning "flutter not found; skipping web static bundle."
  }

  Copy-Item "README.md" $stage
  Copy-Item "apps/rust-service/README.md" (Join-Path $stage "server")

  $zipPath = Join-Path $distRoot "tex2doc-$rid.zip"
  Compress-Archive -Path $stage -DestinationPath $zipPath -Force
  Write-Host "Package ready: $zipPath"
} finally {
  Pop-Location
}
