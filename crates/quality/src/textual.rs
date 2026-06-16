//! 文本层：字符比例 + 22 marker 覆盖 + 7 章节覆盖。
//!
//! 设计见 `docs/study/08-pdf-pipeline/04-quality-comparison.md` §4.6。

use crate::context::Context;
use crate::layer::{Check, Layer, LayerResult, MarkerHit, Severity};
use crate::markers;
use crate::normalize::normalize;
use crate::thresholds::TextualThresholds;
use crate::QualityError;

/// 7 个章节标题（用于 `section_coverage`）。
pub const SECTIONS: &[&str] = &[
    "1 引言",
    "2 相关工作",
    "3 系统总体设计",
    "4 关键算法",
    "5 系统实现",
    "6 实验与分析",
    "7 结束语",
];

#[derive(Default)]
pub struct Runner {
    _priv: (),
}

impl Runner {
    pub fn run(&self, ctx: &Context, thr: &TextualThresholds) -> Result<LayerResult, QualityError> {
        let mut checks = Vec::new();

        // 1. 字符数（已 normalize）。
        let d = normalize(&ctx.docx_text);
        let o = normalize(&ctx.oracle_text);
        let r = normalize(&ctx.rust_text);
        let dn = d.chars().count();
        let on = o.chars().count();
        let rn = r.chars().count();
        let ratio_d_o = ratio(dn, on);
        let ratio_r_o = ratio(rn, on);

        checks.push(Check::new(
            "docx/oracle 字符比例",
            Severity::Critical,
            format!(">={:.2}", thr.min_char_ratio),
            format!("{:.3}", ratio_d_o),
            ratio_d_o >= thr.min_char_ratio,
        ));
        checks.push(Check::new(
            "rust_pdf/oracle 字符比例",
            Severity::Critical,
            format!(">={:.2}", thr.min_char_ratio),
            format!("{:.3}", ratio_r_o),
            ratio_r_o >= thr.min_char_ratio,
        ));

        // 2. 22 marker 三侧命中。
        let hits = markers::coverage(&ctx.docx_text, &ctx.oracle_text, &ctx.rust_text);
        let hit_all_three = hits
            .iter()
            .filter(|h| h.in_docx && h.in_oracle_pdf && h.in_rust_pdf)
            .count();
        let total = hits.len().max(1);
        let cov = hit_all_three as f64 / total as f64;
        checks.push(Check::new(
            "22 marker 三侧覆盖",
            Severity::Critical,
            format!("{:.0}%。0.0 marker * 3 处", 100.0 * thr.min_marker_coverage),
            format!("{}/{}", hit_all_three, hits.len()),
            cov >= thr.min_marker_coverage,
        ));

        // 3. 7 章节覆盖（oracle 与 rust 双向）。
        let mut section_hits = 0;
        for s in SECTIONS {
            let n = normalize(s);
            if o.contains(&n) && r.contains(&n) {
                section_hits += 1;
            }
        }
        let sec_cov = section_hits as f64 / SECTIONS.len() as f64;
        checks.push(Check::new(
            "7 章节 oracle+rust 双向覆盖",
            Severity::Major,
            format!("{:.0}%。0.0", 100.0 * thr.min_section_coverage),
            format!("{}/{}", section_hits, SECTIONS.len()),
            sec_cov >= thr.min_section_coverage,
        ));

        Ok(LayerResult::new(Layer::Textual, checks))
    }
}

/// 字符比例：a/b，b==0 时返回 0。
pub fn char_ratio(a: &str, b: &str) -> f64 {
    ratio(normalize(a).chars().count(), normalize(b).chars().count())
}

fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 {
        0.0
    } else {
        a as f64 / b as f64
    }
}

/// 给 `Context` 写回 marker 命中（被 lib 入口使用）。
pub fn fill_marker_hits(ctx: &mut Context) -> Vec<MarkerHit> {
    let hits = markers::coverage(&ctx.docx_text, &ctx.oracle_text, &ctx.rust_text);
    ctx.marker_hits = hits.clone();
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_hit_for_7_chapters() {
        let t = "1 引言2 相关工作3 系统总体设计4 关键算法5 系统实现6 实验与分析7 结束语";
        for s in SECTIONS {
            assert!(normalize(t).contains(&normalize(s)));
        }
    }

    #[test]
    fn char_ratio_uses_normalize() {
        let a = "摘  要 关键 词";
        let b = "摘要关键词";
        assert!((char_ratio(a, b) - 1.0).abs() < 1e-6);
    }
}
