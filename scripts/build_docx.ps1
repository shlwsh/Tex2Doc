param(
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

. "$PSScriptRoot\_build_common.ps1"

$Root = Get-RepoRoot -ScriptDirectory $PSScriptRoot
$Scripts = Join-Path $Root "scripts"
$PaperRoot = Join-Path $Root "examples\paper3"
$LatexDir = Join-Path $PaperRoot "latex"
$FormatJson = Join-Path $Root "docs\format\jos_2025_docx_format_definitions.json"
$TexSrc = Join-Path $LatexDir "main-jos.tex"
$PdfSrc = Join-Path $LatexDir "main-jos.pdf"
$BblSrc = Join-Path $LatexDir "main-jos.bbl"
$OutputDir = Join-Path $PaperRoot "output\to-docx"
$InputManifest = Join-Path $LatexDir ".main-jos.inputs.sha256"

function Test-BuildDocxDeps {
    $missing = New-Object System.Collections.Generic.List[string]

    try { Get-PythonLauncher | Out-Null } catch { $missing.Add("python or py -3") }
    foreach ($command in @("pdftotext", "pdftoppm", "latexmk", "xelatex", "bibtex")) {
        if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
            $missing.Add($command)
        }
    }
    if (-not (Test-PythonModule -ImportStatement "from PIL import Image")) {
        $missing.Add("Pillow/PIL (used to read image dimensions)")
    }
    if (-not (Test-Path -LiteralPath $FormatJson)) {
        $missing.Add($FormatJson)
    }
    if (-not (Test-Path -LiteralPath $TexSrc)) {
        $missing.Add($TexSrc)
    }

    if ($missing.Count -gt 0) {
        Write-Host "Missing dependencies or inputs:"
        foreach ($dep in $missing) {
            Write-Host "  - $dep"
        }
        exit 1
    }
}

function Write-InputManifest {
    param(
        [Parameter(Mandatory = $true)]
        [string]$OutputPath
    )

    $extensions = @(".tex", ".bib", ".bst", ".cls", ".sty")
    $paths = @()
    $paths += Get-ChildItem -LiteralPath $LatexDir -Recurse -File |
        Where-Object { $extensions -contains $_.Extension.ToLowerInvariant() } |
        ForEach-Object { $_.FullName }
    $paths += $FormatJson

    $lines = foreach ($path in ($paths | Sort-Object { Get-RepoRelativePath -Root $Root -Path $_ })) {
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
        $relative = Get-RepoRelativePath -Root $Root -Path $path
        "$hash  $relative"
    }

    Write-Utf8Lines -Path $OutputPath -Lines $lines
}

function Test-LatexInputsUnchanged {
    param(
        [Parameter(Mandatory = $true)]
        [string]$TempManifest
    )

    if (-not (Test-Path -LiteralPath $InputManifest)) {
        return $false
    }

    $current = Get-Content -LiteralPath $InputManifest -Raw
    $next = Get-Content -LiteralPath $TempManifest -Raw
    return $current -eq $next
}

function Ensure-LatexOutputs {
    $tempManifest = [System.IO.Path]::GetTempFileName()
    try {
        Write-InputManifest -OutputPath $tempManifest
        if ((Test-Path -LiteralPath $PdfSrc) -and (Test-Path -LiteralPath $BblSrc) -and (Test-LatexInputsUnchanged -TempManifest $tempManifest)) {
            Write-Host "  - Reusing $PdfSrc and $BblSrc"
            return
        }

        Write-Host "  - Building PDF/BBL with latexmk"
        Invoke-Native -FilePath "latexmk" -Arguments @("-xelatex", "-bibtex", "-interaction=nonstopmode", "-halt-on-error", "main-jos.tex") -WorkingDirectory $LatexDir
        Copy-Item -LiteralPath $tempManifest -Destination $InputManifest -Force
        Write-Host "  - Updated input manifest $InputManifest"
    } finally {
        Remove-Item -LiteralPath $tempManifest -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "=== Check dependencies and inputs ==="
Test-BuildDocxDeps
Write-Host "  - OK"

Write-Host "=== Ensure LaTeX PDF/BBL ==="
Ensure-LatexOutputs

New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

if ([string]::IsNullOrWhiteSpace($Version)) {
    $maxVersion = 0
    Get-ChildItem -LiteralPath $OutputDir -File -Filter "v*-论文稿件-jos-*" -ErrorAction SilentlyContinue | ForEach-Object {
        if ($_.Name -match '^v([0-9]+)-论文稿件-jos-.*') {
            $number = [int]$Matches[1]
            if ($number -gt $maxVersion) {
                $maxVersion = $number
            }
        }
    }
    $VersionNumber = [string]($maxVersion + 1)
} else {
    $VersionNumber = Get-VersionNumber -Version $Version
}

$Stamp = Get-TimeStamp
$DocxDst = Join-Path $OutputDir "v$VersionNumber-论文稿件-jos-sh-$Stamp.docx"
$ReportDst = Join-Path $OutputDir "v$VersionNumber-论文稿件-jos-sh-$Stamp-docx校验报告.md"
$ReportJson = Join-Path $OutputDir "v$VersionNumber-论文稿件-jos-sh-$Stamp-docx校验报告.json"

Write-Host "=== Version: v$VersionNumber @ $Stamp ==="
Write-Host "=== Generate DOCX ==="
Invoke-Python -Arguments @(
    (Join-Path $Scripts "build_jos_docx.py"),
    "--root", $PaperRoot,
    "--format", $FormatJson,
    "--output", $DocxDst
)

Write-Host "=== Verify DOCX against TeX/PDF/format ==="
Invoke-Python -Arguments @(
    (Join-Path $Scripts "verify_jos_docx.py"),
    "--docx", $DocxDst,
    "--pdf", $PdfSrc,
    "--tex-root", $PaperRoot,
    "--format", $FormatJson,
    "--report", $ReportDst,
    "--json-report", $ReportJson
)

$fileSize = Get-FileSizeBytes -Path $DocxDst

Write-Host ""
Write-Host "=== Done ==="
Write-Host "  DOCX: $DocxDst"
Write-Host "  Report: $ReportDst"
Write-Host "  Size: $(Format-ByteSize -Bytes $fileSize)"
