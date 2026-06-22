$ErrorActionPreference = "Stop"

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("tex2doc-version-test-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $tempDir | Out-Null

try {
    $versionPath = Join-Path $tempDir "VERSION"

    Set-Content -LiteralPath $versionPath -Value "1.26.6.12" -NoNewline
    & "$PSScriptRoot\bump_desktop_version.ps1" -VersionPath $versionPath -DateOverride ([datetime]"2026-06-22") -NoGitAdd | Out-Null
    $actual = (Get-Content -LiteralPath $versionPath -Raw).Trim()
    if ($actual -ne "1.26.6.13") {
        throw "Expected same-month bump to 1.26.6.13, got $actual"
    }

    Set-Content -LiteralPath $versionPath -Value "1.26.6.13" -NoNewline
    & "$PSScriptRoot\bump_desktop_version.ps1" -VersionPath $versionPath -DateOverride ([datetime]"2026-07-01") -NoGitAdd | Out-Null
    $actual = (Get-Content -LiteralPath $versionPath -Raw).Trim()
    if ($actual -ne "1.26.7.1") {
        throw "Expected cross-month reset to 1.26.7.1, got $actual"
    }

    Set-Content -LiteralPath $versionPath -Value "1.26.7.1" -NoNewline
    $env:TEX2DOC_SKIP_VERSION_BUMP = "1"
    & "$PSScriptRoot\bump_desktop_version.ps1" -VersionPath $versionPath -DateOverride ([datetime]"2026-07-02") -NoGitAdd | Out-Null
    $actual = (Get-Content -LiteralPath $versionPath -Raw).Trim()
    if ($actual -ne "1.26.7.1") {
        throw "Expected skipped bump to keep 1.26.7.1, got $actual"
    }
    Remove-Item Env:\TEX2DOC_SKIP_VERSION_BUMP -ErrorAction SilentlyContinue

    Set-Content -LiteralPath $versionPath -Value "bad.version" -NoNewline
    $failed = $false
    try {
        & "$PSScriptRoot\bump_desktop_version.ps1" -VersionPath $versionPath -DateOverride ([datetime]"2026-07-02") -NoGitAdd | Out-Null
    } catch {
        $failed = $true
    }
    if (-not $failed) {
        throw "Expected invalid VERSION to fail"
    }

    Write-Host "Desktop version bump tests passed."
} finally {
    Remove-Item Env:\TEX2DOC_SKIP_VERSION_BUMP -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
}
