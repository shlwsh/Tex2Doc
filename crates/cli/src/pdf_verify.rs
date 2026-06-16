//! `verify-pdf` 子命令：三层质量对比。
//!
//! 设计见 `docs/study/08-pdf-pipeline/04-quality-comparison.md`。

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;

use doc_quality::{
    context::{read_pdf_meta, read_pdf_text},
    Context as QualityContext, Quality, QualityReport, Thresholds,
};

#[derive(Debug, Args)]
pub struct VerifyPdfArgs {
    #[arg(long)]
    pub docx: PathBuf,
    #[arg(long)]
    pub rust_pdf: PathBuf,
    #[arg(long)]
    pub oracle_pdf: PathBuf,
    #[arg(long)]
    pub report: PathBuf,
    #[arg(long)]
    pub json_report: PathBuf,
    #[arg(long)]
    pub diff_outdir: Option<PathBuf>,
    /// 跳过视觉层（结构+文本仍跑）
    #[arg(long, default_value_t = false)]
    pub skip_visual: bool,
    /// 加载阈值 JSON
    #[arg(long)]
    pub thresholds: Option<PathBuf>,
}

pub async fn run(a: VerifyPdfArgs) -> Result<()> {
    // 1. 准备 QualityContext
    let mut ctx = QualityContext::new(a.docx.clone(), a.rust_pdf.clone(), a.oracle_pdf.clone());
    ctx.rust_pdf_meta = read_pdf_meta(&a.rust_pdf)?;
    ctx.oracle_pdf_meta = read_pdf_meta(&a.oracle_pdf)?;
    ctx.rust_text = read_pdf_text(&a.rust_pdf)?;
    ctx.oracle_text = read_pdf_text(&a.oracle_pdf)?;
    ctx.docx_text = read_docx_text(&a.docx)?;
    ctx.docx_paragraphs = count_paragraphs(&a.docx)?;

    // 2. 阈值
    let thresholds = if let Some(ref p) = a.thresholds {
        let bytes = std::fs::read(p)
            .with_context(|| format!("读阈值 JSON 失败：{}", p.display()))?;
        serde_json::from_slice::<Thresholds>(&bytes)
            .with_context(|| format!("解析阈值 JSON 失败：{}", p.display()))?
    } else {
        Thresholds::default()
    };

    // 3. 跑三层
    let mut q = Quality::new(thresholds);
    if let Some(outdir) = a.diff_outdir.as_ref() {
        q.visual.diff_outdir = Some(outdir.clone());
    }
    if a.skip_visual {
        // 移除 VisualRunner：用占位 runner 直接给空 layer。
        let report = run_struct_text_only(&q, &ctx).await?;
        write_reports(&a, &report)?;
        std::process::exit(report.exit_code);
    } else {
        let report = q.run_all(&ctx).await?;
        write_reports(&a, &report)?;
        tracing::info!(
            "verify-pdf 完成：exit_code={} passed={}",
            report.exit_code,
            report.passed
        );
        std::process::exit(report.exit_code);
    }
}

async fn run_struct_text_only(q: &Quality, ctx: &QualityContext) -> Result<QualityReport> {
    use doc_quality::{
        compute_exit_code, markers, normalize::normalize, Layer, LayerResult,
    };
    let s = q.run_layer(Layer::Structural, ctx).await?;
    let t = q.run_layer(Layer::Textual, ctx).await?;
    let v = LayerResult::new(Layer::Visual, vec![]);
    let layers = vec![s, t, v];
    let passed = layers.iter().all(|l| l.passed);
    let exit_code = compute_exit_code(&layers);
    let mut report = QualityReport {
        docx: ctx.docx.clone(),
        rust_pdf: ctx.rust_pdf.clone(),
        oracle_pdf: ctx.oracle_pdf.clone(),
        passed,
        exit_code,
        layer_results: layers,
        marker_coverage: markers::coverage(
            &ctx.docx_text,
            &ctx.oracle_text,
            &ctx.rust_text,
        ),
        docx_chars: normalize(&ctx.docx_text).chars().count(),
        rust_pdf_chars: normalize(&ctx.rust_text).chars().count(),
        oracle_pdf_chars: normalize(&ctx.oracle_text).chars().count(),
        char_ratio_docx_to_oracle: 0.0,
        char_ratio_rust_to_oracle: 0.0,
        paragraphs: ctx.docx_paragraphs,
    };
    if report.oracle_pdf_chars > 0 {
        report.char_ratio_docx_to_oracle =
            report.docx_chars as f64 / report.oracle_pdf_chars as f64;
        report.char_ratio_rust_to_oracle =
            report.rust_pdf_chars as f64 / report.oracle_pdf_chars as f64;
    }
    Ok(report)
}

fn write_reports(a: &VerifyPdfArgs, report: &QualityReport) -> Result<()> {
    if let Some(p) = a.report.parent() {
        std::fs::create_dir_all(p).ok();
    }
    doc_quality::write_markdown(report, &a.report)
        .with_context(|| format!("写 MD 报告失败：{}", a.report.display()))?;
    doc_quality::write_json(report, &a.json_report)
        .with_context(|| format!("写 JSON 报告失败：{}", a.json_report.display()))?;
    tracing::info!(
        "报告：md={} json={}",
        a.report.display(),
        a.json_report.display()
    );
    Ok(())
}

fn read_docx_text(docx: &std::path::Path) -> Result<String> {
    use std::io::Read;
    let bytes = std::fs::read(docx)?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    let mut f = zip.by_name("word/document.xml")?;
    let mut xml = String::new();
    f.read_to_string(&mut xml)?;
    Ok(extract_text_from_docx_xml(&xml))
}

fn count_paragraphs(docx: &std::path::Path) -> Result<usize> {
    use std::io::Read;
    let bytes = std::fs::read(docx)?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    let mut f = zip.by_name("word/document.xml")?;
    let mut xml = String::new();
    f.read_to_string(&mut xml)?;
    Ok(xml.matches("<w:p ").count() + xml.matches("<w:p>").count())
}

fn extract_text_from_docx_xml(xml: &str) -> String {
    let mut out = String::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<w:t") {
        let after_tag_open = &rest[start..];
        let close = after_tag_open.find('>').unwrap_or(0);
        let body = &after_tag_open[close + 1..];
        if let Some(end) = body.find("</w:t>") {
            out.push_str(&body[..end]);
            rest = &body[end + 6..];
        } else {
            break;
        }
    }
    out
}
