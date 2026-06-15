//! `doc-quality` — V2 PDF 流水线路径 C 的核心 crate。
//!
//! 在 V1 [to-docx/08-verification.md](../../../../docs/to-docx/08-verification.md) 33 项结构校验基础上，
//! 扩展为「结构 / 文本 / 视觉」三层对比，让"docx/pdf 质量不低原生"成为可量化的 CI 卡点。
//!
//! M1 阶段仅提供 `version()` 入口；M4 阶段补全 `Layer` / `Check` / `LayerResult` / `QualityReport` 类型与三层 Runner。
//!
//! 详细设计见 `docs/study/08-pdf-pipeline/04-quality-comparison.md`。

/// 当前 crate 版本（与 `Cargo.toml` 一致）。
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
