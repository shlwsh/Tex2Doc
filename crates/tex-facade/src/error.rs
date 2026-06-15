//! `crates/tex-facade` 的统一错误类型。
//!
//! 见 `docs/study/08-pdf-pipeline/02-tex-facade.md` §2.7。
//! 设计原则：所有变体**不 panic**，调用方拿到 `Err` 后可安全降级或退出。

use std::path::PathBuf;

use thiserror::Error;

/// `doc-tex-facade` 的统一错误类型。
#[derive(Debug, Error)]
pub enum TexError {
    /// PATH 中没找到任何 TeX 引擎（xelatex / tectonic / latexmk 都缺）。
    #[error("未找到任何 TeX 引擎（xelatex / tectonic / latexmk 均不在 PATH）")]
    NoEngine,

    /// 调用方显式指定了引擎，但本机不可用（`which` 失败或执行返回非 0）。
    #[error("指定引擎 {0:?} 不可用（which 失败或执行返回非 0）")]
    EngineUnavailable(crate::backend::EngineKind),

    /// 编译失败：跑了 N 轮仍未生成 PDF。
    ///
    /// `log` 字段携带主 .log 末尾 4KB，便于排错（见 §2.6.2 缓存目录 `build.log`）。
    #[error("编译失败：{engine:?} 跑 {passes} 轮仍未生成 {output:?}\n--- log tail ---\n{log}")]
    CompileFailed {
        engine: crate::backend::EngineKind,
        passes: u32,
        output: PathBuf,
        log: String,
    },

    /// 缓存根目录不可写。
    #[error("缓存目录不可写：{0}")]
    CacheUnwritable(PathBuf),

    /// pdftotext / mutool 都缺，路径 A 文本抽取降级为 None。
    #[error("pdftotext / mutool 均不可用，无法抽取文本")]
    NoTextExtractor,

    /// I/O 错误透传。
    #[error("I/O 错误：{0}")]
    Io(#[from] std::io::Error),
}

/// `Result<T, TexError>` 的简写。
pub type TexResult<T> = Result<T, TexError>;
