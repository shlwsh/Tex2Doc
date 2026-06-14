//! 文档级错误类型
//!
//! 全 crate 统一使用 `DocError`，便于上层聚合与跨 FFI 边界序列化。

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DocError {
    /// 虚拟文件系统找不到对应路径
    #[error("VFS 路径未挂载：{0}")]
    VfsMissing(PathBuf),

    /// IO 错误
    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),

    /// 路径解析失败
    #[error("路径解析失败：{0}")]
    InvalidPath(String),

    /// 图片解码失败
    #[error("图片解码失败：{0}")]
    ImageDecode(String),

    /// 不支持的操作 / 占位错误
    #[error("不支持的操作：{0}")]
    Unsupported(String),
}

/// crate 内部 Result 简写
pub type DocResult<T> = std::result::Result<T, DocError>;
