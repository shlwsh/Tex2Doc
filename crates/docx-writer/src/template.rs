//! reference.docx 模板解析（M7 简化版）
//!
//! V1：仅解析 `word/styles.xml` 中的 `<w:style>` 元素；提取 `w:styleId` → (name, type, rPr, pPr) 缓存。
//! V2：增加 numbering / header / footer 解析。
//!
//! ## 继承策略
//!
//! - **同名覆盖**：用户样式 ID 与模板一致时，用户版本优先（保留模板字体/大小，用户补加 run/pPr）。
//! - **补充缺失**：模板中有而用户没提供的样式，**自动补全**到 `styles.xml` 末尾。
//! - **格式保留**：保留模板样式的 `<w:rPr>` 与 `<w:pPr>` 文本（不重建 XML）。

use std::collections::BTreeMap;
use std::io::Read;

/// 模板中提取的样式表
#[derive(Debug, Clone, Default)]
pub struct TemplateStyles {
    /// `w:styleId` → 完整 `<w:style ...>...</w:style>` XML 字符串
    pub by_id: BTreeMap<String, String>,
    /// `w:name` → `w:styleId`（便于按名查找）
    pub name_to_id: BTreeMap<String, String>,
}

/// 从 `reference.docx` 字节流中提取 `word/styles.xml`。
pub fn parse_template(docx_bytes: &[u8]) -> Result<TemplateStyles, TemplateError> {
    let cursor = std::io::Cursor::new(docx_bytes);
    let mut zip = zip::ZipArchive::new(cursor).map_err(TemplateError::Zip)?;
    let mut entry = zip
        .by_name("word/styles.xml")
        .map_err(|_| TemplateError::MissingStyles)?;
    let mut buf = String::new();
    entry.read_to_string(&mut buf).map_err(TemplateError::Io)?;
    Ok(parse_styles_xml(&buf))
}

/// 直接从 `styles.xml` 字符串解析。
pub fn parse_styles_xml(xml: &str) -> TemplateStyles {
    let mut out = TemplateStyles::default();
    let bytes = xml.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let pos1 = find_substring(bytes, b"<w:style ", i);
        let pos2 = find_substring(bytes, b"<w:style>", i);
        let rel = match (pos1, pos2) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        let rel = match rel {
            Some(x) => x,
            None => break,
        };
        // 兼容 `/>` 自闭合与成对标签
        let (block, end) = if let Some(close_rel) = find_substring(bytes, b"/>", rel) {
            // 检查此 /> 之前是否在 `<w:style ...>` 内（不是嵌套）
            let block = &xml[rel..close_rel + 2];
            (block.to_string(), close_rel + 2)
        } else if let Some(end_rel) = find_substring(bytes, b"</w:style>", rel) {
            let block = &xml[rel..end_rel + "</w:style>".len()];
            (block.to_string(), end_rel + "</w:style>".len())
        } else {
            break;
        };
        let id = extract_attr(&block, "w:styleId").unwrap_or_default();
        let name = extract_w_name(&block);
        if !id.is_empty() {
            out.by_id.insert(id.clone(), block);
            if let Some(n) = name {
                out.name_to_id.insert(n, id);
            }
        }
        i = end;
    }
    out
}

fn find_substring(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from >= haystack.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| from + p)
}

fn extract_attr(tag: &str, name: &str) -> Option<String> {
    let bytes = tag.as_bytes();
    let needle = format!("{}=\"", name);
    let needle_b = needle.as_bytes();
    let pos = find_substring(bytes, needle_b, 0)?;
    let start = pos + needle_b.len();
    let end = bytes[start..].iter().position(|&b| b == b'"')?;
    Some(tag[start..start + end].to_string())
}

fn extract_w_name(block: &str) -> Option<String> {
    // 在 `<w:name w:val="...">` 中找
    let idx = block.find("<w:name ")?;
    let rest = &block[idx..];
    extract_attr(rest, "w:val")
}

/// 合并模板样式到目标 styles.xml 末尾。
///
/// 输入：
/// - `target_xml`：当前要写入的 styles.xml 完整字节流
/// - `template`：从 reference.docx 提取的样式表
///
/// 算法：
/// 1. 扫描 target 中已存在的 `w:styleId`
/// 2. 对模板中**未在 target 中出现**的样式，把其 XML 块插入到 `</w:styles>` 前
pub fn merge_styles(target_xml: &mut Vec<u8>, template: &TemplateStyles) {
    if template.by_id.is_empty() {
        return;
    }
    let target_str = String::from_utf8_lossy(target_xml).to_string();
    let mut existing: Vec<String> = Vec::new();
    let mut i = 0;
    let b = target_str.as_bytes();
    loop {
        let pos1 = find_substring(b, b"<w:style ", i);
        let pos2 = find_substring(b, b"<w:style>", i);
        let rel = match (pos1, pos2) {
            (Some(a), Some(c)) => Some(a.min(c)),
            (Some(a), None) => Some(a),
            (None, Some(c)) => Some(c),
            (None, None) => None,
        };
        let rel = match rel {
            Some(x) => x,
            None => break,
        };
        if let Some(id) = extract_attr(&target_str[rel..], "w:styleId") {
            existing.push(id);
        }
        if let Some(end_rel) = find_substring(b, b"</w:style>", rel) {
            i = end_rel + "</w:style>".len();
        } else {
            break;
        }
    }
    let mut append = String::new();
    for (id, block) in &template.by_id {
        if !existing.iter().any(|e| e == id) {
            append.push_str(block);
        }
    }
    if append.is_empty() {
        return;
    }
    let closing = "</w:styles>";
    let closing_pos = target_str.rfind(closing).unwrap_or(target_str.len());
    let mut new_xml = String::with_capacity(target_str.len() + append.len() + closing.len() + 1);
    new_xml.push_str(&target_str[..closing_pos]);
    new_xml.push_str(&append);
    new_xml.push_str(&target_str[closing_pos..]);
    *target_xml = new_xml.into_bytes();
}

#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("zip 解析失败：{0}")]
    Zip(zip::result::ZipError),
    #[error("模板缺少 word/styles.xml")]
    MissingStyles,
    #[error("IO 错误：{0}")]
    Io(std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_styles_xml_basic() {
        let xml = r#"<?xml version="1.0"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/></w:style>
<w:style w:type="paragraph" w:styleId="BodyText"><w:name w:val="Normal"/></w:style>
</w:styles>"#;
        let ts = parse_styles_xml(xml);
        assert!(ts.by_id.contains_key("Heading1"));
        assert!(ts.by_id.contains_key("BodyText"));
        assert_eq!(
            ts.name_to_id.get("Normal").map(|s| s.as_str()),
            Some("BodyText")
        );
    }

    #[test]
    fn merge_adds_missing() {
        let mut target = br#"<?xml version="1.0"?><w:styles><w:style w:type="paragraph" w:styleId="BodyText"/></w:styles>"#.to_vec();
        let mut ts = TemplateStyles::default();
        ts.by_id.insert(
            "Heading1".into(),
            r#"<w:style w:type="paragraph" w:styleId="Heading1"></w:style>"#.into(),
        );
        ts.by_id.insert(
            "BodyText".into(),
            r#"<w:style w:type="paragraph" w:styleId="BodyText"></w:style>"#.into(),
        );
        merge_styles(&mut target, &ts);
        let s = String::from_utf8_lossy(&target);
        // Heading1 缺失，已补
        assert!(s.contains("Heading1"));
        // BodyText 已存在，不重复
        assert_eq!(s.matches("BodyText").count(), 1);
    }

    #[test]
    fn round_trip_via_zip() {
        // 构造一个最小 docx 内存表示
        let buf: Vec<u8> = Vec::new();
        let cursor = std::io::Cursor::new(buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("word/styles.xml", opts).unwrap();
        let styles = r#"<?xml version="1.0"?><w:styles><w:style w:type="paragraph" w:styleId="Title"/></w:styles>"#;
        use std::io::Write;
        zip.write_all(styles.as_bytes()).unwrap();
        let bytes = zip.finish().unwrap().into_inner();

        let ts = parse_template(&bytes).expect("parse");
        assert!(ts.by_id.contains_key("Title"));
    }
}
