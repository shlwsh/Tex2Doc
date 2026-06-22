param(
    [string]$Version = "v12",
    [string]$Timestamp = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

. "$PSScriptRoot\_build_common.ps1"

$Root = Get-RepoRoot -ScriptDirectory $PSScriptRoot
$VersionTag = Get-VersionTag -Version $Version
if ([string]::IsNullOrWhiteSpace($Timestamp)) {
    $Timestamp = Get-TimeStamp
}

$Paper3Dir = Join-Path $Root "examples\paper3"
$LatexDir = Join-Path $Paper3Dir "latex"
$MainTex = Join-Path $LatexDir "main-jos.tex"
$FigDir = Join-Path $Paper3Dir "figures"
$OutDir = Join-Path $Paper3Dir "output\to-docx"
$Base = "$VersionTag-论文稿件-jos-$Timestamp"
$PandocDocx = Join-Path $OutDir "$Base-pandoc.docx"
$WorkDir = Join-Path ([System.IO.Path]::GetTempPath()) ("tex2doc-pandoc-" + [System.Guid]::NewGuid().ToString("N"))

function Test-PandocDeps {
    Assert-Command -Name "pandoc"
    try { Get-PythonLauncher | Out-Null } catch { throw "missing dependency: python or py -3" }
    Assert-PathExists -Path $MainTex -Kind "main tex"
    Assert-PathExists -Path $FigDir -Kind "figure directory"
}

Write-Host "=== building pandoc DOCX ($VersionTag @ $Timestamp) ==="
Test-PandocDeps
New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
New-Item -ItemType Directory -Path $WorkDir -Force | Out-Null

try {
    $MasterTex = Join-Path $WorkDir "master.tex"
    $PythonScript = Join-Path $WorkDir "build_pandoc_master.py"
    $PythonSource = @'
"""Build a pandoc-friendly single master tex for paper3.

- Recursively expand \input{...}
- Remove \graphicspath{...}
- Rewrite \includegraphics{X} to absolute figure paths
- Add .tex for extensionless \input{path}
"""
import re
import sys
from pathlib import Path

latex_dir = Path(sys.argv[1]).resolve()
fig_dir = Path(sys.argv[2]).resolve()
main_tex = Path(sys.argv[3])
out_path = Path(sys.argv[4])

text = main_tex.read_text(encoding="utf-8")

def expand_input(match):
    rel = match.group(1).strip()
    target = (latex_dir / rel).resolve()
    if not target.is_file() and not target.suffix:
        target = target.with_suffix(".tex")
    if target.is_file():
        return (
            f"\n% === expanded from {match.group(1)} ===\n"
            + target.read_text(encoding="utf-8")
            + f"\n% === end {match.group(1)} ===\n"
        )
    print(f"[pandoc] WARN: cannot expand \\input{{{rel}}} (not a file)", file=sys.stderr)
    return match.group(0)

text = re.sub(r"\\input\{([^}]+)\}", expand_input, text)
text = re.sub(r"\\graphicspath\{[^}]*\}", "", text)

def rewrite_includegraphics(match):
    opts = match.group(1) or ""
    target = match.group(2)
    fig_path = (fig_dir / target).resolve()
    if fig_path.is_file():
        return f"\\includegraphics{opts}{{{fig_path}}}"
    print(f"[pandoc] WARN: image not found: {target}", file=sys.stderr)
    return match.group(0)

text = re.sub(
    r"\\includegraphics(\[[^\]]*\])?\{([^}]+)\}",
    rewrite_includegraphics,
    text,
)

out_path.write_text(text, encoding="utf-8")
print(f"[pandoc] master tex: {len(text)} chars -> {out_path}")
'@
    Write-Utf8Lines -Path $PythonScript -Lines @($PythonSource)

    Invoke-Python -Arguments @($PythonScript, $LatexDir, $FigDir, $MainTex, $MasterTex)

    Write-Host "=== invoking pandoc ==="
    $pandocArgs = @(
        "--from=latex",
        "--to=docx",
        "--resource-path=$LatexDir",
        "--output=$PandocDocx",
        $MasterTex
    )
    & pandoc @pandocArgs 2>&1 | ForEach-Object { Write-Host "[pandoc] $_" }
    if ($LASTEXITCODE -ne 0) {
        throw "pandoc failed with exit code $LASTEXITCODE"
    }

    if (-not (Test-Path -LiteralPath $PandocDocx)) {
        throw "pandoc DOCX not generated"
    }

    $fileSize = Get-FileSizeBytes -Path $PandocDocx
    Write-Host "OK $(Split-Path -Leaf $PandocDocx) ($fileSize bytes)"
    Write-Output $PandocDocx
} finally {
    Remove-Item -LiteralPath $WorkDir -Recurse -Force -ErrorAction SilentlyContinue
}
