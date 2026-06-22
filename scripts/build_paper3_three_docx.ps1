param(
    [string]$Version = "14",
    [string]$Zip = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

. "$PSScriptRoot\_build_common.ps1"

$Root = Get-RepoRoot -ScriptDirectory $PSScriptRoot
$VersionNumber = Get-VersionNumber -Version $Version
$VersionTag = "v$VersionNumber"
$Paper3Dir = Join-Path $Root "examples\paper3"
$LatexDir = Join-Path $Paper3Dir "latex"
$MainTex = Join-Path $LatexDir "main-jos.tex"
$OutDir = Join-Path $Paper3Dir "output\to-docx"
$Stamp = Get-TimeStamp
$Base = "$VersionTag-论文稿件-jos-$Stamp"
if ([string]::IsNullOrWhiteSpace($Zip)) {
    $Zip = Join-Path $OutDir "$Base-paper3-upload.zip"
}
$SemanticBackend = if ($env:SEMANTIC_BACKEND) { $env:SEMANTIC_BACKEND } else { "xelatex-hook" }
$StrictSemantic = if ($env:STRICT_SEMANTIC) { $env:STRICT_SEMANTIC } else { "1" }
$SemanticSlug = $SemanticBackend.Replace("-", "_")

$RustRuleDocx = Join-Path $OutDir "$Base-rust-rule.docx"
$SemanticDocx = Join-Path $OutDir "$Base-semantic-engine-$SemanticSlug.docx"
$ShLog = Join-Path $OutDir "$Base-sh.log"
$RustRuleLog = Join-Path $OutDir "$Base-rust-rule.log"
$SemanticLog = Join-Path $OutDir "$Base-semantic-engine-$SemanticSlug.log"
$Report = Join-Path $OutDir "$Base-three-docx-report.md"
$DocxTool = Get-DocEnginePath -Root $Root

function Invoke-ScriptToLog {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ScriptPath,
        [string[]]$Arguments = @(),
        [Parameter(Mandatory = $true)]
        [string]$LogPath
    )

    try {
        & $ScriptPath @Arguments *> $LogPath
    } catch {
        $_ | Out-File -FilePath $LogPath -Encoding utf8 -Append
        throw
    }
}

function Invoke-NativeToLog {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,
        [string[]]$Arguments = @(),
        [Parameter(Mandatory = $true)]
        [string]$LogPath,
        [string]$WorkingDirectory = "",
        [switch]$Append
    )

    if ([string]::IsNullOrWhiteSpace($WorkingDirectory)) {
        $output = & $FilePath @Arguments 2>&1
        $code = $LASTEXITCODE
    } else {
        Push-Location $WorkingDirectory
        try {
            $output = & $FilePath @Arguments 2>&1
            $code = $LASTEXITCODE
        } finally {
            Pop-Location
        }
    }

    if ($Append) {
        $output | Out-File -FilePath $LogPath -Encoding utf8 -Append
    } else {
        $output | Out-File -FilePath $LogPath -Encoding utf8
    }

    if ($code -ne 0) {
        throw "$FilePath failed with exit code $code; see $LogPath"
    }
}

function Write-ReportHeader {
    $lines = @(
        "# paper3 三路径 DOCX 验证报告",
        "",
        "- timestamp: $Stamp",
        "- version: $VersionTag",
        "- main_tex: $MainTex",
        "- zip: $Zip",
        "- output_dir: $OutDir",
        "- semantic_backend: $SemanticBackend",
        "- strict_semantic: $StrictSemantic",
        "",
        "| path | docx | bytes | media | log |",
        "|---|---|---:|---:|---|"
    )
    Write-Utf8Lines -Path $Report -Lines $lines
}

function Add-ReportLine {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Line
    )

    $Line | Out-File -FilePath $Report -Encoding utf8 -Append
}

function Append-ReportRow {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Label,
        [Parameter(Mandatory = $true)]
        [string]$Docx,
        [Parameter(Mandatory = $true)]
        [string]$Log
    )

    $bytes = Get-FileSizeBytes -Path $Docx
    $media = Get-DocxMediaCount -DocxPath $Docx
    $docxName = Split-Path -Leaf $Docx
    $logName = Split-Path -Leaf $Log
    Add-ReportLine -Line "| ``$Label`` | [$docxName](./$docxName) | $bytes | $media | [$logName](./$logName) |"
}

try { Get-PythonLauncher | Out-Null } catch { throw "missing dependency: python or py -3" }
Assert-Command -Name "cargo"
Assert-PathExists -Path $MainTex -Kind "main tex"

New-Item -ItemType Directory -Path $OutDir -Force | Out-Null

Write-Host "=== prepare paper3 zip ==="
New-Paper3UploadZip -Paper3Directory $Paper3Dir -LatexDirectory $LatexDir -MainTex $MainTex -OutputZip $Zip

Write-ReportHeader

Write-Host "=== build sh DOCX ==="
$beforeSh = Get-LatestFile -Directory $OutDir -Filter "v$VersionNumber-论文稿件-jos-sh-*.docx"
Invoke-ScriptToLog -ScriptPath (Join-Path $Root "scripts\build_docx.ps1") -Arguments @($VersionNumber) -LogPath $ShLog
$shDocx = Get-LatestFile -Directory $OutDir -Filter "v$VersionNumber-论文稿件-jos-sh-*.docx"
if (-not $shDocx -or -not (Test-Path -LiteralPath $shDocx) -or $shDocx -eq $beforeSh) {
    throw "sh DOCX was not generated; see $ShLog"
}
Append-ReportRow -Label "sh" -Docx $shDocx -Log $ShLog

Write-Host "=== build rust-rule DOCX ==="
if (-not (Test-Path -LiteralPath $DocxTool)) {
    Invoke-NativeToLog -FilePath "cargo" -Arguments @("build", "--release", "-p", "doc-engine") -LogPath $RustRuleLog -WorkingDirectory $Root
} else {
    Write-Utf8Lines -Path $RustRuleLog -Lines @()
}

if (-not (Test-Path -LiteralPath $DocxTool)) {
    throw "missing doc-engine binary: $DocxTool"
}

Invoke-NativeToLog -FilePath $DocxTool -Arguments @(
    "convert",
    "--zip", $Zip,
    "--main-tex", "main-jos.tex",
    "--page-setup", "jos-paper3",
    "--out", $RustRuleDocx
) -LogPath $RustRuleLog -Append

if (-not (Test-Path -LiteralPath $RustRuleDocx)) {
    throw "rust-rule DOCX was not generated: $RustRuleDocx"
}
Append-ReportRow -Label "rust-rule" -Docx $RustRuleDocx -Log $RustRuleLog

Write-Host "=== build semantic-engine DOCX ==="
$semanticArgs = @(
    "--project-root", $LatexDir,
    "--main-tex", $MainTex,
    "--profile", "jos-paper",
    "--semantic-backend", $SemanticBackend,
    "--out", $SemanticDocx
)
if ($StrictSemantic -eq "1") {
    $semanticArgs += "--no-backend-fallback"
}

$cargoArgs = @("run", "-p", "doc-compiler-engine", "--example", "paper3_to_docx", "--") + $semanticArgs
Invoke-NativeToLog -FilePath "cargo" -Arguments $cargoArgs -LogPath $SemanticLog -WorkingDirectory $Root

if (-not (Test-Path -LiteralPath $SemanticDocx)) {
    throw "semantic-engine DOCX was not generated: $SemanticDocx"
}
Append-ReportRow -Label "semantic-engine" -Docx $SemanticDocx -Log $SemanticLog

Add-ReportLine -Line ""
Add-ReportLine -Line "## Semantic Backend"
Add-ReportLine -Line ""
$semanticPattern = '^(backend-(requested|selected|fallback-from|reason)|compatibility-(score|unsupported|warnings|custom-macros)|sidecars|profile-(id|page-setup)|reference-(labels|edges)|citations|unresolved-references|bookmarks|hyperlinks|omml-equations|omml-equation-fallbacks):'
if (Test-Path -LiteralPath $SemanticLog) {
    Select-String -LiteralPath $SemanticLog -Pattern $semanticPattern | ForEach-Object {
        Add-ReportLine -Line $_.Line
    }
}

Write-Host "=== done ==="
Write-Host "sh              : $shDocx"
Write-Host "rust-rule       : $RustRuleDocx"
Write-Host "semantic-engine : $SemanticDocx"
Write-Host "report          : $Report"
