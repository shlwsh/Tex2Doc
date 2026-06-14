//! 核心层错误模型
//!
//! 在 `doc_utils::DocError` 之上做适配；FFI 边界要求可序列化、平铺。

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum CoreError {
    #[error("IO 错误：{0}")]
    Io(String),

    #[error("解析错误：{0}")]
    Parse(String),

    #[error("序列化错误：{0}")]
    Serialize(String),

    #[error("不支持的操作：{0}")]
    Unsupported(String),
}

impl From<doc_utils::DocError> for CoreError {
    fn from(err: doc_utils::DocError) -> Self {
        match err {
            doc_utils::DocError::Io(e) => Self::Io(e.to_string()),
            doc_utils::DocError::VfsMissing(p) => Self::Parse(format!("VFS 缺失：{}", p.display())),
            doc_utils::DocError::InvalidPath(s) => Self::Parse(s),
            doc_utils::DocError::ImageDecode(s) => Self::Serialize(s),
            doc_utils::DocError::Unsupported(s) => Self::Unsupported(s),
        }
    }
}

impl From<std::io::Error> for CoreError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}
