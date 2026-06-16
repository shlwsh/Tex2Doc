//! 报告序列化：MD + JSON。
//!
//! 设计见 `docs/study/08-pdf-pipeline/04-quality-comparison.md` §4.8。

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::layer::QualityReport;

pub fn write_markdown(report: &QualityReport, out: &Path) -> std::io::Result<()> {
    let mut s = String::new();
    s.push_str("# V2 三层质量报告\n\n");
    s.push_str(&format!("- DOCX: `{}`\n", report.docx.display()));
    s.push_str(&format!("- Rust PDF: `{}`\n", report.rust_pdf.display()));
    s.push_str(&format!("- Oracle PDF: `{}`\n", report.oracle_pdf.display()));
    s.push_str(&format!(
        "- 结论: {}\n- 退出码: {}\n\n",
        verdict(report),
        report.exit_code
    ));

    s.push_str("## 1. 结构层\n\n");
    if let Some(layer) = report.layer_results.iter().find(|l| l.layer == crate::layer::Layer::Structural) {
        s.push_str("| # | 名称 | 期望 | 实际 | 状态 |\n");
        s.push_str("|---|------|------|------|------|\n");
        for (i, c) in layer.checks.iter().enumerate() {
            let status = if c.passed { "✓" } else { "✗" };
            s.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                i + 1,
                c.name,
                c.expected,
                c.actual,
                status
            ));
        }
        s.push('\n');
    }

    s.push_str("## 2. 文本层\n\n");
    s.push_str("| 项 | 期望 | 实际 | 状态 |\n");
    s.push_str("|----|------|------|------|\n");
    let passed_ratio = |want: f64, got: f64| -> bool { got >= want };
    s.push_str(&format!(
        "| docx_chars | - | {} | - |\n| rust_pdf_chars | - | {} | - |\n| oracle_chars | - | {} | - |\n",
        report.docx_chars, report.rust_pdf_chars, report.oracle_pdf_chars
    ));
    s.push_str(&format!(
        "| docx/oracle 字符比例 | >=0.75 | {:.3} | {} |\n",
        report.char_ratio_docx_to_oracle,
        if passed_ratio(0.75, report.char_ratio_docx_to_oracle) { "✓" } else { "✗" }
    ));
    s.push_str(&format!(
        "| rust/oracle 字符比例 | >=0.75 | {:.3} | {} |\n",
        report.char_ratio_rust_to_oracle,
        if passed_ratio(0.75, report.char_ratio_rust_to_oracle) { "✓" } else { "✗" }
    ));
    s.push_str(&format!(
        "| 22 marker 三侧覆盖 | 22/22 | {} |\n| 7 章节 oracle+rust 双向覆盖 | 7/7 | {} |\n",
        if !report.marker_coverage.is_empty() {
            let hits = report
                .marker_coverage
                .iter()
                .filter(|h| h.in_docx && h.in_oracle_pdf && h.in_rust_pdf)
                .count();
            format!("{}/{}", hits, report.marker_coverage.len())
        } else {
            "-".to_string()
        },
        if !report.marker_coverage.is_empty() {
            // 章节覆盖没有显式字段；用 marker_coverage 末 7 行代理。
            let last7 = &report.marker_coverage[report.marker_coverage.len().saturating_sub(7)..];
            let ok = last7
                .iter()
                .filter(|h| h.in_docx && h.in_oracle_pdf && h.in_rust_pdf)
                .count();
            format!("{}/{}", ok, last7.len())
        } else {
            "-".to_string()
        }
    ));
    s.push('\n');

    s.push_str("## 3. 视觉层\n\n");
    if let Some(layer) = report.layer_results.iter().find(|l| l.layer == crate::layer::Layer::Visual) {
        s.push_str("| # | 名称 | 期望 | 实际 | 状态 |\n");
        s.push_str("|---|------|------|------|------|\n");
        for (i, c) in layer.checks.iter().enumerate() {
            let status = if c.passed { "✓" } else { "✗" };
            s.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                i + 1,
                c.name,
                c.expected,
                c.actual,
                status
            ));
        }
    } else {
        s.push_str("(未运行)\n");
    }

    fs::write(out, s)?;
    Ok(())
}

pub fn write_json(report: &QualityReport, out: &Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(report)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let mut f = fs::File::create(out)?;
    f.write_all(json.as_bytes())?;
    Ok(())
}

fn verdict(r: &QualityReport) -> &'static str {
    if r.passed {
        "通过"
    } else if r.exit_code == 2 {
        "视觉降级"
    } else {
        "未通过"
    }
}
