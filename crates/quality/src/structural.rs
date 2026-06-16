//! 结构层（33 + 4 项 = 37 项）的核心子集实现。
//!
//! 设计见 `docs/study/08-pdf-pipeline/04-quality-comparison.md` §4.5。
//!
//! 完整 33 项 V1 沿用 → 放在 `doc-quality::structural` 中；本期先落地**核心子集**：
//!
//! - #1 表格对象数
//! - #2 图片数
//! - #5 编号表题数
//! - #29 docx / oracle 字符比例
//! - #30 页面尺寸（w/h）
//! - #31 页边距（top/right/bottom/left）
//! - #32 分栏数
//! - #33 22 marker 覆盖（在 textual 层也跑一份「docx 命中」）
//!
//! PDF 端 4 项放 [`crate::structural_pdf`]。
//!
//> 余下 24 项（图像 keep_next / 表头加粗 / 公式上下标 / 参考文献悬挂缩进 / etc.）随 M4 阶段
//!> 补完，每条都对应 `to-docx/08-verification.md §8.3` 的一行。

use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::context::Context;
use crate::layer::{Check, Layer, LayerResult, Severity};
use crate::thresholds::StructuralThresholds;
use crate::QualityError;

#[derive(Default)]
pub struct Runner {
    _priv: (),
}

impl Runner {
    pub fn run(
        &self,
        ctx: &Context,
        thr: &StructuralThresholds,
    ) -> Result<LayerResult, QualityError> {
        let mut checks = Vec::new();
        let docx_xml = read_part(&ctx.docx, "word/document.xml")?;
        let styles_xml = read_part(&ctx.docx, "word/styles.xml").ok();

        checks.push(check_tables(&docx_xml, thr)?);
        checks.push(check_images(&docx_xml, thr)?);
        checks.push(check_table_captions(&docx_xml, thr)?);
        checks.push(check_page_size(&docx_xml, thr)?);
        checks.push(check_margins(&docx_xml)?);
        checks.push(check_columns(&docx_xml)?);

        if let Some(s) = styles_xml.as_ref() {
            checks.push(check_reference_indent(s)?);
        }

        // 字符比例 (docx / oracle)
        let dc = crate::normalize::normalize(&ctx.docx_text).chars().count();
        let oc = crate::normalize::normalize(&ctx.oracle_text).chars().count();
        let ratio = if oc == 0 { 0.0 } else { dc as f64 / oc as f64 };
        checks.push(Check::new(
            "DOCX/PDF 字符比例 (#29)",
            Severity::Major,
            format!(">={:.2}", thr.min_char_ratio),
            format!("{:.3}", ratio),
            ratio >= thr.min_char_ratio,
        ));

        Ok(LayerResult::new(Layer::Structural, checks))
    }
}

/// 读取 docx 中的指定 entry（zip 内 POSIX 路径）。
pub(crate) fn read_part(docx: &Path, entry: &str) -> Result<String, QualityError> {
    let bytes = std::fs::read(docx).map_err(QualityError::Io)?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| QualityError::Docx(format!("zip 打开失败：{e}")))?;
    let mut f = zip
        .by_name(entry)
        .map_err(|e| QualityError::Docx(format!("缺少 {entry}：{e}")))?;
    let mut s = String::new();
    use std::io::Read;
    f.read_to_string(&mut s)
        .map_err(|e| QualityError::Docx(format!("读取 {entry} 失败：{e}")))?;
    Ok(s)
}

fn quick_count(reader: &mut Reader<&[u8]>, local_name: &[u8], nsm: &[(&[u8], &[u8])]) -> usize {
    let mut count = 0;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if local_eq(&e.name().as_ref(), local_name, nsm) {
                    count += 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    count
}

fn local_eq(actual: &[u8], local: &[u8], nsm: &[(&[u8], &[u8])]) -> bool {
    // quick-xml 给出的是完整限定名；我们做的是简化版：忽略命名空间前缀。
    if actual == local {
        return true;
    }
    if let Some(colon) = actual.iter().position(|&c| c == b':') {
        let local_only = &actual[colon + 1..];
        if local_only == local {
            return true;
        }
    }
    for (ns, _) in nsm {
        let prefixed = format!("{}:{}", String::from_utf8_lossy(ns), String::from_utf8_lossy(local));
        if actual == prefixed.as_bytes() {
            return true;
        }
    }
    false
}

fn check_tables(doc_xml: &str, thr: &StructuralThresholds) -> Result<Check, QualityError> {
    let mut r = Reader::from_str(doc_xml);
    let nsm: &[(&[u8], &[u8])] = &[(b"w", b"http://schemas.openxmlformats.org/wordprocessingml/2006/main")];
    let n = quick_count(&mut r, b"tbl", nsm);
    Ok(Check::new(
        "表格对象数 (#1)",
        Severity::Major,
        format!(">={}", thr.min_tables),
        format!("{}", n),
        n as u32 >= thr.min_tables,
    ))
}

fn check_images(doc_xml: &str, thr: &StructuralThresholds) -> Result<Check, QualityError> {
    let mut r = Reader::from_str(doc_xml);
    let nsm: &[(&[u8], &[u8])] = &[(b"w", b"http://schemas.openxmlformats.org/wordprocessingml/2006/main")];
    let n = quick_count(&mut r, b"drawing", nsm);
    Ok(Check::new(
        "图片数 (#2)",
        Severity::Major,
        format!("={}", thr.expected_images),
        format!("{}", n),
        n as u32 == thr.expected_images,
    ))
}

fn check_table_captions(doc_xml: &str, thr: &StructuralThresholds) -> Result<Check, QualityError> {
    // 简版：统计以"表"开头的段数。
    let mut r = Reader::from_str(doc_xml);
    let mut buf = Vec::new();
    let mut caption_count = 0;
    loop {
        match r.read_event_into(&mut buf) {
            Ok(Event::Text(t)) => {
                let s = t.unescape().unwrap_or_default();
                if s.trim_start().starts_with('表') {
                    caption_count += 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(Check::new(
        "编号表题数 (#5)",
        Severity::Major,
        format!(">={}", thr.min_captions),
        format!("{}", caption_count),
        caption_count as u32 >= thr.min_captions,
    ))
}

fn check_page_size(doc_xml: &str, thr: &StructuralThresholds) -> Result<Check, QualityError> {
    let vals = extract_sectpr_attr(doc_xml, "pgSz", &["w", "h"]);
    let w = vals.first().cloned().unwrap_or_default();
    let h = vals.get(1).cloned().unwrap_or_default();
    let w_ok = w.parse::<u32>().map(|x| x == thr.expected_page_w).unwrap_or(false);
    let h_ok = h.parse::<u32>().map(|x| x == thr.expected_page_h).unwrap_or(false);
    Ok(Check::new(
        "页面尺寸 (#30)",
        Severity::Critical,
        format!("w={}, h={}", thr.expected_page_w, thr.expected_page_h),
        format!("w={}, h={}", w, h),
        w_ok && h_ok,
    ))
}

fn check_margins(doc_xml: &str) -> Result<Check, QualityError> {
    let vals = extract_sectpr_attr(doc_xml, "pgMar", &["top", "right", "bottom", "left"]);
    let (top, right, bottom, left) = (
        vals.first().cloned().unwrap_or_default(),
        vals.get(1).cloned().unwrap_or_default(),
        vals.get(2).cloned().unwrap_or_default(),
        vals.get(3).cloned().unwrap_or_default(),
    );
    Ok(Check::new(
        "页边距 (#31)",
        Severity::Critical,
        "top/right/bottom/left 非空".to_string(),
        format!("{}/{}/{}/{}", top, right, bottom, left),
        !(top.is_empty() || right.is_empty() || bottom.is_empty() || left.is_empty()),
    ))
}

fn check_columns(doc_xml: &str) -> Result<Check, QualityError> {
    let vals = extract_sectpr_attr(doc_xml, "cols", &["space", "num"]);
    let (space, num) = (
        vals.first().cloned().unwrap_or_default(),
        vals.get(1).cloned().unwrap_or_default(),
    );
    Ok(Check::new(
        "分栏 (#32)",
        Severity::Major,
        "space & num 非空".to_string(),
        format!("space={}, num={}", space, num),
        !space.is_empty() && !num.is_empty(),
    ))
}

fn check_reference_indent(styles_xml: &str) -> Result<Check, QualityError> {
    // 简化：检查是否存在 JOSReference 样式。
    let found = styles_xml.contains("JOSReference");
    Ok(Check::new(
        "参考文献悬挂缩进样式 (#19)",
        Severity::Major,
        "含 JOSReference 样式",
        if found { "present" } else { "missing" }.to_string(),
        found,
    ))
}

/// 在 `w:sectPr` 中读取指定 tag 的若干属性值；按出现顺序返回。
fn extract_sectpr_attr(doc_xml: &str, tag: &str, attrs: &[&str]) -> Vec<String> {
    let mut out = vec![String::new(); attrs.len()];
    let mut r = Reader::from_str(doc_xml);
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let local = e.name();
                let local = local.as_ref();
                let local = if let Some(c) = local.iter().position(|&x| x == b':') {
                    &local[c + 1..]
                } else {
                    local
                };
                if local == tag.as_bytes() {
                    for a in e.attributes().flatten() {
                        let k = a.key.as_ref();
                        let k = if let Some(c) = k.iter().position(|&x| x == b':') {
                            &k[c + 1..]
                        } else {
                            k
                        };
                        let v = a.unescape_value().unwrap_or_default().to_string();
                        for (i, name) in attrs.iter().enumerate() {
                            if k == name.as_bytes() {
                                out[i] = v.clone();
                            }
                        }
                    }
                    return out;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    out
}
