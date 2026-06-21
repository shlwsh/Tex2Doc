//! `doc-tex-facade` — V2 PDF 流水线路径 A 的核心 crate。
//!
//! 在 V1「不调用 TeX」边界之上，**可插拔地**封装外部 TeX 进程（xelatex / tectonic / latexmk），
//! 生成 oracle PDF、抽取 oracle 文本，并提供内容寻址缓存与信号量限流。
//!
//! 详细设计见 `docs/study/08-pdf-pipeline/02-tex-facade.md`。

#![allow(clippy::needless_return)] // 调试期保留

mod backend;
mod cache;
mod error;
mod extract;
mod facade;
pub mod latexmk;
pub mod tectonic;
pub mod xelatex;
pub mod rasterize;

pub use backend::{EngineKind, TexBackend, TexProject, TexRun};
pub use cache::{compute_key, referenced_tex_files, Cache, CacheKey};
pub use error::{TexError, TexResult};
pub use extract::{detect_extractor, extract_text};
pub use facade::TexFacade;
pub use rasterize::rasterize_tikz_to_png;

/// 当前 crate 版本（与 `Cargo.toml` 一致）。
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
