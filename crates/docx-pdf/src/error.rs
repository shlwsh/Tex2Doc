//! `docx-pdf` 错误类型。

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PdfError {
    #[error("未找到任何 docx→pdf 后端（PATH 中无 soffice；macOS 还需检查 /Applications/LibreOffice.app）")]
    NoBackend,

    #[error("soffice 执行失败：code={code:?} stderr={stderr}")]
    LibreOfficeFailed { code: Option<i32>, stderr: String },

    #[error("soffice 跑 {timeout_secs}s 仍未结束（已 kill）")]
    Timeout { timeout_secs: u64 },

    #[error("期望输出 {0:?} 不存在（soffice 退出 0 但 PDF 缺失）")]
    OutputMissing(PathBuf),

    #[error("user-profile 目录创建失败：{0}")]
    ProfileCreateFailed(PathBuf),

    #[error("docx 不可读：{0}")]
    DocxUnreadable(PathBuf),

    #[error("PDF 元数据解析失败：{0}")]
    MetaParse(String),
}
