//! `doc-docx-pdf` — V2 PDF 流水线路径 B 的核心 crate。
//!
//! 在 V1 端产出的 docx 之上，用 LibreOffice headless 二次转换为 PDF，
//! 不重新发明排版引擎——直接复用 LibreOffice 的 docx→pdf 能力，并把它封装成可替换的 Rust trait。
//!
//! M1 阶段仅提供 `version()` 入口；M3 阶段补全 `DocxToPdfBackend` trait / LibreOffice 后端 / 进程管理。
//!
//! 详细设计见 `docs/study/08-pdf-pipeline/03-docx-to-pdf.md`。

/// 当前 crate 版本（与 `Cargo.toml` 一致）。
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
