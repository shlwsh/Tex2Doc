//! `doc-tex-facade` — V2 PDF 流水线路径 A 的核心 crate。
//!
//! 在 V1「不调用 TeX」边界之上，**可插拔地**封装外部 TeX 进程（xelatex / tectonic / latexmk），
//! 生成 oracle PDF、抽取 oracle 文本，并提供内容寻址缓存与信号量限流。
//!
//! M1 阶段仅提供 `version()` 入口；M2 阶段补全 `TexProject` / `TexBackend` / 三个后端 / 缓存 / 抽取。
//!
//! 详细设计见 `docs/study/08-pdf-pipeline/02-tex-facade.md`。

/// 当前 crate 版本（与 `Cargo.toml` 一致）。
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
