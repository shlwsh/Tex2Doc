//! 转换选项
//!
//! 桥接层（FFI / WASM / HTTP）以此为入参契约。

use doc_docx_writer::PageSetup;
use serde::{Deserialize, Serialize};

/// BibTeX 渲染样式（V1 内置）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BibStyle {
    #[default]
    Numeric,
    AuthorYear,
}

/// 转换选项。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConvertOptions {
    /// 引用样式
    pub bib_style: BibStyle,
    /// 可选 reference.docx 模板字节（PNG/JPEG/PDF 等不支持）
    pub template: Option<Vec<u8>>,
    /// 资源文件（图片、bib、嵌套 tex）的内联二进制
    pub attachments: Vec<Attachment>,
    /// 可选 reference.docx 模板字节流（用于样式继承）
    pub template_bytes: Option<Vec<u8>>,
    /// V2 新增：显式页面设置（缺省 = Letter）
    pub page_setup: Option<PageSetup>,
}

/// 内联资源附件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// 相对路径（如 `figs/a.png`）
    pub path: String,
    pub bytes: Vec<u8>,
}
