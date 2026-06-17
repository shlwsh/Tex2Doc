//! V2 新增 PDF 端 4 项结构校验。
//!
//! 设计见 `docs/study/08-pdf-pipeline/04-quality-comparison.md` §4.5.2。
//!
//! | # | 名称 | 期望 |
//! |---|------|------|
//! | 34 | rust pdf 页数 vs docx 段数 / 6 | ±20% |
//! | 35 | rust pdf 文件大小 | < 5 MB |
//! | 36 | rust pdf 嵌入字体 | ≥ 2 |
//! | 37 | rust pdf ToUnicode | true |

use crate::context::Context;
use crate::layer::{Check, Layer, LayerResult, Severity};
use crate::thresholds::StructuralThresholds;
use crate::QualityError;

pub fn pdf_page_count_within(
    docx_paragraphs: usize,
    pdf_meta: &crate::context::PdfMetaSnapshot,
) -> Check {
    let expected = (docx_paragraphs as f64 / 6.0).max(1.0);
    let actual = pdf_meta.page_count as f64;
    let within = if expected == 0.0 {
        actual == 0.0
    } else {
        (actual - expected).abs() / expected <= 0.20
    };
    Check::new(
        "rust pdf 页数 vs docx 段数 / 6 (#34)",
        Severity::Major,
        format!("±20% of {:.1}", expected),
        format!("{}", pdf_meta.page_count),
        within,
    )
}

pub fn pdf_size_within(pdf_meta: &crate::context::PdfMetaSnapshot, max: u64) -> Check {
    Check::new(
        "rust pdf 文件大小 (#35)",
        Severity::Major,
        format!("<{} bytes", max),
        format!("{} bytes", pdf_meta.file_size),
        pdf_meta.file_size < max,
    )
}

pub fn pdf_embedded_fonts_nonempty(
    pdf_meta: &crate::context::PdfMetaSnapshot,
    min: usize,
) -> Check {
    Check::new(
        "rust pdf 嵌入字体 (#36)",
        Severity::Major,
        format!(">={}", min),
        format!("{}", pdf_meta.embedded_fonts.len()),
        pdf_meta.embedded_fonts.len() >= min,
    )
}

pub fn pdf_has_tounicode(pdf_meta: &crate::context::PdfMetaSnapshot) -> Check {
    Check::new(
        "rust pdf ToUnicode (#37)",
        Severity::Minor,
        "true".to_string(),
        format!("{}", pdf_meta.has_tounicode),
        pdf_meta.has_tounicode,
    )
}

/// 一次性跑完 4 项 PDF 端结构校验（V2 增量）。
pub fn run_all(ctx: &Context, thr: &StructuralThresholds) -> LayerResult {
    let checks = vec![
        pdf_page_count_within(ctx.docx_paragraphs, &ctx.rust_pdf_meta),
        pdf_size_within(&ctx.rust_pdf_meta, thr.max_pdf_size_bytes),
        pdf_embedded_fonts_nonempty(&ctx.rust_pdf_meta, thr.min_embedded_fonts),
        pdf_has_tounicode(&ctx.rust_pdf_meta),
    ];
    LayerResult::new(Layer::Structural, checks)
}

/// 暴露一个简版入口供 `Runner::run` 调：在已生成的 7 项后追加 4 项。
pub fn extend(checks: &mut Vec<Check>, ctx: &Context, thr: &StructuralThresholds) {
    checks.push(pdf_page_count_within(
        ctx.docx_paragraphs,
        &ctx.rust_pdf_meta,
    ));
    checks.push(pdf_size_within(&ctx.rust_pdf_meta, thr.max_pdf_size_bytes));
    checks.push(pdf_embedded_fonts_nonempty(
        &ctx.rust_pdf_meta,
        thr.min_embedded_fonts,
    ));
    checks.push(pdf_has_tounicode(&ctx.rust_pdf_meta));
}

/// 兜底：当 `Runner` 想直接跑 4 项时使用。
pub fn run(ctx: &Context, thr: &StructuralThresholds) -> Result<LayerResult, QualityError> {
    let mut checks = Vec::new();
    extend(&mut checks, ctx, thr);
    Ok(LayerResult::new(Layer::Structural, checks))
}
