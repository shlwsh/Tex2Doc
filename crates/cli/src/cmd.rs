//! `convert` + `build` 子命令。

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;

use doc_core::options::ConvertOptions;

#[derive(Debug, Args)]
pub struct ConvertArgs {
    /// 包含 .tex 的 zip 路径
    #[arg(long)]
    pub zip: PathBuf,
    /// zip 内的主 .tex 相对路径（POSIX）
    #[arg(long)]
    pub main_tex: String,
    /// 输出 docx 路径
    #[arg(long)]
    pub out: PathBuf,
}

pub fn run_convert(a: ConvertArgs) -> Result<()> {
    let bytes = std::fs::read(&a.zip)
        .with_context(|| format!("读取 zip 失败：{}", a.zip.display()))?;
    let options = ConvertOptions::default();
    let r = doc_core::convert_zip(&bytes, &a.main_tex, &options)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if let Some(parent) = a.out.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&a.out, &r.docx)
        .with_context(|| format!("写 docx 失败：{}", a.out.display()))?;
    tracing::info!("写入 docx：{} ({} bytes)", a.out.display(), r.docx.len());
    Ok(())
}

#[derive(Debug, Args)]
pub struct BuildArgs {
    #[arg(long)]
    pub zip: PathBuf,
    #[arg(long)]
    pub main_tex: String,
    #[arg(long)]
    pub outdir: PathBuf,
    #[arg(long, default_value_t = false)]
    pub skip_visual: bool,
    /// `--latex-main` 在 zip 内相对路径；缺省 = `--main-tex`
    #[arg(long)]
    pub latex_main: Option<String>,
}

pub fn run_build(a: BuildArgs) -> Result<()> {
    use crate::{docx2pdf, pdf_verify, tex_compile};
    std::fs::create_dir_all(&a.outdir).ok();
    let docx = a.outdir.join("out.docx");
    let oracle_pdf = a.outdir.join("out.oracle.pdf");
    let rust_pdf = a.outdir.join("out.pdf");
    let report_md = a.outdir.join("quality-report.md");
    let report_json = a.outdir.join("quality-report.json");

    // 1. zip → docx
    let convert_a = ConvertArgs {
        zip: a.zip.clone(),
        main_tex: a.main_tex.clone(),
        out: docx.clone(),
    };
    run_convert(convert_a)?;

    // 2. tex → oracle PDF（独立 from workdir 准备）
    let tex_a = tex_compile::TexCompileArgs {
        zip: a.zip.clone(),
        main_tex: a.latex_main.clone().unwrap_or_else(|| a.main_tex.clone()),
        out: oracle_pdf.clone(),
        engine: None,
        max_passes: 2,
    };
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(tex_compile::run(tex_a))?;

    // 3. docx → PDF
    let d2p = docx2pdf::DocxToPdfArgs {
        docx: docx.clone(),
        outdir: a.outdir.clone(),
    };
    rt.block_on(docx2pdf::run(d2p))?;

    // 4. 验证
    let v = pdf_verify::VerifyPdfArgs {
        docx: docx.clone(),
        rust_pdf: rust_pdf.clone(),
        oracle_pdf: oracle_pdf.clone(),
        report: report_md,
        json_report: report_json,
        diff_outdir: Some(a.outdir.join("diff")),
        skip_visual: a.skip_visual,
        thresholds: None,
    };
    rt.block_on(pdf_verify::run(v))?;
    Ok(())
}
