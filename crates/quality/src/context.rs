//! 跨层运行所需的输入。

use std::path::PathBuf;

use crate::layer::MarkerHit;
use crate::QualityError;

/// PDF 元数据快照（结构层 / 文本层会读它）。
#[derive(Debug, Clone, Default)]
pub struct PdfMetaSnapshot {
    pub page_count: u32,
    pub file_size: u64,
    pub embedded_fonts: Vec<String>,
    pub has_tounicode: bool,
}

/// 顶层运行上下文：一次性把所有文件路径 + 抽取出的纯文本准备好，
/// 三层 Runner 共享。
#[derive(Debug, Clone)]
pub struct Context {
    /// V1 产出的 docx。
    pub docx: PathBuf,
    /// V2 docx→pdf 产出的 rust PDF。
    pub rust_pdf: PathBuf,
    /// V2 tex→pdf 产出的 oracle PDF。
    pub oracle_pdf: PathBuf,
    /// docx 全文本（已 normalize）。
    pub docx_text: String,
    /// oracle PDF 全文本。
    pub oracle_text: String,
    /// rust PDF 全文本。
    pub rust_text: String,
    /// docx 段落数。
    pub docx_paragraphs: usize,
    /// rust PDF / oracle PDF 元数据快照。
    pub rust_pdf_meta: PdfMetaSnapshot,
    pub oracle_pdf_meta: PdfMetaSnapshot,
    /// 22 marker 命中矩阵（V2 三列）。
    pub marker_hits: Vec<MarkerHit>,
}

impl Context {
    /// 构造最小可用 Context（仅路径；文本/元数据由各 Runner 自行按需填充）。
    pub fn new(docx: PathBuf, rust_pdf: PathBuf, oracle_pdf: PathBuf) -> Self {
        Self {
            docx,
            rust_pdf,
            oracle_pdf,
            docx_text: String::new(),
            oracle_text: String::new(),
            rust_text: String::new(),
            docx_paragraphs: 0,
            rust_pdf_meta: PdfMetaSnapshot::default(),
            oracle_pdf_meta: PdfMetaSnapshot::default(),
            marker_hits: Vec::new(),
        }
    }
}

/// 同步读 PDF 元数据（不抽文本）。
pub fn read_pdf_meta(pdf: &std::path::Path) -> Result<PdfMetaSnapshot, QualityError> {
    use lopdf::Object;
    let doc = lopdf::Document::load(pdf)
        .map_err(|e| QualityError::PdfMeta(format!("lopdf load 失败：{e}")))?;
    let page_count = doc.get_pages().len() as u32;
    let file_size = std::fs::metadata(pdf)
        .map_err(|e| QualityError::Io(e))?
        .len();
    let mut fonts: std::collections::BTreeSet<String> = Default::default();
    let mut tounicode = false;
    for (_id, obj) in &doc.objects {
        if let Ok(dict) = obj.as_dict() {
            if let Ok(name) = dict.get(b"BaseFont") {
                if let Ok(name) = name.as_name() {
                    fonts.insert(String::from_utf8_lossy(name).into_owned());
                } else if let Ok(s) = name.as_str() {
                    fonts.insert(String::from_utf8_lossy(s).into_owned());
                }
            }
            if !tounicode {
                if let Ok(to_unicode) = dict.get(b"ToUnicode") {
                    if !matches!(to_unicode, Object::Null) {
                        tounicode = true;
                    }
                }
            }
        }
    }
    Ok(PdfMetaSnapshot {
        page_count,
        file_size,
        embedded_fonts: fonts.into_iter().collect(),
        has_tounicode: tounicode,
    })
}

/// 同步提取 PDF 文本（pdftotext 优先，否则 lopdf 退化）。
pub fn read_pdf_text(pdf: &std::path::Path) -> Result<String, QualityError> {
    // 1. pdftotext
    if let Ok(out) = std::process::Command::new("pdftotext")
        .arg(pdf)
        .arg("-")
        .output()
    {
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
        }
    }
    // 2. mutool draw -F text
    if let Ok(out) = std::process::Command::new("mutool")
        .args(["draw", "-F", "text", "-i"])
        .arg(pdf)
        .output()
    {
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
        }
    }
    // 3. lopdf 兜底
    let doc = lopdf::Document::load(pdf)
        .map_err(|e| QualityError::PdfText(format!("lopdf load 失败：{e}")))?;
    let mut s = String::new();
    let pages = doc.get_pages();
    for (_page_num, id) in pages.iter() {
        let obj_id: u32 = id.1 as u32;
        if let Ok(text) = doc.extract_text(&[obj_id]) {
            s.push_str(&text);
        }
    }
    Ok(s)
}

/// 异步包装：内部 `spawn_blocking` 跑 `read_pdf_text`。
pub async fn read_pdf_text_async(pdf: std::path::PathBuf) -> Result<String, QualityError> {
    tokio::task::spawn_blocking(move || read_pdf_text(&pdf))
        .await
        .map_err(|e| QualityError::PdfText(format!("join 失败：{e}")))?
}
