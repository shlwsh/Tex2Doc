//! BibLaTeX 解析器（M3 完整版）
//!
//! 支持的条目类型：`@inproceedings` / `@article` / `@book` / `@misc` / `@techreport`。
//! 字段：`author` / `title` / `year` / `booktitle` / `journal` / `publisher` / `url`。
//!
//! ## 解析策略
//!
//! 1. 字符级扫描定位 `@type{key, field1 = {value1}, field2 = "value2", ... }`。
//! 2. `key` 读到第一个 `,` 或空白。
//! 3. 字段部分在 `,` 顶部分割；每个字段支持 `value = {...}` / `value = "..."` / `value = bare`。
//! 4. 嵌套花括号正确处理。
//! 5. 错误降级：未闭合自动补；非法条目跳过。

use doc_semantic_ast::BibEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibRawEntry {
    pub entry_type: String,
    pub key: String,
    pub fields: Vec<(String, String)>,
}

/// 解析 BibTeX 源；返回强类型 [`BibEntry`] 列表（仅保留已识别的核心字段）。
pub fn parse(bib: &str) -> Vec<BibEntry> {
    let mut out = Vec::new();
    for raw in parse_raw(bib) {
        if let Some(e) = from_raw(&raw) {
            out.push(e);
        }
    }
    out
}

fn from_raw(r: &BibRawEntry) -> Option<BibEntry> {
    let title = get_field(r, "title")?;
    let year = get_field(r, "year").unwrap_or_default();
    let venue = get_field(r, "booktitle")
        .or_else(|| get_field(r, "journal"))
        .or_else(|| get_field(r, "publisher"));
    let authors = parse_authors(get_field(r, "author").unwrap_or_default().as_str());
    Some(BibEntry {
        key: r.key.clone(),
        authors,
        title,
        year,
        venue,
    })
}

fn get_field(r: &BibRawEntry, name: &str) -> Option<String> {
    r.fields
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

fn parse_authors(s: &str) -> Vec<String> {
    s.split(|c| c == ' ' || c == '\n' || c == '\t')
        .filter(|t| !t.is_empty() && *t != "and")
        .fold(Vec::<String>::new(), |mut acc, t| {
            // 处理 "First Last" / "Last, First"：把单词累积到当前项，遇到 "and" 关闭
            // V1 简化：仅按 "and" 分隔，其它情况原样拼接
            if t == "and" {
                return acc;
            }
            if let Some(last) = acc.last_mut() {
                if !last.is_empty() {
                    last.push(' ');
                }
                last.push_str(t);
            } else {
                acc.push(t.to_string());
            }
            acc
        })
}

/// 解析所有 `@type{...}` 条目。
pub fn parse_raw(bib: &str) -> Vec<BibRawEntry> {
    let bytes = bib.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            // 读 entry_type
            let type_start = i + 1;
            let mut j = type_start;
            while j < bytes.len() && bytes[j] != b'{' && !bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            let entry_type = bib[type_start..j].to_string();
            if entry_type.is_empty() {
                i = j + 1;
                continue;
            }
            // 跳到 `{`
            let mut k = j;
            while k < bytes.len() && bytes[k] != b'{' {
                k += 1;
            }
            if k >= bytes.len() {
                break;
            }
            // 找配对 `}`
            let body_start = k + 1;
            let close = find_matching_brace(bib, k);
            let body_end = close.map(|c| body_start + c).unwrap_or(bib.len());
            let body = &bib[body_start..body_end];
            if let Some(parsed) = parse_one_entry(&entry_type, body) {
                out.push(parsed);
            }
            i = body_end + 1;
        } else {
            i += 1;
        }
    }
    out
}

fn parse_one_entry(entry_type: &str, body: &str) -> Option<BibRawEntry> {
    // 拆 key 与 fields
    let (key, rest) = split_key(body);
    let mut fields = Vec::new();
    for (k, v) in split_fields(rest) {
        fields.push((k, v));
    }
    Some(BibRawEntry {
        entry_type: entry_type.to_string(),
        key,
        fields,
    })
}

fn split_key(body: &str) -> (String, &str) {
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
        i += 1;
    }
    let start = i;
    while i < bytes.len() && bytes[i] != b',' && !bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let key = body[start..i].trim().to_string();
    (key, &body[i..])
}

fn split_fields(body: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut i = 0;
    let bytes = body.as_bytes();
    while i < bytes.len() {
        // 跳空白与逗号
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        // 读 key（到第一个 `=` 之前）
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let key = body[key_start..i].trim().to_string();
        // 跳到 `=`
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] != b'=') {
            if bytes[i] == b'=' {
                break;
            }
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            break;
        }
        i += 1; // 跳过 `=`
        // 跳空白
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        // 读 value
        let val = if bytes[i] == b'{' {
            // 找配对 `}`
            if let Some(off) = find_matching_brace(body, i) {
                let v = body[i + 1..i + 1 + off].to_string();
                i = i + 1 + off + 1;
                v
            } else {
                body[i + 1..].to_string()
            }
        } else if bytes[i] == b'"' {
            // 找下一个 `"`
            let mut m = i + 1;
            while m < bytes.len() && bytes[m] != b'"' {
                m += 1;
            }
            let v = body[i + 1..m].to_string();
            i = m + 1;
            v
        } else {
            // bare value（读到 `,` 顶部）
            let start = i;
            let mut depth = 0i32;
            while i < bytes.len() {
                match bytes[i] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    b',' if depth == 0 => break,
                    _ => {}
                }
                i += 1;
            }
            body[start..i].trim().to_string()
        };
        if !key.is_empty() {
            out.push((key, val));
        }
    }
    out
}

fn find_matching_brace(s: &str, pos: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.get(pos) != Some(&b'{') {
        return None;
    }
    let mut depth = 0i32;
    for (i, &b) in bytes.iter().enumerate().skip(pos) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i - pos - 1);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_article() {
        let src = r#"
@inproceedings{key1,
  author    = {Alice and Bob},
  title     = {A Great Paper},
  booktitle = {Proc. of STOC},
  year      = {2024},
}
"#;
        let entries = parse(src);
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.key, "key1");
        assert_eq!(e.title, "A Great Paper");
        assert_eq!(e.year, "2024");
        assert_eq!(e.venue.as_deref(), Some("Proc. of STOC"));
        assert!(e.authors.iter().any(|a| a.contains("Alice")));
    }

    #[test]
    fn parse_article_journal() {
        let src = r#"
@article{abc,
  author  = "Carol and Dave",
  title   = "Better Things",
  journal = "JACM",
  year    = 2023,
}
"#;
        let entries = parse(src);
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.venue.as_deref(), Some("JACM"));
        assert_eq!(e.year, "2023");
    }

    #[test]
    fn parse_multiple_entries() {
        let src = r#"
@inproceedings{a, author = {A}, title = {TA}, year = {2020}, booktitle = {B1} }
@article{b, author = {B}, title = {TB}, year = {2021}, journal = {J1} }
"#;
        let entries = parse(src);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "a");
        assert_eq!(entries[1].key, "b");
    }

    #[test]
    fn parse_unclosed_recovers() {
        let src = "@article{x, title = {Oops";
        let entries = parse(src);
        // 不 panic
        assert!(entries.len() <= 1);
    }

    #[test]
    fn parse_skips_comments() {
        let src = "
% this is a comment
@book{xx, title = {Book}, year = {2010}}
";
        let entries = parse(src);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Book");
    }
}
