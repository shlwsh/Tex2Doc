param(
    [string]$OutputDir = "examples\output\journal-profile-generalization",
    [string[]]$Papers = @("paper2", "paper3"),
    [string[]]$Profiles = @("generic", "jos-paper", "tacl", "cvpr", "nature", "springer", "chinese-academic"),
    [string]$Backend = "auto",
    [string]$Quality = "preview"
)

$ErrorActionPreference = "Stop"

function Resolve-RepoPath([string]$Path) {
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return (Join-Path (Get-Location) $Path)
}

function Ensure-Dir([string]$Path) {
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Get-CargoPath {
    $cmd = Get-Command cargo -ErrorAction SilentlyContinue
    if ($cmd) {
        return $cmd.Source
    }
    $candidate = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (Test-Path $candidate) {
        return $candidate
    }
    throw "cargo not found"
}

function Get-MainTex([string]$Paper, [string]$Profile) {
    if ($Paper -eq "paper2") {
        switch ($Profile) {
            "jos-paper" { return "main-ral.tex" }
            "cvpr" { return "main-ral.tex" }
            "chinese-academic" { return "main-zh.tex" }
            default { return "main.tex" }
        }
    }
    if ($Paper -eq "paper3") {
        switch ($Profile) {
            "chinese-academic" { return "main-zh.tex" }
            default { return "main-jos.tex" }
        }
    }
    throw "unsupported paper: $Paper"
}

function Get-ProjectRoot([string]$Paper) {
    return "examples\$Paper\latex"
}

function Read-ZipEntryText($Zip, [string]$Name) {
    $entry = $Zip.GetEntry($Name)
    if (-not $entry) {
        return $null
    }
    $stream = $entry.Open()
    try {
        $reader = [System.IO.StreamReader]::new($stream, [System.Text.Encoding]::UTF8)
        try {
            return $reader.ReadToEnd()
        } finally {
            $reader.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
}

function Convert-DocxText([string]$Xml) {
    if (-not $Xml) {
        return ""
    }
    $matches = [regex]::Matches($Xml, '<w:t(?:\s[^>]*)?>(.*?)</w:t>', [System.Text.RegularExpressions.RegexOptions]::Singleline)
    $parts = foreach ($m in $matches) {
        [System.Net.WebUtility]::HtmlDecode($m.Groups[1].Value)
    }
    return ($parts -join " ")
}

function Count-Matches([string]$Text, [string]$Pattern) {
    if (-not $Text) {
        return 0
    }
    return ([regex]::Matches($Text, $Pattern)).Count
}

function Inspect-Docx([string]$DocxPath) {
    $result = [ordered]@{
        exists = (Test-Path $DocxPath)
        bytes = 0
        zip_openable = $false
        has_content_types = $false
        has_document_xml = $false
        has_styles_xml = $false
        has_relationships = $false
        paragraphs = 0
        tables = 0
        images = 0
        media_files = 0
        omml_equations = 0
        raw_latex_hits = 0
        text_chars = 0
        header1_text = ""
        header2_text = ""
    }
    if (-not (Test-Path $DocxPath)) {
        return $result
    }

    $file = Get-Item $DocxPath
    $result.bytes = $file.Length

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    try {
        $zip = [System.IO.Compression.ZipFile]::OpenRead($DocxPath)
    } catch {
        return $result
    }
    try {
        $result.zip_openable = $true
        $names = @($zip.Entries | ForEach-Object { $_.FullName })
        $result.has_content_types = $names -contains "[Content_Types].xml"
        $result.has_document_xml = $names -contains "word/document.xml"
        $result.has_styles_xml = $names -contains "word/styles.xml"
        $result.has_relationships = $names -contains "word/_rels/document.xml.rels"
        $result.media_files = @($names | Where-Object { $_ -like "word/media/*" }).Count

        $docXml = Read-ZipEntryText $zip "word/document.xml"
        $header1Xml = Read-ZipEntryText $zip "word/header1.xml"
        $header2Xml = Read-ZipEntryText $zip "word/header2.xml"
        $text = Convert-DocxText $docXml
        $result.text_chars = $text.Length
        $result.paragraphs = Count-Matches $docXml '<w:p[\s>]'
        $result.tables = Count-Matches $docXml '<w:tbl[\s>]'
        $result.images = (Count-Matches $docXml '<w:drawing[\s>]') + (Count-Matches $docXml '<w:pict[\s>]')
        $result.omml_equations = Count-Matches $docXml '<m:oMath'
        $result.raw_latex_hits = Count-Matches $text '\\(begin|end|cite|ref|label|includegraphics|section|subsection|frac|alpha|beta|gamma|delta)'
        $result.header1_text = (Convert-DocxText $header1Xml).Trim()
        $result.header2_text = (Convert-DocxText $header2Xml).Trim()
        return $result
    } finally {
        $zip.Dispose()
    }
}

function Get-Grade($Result) {
    if (-not $Result.metrics.exists -or -not $Result.metrics.zip_openable) {
        return "F"
    }
    if (-not $Result.metrics.has_document_xml -or -not $Result.metrics.has_styles_xml -or -not $Result.metrics.has_relationships) {
        return "F"
    }
    if ($Result.exit_code -ne 0) {
        return "D"
    }
    if ($Result.metrics.bytes -lt 20480 -or $Result.metrics.paragraphs -lt 20) {
        return "D"
    }
    if ($Result.metrics.raw_latex_hits -gt 80) {
        return "D"
    }
    if ($Result.compatibility_score -lt 60 -or $Result.metrics.raw_latex_hits -gt 25) {
        return "C"
    }
    if ($Result.unresolved_references -gt 20) {
        return "C"
    }
    if ($Result.compatibility_score -ge 80 -and $Result.metrics.raw_latex_hits -le 5 -and $Result.unresolved_references -eq 0) {
        return "A"
    }
    return "B"
}

function Get-Issues($Result) {
    $issues = New-Object System.Collections.Generic.List[string]
    if ($Result.exit_code -ne 0) { $issues.Add("command-exit-$($Result.exit_code)") }
    if (-not $Result.metrics.exists) { $issues.Add("docx-missing") }
    if (-not $Result.metrics.zip_openable) { $issues.Add("docx-not-openable-as-zip") }
    if (-not $Result.metrics.has_document_xml) { $issues.Add("missing-document-xml") }
    if (-not $Result.metrics.has_styles_xml) { $issues.Add("missing-styles-xml") }
    if (-not $Result.metrics.has_relationships) { $issues.Add("missing-document-relationships") }
    if ($Result.metrics.bytes -lt 20480) { $issues.Add("docx-too-small") }
    if ($Result.metrics.media_files -eq 0) { $issues.Add("no-media-files") }
    if ($Result.metrics.omml_equations -eq 0) { $issues.Add("no-omml-equations") }
    if ($Result.metrics.raw_latex_hits -gt 25) { $issues.Add("raw-latex-residue") }
    if ($Result.unresolved_references -gt 20) { $issues.Add("many-unresolved-references") }
    if ($Result.compatibility_score -lt 70) { $issues.Add("compatibility-below-plan-threshold") }
    if ($Result.profile_requested -ne $Result.profile_effective) { $issues.Add("requested-effective-profile-mismatch") }
    return @($issues)
}

$root = Resolve-RepoPath $OutputDir
$docxDir = Join-Path $root "docx"
$reportsDir = Join-Path $root "reports"
$logsDir = Join-Path $root "logs"
$snapshotsDir = Join-Path $root "snapshots"
$inputsDir = Join-Path $root "inputs"
Ensure-Dir $docxDir
Ensure-Dir $reportsDir
Ensure-Dir $logsDir
Ensure-Dir $snapshotsDir
Ensure-Dir $inputsDir

$cargo = Get-CargoPath
& $cargo build -p doc-engine | Out-Host
if ($LASTEXITCODE -ne 0) {
    throw "cargo build -p doc-engine failed"
}

$exe = Resolve-RepoPath "target\debug\doc-engine.exe"
$results = New-Object System.Collections.Generic.List[object]
$manifest = New-Object System.Collections.Generic.List[object]

foreach ($paper in $Papers) {
    foreach ($profile in $Profiles) {
        $projectRootRel = Get-ProjectRoot $paper
        $mainTex = Get-MainTex $paper $profile
        $projectRoot = Resolve-RepoPath $projectRootRel
        $sourceMain = Join-Path $projectRoot $mainTex
        $caseId = "${paper}__${profile}__${Backend}"
        $docxPath = Join-Path $docxDir "$caseId.docx"
        $reportPath = Join-Path $reportsDir "$caseId.report.json"
        $logPath = Join-Path $logsDir "$caseId.log"
        $snapshotPath = Join-Path $snapshotsDir "$caseId.word-open.txt"
        $inputCaseDir = Join-Path (Join-Path $inputsDir $paper) $profile
        Ensure-Dir $inputCaseDir

        if (Test-Path $sourceMain) {
            Copy-Item -Force $sourceMain (Join-Path $inputCaseDir "main.tex")
        }
        $manifest.Add([ordered]@{
            paper = $paper
            profile_requested = $profile
            backend = $Backend
            project_root = $projectRootRel
            main_tex = $mainTex
            generated_input = "inputs/$paper/$profile/main.tex"
        })

        $args = @(
            "semantic-convert",
            "--project-root", $projectRootRel,
            "--main-tex", $mainTex,
            "--profile", $profile,
            "--backend", $Backend,
            "--quality", $Quality,
            "--out", $docxPath,
            "--report", $reportPath
        )
        $oldErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        $output = & $exe @args 2>&1
        $exit = $LASTEXITCODE
        $ErrorActionPreference = $oldErrorActionPreference
        $output | Set-Content -Encoding UTF8 $logPath

        $metrics = Inspect-Docx $docxPath
        if (-not (Test-Path $reportPath)) {
            $syntheticReport = [ordered]@{
                synthetic = $true
                reason = "semantic-convert exited before writing a CompileReport; see log for quality gate failure details."
                paper = $paper
                profile_requested = $profile
                backend_requested = $Backend
                exit_code = $exit
                log = "logs/$caseId.log"
                docx = "docx/$caseId.docx"
                docx_metrics = $metrics
                log_excerpt = @($output | Select-Object -Last 20)
            }
            $syntheticReport | ConvertTo-Json -Depth 12 | Set-Content -Encoding UTF8 $reportPath
        }

        $report = $null
        if (Test-Path $reportPath) {
            $report = Get-Content -Raw -Encoding UTF8 $reportPath | ConvertFrom-Json
        }
        $snapshot = @(
            "word_open_check=not-run",
            "reason=Microsoft Word/WPS/LibreOffice automation was not invoked in this run.",
            "zip_openable=$($metrics.zip_openable)",
            "has_document_xml=$($metrics.has_document_xml)",
            "has_styles_xml=$($metrics.has_styles_xml)",
            "has_relationships=$($metrics.has_relationships)",
            "header1_text=$($metrics.header1_text)",
            "header2_text=$($metrics.header2_text)"
        )
        $snapshot | Set-Content -Encoding UTF8 $snapshotPath

        $result = [ordered]@{
            paper = $paper
            profile_requested = $profile
            profile_effective = if ($report -and $report.active_profile) { $report.active_profile.id } elseif ($report) { $report.profile } else { $null }
            backend_requested = $Backend
            backend_selected = if ($report) { $report.backend.selected } else { $null }
            exit_code = $exit
            docx = "docx/$caseId.docx"
            report = "reports/$caseId.report.json"
            log = "logs/$caseId.log"
            snapshot = "snapshots/$caseId.word-open.txt"
            compatibility_score = if ($report) { [int]$report.compatibility.score } else { 0 }
            unresolved_references = if ($report) { [int]$report.unresolved_reference_count } else { 0 }
            image_asset_count = if ($report) { [int]$report.image_asset_count } else { 0 }
            omml_equation_count = if ($report) { [int]$report.omml_equation_count } else { [int]$metrics.omml_equations }
            quality_status = if ($report -and $report.quality_gate) { $report.quality_gate.status } else { "NotAvailable" }
            metrics = $metrics
        }
        $result.grade = Get-Grade $result
        $result.status = if ($result.grade -in @("A", "B", "C", "D")) { "generated" } else { "failed" }
        $result.issues = Get-Issues $result
        $results.Add([pscustomobject]$result)
        Write-Host "$caseId => exit=$exit grade=$($result.grade) score=$($result.compatibility_score) bytes=$($metrics.bytes)"
    }
}

$summary = [ordered]@{
    total = $results.Count
    generated = @($results | Where-Object { $_.status -eq "generated" }).Count
    failed = @($results | Where-Object { $_.status -eq "failed" }).Count
    grade_a_or_b = @($results | Where-Object { $_.grade -in @("A", "B") }).Count
    grade_counts = [ordered]@{
        A = @($results | Where-Object { $_.grade -eq "A" }).Count
        B = @($results | Where-Object { $_.grade -eq "B" }).Count
        C = @($results | Where-Object { $_.grade -eq "C" }).Count
        D = @($results | Where-Object { $_.grade -eq "D" }).Count
        F = @($results | Where-Object { $_.grade -eq "F" }).Count
    }
}

$json = [ordered]@{
    version = "1.0"
    generated_at = (Get-Date).ToString("o")
    output_dir = "examples/output/journal-profile-generalization"
    profiles = $Profiles
    papers = $Papers
    backend = $Backend
    quality = $Quality
    results = $results
    summary = $summary
}

$json | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 (Join-Path $root "verify-summary.json")
$manifest | ConvertTo-Json -Depth 8 | Set-Content -Encoding UTF8 (Join-Path $inputsDir "manifest.json")

$md = New-Object System.Collections.Generic.List[string]
$md.Add("# Paper2/Paper3 journal profile verification summary")
$md.Add("")
$md.Add("- Generated at: $($json.generated_at)")
$md.Add("- Output dir: ``examples/output/journal-profile-generalization``")
$md.Add("- Matrix: $($summary.total) cases; generated $($summary.generated); failed $($summary.failed); A/B $($summary.grade_a_or_b)")
$md.Add("- Grade counts: A=$($summary.grade_counts.A), B=$($summary.grade_counts.B), C=$($summary.grade_counts.C), D=$($summary.grade_counts.D), F=$($summary.grade_counts.F)")
$md.Add("")
$md.Add("| Paper | Requested profile | Effective profile | Backend | Score | Bytes | Para | Tables | Media | OMML | Raw hits | Unresolved | Grade | Issues |")
$md.Add("| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |")
foreach ($r in $results) {
    $issuesText = if ($r.issues.Count -gt 0) { $r.issues -join ", " } else { "" }
    $md.Add("| $($r.paper) | $($r.profile_requested) | $($r.profile_effective) | $($r.backend_selected) | $($r.compatibility_score) | $($r.metrics.bytes) | $($r.metrics.paragraphs) | $($r.metrics.tables) | $($r.metrics.media_files) | $($r.metrics.omml_equations) | $($r.metrics.raw_latex_hits) | $($r.unresolved_references) | $($r.grade) | $issuesText |")
}
$md.Add("")
$md.Add("## Notes")
$md.Add("")
$md.Add("- Word/WPS/LibreOffice visual opening was not automated; snapshots record package-level openability and header text extracted from DOCX XML.")
$md.Add("- ``compatibility-below-plan-threshold`` follows the plan threshold of 70; preview quality was used so reports are still emitted for borderline cases.")
$md.Add("- Profile-specific wrapper generation is represented by copied entry files plus ``inputs/manifest.json``; full template-shell rewriting remains a follow-up task.")
$md | Set-Content -Encoding UTF8 (Join-Path $root "verify-summary.md")

$summaryPath = Join-Path $root "verify-summary.json"
Write-Host "summary: $summaryPath"
