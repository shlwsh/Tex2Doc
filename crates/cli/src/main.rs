//! `doc-engine` CLI 工具。
//!
//! 设计见 `docs/study/08-pdf-pipeline/` + `docs/study/08-pdf-pipeline/05-implementation-roadmap.md` §5.6。
//!
//! 子命令：
//!
//! - `convert`         — 沿用 V1 `doc-core`（zip → docx）
//! - `tex-compile`     — V2 路径 A（TeX → oracle PDF）
//! - `docx-to-pdf`     — V2 路径 B（DOCX → PDF）
//! - `verify-pdf`      — V2 路径 C（结构 / 文本 / 视觉 三层质量对比 + 报告）
//! - `build`           — 一键串联：tex-compile → doc-core convert → docx-to-pdf → verify-pdf

mod ast_dump;
mod cmd;
mod docx2pdf;
mod docx_diff;
mod pdf_verify;
mod render_dump;
mod tex_compile;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "doc-engine",
    version,
    about = "Doc-engine CLI (V2 Tex2Doc engine)"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// V1：zip → docx（沿用 `doc-core`）
    Convert(cmd::ConvertArgs),
    /// V2 路径 A：tex → oracle PDF
    TexCompile(tex_compile::TexCompileArgs),
    /// V2 路径 B：docx → PDF（默认 LibreOffice）
    DocxToPdf(docx2pdf::DocxToPdfArgs),
    /// V2 路径 C：三层质量对比（结构 / 文本 / 视觉）+ 报告
    VerifyPdf(pdf_verify::VerifyPdfArgs),
    /// 一键：tex-compile → convert → docx-to-pdf → verify-pdf
    Build(cmd::BuildArgs),
    /// 输出标准文档 AST（Markdown/JSON），用于人工核验 TeX 结构提炼结果
    AstDump(ast_dump::AstDumpArgs),
    /// 输出 DOCX 渲染树（Markdown/JSON），用于核验 AST 到 OOXML 的映射
    RenderDump(render_dump::RenderDumpArgs),
    /// 对比两个 DOCX 的内容、段落样式、run 格式与规范化 OOXML hash
    DocxDiff(docx_diff::DocxDiffArgs),
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_target(false)
        .init();

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Convert(a) => cmd::run_convert(a),
        Cmd::TexCompile(a) => tokio::runtime::Runtime::new()?.block_on(tex_compile::run(a)),
        Cmd::DocxToPdf(a) => tokio::runtime::Runtime::new()?.block_on(docx2pdf::run(a)),
        Cmd::VerifyPdf(a) => tokio::runtime::Runtime::new()?.block_on(pdf_verify::run(a)),
        Cmd::Build(a) => cmd::run_build(a),
        Cmd::AstDump(a) => ast_dump::run(a),
        Cmd::RenderDump(a) => render_dump::run(a),
        Cmd::DocxDiff(a) => docx_diff::run(a),
    }
}
