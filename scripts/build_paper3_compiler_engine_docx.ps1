param(
    [string]$Version = "v13"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

. "$PSScriptRoot\_build_common.ps1"

$Root = Get-RepoRoot -ScriptDirectory $PSScriptRoot
$VersionTag = Get-VersionTag -Version $Version
$Paper3Dir = Join-Path $Root "examples\paper3"
$LatexDir = Join-Path $Paper3Dir "latex"
$MainTex = Join-Path $LatexDir "main-jos.tex"
$OutDir = Join-Path $Paper3Dir "output\to-docx"
$Stamp = Get-TimeStamp
$OutDocx = Join-Path $OutDir "$VersionTag-论文稿件-jos-$Stamp-compiler-engine.docx"

Assert-Command -Name "cargo"
Assert-PathExists -Path $LatexDir -Kind "paper3 latex directory"
Assert-PathExists -Path $MainTex -Kind "main tex"

New-Item -ItemType Directory -Path $OutDir -Force | Out-Null

Write-Host "=== building paper3 DOCX via doc-compiler-engine ==="
Write-Host "main : $MainTex"
Write-Host "out  : $OutDocx"

Invoke-Native -FilePath "cargo" -Arguments @(
    "run", "-p", "doc-compiler-engine", "--example", "paper3_to_docx", "--",
    "--project-root", $LatexDir,
    "--main-tex", $MainTex,
    "--profile", "jos-paper",
    "--out", $OutDocx
) -WorkingDirectory $Root

if (-not (Test-Path -LiteralPath $OutDocx)) {
    throw "DOCX not generated: $OutDocx"
}

$size = Get-FileSizeBytes -Path $OutDocx
Write-Host "=== done ==="
Write-Host "$OutDocx ($size bytes)"
