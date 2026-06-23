# Convert CRLF to LF for all text source files.
# Must be run with: git config --global core.autocrlf false
# in a repository that has .gitattributes with * text=auto eol=lf
param(
    [string]$Root = $PWD
)

$extensions = @('*.rs', '*.toml', '*.slint', '*.md', '*.yml', '*.yaml', '*.txt', '*.sh', '*.ps1', '*.py', '*.json')

$dirs = @(
    "$Root\crates",
    "$Root\scripts",
    "$Root\.github",
    "$Root\standards",
    "$Root\profiles",
    "$Root\flutter_app\lib",
    "$Root\flutter_app\test"
)

$totalFiles = 0
$fixedFiles = 0

foreach ($dir in $dirs) {
    if (-not (Test-Path $dir)) { continue }
    foreach ($ext in $extensions) {
        $files = Get-ChildItem -Path $dir -Recurse -Include $ext -File -ErrorAction SilentlyContinue
        foreach ($file in $files) {
            $totalFiles++
            $bytes = [System.IO.File]::ReadAllBytes($file.FullName)
            $hasCRLF = $false
            for ($i = 0; $i -lt $bytes.Length - 1; $i++) {
                if ($bytes[$i] -eq 13 -and $bytes[$i+1] -eq 10) {
                    $hasCRLF = $true
                    break
                }
            }

            if ($hasCRLF) {
                $content = [System.IO.File]::ReadAllText($file.FullName)
                # Normalize to LF
                $content = $content -replace "`r`n", "`n"
                # Detect encoding from original (UTF-8 with BOM check)
                $isUtf8WithBom = $bytes.Length -ge 3 -and $bytes[0] -eq 239 -and $bytes[1] -eq 187 -and $bytes[2] -eq 191
                if ($isUtf8WithBom) {
                    $utf8Bom = New-Object System.Text.UTF8Encoding $true
                    [System.IO.File]::WriteAllText($file.FullName, $content, $utf8Bom)
                } else {
                    # Try to detect other encodings; default to UTF8 no BOM
                    $encoding = New-Object System.Text.UTF8Encoding $false
                    [System.IO.File]::WriteAllText($file.FullName, $content, $encoding)
                }
                $fixedFiles++
                Write-Host "Fixed: $($file.FullName)" -ForegroundColor Green
            }
        }
    }
}

Write-Host ""
Write-Host "Total files scanned: $totalFiles" -ForegroundColor Cyan
Write-Host "Files fixed (CRLF->LF): $fixedFiles" -ForegroundColor Yellow
