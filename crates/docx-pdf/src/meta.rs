//! PDF 元数据解析（页数 / 嵌入字体 / ToUnicode）。
//!
//! 设计见 `docs/study/08-pdf-pipeline/03-docx-to-pdf.md` §3.7。

use std::path::Path;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct PdfMeta {
    pub page_count: u32,
    pub file_size: u64,
    /// 去重后的嵌入字体名。
    pub embedded_fonts: Vec<String>,
    /// 至少一个 CMap ToUnicode（决定能否复制粘贴 CJK）。
    pub has_tounicode: bool,
}

/// 同步读取 PDF 元数据。
///
/// 阻塞：在异步上下文里请用 `tokio::task::spawn_blocking` 包一层。
pub fn inspect(pdf: &Path) -> Result<PdfMeta> {
    let doc = lopdf::Document::load(pdf)
        .with_context(|| format!("lopdf 打开 PDF 失败：{}", pdf.display()))?;
    let page_count = doc.get_pages().len() as u32;
    let file_size = std::fs::metadata(pdf)
        .with_context(|| format!("stat 失败：{}", pdf.display()))?
        .len();

    let mut fonts: std::collections::BTreeSet<String> = Default::default();
    let mut tounicode = false;
    for (_id, obj) in &doc.objects {
        if let Ok(dict) = obj.as_dict() {
            if let Ok(name) = dict.get(b"BaseFont") {
                if let Ok(name) = name.as_name() {
                    fonts.insert(String::from_utf8_lossy(name).into_owned());
                } else if let Ok(s) = name.as_str() {
                    // 兜底：某些 PDF 把 BaseFont 写成字面量
                    fonts.insert(String::from_utf8_lossy(s).into_owned());
                }
            }
            if !tounicode {
                if let Ok(to_unicode) = dict.get(b"ToUnicode") {
                    // 任何引用即视为有 ToUnicode
                    if !matches!(to_unicode, lopdf::Object::Null) {
                        tounicode = true;
                    }
                }
            }
        }
    }
    Ok(PdfMeta {
        page_count,
        file_size,
        embedded_fonts: fonts.into_iter().collect(),
        has_tounicode: tounicode,
    })
}

/// 异步包装：内部用 `spawn_blocking` 跑 `inspect`。
pub async fn inspect_async(pdf: &Path) -> Result<PdfMeta> {
    let pdf = pdf.to_path_buf();
    tokio::task::spawn_blocking(move || inspect(&pdf))
        .await
        .context("PDF meta inspect join 失败")?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_missing_pdf_returns_error() {
        let p = std::path::PathBuf::from("Z:/__definitely_missing__.pdf");
        let r = inspect(&p);
        assert!(r.is_err(), "不存在的 PDF 应返回 Err");
    }
}
