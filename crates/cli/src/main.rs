//! `doc-engine` CLI 工具。

mod ast_dump;
mod cmd;
mod docx2pdf;
mod docx_diff;
mod pdf_verify;
mod render_dump;
mod semantic_cmd;
mod tex_compile;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use doc_compiler_engine::{
    CompatibilityAnalyzer, CompileOptions, EngineProfile,
    JournalDetector, ProfileKind, SemanticBackendKind, SemanticTexEngine,
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
        Cmd::SemanticVerify(_a) => {
            anyhow::bail!("semantic verify is not yet implemented (P6 milestone)")
        }
    }
}

// ── Semantic handlers ────────────────────────────────────────────────────────

fn run_detect(args: semantic_cmd::SemanticDetectArgs) -> Result<()> {
    let vfs = build_vfs(&args.project_root)?;
    let detector = JournalDetector::new();
    let report = detector.detect(&vfs);

    if let Some(output_path) = &args.output {
        let json = serde_json::to_string_pretty(&report)?;
        std::fs::write(output_path, json)?;
    }

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

    Ok(())
}

type CompatReport = doc_compiler_engine::CompatibilityReport;

fn run_analyze(args: semantic_cmd::SemanticAnalyzeArgs) -> Result<CompatReport> {
    let vfs = build_vfs(&args.project_root)?;
    let profile = parse_profile_kind(&args.profile)?;
    let analyzer = CompatibilityAnalyzer::default();
    let report = analyzer.analyze(&vfs, profile);

    if let Some(output_path) = &args.output {
        let json = serde_json::to_string_pretty(&report)?;
        std::fs::write(output_path, json)?;
    }

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

    Ok(report)
}

fn run_convert(args: semantic_cmd::SemanticConvertArgs) -> Result<()> {
    let profile = parse_engine_profile(&args.profile)?;
    let backend = parse_backend(&args.backend)?;

    let options = CompileOptions {
        profile,
        semantic_backend: backend,
        allow_backend_fallback: !args.no_backend_fallback,
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
    let artifact = engine.compile_dir_to_docx(
        &args.project_root,
        &PathBuf::from(main_tex),
        &options,
    )?;

    std::fs::write(&args.out, &artifact.docx)
        .with_context(|| format!("failed to write DOCX: {}", args.out.display()))?;

    println!("docx: {}", args.out.display());
    println!("profile: {}", artifact.report.profile_spec.id);
    println!("compatibility-score: {}", artifact.report.compatibility.score);
    println!(
        "backend: {} (requested: {})",
        artifact.report.backend.selected.id(),
        artifact.report.backend.requested.id()
    );

    if let Some(report_path) = &args.report {
        let json = serde_json::to_string_pretty(&artifact.report)?;
        std::fs::write(report_path, json)?;
        println!("report: {}", report_path.display());
    }

    Ok(())
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

fn parse_engine_profile(raw: &str) -> Result<EngineProfile> {
    match raw {
        "generic" | "generic-article" | "auto" => Ok(EngineProfile::GenericArticle),
        "chinese" | "chinese-academic" => Ok(EngineProfile::ChineseAcademic),
        "jos" | "jos-paper" => Ok(EngineProfile::JosPaper),
        "medical" | "medical-journal" => Ok(EngineProfile::MedicalJournal),
        "tacl" | "cvpr" => Ok(EngineProfile::JosPaper),
        "nature" | "springer" => Ok(EngineProfile::GenericArticle),
        other => Err(anyhow::anyhow!(
            "unsupported profile '{other}'. Available: auto, generic, chinese, jos, tacl, cvpr, nature, springer"
        )),
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
