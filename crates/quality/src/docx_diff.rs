//! DOCX ↔ DOCX 内容与格式对比。
//!
//! 该模块用于把 Rust 引擎产物与脚本/oracle DOCX 做可重复对比：
//! - 语义轨：抽取段落、run 样式、表格、图片并做段落 LCS 对齐。
//! - 底层轨：规范化 OOXML，剥离 `rsid*` 噪音后比较 document/styles hash。

use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use serde::{Deserialize, Serialize};

use crate::error::{QualityError, Result};
use crate::structural::read_part;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocxDiffOptions {
    pub max_diffs: usize,
    pub compare_xml_hash: bool,
}

impl Default for DocxDiffOptions {
    fn default() -> Self {
        Self {
            max_diffs: 80,
            compare_xml_hash: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocxSnapshot {
    pub path: PathBuf,
    pub paragraphs: Vec<DocxParagraph>,
    pub tables: usize,
    pub drawings: usize,
    pub media_files: usize,
    pub paragraph_styles: BTreeMap<String, usize>,
    pub document_xml_hash: Option<String>,
    pub styles_xml_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocxParagraph {
    pub index: usize,
    pub style: Option<String>,
    pub text: String,
    pub normalized_text: String,
    pub has_drawing: bool,
    pub runs: Vec<DocxRun>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocxRun {
    pub text: String,
    pub style: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: Option<String>,
    pub vert_align: Option<String>,
    pub color: Option<String>,
    pub size: Option<String>,
    pub font_ascii: Option<String>,
    pub font_east_asia: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocxDiffReport {
    pub left: DocxSnapshot,
    pub right: DocxSnapshot,
    pub summary: DocxDiffSummary,
    pub content_diffs: Vec<ContentDiff>,
    pub format_diffs: Vec<FormatDiff>,
    pub xml_diffs: Vec<XmlDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocxDiffSummary {
    pub paragraph_delta: isize,
    pub table_delta: isize,
    pub drawing_delta: isize,
    pub media_delta: isize,
    pub equal_paragraphs: usize,
    pub modified_paragraphs: usize,
    pub inserted_paragraphs: usize,
    pub deleted_paragraphs: usize,
    pub format_changed_paragraphs: usize,
    pub document_xml_equal: Option<bool>,
    pub styles_xml_equal: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentDiff {
    pub kind: DiffKind,
    pub left_index: Option<usize>,
    pub right_index: Option<usize>,
    pub left_style: Option<String>,
    pub right_style: Option<String>,
    pub left_text: Option<String>,
    pub right_text: Option<String>,
    pub similarity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatDiff {
    pub left_index: usize,
    pub right_index: usize,
    pub text_preview: String,
    pub changes: Vec<FormatChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatChange {
    pub field: String,
    pub left: String,
    pub right: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlDiff {
    pub part: String,
    pub left_hash: Option<String>,
    pub right_hash: Option<String>,
    pub equal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    Equal,
    Modified,
    Insert,
    Delete,
}

#[derive(Debug, Clone, Copy)]
enum AlignmentOp {
    Equal(usize, usize),
    Delete(usize),
    Insert(usize),
}

pub fn compare_docx(
    left: impl AsRef<Path>,
    right: impl AsRef<Path>,
    options: &DocxDiffOptions,
) -> Result<DocxDiffReport> {
    let left = load_docx_snapshot(left.as_ref(), options.compare_xml_hash)?;
    let right = load_docx_snapshot(right.as_ref(), options.compare_xml_hash)?;
    Ok(compare_snapshots(left, right, options))
}

pub fn load_docx_snapshot(path: &Path, include_xml_hash: bool) -> Result<DocxSnapshot> {
    let document_xml = read_part(path, "word/document.xml")?;
    let styles_xml = read_part(path, "word/styles.xml").ok();
    let paragraphs = parse_document_paragraphs(&document_xml)?;
    let mut paragraph_styles = BTreeMap::new();
    for paragraph in &paragraphs {
        let style = paragraph.style.as_deref().unwrap_or("(none)");
        *paragraph_styles.entry(style.to_string()).or_insert(0) += 1;
    }

    Ok(DocxSnapshot {
        path: path.to_path_buf(),
        paragraphs,
        tables: count_tag(&document_xml, b"tbl"),
        drawings: count_tag(&document_xml, b"drawing"),
        media_files: count_media_files(path)?,
        paragraph_styles,
        document_xml_hash: include_xml_hash
            .then(|| stable_hash(&canonicalize_ooxml(&document_xml))),
        styles_xml_hash: include_xml_hash
            .then(|| {
                styles_xml
                    .as_deref()
                    .map(canonicalize_ooxml)
                    .map(|s| stable_hash(&s))
            })
            .flatten(),
    })
}

pub fn compare_snapshots(
    left: DocxSnapshot,
    right: DocxSnapshot,
    options: &DocxDiffOptions,
) -> DocxDiffReport {
    let alignment = align_paragraphs(&left.paragraphs, &right.paragraphs);
    let mut content_diffs = Vec::new();
    let mut format_diffs = Vec::new();
    let mut equal_paragraphs = 0usize;
    let mut modified_paragraphs = 0usize;
    let mut inserted_paragraphs = 0usize;
    let mut deleted_paragraphs = 0usize;

    let mut pos = 0usize;
    while pos < alignment.len() {
        match alignment[pos] {
            AlignmentOp::Equal(li, ri) => {
                equal_paragraphs += 1;
                if let Some(diff) =
                    compare_paragraph_format(&left.paragraphs[li], &right.paragraphs[ri])
                {
                    if format_diffs.len() < options.max_diffs {
                        format_diffs.push(diff);
                    }
                }
            }
            AlignmentOp::Delete(li) => {
                if let Some((ri, similarity)) =
                    adjacent_insert_match(&alignment, pos, &left.paragraphs[li], &right.paragraphs)
                {
                    modified_paragraphs += 1;
                    if content_diffs.len() < options.max_diffs {
                        let lp = &left.paragraphs[li];
                        let rp = &right.paragraphs[ri];
                        content_diffs.push(ContentDiff {
                            kind: DiffKind::Modified,
                            left_index: Some(lp.index),
                            right_index: Some(rp.index),
                            left_style: lp.style.clone(),
                            right_style: rp.style.clone(),
                            left_text: Some(preview(&lp.text, 220)),
                            right_text: Some(preview(&rp.text, 220)),
                            similarity: Some(similarity),
                        });
                    }
                    pos += 2;
                    continue;
                }
                deleted_paragraphs += 1;
                if content_diffs.len() < options.max_diffs {
                    let p = &left.paragraphs[li];
                    content_diffs.push(ContentDiff {
                        kind: DiffKind::Delete,
                        left_index: Some(p.index),
                        right_index: None,
                        left_style: p.style.clone(),
                        right_style: None,
                        left_text: Some(preview(&p.text, 220)),
                        right_text: None,
                        similarity: None,
                    });
                }
            }
            AlignmentOp::Insert(ri) => {
                if let Some((li, similarity)) =
                    adjacent_delete_match(&alignment, pos, &left.paragraphs, &right.paragraphs[ri])
                {
                    modified_paragraphs += 1;
                    if content_diffs.len() < options.max_diffs {
                        let lp = &left.paragraphs[li];
                        let rp = &right.paragraphs[ri];
                        content_diffs.push(ContentDiff {
                            kind: DiffKind::Modified,
                            left_index: Some(lp.index),
                            right_index: Some(rp.index),
                            left_style: lp.style.clone(),
                            right_style: rp.style.clone(),
                            left_text: Some(preview(&lp.text, 220)),
                            right_text: Some(preview(&rp.text, 220)),
                            similarity: Some(similarity),
                        });
                    }
                    pos += 2;
                    continue;
                }
                inserted_paragraphs += 1;
                if content_diffs.len() < options.max_diffs {
                    let p = &right.paragraphs[ri];
                    content_diffs.push(ContentDiff {
                        kind: DiffKind::Insert,
                        left_index: None,
                        right_index: Some(p.index),
                        left_style: None,
                        right_style: p.style.clone(),
                        left_text: None,
                        right_text: Some(preview(&p.text, 220)),
                        similarity: None,
                    });
                }
            }
        }
        pos += 1;
    }

    let mut xml_diffs = Vec::new();
    if options.compare_xml_hash {
        xml_diffs.push(XmlDiff {
            part: "word/document.xml".to_string(),
            left_hash: left.document_xml_hash.clone(),
            right_hash: right.document_xml_hash.clone(),
            equal: left.document_xml_hash == right.document_xml_hash,
        });
        xml_diffs.push(XmlDiff {
            part: "word/styles.xml".to_string(),
            left_hash: left.styles_xml_hash.clone(),
            right_hash: right.styles_xml_hash.clone(),
            equal: left.styles_xml_hash == right.styles_xml_hash,
        });
    }

    let summary = DocxDiffSummary {
        paragraph_delta: right.paragraphs.len() as isize - left.paragraphs.len() as isize,
        table_delta: right.tables as isize - left.tables as isize,
        drawing_delta: right.drawings as isize - left.drawings as isize,
        media_delta: right.media_files as isize - left.media_files as isize,
        equal_paragraphs,
        modified_paragraphs,
        inserted_paragraphs,
        deleted_paragraphs,
        format_changed_paragraphs: format_diffs.len(),
        document_xml_equal: options
            .compare_xml_hash
            .then(|| left.document_xml_hash == right.document_xml_hash),
        styles_xml_equal: options
            .compare_xml_hash
            .then(|| left.styles_xml_hash == right.styles_xml_hash),
    };

    DocxDiffReport {
        left,
        right,
        summary,
        content_diffs,
        format_diffs,
        xml_diffs,
    }
}

impl DocxDiffReport {
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# DOCX 内容与格式对比报告\n\n");
        out.push_str("## 输入\n\n");
        out.push_str(&format!("- Left: `{}`\n", self.left.path.display()));
        out.push_str(&format!("- Right: `{}`\n\n", self.right.path.display()));

        out.push_str("## 摘要\n\n");
        out.push_str("| 指标 | Left | Right | Delta |\n");
        out.push_str("| --- | ---: | ---: | ---: |\n");
        out.push_str(&format!(
            "| 段落数 | {} | {} | {} |\n",
            self.left.paragraphs.len(),
            self.right.paragraphs.len(),
            self.summary.paragraph_delta
        ));
        out.push_str(&format!(
            "| 表格数 | {} | {} | {} |\n",
            self.left.tables, self.right.tables, self.summary.table_delta
        ));
        out.push_str(&format!(
            "| 图片 drawing 数 | {} | {} | {} |\n",
            self.left.drawings, self.right.drawings, self.summary.drawing_delta
        ));
        out.push_str(&format!(
            "| media 文件数 | {} | {} | {} |\n",
            self.left.media_files, self.right.media_files, self.summary.media_delta
        ));
        out.push('\n');
        out.push_str(&format!(
            "- 相同段落：{}\n- 近似修改段落：{}\n- 新增段落：{}\n- 删除段落：{}\n- 格式变更段落：{}\n",
            self.summary.equal_paragraphs,
            self.summary.modified_paragraphs,
            self.summary.inserted_paragraphs,
            self.summary.deleted_paragraphs,
            self.summary.format_changed_paragraphs
        ));
        if let Some(equal) = self.summary.document_xml_equal {
            out.push_str(&format!("- document.xml 规范化 hash 相同：{}\n", equal));
        }
        if let Some(equal) = self.summary.styles_xml_equal {
            out.push_str(&format!("- styles.xml 规范化 hash 相同：{}\n", equal));
        }

        out.push_str("\n## 段落样式分布\n\n");
        out.push_str("### Left\n\n");
        write_style_table(&mut out, &self.left.paragraph_styles);
        out.push_str("\n### Right\n\n");
        write_style_table(&mut out, &self.right.paragraph_styles);

        out.push_str("\n## 内容差异\n\n");
        if self.content_diffs.is_empty() {
            out.push_str("未发现段落级内容插入/删除。\n");
        } else {
            out.push_str("| 类型 | Left# | Right# | 相似度 | Left 样式 | Right 样式 | 文本 |\n");
            out.push_str("| --- | ---: | ---: | ---: | --- | --- | --- |\n");
            for diff in &self.content_diffs {
                let text = match (&diff.left_text, &diff.right_text) {
                    (Some(left), Some(right)) if diff.kind == DiffKind::Modified => {
                        format!("L: {}<br>R: {}", escape_md(left), escape_md(right))
                    }
                    _ => diff
                        .left_text
                        .as_ref()
                        .or(diff.right_text.as_ref())
                        .map(|s| escape_md(s))
                        .unwrap_or_default(),
                };
                out.push_str(&format!(
                    "| {:?} | {} | {} | {} | {} | {} | {} |\n",
                    diff.kind,
                    fmt_idx(diff.left_index),
                    fmt_idx(diff.right_index),
                    fmt_similarity(diff.similarity),
                    fmt_opt(&diff.left_style),
                    fmt_opt(&diff.right_style),
                    text
                ));
            }
        }

        out.push_str("\n## 格式差异\n\n");
        if self.format_diffs.is_empty() {
            out.push_str("未发现相同文本段落的格式差异。\n");
        } else {
            out.push_str("| Left# | Right# | 文本 | 字段变化 |\n");
            out.push_str("| ---: | ---: | --- | --- |\n");
            for diff in &self.format_diffs {
                let changes = diff
                    .changes
                    .iter()
                    .map(|c| format!("{}: `{}` -> `{}`", c.field, c.left, c.right))
                    .collect::<Vec<_>>()
                    .join("<br>");
                out.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    diff.left_index,
                    diff.right_index,
                    escape_md(&diff.text_preview),
                    changes
                ));
            }
        }

        if !self.xml_diffs.is_empty() {
            out.push_str("\n## OOXML Hash\n\n");
            out.push_str("| Part | Equal | Left hash | Right hash |\n");
            out.push_str("| --- | --- | --- | --- |\n");
            for diff in &self.xml_diffs {
                out.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    diff.part,
                    diff.equal,
                    diff.left_hash.as_deref().unwrap_or("-"),
                    diff.right_hash.as_deref().unwrap_or("-")
                ));
            }
        }

        out
    }
}

fn parse_document_paragraphs(xml: &str) -> Result<Vec<DocxParagraph>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut paragraphs = Vec::new();
    let mut current_paragraph: Option<DocxParagraph> = None;
    let mut current_run: Option<DocxRun> = None;
    let mut in_text = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let local = local_name(name.as_ref());
                match local {
                    b"p" => {
                        current_paragraph = Some(DocxParagraph {
                            index: paragraphs.len() + 1,
                            style: None,
                            text: String::new(),
                            normalized_text: String::new(),
                            has_drawing: false,
                            runs: Vec::new(),
                        });
                    }
                    b"r" if current_paragraph.is_some() => {
                        current_run = Some(DocxRun::default());
                    }
                    b"t" if current_run.is_some() => {
                        in_text = true;
                    }
                    _ => apply_property_start(local, &e, &mut current_paragraph, &mut current_run),
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.name();
                let local = local_name(name.as_ref());
                apply_property_start(local, &e, &mut current_paragraph, &mut current_run);
            }
            Ok(Event::Text(t)) if in_text => {
                let text = t
                    .unescape()
                    .map_err(|e| QualityError::Xml(format!("文本解码失败：{e}")))?
                    .to_string();
                if let Some(run) = current_run.as_mut() {
                    run.text.push_str(&text);
                }
                if let Some(paragraph) = current_paragraph.as_mut() {
                    paragraph.text.push_str(&text);
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let local = local_name(name.as_ref());
                match local {
                    b"t" => in_text = false,
                    b"r" => {
                        if let (Some(paragraph), Some(run)) =
                            (current_paragraph.as_mut(), current_run.take())
                        {
                            if !run.text.is_empty() || run_has_format(&run) {
                                paragraph.runs.push(run);
                            }
                        }
                    }
                    b"p" => {
                        if let Some(mut paragraph) = current_paragraph.take() {
                            paragraph.normalized_text = normalize_docx_text(&paragraph.text);
                            if !paragraph.normalized_text.is_empty() || paragraph.has_drawing {
                                paragraphs.push(paragraph);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(QualityError::Xml(format!("document.xml 解析失败：{e}"))),
            _ => {}
        }
        buf.clear();
    }

    Ok(paragraphs)
}

fn apply_property_start(
    local: &[u8],
    e: &BytesStart<'_>,
    paragraph: &mut Option<DocxParagraph>,
    run: &mut Option<DocxRun>,
) {
    match local {
        b"pStyle" => {
            if let Some(p) = paragraph.as_mut() {
                p.style = attr_value(e, b"val");
            }
        }
        b"drawing" => {
            if let Some(p) = paragraph.as_mut() {
                p.has_drawing = true;
            }
        }
        b"rStyle" => {
            if let Some(r) = run.as_mut() {
                r.style = attr_value(e, b"val");
            }
        }
        b"b" => {
            if let Some(r) = run.as_mut() {
                r.bold = attr_bool(e).unwrap_or(true);
            }
        }
        b"i" => {
            if let Some(r) = run.as_mut() {
                r.italic = attr_bool(e).unwrap_or(true);
            }
        }
        b"u" => {
            if let Some(r) = run.as_mut() {
                r.underline = attr_value(e, b"val").or_else(|| Some("single".to_string()));
            }
        }
        b"vertAlign" => {
            if let Some(r) = run.as_mut() {
                r.vert_align = attr_value(e, b"val");
            }
        }
        b"color" => {
            if let Some(r) = run.as_mut() {
                r.color = attr_value(e, b"val");
            }
        }
        b"sz" => {
            if let Some(r) = run.as_mut() {
                r.size = attr_value(e, b"val");
            }
        }
        b"rFonts" => {
            if let Some(r) = run.as_mut() {
                r.font_ascii = attr_value(e, b"ascii").or_else(|| attr_value(e, b"hAnsi"));
                r.font_east_asia = attr_value(e, b"eastAsia");
            }
        }
        _ => {}
    }
}

fn compare_paragraph_format(left: &DocxParagraph, right: &DocxParagraph) -> Option<FormatDiff> {
    let mut changes = Vec::new();
    push_change(
        &mut changes,
        "paragraph.style",
        left.style.as_deref(),
        right.style.as_deref(),
    );
    push_change(
        &mut changes,
        "paragraph.has_drawing",
        Some(if left.has_drawing { "true" } else { "false" }),
        Some(if right.has_drawing { "true" } else { "false" }),
    );

    let left_signature = run_signature(&left.runs);
    let right_signature = run_signature(&right.runs);
    if left_signature != right_signature {
        changes.push(FormatChange {
            field: "runs".to_string(),
            left: left_signature,
            right: right_signature,
        });
    }

    (!changes.is_empty()).then(|| FormatDiff {
        left_index: left.index,
        right_index: right.index,
        text_preview: preview(&left.text, 160),
        changes,
    })
}

fn adjacent_insert_match(
    alignment: &[AlignmentOp],
    pos: usize,
    left: &DocxParagraph,
    right: &[DocxParagraph],
) -> Option<(usize, f64)> {
    let AlignmentOp::Insert(ri) = *alignment.get(pos + 1)? else {
        return None;
    };
    let similarity = paragraph_similarity(left, &right[ri]);
    (similarity >= 0.82).then_some((ri, similarity))
}

fn adjacent_delete_match(
    alignment: &[AlignmentOp],
    pos: usize,
    left: &[DocxParagraph],
    right: &DocxParagraph,
) -> Option<(usize, f64)> {
    let AlignmentOp::Delete(li) = *alignment.get(pos + 1)? else {
        return None;
    };
    let similarity = paragraph_similarity(&left[li], right);
    (similarity >= 0.82).then_some((li, similarity))
}

fn paragraph_similarity(left: &DocxParagraph, right: &DocxParagraph) -> f64 {
    if left.style != right.style {
        return 0.0;
    }
    text_similarity(&left.normalized_text, &right.normalized_text)
}

fn text_similarity(left: &str, right: &str) -> f64 {
    let left = canonical_text_for_similarity(left);
    let right = canonical_text_for_similarity(right);
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }
    if left == right {
        return 1.0;
    }
    let left_chars: Vec<char> = left.chars().collect();
    let right_chars: Vec<char> = right.chars().collect();
    let max_len = left_chars.len().max(right_chars.len());
    if max_len == 0 {
        return 1.0;
    }
    let lcs = lcs_len(&left_chars, &right_chars);
    lcs as f64 / max_len as f64
}

fn canonical_text_for_similarity(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .filter(|c| !matches!(c, '(' | ')' | '（' | '）'))
        .collect()
}

fn lcs_len(left: &[char], right: &[char]) -> usize {
    if left.is_empty() || right.is_empty() {
        return 0;
    }
    let mut prev = vec![0usize; right.len() + 1];
    let mut curr = vec![0usize; right.len() + 1];
    for &lc in left {
        for (j, &rc) in right.iter().enumerate() {
            curr[j + 1] = if lc == rc {
                prev[j] + 1
            } else {
                prev[j + 1].max(curr[j])
            };
        }
        std::mem::swap(&mut prev, &mut curr);
        curr.fill(0);
    }
    prev[right.len()]
}

fn align_paragraphs(left: &[DocxParagraph], right: &[DocxParagraph]) -> Vec<AlignmentOp> {
    let m = left.len();
    let n = right.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in (0..m).rev() {
        for j in (0..n).rev() {
            dp[i][j] = if left[i].normalized_text == right[j].normalized_text {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut ops = Vec::new();
    let mut i = 0;
    let mut j = 0;
    while i < m && j < n {
        if left[i].normalized_text == right[j].normalized_text {
            ops.push(AlignmentOp::Equal(i, j));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            ops.push(AlignmentOp::Delete(i));
            i += 1;
        } else {
            ops.push(AlignmentOp::Insert(j));
            j += 1;
        }
    }
    while i < m {
        ops.push(AlignmentOp::Delete(i));
        i += 1;
    }
    while j < n {
        ops.push(AlignmentOp::Insert(j));
        j += 1;
    }
    ops
}

fn canonicalize_ooxml(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut out = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                out.push('<');
                out.push_str(&String::from_utf8_lossy(e.name().as_ref()));
                push_canonical_attrs(&mut out, &e);
                out.push('>');
            }
            Ok(Event::Empty(e)) => {
                out.push('<');
                out.push_str(&String::from_utf8_lossy(e.name().as_ref()));
                push_canonical_attrs(&mut out, &e);
                out.push_str("/>");
            }
            Ok(Event::End(e)) => {
                out.push_str("</");
                out.push_str(&String::from_utf8_lossy(e.name().as_ref()));
                out.push('>');
            }
            Ok(Event::Text(t)) => {
                let text = t.unescape().unwrap_or_default();
                let text = text.trim();
                if !text.is_empty() {
                    out.push_str(text);
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

fn push_canonical_attrs(out: &mut String, e: &BytesStart<'_>) {
    let mut attrs = Vec::new();
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        if key.to_ascii_lowercase().contains("rsid") {
            continue;
        }
        let value = attr.unescape_value().unwrap_or_default().to_string();
        attrs.push((key, value));
    }
    attrs.sort_by(|a, b| a.0.cmp(&b.0));
    for (key, value) in attrs {
        out.push(' ');
        out.push_str(&key);
        out.push_str("=\"");
        out.push_str(&value);
        out.push('"');
    }
}

fn count_tag(xml: &str, tag: &[u8]) -> usize {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut count = 0usize;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if local_name(e.name().as_ref()) == tag {
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

fn count_media_files(path: &Path) -> Result<usize> {
    let bytes = std::fs::read(path)?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| QualityError::Docx(format!("zip 打开失败：{e}")))?;
    let mut count = 0usize;
    for i in 0..zip.len() {
        let file = zip
            .by_index(i)
            .map_err(|e| QualityError::Docx(format!("zip entry 读取失败：{e}")))?;
        if file.name().starts_with("word/media/") {
            count += 1;
        }
    }
    Ok(count)
}

fn run_signature(runs: &[DocxRun]) -> String {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for run in runs {
        let key = format!(
            "style={};b={};i={};u={};va={};sz={};font={}/{}",
            run.style.as_deref().unwrap_or("-"),
            run.bold,
            run.italic,
            run.underline.as_deref().unwrap_or("-"),
            run.vert_align.as_deref().unwrap_or("-"),
            run.size.as_deref().unwrap_or("-"),
            run.font_ascii.as_deref().unwrap_or("-"),
            run.font_east_asia.as_deref().unwrap_or("-")
        );
        *counts.entry(key).or_insert(0) += 1;
    }
    let mut items = counts.into_iter().collect::<Vec<_>>();
    items.sort_by(|a, b| a.0.cmp(&b.0));
    items
        .into_iter()
        .map(|(k, v)| format!("{k} x{v}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn push_change(
    changes: &mut Vec<FormatChange>,
    field: &str,
    left: Option<&str>,
    right: Option<&str>,
) {
    if left != right {
        changes.push(FormatChange {
            field: field.to_string(),
            left: left.unwrap_or("-").to_string(),
            right: right.unwrap_or("-").to_string(),
        });
    }
}

fn attr_value(e: &BytesStart<'_>, local: &[u8]) -> Option<String> {
    for attr in e.attributes().flatten() {
        if local_name(attr.key.as_ref()) == local {
            return Some(attr.unescape_value().unwrap_or_default().to_string());
        }
    }
    None
}

fn attr_bool(e: &BytesStart<'_>) -> Option<bool> {
    attr_value(e, b"val").map(|v| !matches!(v.as_str(), "0" | "false" | "off"))
}

fn run_has_format(run: &DocxRun) -> bool {
    run.style.is_some()
        || run.bold
        || run.italic
        || run.underline.is_some()
        || run.vert_align.is_some()
        || run.color.is_some()
        || run.size.is_some()
        || run.font_ascii.is_some()
        || run.font_east_asia.is_some()
}

fn normalize_docx_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn stable_hash(text: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn preview(text: &str, max_chars: usize) -> String {
    let normalized = normalize_docx_text(text);
    let mut out = String::new();
    for (idx, ch) in normalized.chars().enumerate() {
        if idx >= max_chars {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out
}

fn local_name(name: &[u8]) -> &[u8] {
    if let Some(colon) = name.iter().position(|&b| b == b':') {
        &name[colon + 1..]
    } else {
        name
    }
}

fn write_style_table(out: &mut String, styles: &BTreeMap<String, usize>) {
    out.push_str("| 样式 | 段落数 |\n");
    out.push_str("| --- | ---: |\n");
    for (style, count) in styles {
        out.push_str(&format!("| {} | {} |\n", escape_md(style), count));
    }
}

fn fmt_idx(idx: Option<usize>) -> String {
    idx.map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn fmt_opt(value: &Option<String>) -> String {
    value
        .as_deref()
        .map(escape_md)
        .unwrap_or_else(|| "-".to_string())
}

fn fmt_similarity(value: Option<f64>) -> String {
    value
        .map(|v| format!("{v:.3}"))
        .unwrap_or_else(|| "-".to_string())
}

fn escape_md(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_detects_insert_and_delete() {
        let left = vec![
            para(1, "A", "Body"),
            para(2, "B", "Body"),
            para(3, "C", "Body"),
        ];
        let right = vec![
            para(1, "A", "Body"),
            para(2, "X", "Body"),
            para(3, "C", "Body"),
        ];
        let ops = align_paragraphs(&left, &right);
        assert!(matches!(ops[0], AlignmentOp::Equal(0, 0)));
        assert!(ops.iter().any(|op| matches!(op, AlignmentOp::Delete(1))));
        assert!(ops.iter().any(|op| matches!(op, AlignmentOp::Insert(1))));
    }

    #[test]
    fn equal_text_with_different_style_is_format_diff() {
        let report = compare_snapshots(
            DocxSnapshot {
                path: "left.docx".into(),
                paragraphs: vec![para(1, "正文", "JOSBody")],
                tables: 0,
                drawings: 0,
                media_files: 0,
                paragraph_styles: BTreeMap::new(),
                document_xml_hash: None,
                styles_xml_hash: None,
            },
            DocxSnapshot {
                path: "right.docx".into(),
                paragraphs: vec![para(1, "正文", "JOSReference")],
                tables: 0,
                drawings: 0,
                media_files: 0,
                paragraph_styles: BTreeMap::new(),
                document_xml_hash: None,
                styles_xml_hash: None,
            },
            &DocxDiffOptions {
                compare_xml_hash: false,
                ..Default::default()
            },
        );
        assert_eq!(report.summary.equal_paragraphs, 1);
        assert_eq!(report.summary.format_changed_paragraphs, 1);
    }

    #[test]
    fn adjacent_delete_insert_with_similar_text_becomes_modified() {
        let report = compare_snapshots(
            DocxSnapshot {
                path: "left.docx".into(),
                paragraphs: vec![para(1, "式 1 和 O(Nlog N)", "JOSBody")],
                tables: 0,
                drawings: 0,
                media_files: 0,
                paragraph_styles: BTreeMap::new(),
                document_xml_hash: None,
                styles_xml_hash: None,
            },
            DocxSnapshot {
                path: "right.docx".into(),
                paragraphs: vec![para(1, "式 (1) 和 O(N log N)", "JOSBody")],
                tables: 0,
                drawings: 0,
                media_files: 0,
                paragraph_styles: BTreeMap::new(),
                document_xml_hash: None,
                styles_xml_hash: None,
            },
            &DocxDiffOptions {
                compare_xml_hash: false,
                ..Default::default()
            },
        );
        assert_eq!(report.summary.modified_paragraphs, 1);
        assert_eq!(report.summary.inserted_paragraphs, 0);
        assert_eq!(report.summary.deleted_paragraphs, 0);
        assert_eq!(report.content_diffs[0].kind, DiffKind::Modified);
    }

    fn para(index: usize, text: &str, style: &str) -> DocxParagraph {
        DocxParagraph {
            index,
            style: Some(style.to_string()),
            text: text.to_string(),
            normalized_text: normalize_docx_text(text),
            has_drawing: false,
            runs: vec![DocxRun {
                text: text.to_string(),
                ..Default::default()
            }],
        }
    }
}
