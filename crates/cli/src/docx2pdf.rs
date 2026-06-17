//! `docx-to-pdf` 子命令。
//!
//! 设计见 `docs/study/08-pdf-pipeline/03-docx-to-pdf.md`。

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct DocxToPdfArgs {
    /// 输入 docx
    #[arg(long)]
    pub docx: PathBuf,
    /// 输出目录（PDF 写在 `<outdir>/<docx-stem>.pdf`）
    #[arg(long)]
    pub outdir: PathBuf,
}

pub async fn run(a: DocxToPdfArgs) -> Result<()> {
    std::fs::create_dir_all(&a.outdir).ok();
    let engine =
        doc_docx_pdf::DocxToPdf::probe().map_err(|e| anyhow::anyhow!("docx-pdf 探测失败：{e}"))?;
    let run = engine
        .convert(&a.docx, &a.outdir)
        .await
        .map_err(|e| anyhow::anyhow!("docx → pdf 失败：{e}"))?;
    tracing::info!(
        "docx → pdf 完成：{} → {} ({} ms, {} bytes, pages={})",
        run.docx.display(),
        run.pdf.display(),
        run.elapsed_ms,
        run.file_size,
        run.page_count
    );
    Ok(())
}
