use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use doc_quality::{compare_docx, DocxDiffOptions, DocxDiffReport};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DocxDiffFormat {
    Md,
    Json,
}

#[derive(Debug, Args)]
pub struct DocxDiffArgs {
    /// Left/base DOCX path, usually the Rust engine output.
    #[arg(long)]
    pub left: PathBuf,
    /// Right/target DOCX path, usually the sh/oracle output.
    #[arg(long)]
    pub right: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = DocxDiffFormat::Md)]
    pub format: DocxDiffFormat,
    /// Output path.
    #[arg(long)]
    pub out: PathBuf,
    /// Maximum content/format diff rows included in the report.
    #[arg(long, default_value_t = 80)]
    pub max_diffs: usize,
    /// Skip normalized OOXML hash comparison.
    #[arg(long)]
    pub no_xml_hash: bool,
}

pub fn run(a: DocxDiffArgs) -> Result<()> {
    let options = DocxDiffOptions {
        max_diffs: a.max_diffs,
        compare_xml_hash: !a.no_xml_hash,
    };
    let report = compare_docx(&a.left, &a.right, &options).with_context(|| {
        format!(
            "DOCX 对比失败：{} vs {}",
            a.left.display(),
            a.right.display()
        )
    })?;
    write_docx_diff(&report, a.format, &a.out)?;
    tracing::info!("写入 DOCX diff：{}", a.out.display());
    Ok(())
}

fn write_docx_diff(report: &DocxDiffReport, format: DocxDiffFormat, out: &PathBuf) -> Result<()> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建输出目录失败：{}", parent.display()))?;
    }
    match format {
        DocxDiffFormat::Md => {
            std::fs::write(out, report.to_markdown())
                .with_context(|| format!("写 DOCX diff Markdown 失败：{}", out.display()))?;
        }
        DocxDiffFormat::Json => {
            let json = serde_json::to_string_pretty(report)?;
            std::fs::write(out, json)
                .with_context(|| format!("写 DOCX diff JSON 失败：{}", out.display()))?;
        }
    }
    Ok(())
}
