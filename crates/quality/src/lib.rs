//! `doc-quality` — V2 PDF 流水线路径 C 的核心 crate。
//!
//! 在 V1 [to-docx/08-verification.md](../../../../docs/to-docx/08-verification.md) 33 项结构校验基础上，
//! 扩展为「结构 / 文本 / 视觉」三层对比，让"docx/pdf 质量不低原生"成为可量化的 CI 卡点。
//!
//! 详细设计见 `docs/study/08-pdf-pipeline/04-quality-comparison.md`。
//!
//! ## M1 / M2 阶段状态
//!
//! - M1 阶段已提供 `version()` 入口（向后兼容）。
//! - M2 阶段提供本库全量类型与三层 Runner（不含 OCR / 不含 PDFium 完整渲染；视觉层仅 SSIM/像素差桩实现，
//!   PDF 端 4 项结构校验可用、SSIM/像素差可计算、OCR 在 `feature = "ocr"` 时才打开）。

#![forbid(unsafe_code)]

pub mod error;
pub mod layer;
pub mod report;
pub mod markers;
pub mod normalize;
pub mod thresholds;
pub mod textual;
pub mod structural;
pub mod structural_pdf;
pub mod visual;
pub mod context;

pub use context::{Context, PdfMetaSnapshot};
pub use error::QualityError;
pub use layer::{Check, Layer, LayerResult, MarkerHit, QualityReport, Severity};
pub use report::{write_json, write_markdown};
pub use thresholds::{
    StructuralThresholds, TextualThresholds, Thresholds, VisualThresholds,
};
pub use visual::VisualRunner;

/// 当前 crate 版本（与 `Cargo.toml` 一致）。
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// 给定阈值，组装一个 `Quality` 入口；视觉层默认 `dpi=150`、`threshold_ssim=0.95`。
pub struct Quality {
    pub structural: structural::Runner,
    pub textual: textual::Runner,
    pub visual: visual::VisualRunner,
    pub thresholds: Thresholds,
}

impl Quality {
    /// 构造入口；`thresholds` 控制结构 / 文本 / 视觉三层门槛。
    pub fn new(thresholds: Thresholds) -> Self {
        Self {
            structural: structural::Runner::default(),
            textual: textual::Runner::default(),
            visual: visual::VisualRunner::default(),
            thresholds,
        }
    }

    /// 跑单层。
    pub async fn run_layer(
        &self,
        layer: Layer,
        ctx: &Context,
    ) -> Result<LayerResult, QualityError> {
        match layer {
            Layer::Structural => self.structural.run(ctx, &self.thresholds.structural),
            Layer::Textual => self.textual.run(ctx, &self.thresholds.textual),
            Layer::Visual => self.visual.run(ctx, &self.thresholds.visual).await,
        }
    }

    /// 跑全部三层，汇总为 [`QualityReport`]。
    pub async fn run_all(&self, ctx: &Context) -> Result<QualityReport, QualityError> {
        let mut layers = Vec::new();
        for layer in [Layer::Structural, Layer::Textual, Layer::Visual] {
            layers.push(self.run_layer(layer, ctx).await?);
        }
        let passed = layers.iter().all(|l| l.passed);
        let exit_code = compute_exit_code(&layers);

        // 顶层汇总：marker 命中 / 字符数 / 字符比例 / 段落数
        let mut report = QualityReport {
            docx: ctx.docx.clone(),
            rust_pdf: ctx.rust_pdf.clone(),
            oracle_pdf: ctx.oracle_pdf.clone(),
            passed,
            exit_code,
            layer_results: layers,
            marker_coverage: Vec::new(),
            docx_chars: crate::normalize::normalize(&ctx.docx_text).chars().count(),
            rust_pdf_chars: crate::normalize::normalize(&ctx.rust_text).chars().count(),
            oracle_pdf_chars: crate::normalize::normalize(&ctx.oracle_text).chars().count(),
            char_ratio_docx_to_oracle: 0.0,
            char_ratio_rust_to_oracle: 0.0,
            paragraphs: ctx.docx_paragraphs,
        };
        report.char_ratio_docx_to_oracle = if report.oracle_pdf_chars == 0 {
            0.0
        } else {
            report.docx_chars as f64 / report.oracle_pdf_chars as f64
        };
        report.char_ratio_rust_to_oracle = if report.oracle_pdf_chars == 0 {
            0.0
        } else {
            report.rust_pdf_chars as f64 / report.oracle_pdf_chars as f64
        };
        report.marker_coverage =
            crate::markers::coverage(&ctx.docx_text, &ctx.oracle_text, &ctx.rust_text);
        Ok(report)
    }
}

/// 复刻 `docs/study/08-pdf-pipeline/04-quality-comparison.md` §4.4.2 中的
/// `compute_exit_code`：结构/文本 fail → 1；视觉 fail → 2；全 pass → 0。
pub fn compute_exit_code(layers: &[LayerResult]) -> i32 {
    let structural_fail = layers
        .iter()
        .find(|l| l.layer == Layer::Structural)
        .map_or(false, |l| !l.passed);
    let textual_fail = layers
        .iter()
        .find(|l| l.layer == Layer::Textual)
        .map_or(false, |l| !l.passed);
    let visual_fail = layers
        .iter()
        .find(|l| l.layer == Layer::Visual)
        .map_or(false, |l| !l.passed);
    if structural_fail || textual_fail {
        1
    } else if visual_fail {
        2
    } else {
        0
    }
}
