//! `doc-docx-pdf` — V2 PDF 流水线路径 B 的核心 crate。
//!
//! 在 V1 端产出的 docx 之上，用 LibreOffice headless 二次转换为 PDF，
//! 不重新发明排版引擎——直接复用 LibreOffice 的 docx→pdf 能力，并把它封装成可替换的 Rust trait。
//!
//! 详细设计见 `docs/study/08-pdf-pipeline/03-docx-to-pdf.md`。
//!
//! ## 平台特性
//!
//! - **默认后端**：[`LibreOfficeBackend`] —— `soffice --headless --convert-to pdf`。
//! - **可插拔**：实现 [`DocxToPdfBackend`] trait 可加入新后端（如远程 API）。
//! - **零侵入**：V1 crate **不依赖** `docx-pdf`；`docx-pdf` 只在 V2 校验子命令被引用。

#![forbid(unsafe_code)]

mod backend;
mod error;
mod libreoffice;
mod meta;
mod profile;
mod timeout;

pub use backend::{BackendKind, DocxToPdf, DocxToPdfBackend, DocxToPdfRun};
pub use error::PdfError;

/// 当前 crate 版本（与 `Cargo.toml` 一致）。
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
