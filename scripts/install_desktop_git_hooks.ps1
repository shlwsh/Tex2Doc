$ErrorActionPreference = "Stop"

$hookDir = Join-Path (Get-Location) ".git/hooks"
$hookPath = Join-Path $hookDir "pre-commit"

if (-not (Test-Path -LiteralPath $hookDir)) {
    New-Item -ItemType Directory -Path $hookDir | Out-Null
}

$content = @'
#!/usr/bin/env pwsh
powershell -ExecutionPolicy Bypass -File scripts\bump_desktop_version.ps1
'@

Set-Content -LiteralPath $hookPath -Value $content -NoNewline
Write-Host "Installed desktop version pre-commit hook at $hookPath"
