//! `doc-engine` CLI 工具。

mod ast_dump;
mod cmd;
mod docx2pdf;
mod docx_diff;
mod pdf_verify;
mod render_dump;
mod semantic_cmd;
mod tex_compile;

use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use doc_compiler_engine::{
    CompatibilityAnalyzer, CompileOptions,
    JournalDetector, ProfileKind, ProfileRef, SemanticBackendKind, SemanticTexEngine,
};
use doc_utils::VirtualFs;
use tracing_subscriber::EnvFilter;

#[derive(Debug, clap::Parser)]
#[command(name = "doc-engine", version, about = "Doc-engine CLI (V2 Tex2Doc engine)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, clap::Subcommand)]
enum Cmd {
    Convert(cmd::ConvertArgs),
    TexCompile(tex_compile::TexCompileArgs),
    DocxToPdf(docx2pdf::DocxToPdfArgs),
    VerifyPdf(pdf_verify::VerifyPdfArgs),
    Build(cmd::BuildArgs),
    AstDump(ast_dump::AstDumpArgs),
    RenderDump(render_dump::RenderDumpArgs),
    DocxDiff(docx_diff::DocxDiffArgs),
    /// 检测 TeX 项目的期刊 Profile
    SemanticDetect(semantic_cmd::SemanticDetectArgs),
    /// 分析 TeX 项目的兼容性
    SemanticAnalyze(semantic_cmd::SemanticAnalyzeArgs),
    /// TeX 项目转换为 DOCX（Semantic Engine）
    SemanticConvert(semantic_cmd::SemanticConvertArgs),
    /// 验证 DOCX 质量（结构 / 引用 / 样式）
    SemanticVerify(semantic_cmd::SemanticVerifyArgs),
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_target(false)
        .init();

    match Cli::parse().cmd {
        Cmd::Convert(a) => cmd::run_convert(a),
        Cmd::TexCompile(a) => tokio::runtime::Runtime::new()?.block_on(tex_compile::run(a)),
        Cmd::DocxToPdf(a) => tokio::runtime::Runtime::new()?.block_on(docx2pdf::run(a)),
        Cmd::VerifyPdf(a) => tokio::runtime::Runtime::new()?.block_on(pdf_verify::run(a)),
        Cmd::Build(a) => cmd::run_build(a),
        Cmd::AstDump(a) => ast_dump::run(a),
        Cmd::RenderDump(a) => render_dump::run(a),
        Cmd::DocxDiff(a) => docx_diff::run(a),
        Cmd::SemanticDetect(a) => run_detect(a),
        Cmd::SemanticAnalyze(a) => run_analyze(a).map(|_| ()),
        Cmd::SemanticConvert(a) => run_convert(a),
        Cmd::SemanticVerify(a) => run_verify(a),
    }
}

// ── Semantic handlers ────────────────────────────────────────────────────────

fn run_detect(args: semantic_cmd::SemanticDetectArgs) -> Result<()> {
    let vfs = build_vfs(&args.project_root)?;
    let detector = JournalDetector::new();
    let report = detector.detect(&vfs);

    if args.json || args.output.is_some() {
        let json = serde_json::to_string_pretty(&report)?;
        if args.json {
            println!("{}", json);
        }
        if let Some(output_path) = &args.output {
            std::fs::write(output_path, json)?;
        }
    } else {
        println!("profile: {}", report.selected_profile_id);
        println!("confidence: {:.2}", report.confidence);

        if !report.diagnostics.is_empty() {
            println!("\nDiagnostics:");
            for d in &report.diagnostics {
                println!("  [{:?}] {}: {}", d.level, d.code, d.message);
            }
        }

        if !report.candidates.is_empty() {
            println!("\nAll candidates:");
            for c in &report.candidates {
                println!("  {}: {:.2}", c.profile_id, c.confidence);
            }
        }
    }

    Ok(())
}

type CompatReport = doc_compiler_engine::CompatibilityReport;

fn run_analyze(args: semantic_cmd::SemanticAnalyzeArgs) -> Result<CompatReport> {
    let vfs = build_vfs(&args.project_root)?;
    let profile = parse_profile_kind(&args.profile)?;
    let analyzer = CompatibilityAnalyzer::default();
    let report = analyzer.analyze(&vfs, profile);

    if args.json || args.output.is_some() {
        let json = serde_json::to_string_pretty(&report)?;
        if args.json {
            println!("{}", json);
        }
        if let Some(output_path) = &args.output {
            std::fs::write(output_path, json)?;
        }
    } else {
        println!("compatibility-score: {}", report.score);
        println!("scanned-files: {}", report.scanned_files);
        println!("document-classes: {}", report.document_classes.join(", "));
        println!("packages: {}", report.packages.join(", "));
        println!("custom-macros: {}", report.custom_macro_count);
        println!("unsupported: {}", report.unsupported.len());
        println!("warnings: {}", report.warnings.len());

        if !report.unsupported.is_empty() {
            println!("\nUnsupported features:");
            for issue in &report.unsupported {
                println!("  [{}] {}", issue.code, issue.message);
            }
        }
        if !report.warnings.is_empty() {
            println!("\nWarnings:");
            for issue in &report.warnings {
                println!("  [{}] {}", issue.code, issue.message);
            }
        }
    }

    Ok(report)
}

fn run_convert(args: semantic_cmd::SemanticConvertArgs) -> Result<()> {
    // P1: Use ProfileRef for all profile IDs, including tacl/cvpr/nature/springer
    let profile_ref = parse_profile_ref(&args.profile)?;
    let backend = parse_backend(&args.backend)?;

    // P4.3: Parse --quality level and compute min_score override
    let quality_level: semantic_cmd::QualityLevel = args.quality.parse()
        .map_err(|e: String| anyhow::anyhow!("--quality: {}", e))?;
    let min_score_override = match quality_level {
        semantic_cmd::QualityLevel::Preview => 60,
        semantic_cmd::QualityLevel::Standard => 75,
        semantic_cmd::QualityLevel::Strict => 90,
    };

    let options = CompileOptions {
        profile_ref: Some(profile_ref),
        semantic_backend: backend,
        allow_backend_fallback: !args.no_backend_fallback,
        min_compatibility_score_override: Some(min_score_override),
        ..CompileOptions::default()
    };

    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let main_tex = args
        .main_tex
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("main_tex must be a valid string"))?;

    let engine = SemanticTexEngine::new();
    let artifact = match engine.compile_dir_to_docx(
        &args.project_root,
        &PathBuf::from(main_tex),
        &options,
    ) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Conversion failed: {}", e);
            std::process::exit(semantic_cmd::CliExitCode::EConvertFailed.code());
        }
    };

    std::fs::write(&args.out, &artifact.docx)
        .with_context(|| format!("failed to write DOCX: {}", args.out.display()))?;

    let report = artifact.report;
    let active_profile_id = report
        .active_profile
        .as_ref()
        .map(|ap| ap.id.clone())
        .unwrap_or_else(|| report.profile.id().to_string());
    let compatibility_score = report.compatibility.score;
    let backend_id = report.backend.selected.id().to_string();
    let backend_requested_id = report.backend.requested.id().to_string();

    // P4.2: Quality gate error codes
    if let Some(ref qg) = report.quality_gate {
        match qg.status {
            doc_compiler_engine::QualityStatus::Failed => {
                eprintln!("Quality gate FAILED: {} check(s) failed", qg.failed_checks.len());
                for check in &qg.failed_checks {
                    eprintln!("  [{}] {}: {}", check.severity.as_str(), check.name, check.message);
                }
                std::process::exit(semantic_cmd::CliExitCode::EQualityFailed.code());
            }
            doc_compiler_engine::QualityStatus::PassedWithWarnings => {
                if !args.json {
                    eprintln!("Quality gate: passed with {} warning(s)", qg.warnings.len());
                }
            }
            doc_compiler_engine::QualityStatus::Passed => {}
        }
    }

    if args.json {
        #[derive(serde::Serialize)]
        struct ConvertOutput {
            docx: String,
            profile: String,
            detected_profile: String,
            compatibility_score: u8,
            backend: String,
            backend_requested: String,
            report: doc_compiler_engine::CompileReport,
        }
        let output = ConvertOutput {
            docx: args.out.display().to_string(),
            profile: active_profile_id.clone(),
            detected_profile: report
                .journal_detection
                .as_ref()
                .map(|d| d.selected_profile_id.clone())
                .unwrap_or_else(|| active_profile_id.clone()),
            compatibility_score,
            backend: backend_id,
            backend_requested: backend_requested_id,
            report: report.clone(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("docx: {}", args.out.display());
        println!("profile: {}", active_profile_id);
        println!("detected_profile: {}",
            report.journal_detection.as_ref().map(|d| d.selected_profile_id.clone()).unwrap_or_else(|| active_profile_id.clone()));
        println!("compatibility-score: {}", compatibility_score);
        println!("backend: {} (requested: {})", backend_id, backend_requested_id);
        if let Some(ref qg) = report.quality_gate {
            println!("quality-gate: {} ({}/{} checks passed)", qg.status.as_str(), qg.passed_checks, qg.total_checks);
        }
    }

    if let Some(report_path) = &args.report {
        let json = serde_json::to_string_pretty(&report)?;
        std::fs::write(report_path, json)?;
        if !args.json {
            println!("report: {}", report_path.display());
        }
    }

    Ok(())
}

// ── Semantic verify (P0) ─────────────────────────────────────────────────────

use quick_xml::events::Event;
use quick_xml::Reader;

/// P0: DOCX structural quality verification.
/// Full quality layers (structural/textual/visual) are wired in P3/P4.
fn run_verify(args: semantic_cmd::SemanticVerifyArgs) -> Result<()> {
    let docx_path = &args.docx_file;
    if !docx_path.exists() {
        anyhow::bail!("DOCX not found: {}", docx_path.display());
    }

    // Read ZIP contents
    let docx_bytes = std::fs::read(docx_path)?;
    let zip_bytes: &[u8] = &docx_bytes;
    let mut zip_reader =
        zip::ZipArchive::new(std::io::Cursor::new(zip_bytes))
            .map_err(|e| anyhow::anyhow!("failed to open DOCX as ZIP: {}", e))?;

    // Extract document.xml
    let doc_xml = extract_docx_part(&mut zip_reader, "word/document.xml")
        .map_err(|e| anyhow::anyhow!("failed to extract document.xml: {}", e))?;

    let styles_xml = extract_docx_part(&mut zip_reader, "word/styles.xml").ok();
    let rels_xml = extract_docx_part(&mut zip_reader, "word/_rels/document.xml.rels").ok();

    // Count elements via quick XML scan
    let mut xml_reader = Reader::from_str(&doc_xml);
    xml_reader.config_mut().trim_text(true);

    let mut table_count = 0usize;
    let mut image_count = 0usize;
    let mut para_count = 0usize;
    let mut buf = Vec::new();

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let name = e.name();
                let raw = std::str::from_utf8(name.as_ref()).unwrap_or("");
                let local = raw.split(':').last().unwrap_or(raw);
                match local {
                    "tbl" => table_count += 1,
                    "drawing" | "pict" => image_count += 1,
                    "p" => para_count += 1,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    // Extract docx text for character count
    let docx_text = extract_docx_text(&doc_xml);
    let docx_chars = docx_text.chars().count();

    // Build checks
    let mut checks = Vec::new();

    // File size check
    let file_size = docx_bytes.len();
    checks.push(VerifyCheck {
        name: "file_size".to_string(),
        passed: file_size > 1024,
        severity: "error".to_string(),
        message: format!("{} bytes (min 1024)", file_size),
    });

    // styles.xml presence
    let has_styles = styles_xml.is_some();
    checks.push(VerifyCheck {
        name: "styles_present".to_string(),
        passed: has_styles,
        severity: "error".to_string(),
        message: if has_styles { "present".to_string() } else { "missing".to_string() },
    });

    // document.xml presence
    checks.push(VerifyCheck {
        name: "document_present".to_string(),
        passed: true,
        severity: "error".to_string(),
        message: "present".to_string(),
    });

    // rels presence
    let has_rels = rels_xml.is_some();
    checks.push(VerifyCheck {
        name: "rels_present".to_string(),
        passed: has_rels,
        severity: "error".to_string(),
        message: if has_rels { "present".to_string() } else { "missing".to_string() },
    });

    // Table count
    checks.push(VerifyCheck {
        name: "table_count".to_string(),
        passed: true, // Tables are optional
        severity: "info".to_string(),
        message: format!("{}", table_count),
    });

    // Image count
    checks.push(VerifyCheck {
        name: "image_count".to_string(),
        passed: true, // Images are optional
        severity: "info".to_string(),
        message: format!("{}", image_count),
    });

    // Paragraph count (info only - documents can have zero paragraphs)
    checks.push(VerifyCheck {
        name: "paragraph_count".to_string(),
        passed: true,
        severity: "info".to_string(),
        message: format!("{}", para_count),
    });

    // Character count
    checks.push(VerifyCheck {
        name: "docx_char_count".to_string(),
        passed: docx_chars > 0,
        severity: "info".to_string(),
        message: format!("{}", docx_chars),
    });

    let all_passed = checks.iter().filter(|c| c.severity == "error").all(|c| c.passed);
    let passed_count = checks.iter().filter(|c| c.passed).count();
    let total_count = checks.len();

    #[derive(serde::Serialize)]
    struct VerifyReport {
        version: String,
        docx: String,
        passed: bool,
        total_checks: usize,
        passed_checks: usize,
        failed_checks: usize,
        checks: Vec<VerifyCheck>,
    }

    #[derive(serde::Serialize)]
    struct VerifyCheck {
        name: String,
        passed: bool,
        severity: String,
        message: String,
    }

    let report = VerifyReport {
        version: "1.0".to_string(),
        docx: docx_path.display().to_string(),
        passed: all_passed,
        total_checks: total_count,
        passed_checks: passed_count,
        failed_checks: total_count - passed_count,
        checks,
    };

    if args.json || args.report.is_some() {
        let json = serde_json::to_string_pretty(&report)?;
        if args.json {
            println!("{}", json);
        }
        if let Some(report_path) = &args.report {
            std::fs::write(report_path, json)?;
        }
    } else {
        println!("passed: {}", all_passed);
        println!("checks: {}/{}", passed_count, total_count);
        for check in &report.checks {
            let status = if check.passed { "PASS" } else { "FAIL" };
            println!("  [{}] {}: {}", status, check.name, check.message);
        }
    }

    if !all_passed {
        std::process::exit(1);
    }
    Ok(())
}

/// Extract a file from a DOCX (ZIP archive).
fn extract_docx_part<R: std::io::Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    entry: &str,
) -> Result<String> {
    let mut f = zip
        .by_name(entry)
        .map_err(|_| anyhow::anyhow!("missing entry: {}", entry))?;
    let mut s = String::new();
    f.read_to_string(&mut s)
        .map_err(|e| anyhow::anyhow!("read failed: {}", e))?;
    Ok(s)
}

/// Strip XML tags to get plain text from document.xml.
fn extract_docx_text(doc_xml: &str) -> String {
    let mut reader = Reader::from_str(doc_xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut out = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                if let Ok(t) = e.unescape() {
                    let s = t.trim();
                    if !s.is_empty() {
                        out.push_str(s);
                        out.push(' ');
                    }
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    out
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn build_vfs(project_root: &Path) -> Result<VirtualFs> {
    let mut vfs = VirtualFs::new();
    vfs.mount_dir(project_root)
        .map_err(|e| anyhow::anyhow!(
            "failed to scan project directory {}: {}",
            project_root.display(),
            e
        ))?;
    Ok(vfs)
}

fn parse_profile_ref(raw: &str) -> Result<ProfileRef> {
    match raw {
        "auto" => Ok(ProfileRef::Auto),
        "generic" | "generic-article" => Ok(ProfileRef::Id("generic-article".to_string())),
        "chinese" | "chinese-academic" => Ok(ProfileRef::Id("chinese-academic".to_string())),
        "jos" | "jos-paper" => Ok(ProfileRef::Id("jos-paper".to_string())),
        "medical" | "medical-journal" => Ok(ProfileRef::Id("medical-journal".to_string())),
        "tacl" => Ok(ProfileRef::Id("tacl".to_string())),
        "cvpr" => Ok(ProfileRef::Id("cvpr".to_string())),
        "nature" => Ok(ProfileRef::Id("nature".to_string())),
        "springer" => Ok(ProfileRef::Id("springer".to_string())),
        // Path-based: if it looks like a file path
        other => {
            let p = PathBuf::from(other);
            if p.exists() {
                Ok(ProfileRef::Path(p))
            } else {
                // Treat as profile ID
                Ok(ProfileRef::Id(other.to_string()))
            }
        }
    }
}

fn parse_profile_kind(raw: &str) -> Result<ProfileKind> {
    match raw {
        "generic" | "generic-article" => Ok(ProfileKind::Generic),
        "chinese" | "chinese-academic" => Ok(ProfileKind::ChineseAcademic),
        "jos" | "jos-paper" => Ok(ProfileKind::JosPaper),
        "medical" | "medical-journal" => Ok(ProfileKind::MedicalJournal),
        "tacl" => Ok(ProfileKind::Tacl),
        "cvpr" => Ok(ProfileKind::Cvpr),
        "nature" => Ok(ProfileKind::Nature),
        "springer" => Ok(ProfileKind::Springer),
        other => Err(anyhow::anyhow!(
            "unsupported profile '{other}'. Available: generic, chinese, jos, medical, tacl, cvpr, nature, springer"
        )),
    }
}

fn parse_backend(raw: &str) -> Result<SemanticBackendKind> {
    match raw {
        "auto" => Ok(SemanticBackendKind::Auto),
        "rule" | "rule-based" => Ok(SemanticBackendKind::RuleBased),
        "xelatex" | "xelatex-hook" => Ok(SemanticBackendKind::XeLaTeXHook),
        "lualatex" | "luatex" | "luatex-node" => Ok(SemanticBackendKind::LuaTeXNode),
        other => Err(anyhow::anyhow!(
            "unsupported backend '{other}'. Available: auto, rule-based, xelatex-hook, luatex-node"
        )),
    }
}
