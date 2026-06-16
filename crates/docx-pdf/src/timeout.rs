//! 预留：超时与重试策略（占位，逻辑已内联到 `backend.rs::DocxToPdf::convert`）。
//!
//! 设计见 `docs/study/08-pdf-pipeline/03-docx-to-pdf.md` §3.5.3。
//!
//! 目前 `DocxToPdf::convert` 直接用 `tokio::time::timeout` + 手写指数退避。
//! 保留此模块为后续给外部调用方复用（如 CLI 内部直接调）。
