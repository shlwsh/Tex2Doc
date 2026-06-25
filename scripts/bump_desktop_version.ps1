param(
    [string]$VersionPath = "apps/slint-user/VERSION",
    [datetime]$DateOverride,
    [switch]$NoGitAdd
)

if ($env:TEX2DOC_SKIP_VERSION_BUMP -eq "1") {
    Write-Host "Tex2Doc desktop version bump skipped."
    exit 0
}

$now = if ($PSBoundParameters.ContainsKey("DateOverride")) { $DateOverride } else { Get-Date }
$prefix = "1.{0}.{1}" -f ($now.Year % 100), $now.Month

if (-not (Test-Path -LiteralPath $VersionPath)) {
    $next = "$prefix.1"
} else {
    $current = (Get-Content -LiteralPath $VersionPath -Raw).Trim()
    $parts = $current -split '\.'
    if ($parts.Length -ne 4 -or $parts[0] -ne "1" -or -not ($parts[3] -match '^\d+$')) {
        Write-Error "Invalid desktop VERSION value: $current"
        exit 1
    }

    $currentPrefix = "{0}.{1}.{2}" -f $parts[0], $parts[1], $parts[2]
    if ($currentPrefix -eq $prefix) {
        $next = "$prefix.$([int]$parts[3] + 1)"
    } else {
        $next = "$prefix.1"
    }
}

Set-Content -LiteralPath $VersionPath -Value $next -NoNewline
if (-not $NoGitAdd) {
    git add -- $VersionPath
}
Write-Host "Tex2Doc desktop version bumped to $next"
