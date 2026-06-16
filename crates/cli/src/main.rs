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

mod cmd;
mod pdf_verify;
mod tex_compile;
mod docx2pdf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "doc-engine", version, about = "Doc-engine CLI (V1 + V2 PDF pipeline)")]
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
    }
}
