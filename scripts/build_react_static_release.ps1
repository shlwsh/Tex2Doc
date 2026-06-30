param(
  [string]$RepoRoot = (Resolve-Path "$PSScriptRoot\..").Path,
  [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$reactRoot = Join-Path $RepoRoot "apps\react-web"
$distRoot = Join-Path $reactRoot "dist"
$staticRoot = Join-Path $RepoRoot "apps\rust-service\static"

if (-not $SkipBuild) {
  Push-Location $reactRoot
  try {
    npm run build
  } finally {
    Pop-Location
  }
}

if (-not (Test-Path $distRoot)) {
  throw "React dist directory not found: $distRoot"
}

foreach ($target in @("home", "user", "admin")) {
  $targetDir = Join-Path $staticRoot $target
  $expectedRoot = [System.IO.Path]::GetFullPath($staticRoot)
  $resolvedTarget = [System.IO.Path]::GetFullPath($targetDir)
  if (-not $resolvedTarget.StartsWith($expectedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to modify path outside static root: $resolvedTarget"
  }
  if (Test-Path $targetDir) {
    Remove-Item -LiteralPath $targetDir -Recurse -Force
  }
  New-Item -ItemType Directory -Path $targetDir | Out-Null
  Copy-Item -Path (Join-Path $distRoot "*") -Destination $targetDir -Recurse -Force
}

Write-Host "React static release copied to $staticRoot\{home,user,admin}"
