param(
    [string]$DocxPath = "D:\output3.docx",
    [switch]$GenerateDocx,
    [switch]$SkipCargo,
    [switch]$SkipExistingDocxCheck
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

. "$PSScriptRoot\_build_common.ps1"

$Root = Get-RepoRoot -ScriptDirectory $PSScriptRoot

function Count-Text {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Text,
        [Parameter(Mandatory = $true)]
        [string]$Needle
    )

    return [regex]::Matches($Text, [regex]::Escape($Needle)).Count
}

function Read-DocxDocumentXml {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    Add-Type -AssemblyName System.IO.Compression | Out-Null
    Add-Type -AssemblyName System.IO.Compression.FileSystem | Out-Null

    $stream = [System.IO.File]::Open(
        $Path,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::Read,
        [System.IO.FileShare]::ReadWrite
    )
    try {
        $archive = [System.IO.Compression.ZipArchive]::new(
            $stream,
            [System.IO.Compression.ZipArchiveMode]::Read,
            $false
        )
        try {
            $entry = $archive.GetEntry("word/document.xml")
            if ($null -eq $entry) {
                throw "word/document.xml not found in DOCX: $Path"
            }
            $reader = [System.IO.StreamReader]::new($entry.Open(), [System.Text.Encoding]::UTF8)
            try {
                return $reader.ReadToEnd()
            } finally {
                $reader.Dispose()
            }
        } finally {
            $archive.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
}

function Test-NeedleInsideTable {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Xml,
        [Parameter(Mandatory = $true)]
        [string]$Needle
    )

    $pos = $Xml.IndexOf($Needle, [System.StringComparison]::Ordinal)
    if ($pos -lt 0) {
        return $false
    }

    $before = $Xml.Substring(0, $pos)
    $tableStart = $before.LastIndexOf("<w:tbl", [System.StringComparison]::Ordinal)
    $tableEnd = $before.LastIndexOf("</w:tbl>", [System.StringComparison]::Ordinal)
    return ($tableStart -ge 0) -and ($tableStart -gt $tableEnd)
}

Assert-Command -Name "cargo"

if (-not $SkipCargo) {
    if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
        $env:CARGO_TARGET_DIR = Join-Path ([System.IO.Path]::GetTempPath()) "tex2doc-paper3-front-docx-target"
    }

    Write-Host "=== Rust regression checks ==="
    Invoke-Native -FilePath "cargo" -Arguments @(
        "test", "-p", "doc-mathml", "--", "--nocapture"
    ) -WorkingDirectory $Root
    Invoke-Native -FilePath "cargo" -Arguments @(
        "test", "-p", "doc-docx-writer", "inline_math_uses_parsed_omml_not_raw_latex", "--", "--nocapture"
    ) -WorkingDirectory $Root
    Invoke-Native -FilePath "cargo" -Arguments @(
        "test", "-p", "doc-docx-writer", "algorithm_serializes_as_joscode", "--", "--nocapture"
    ) -WorkingDirectory $Root
    $oldFrontendOut = $env:TEX2DOC_PAPER3_FRONTEND_OUT
    if ($GenerateDocx) {
        $env:TEX2DOC_PAPER3_FRONTEND_OUT = $DocxPath
    }
    try {
        Invoke-Native -FilePath "cargo" -Arguments @(
            "test", "-p", "doc-desktop-slint", "paper3_conversion_overrides_stale_chinese_profile", "--", "--nocapture"
        ) -WorkingDirectory $Root
    } finally {
        if ($null -eq $oldFrontendOut) {
            Remove-Item Env:\TEX2DOC_PAPER3_FRONTEND_OUT -ErrorAction SilentlyContinue
        } else {
            $env:TEX2DOC_PAPER3_FRONTEND_OUT = $oldFrontendOut
        }
    }
}

if (-not $SkipExistingDocxCheck) {
    if (-not (Test-Path -LiteralPath $DocxPath)) {
        throw "DOCX not found: $DocxPath"
    }

    Write-Host "=== Existing DOCX XML checks ==="
    Write-Host "docx: $DocxPath"
    $xml = Read-DocxDocumentXml -Path $DocxPath

    $rawCommands = @("\varepsilon", "\rightarrow", "\emptyset")
    $rawCount = 0
    foreach ($command in $rawCommands) {
        $count = Count-Text -Text $xml -Needle $command
        $rawCount += $count
        if ($count -gt 0) {
            Write-Host "raw latex leak: $command = $count"
        }
    }

    $ommlCount = Count-Text -Text $xml -Needle "<m:oMath"
    $tableCount = Count-Text -Text $xml -Needle "<w:tbl"
    $cellCount = Count-Text -Text $xml -Needle "<w:tc"
    $hasCodeStyle = $xml.Contains('<w:pStyle w:val="JOSCode"/>')
    $algorithmCaptionInTable = Test-NeedleInsideTable -Xml $xml -Needle "算法 1"

    Write-Host "OMML equations : $ommlCount"
    Write-Host "tables         : $tableCount"
    Write-Host "table cells    : $cellCount"
    Write-Host "JOSCode style  : $hasCodeStyle"
    Write-Host "algorithm tbl  : $algorithmCaptionInTable"

    if ($rawCount -gt 0) {
        throw "raw LaTeX math commands remain in DOCX"
    }
    if ($ommlCount -le 0) {
        throw "no OMML equations found in DOCX"
    }
    if (-not $hasCodeStyle) {
        throw "JOSCode paragraphs not found"
    }
    if ($algorithmCaptionInTable) {
        throw "algorithm caption is still inside a table"
    }
}

Write-Host "=== paper3 frontend DOCX checks passed ==="
