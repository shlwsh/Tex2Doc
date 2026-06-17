//! 文本抽取：pdftotext > mutool。
//!
//! 设计见 `docs/study/08-pdf-pipeline/02-tex-facade.md` §2.9。

use std::path::Path;

use anyhow::{Context, Result};
use tokio::process::Command;

use crate::error::TexError;

/// 抽取 `pdf` 的纯文本。
///
/// 优先级：`pdftotext` > `mutool` > `Err(TexError::NoTextExtractor)`。
pub async fn extract_text(pdf: &Path) -> Result<String> {
    if let Ok(pdftotext) = which::which("pdftotext") {
        let out = Command::new(&pdftotext)
            .arg(pdf)
            .arg("-")
            .output()
            .await
            .with_context(|| format!("启动 pdftotext 失败：{}", pdftotext.display()))?;
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
        }
    }
    if let Ok(mutool) = which::which("mutool") {
        let out = Command::new(&mutool)
            .args(["convert", "-F", "text", "-o", "-"])
            .arg(pdf)
            .output()
            .await
            .with_context(|| format!("启动 mutool 失败：{}", mutool.display()))?;
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
        }
    }
    Err(TexError::NoTextExtractor.into())
}

/// 探测可用文本抽取器（按优先级返回第一个存在的）。
///
/// 测试用：可以注入 `PATH` 改变探测结果。
pub fn detect_extractor() -> Option<&'static str> {
    if which::which("pdftotext").is_ok() {
        Some("pdftotext")
    } else if which::which("mutool").is_ok() {
        Some("mutool")
    } else {
        None
    }
}
