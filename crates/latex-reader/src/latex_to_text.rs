//! V2 引入：bbl 解析、newcommands 提取、front matter 抽取、cite 编号压缩。
//!
//! 与 `docs/to-docx/02-tex-parsing.md` 对齐：把 LaTeX 源文件中的"结构化锚点"提取为
//! 强类型 `FrontMatter` / `CitationMap` / `MacroMap` / `References`，下游 `lower.rs`
//! 在归一化与降级阶段会用到。
//!
//! 关键不变量：
//! - `parse_bbl` 输出的 `references` 顺序就是 `\bibitem` 出现顺序，**不重排**，
//!   这样 cite_map 编号 = `\cite` 中 `[N]` 编号（与 PDF 一致）。
//! - `parse_newcommands` 只识别单行 / 多行 `\newcommand{\X}[n?]{body}`，
//!   已被 `expand.rs` 覆盖到的宏体不重复处理；这里只收集**额外**的宏。

use std::collections::HashMap;

// ─── compress_numbers ──────────────────────────────────────────────────────

/// 数字列表压缩（`[1,2,3] → "1-3"`）。详见 `docs/to-docx/03` §3.3。
pub fn compress_numbers(mut numbers: Vec<usize>) -> String {
    if numbers.is_empty() {
        return String::new();
    }
    numbers.sort_unstable();
    numbers.dedup();
    let mut ranges: Vec<String> = Vec::new();
    let mut start = numbers[0];
    let mut prev = numbers[0];
    for n in numbers.into_iter().skip(1) {
        if n == prev + 1 {
            prev = n;
            continue;
        }
        ranges.push(if start == prev {
            start.to_string()
        } else {
            format!("{start}-{prev}")
        });
        start = n;
        prev = n;
    }
    ranges.push(if start == prev {
        start.to_string()
    } else {
        format!("{start}-{prev}")
    });
    ranges.join(",")
}

// ─── parse_bbl ─────────────────────────────────────────────────────────────

/// 单条参考文献（清洗后纯文本）。
#[derive(Debug, Clone, PartialEq)]
pub struct BibItem {
    pub key: String,
    pub text: String,
}

/// `\bibitem{key}` + body 列表（按出现顺序）。
pub fn parse_bbl(raw: &str) -> (HashMap<String, usize>, Vec<BibItem>) {
    let mut cite_map: HashMap<String, usize> = HashMap::new();
    let mut refs: Vec<BibItem> = Vec::new();
    // 单遍扫描：\bibitem{key} 切片。
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut current_key: Option<String> = None;
    let mut current_body_start: usize = 0;
    let mut refs_in_order: Vec<(String, String)> = Vec::new();

    while i < len {
        // 找 `\bibitem`
        if i + 8 <= len && &raw[i..i + 8] == "\\bibitem" {
            // 可选 [opt]
            let mut p = i + 8;
            if p < len && bytes[p] == b'[' {
                if let Some(end) = crate::normalize::find_matching_bracket(raw, p) {
                    p = end + 1;
                }
            }
            // 跳过空白
            while p < len && (bytes[p] == b' ' || bytes[p] == b'\t' || bytes[p] == b'\n') {
                p += 1;
            }
            // {key}
            if p < len && bytes[p] == b'{' {
                if let Some(end) = crate::normalize::find_matching_brace(raw, p) {
                    // 提交前一个
                    if let Some(k) = current_key.take() {
                        let body = raw[current_body_start..i].to_string();
                        refs_in_order.push((k, body));
                    }
                    let key = raw[p + 1..end].to_string();
                    current_key = Some(key);
                    current_body_start = end + 1;
                    i = end + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    // 提交最后一个
    if let Some(k) = current_key.take() {
        let body = raw[current_body_start..].to_string();
        refs_in_order.push((k, body));
    }

    // 2. 清洗 body：\begin{thebibliography} \end{thebibliography} \newblock {\em …}
    for (idx, (key, body)) in refs_in_order.into_iter().enumerate() {
        let text = clean_bibitem_body(&body);
        let no = idx + 1;
        cite_map.insert(key.clone(), no);
        refs.push(BibItem { key, text });
    }
    (cite_map, refs)
}

/// 解析 `.bib` 文件（`@type{key, field={value}, ...}` 格式），转成 `BibItem` 列表。
///
/// 用于 `\bibliography{refs}` 但 vfs 不含 `.bbl` 的场景：直接把 `.bib` 解析成
/// 55 条 plain text 引用，拼成与 `parse_bbl` 等价的结果。
/// 与 sh oracle 的 `parse_bbl` 输出格式对齐——v13.2 F13 增加支持。
pub fn parse_bib(raw: &str) -> Vec<BibItem> {
    let mut refs: Vec<BibItem> = Vec::new();
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // 找 `@type{`
        if i + 1 < len && bytes[i] == b'@' {
            // 找 `{`
            let mut p = i + 1;
            while p < len && bytes[p] != b'{' {
                p += 1;
            }
            if p >= len {
                break;
            }
            // 找匹配的 `}` —— BibItem body 跨多行可能含 `}`，找第一个匹配到外层 `,` 即可
            // 简单做法：找匹配大括号（嵌套计算）
            let mut depth = 1;
            let mut q = p + 1;
            while q < len && depth > 0 {
                if bytes[q] == b'{' {
                    depth += 1;
                } else if bytes[q] == b'}' {
                    depth -= 1;
                }
                if depth == 0 {
                    break;
                }
                q += 1;
            }
            if q >= len {
                break;
            }
            // body in p+1..q
            let body = &raw[p + 1..q];
            // 第一个 `,` 之前是 key
            let mut key_end = 0;
            let mut brace_depth = 0;
            for (k, c) in body.char_indices() {
                if c == '{' {
                    brace_depth += 1;
                } else if c == '}' {
                    brace_depth -= 1;
                } else if c == ',' && brace_depth == 0 {
                    key_end = k;
                    break;
                }
            }
            let key = body[..key_end].trim().to_string();
            if key.is_empty() {
                i = q + 1;
                continue;
            }
            // body 转 plain text（清洗 `field = {value}, field2 = "value2"`）
            let text = bib_body_to_text(&body[key_end + 1..]);
            refs.push(BibItem { key, text });
            i = q + 1;
        } else {
            i += 1;
        }
    }
    refs
}

/// 把 `.bib` 单条目 body（`field = {value}, field2 = "value2"`）按常见引用格式
/// 转成纯文本：`Author. Title. Journal, Year, vol(issue): pages.`。
/// 与 sh oracle 输出的英文引用格式对齐。
fn bib_body_to_text(body: &str) -> String {
    // 拆 field=value 对
    let mut fields: Vec<(String, String)> = Vec::new();
    let mut current_key: Option<String> = None;
    let mut current_value_start = 0;
    let mut depth = 0;
    let bytes = body.as_bytes();
    let mut i = 0;

    while i < body.len() {
        let c = body[i..].chars().next().unwrap();
        if c == '{' {
            if depth == 0 && current_key.is_none() {
                if let Some(eq) = body[current_value_start..i].find('=') {
                    let k = body[current_value_start..current_value_start + eq]
                        .trim()
                        .to_string();
                    if !k.is_empty() {
                        current_key = Some(k);
                    }
                }
            }
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 && current_key.is_some() {
                let val = body[current_value_start..i].to_string();
                fields.push((current_key.take().unwrap(), val));
                let mut j = i + 1;
                while j < body.len()
                    && (bytes[j] == b','
                        || bytes[j] == b' '
                        || bytes[j] == b'\n'
                        || bytes[j] == b'\t')
                {
                    j += 1;
                }
                current_value_start = j;
                i = j;
                continue;
            }
        } else if c == '=' && depth == 0 && current_key.is_none() {
            let k = body[current_value_start..i].trim();
            if !k.is_empty()
                && k.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            {
                current_key = Some(k.to_string());
                let mut j = i + 1;
                while j < body.len() && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n')
                {
                    j += 1;
                }
                if j < body.len() && bytes[j] == b'{' {
                    current_value_start = j + 1;
                    depth = 1;
                    i = j + 1;
                    continue;
                } else if j < body.len() && bytes[j] == b'"' {
                    if let Some(end) = body[j + 1..].find('"') {
                        let val = body[j + 1..j + 1 + end].to_string();
                        fields.push((current_key.take().unwrap(), val));
                        let mut k = j + 2 + end;
                        while k < body.len()
                            && (bytes[k] == b','
                                || bytes[k] == b' '
                                || bytes[k] == b'\n'
                                || bytes[k] == b'\t')
                        {
                            k += 1;
                        }
                        current_value_start = k;
                        i = k;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    // 拼出 plain text：Author. Title. Journal, Year, vol(issue): pages.
    let mut by_key: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (k, v) in &fields {
        by_key.insert(k.clone(), v.trim().to_string());
    }
    let author = by_key.get("author").cloned().unwrap_or_default();
    let title = by_key.get("title").cloned().unwrap_or_default();
    let journal = by_key.get("journal").cloned().unwrap_or_default();
    let year = by_key.get("year").cloned().unwrap_or_default();
    let volume = by_key.get("volume").cloned().unwrap_or_default();
    let number = by_key.get("number").cloned().unwrap_or_default();
    let pages = by_key.get("pages").cloned().unwrap_or_default();

    let author_str = bib_author_to_text(&author);
    let title_clean = title.replace(['{', '}'], "");
    let title_clean = title_clean
        .replace("``", "\u{201C}")
        .replace("''", "\u{201D}");

    let mut parts = Vec::new();
    if !author_str.is_empty() {
        parts.push(author_str);
    }
    if !title_clean.is_empty() {
        parts.push(format!("{title_clean}."));
    }
    let mut venue = String::new();
    if !journal.is_empty() {
        venue.push_str(&journal.replace(['{', '}'], ""));
    }
    if !volume.is_empty() {
        if !venue.is_empty() {
            venue.push_str(", ");
        }
        venue.push_str(&volume);
        if !number.is_empty() {
            venue.push_str(&format!("({number})"));
        }
    }
    if !pages.is_empty() {
        if !venue.is_empty() {
            venue.push_str(": ");
        }
        let pages_clean = pages.replace("--", "\u{2013}").replace("---", "\u{2014}");
        venue.push_str(&pages_clean);
    }
    if !venue.is_empty() {
        parts.push(format!("{venue}."));
    }
    if !year.is_empty() {
        parts.push(format!("{year}."));
    }
    parts.join(" ")
}

/// 把 BibTeX author 字段（"Last, First and Last2, First2 and ..."）
/// 转成英文引用格式："Last, First, Last2, First2, ..."。
fn bib_author_to_text(s: &str) -> String {
    s.split(" and ")
        .map(|a| a.trim().replace(['{', '}'], ""))
        .filter(|a| !a.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

fn clean_bibitem_body(body: &str) -> String {
    let mut s = body.to_string();
    // 剥 \begin{thebibliography}{N} ... \end{thebibliography}（只在首尾）
    s = strip_outer_env(&s, "thebibliography");
    // \newblock → " "
    s = s.replace("\\newblock", " ");
    // {\em X} / {\it X} / {\bf X} → X
    for cmd in ["em", "it", "bf", "sl", "rm", "sf", "sc", "tt", "up"] {
        s = replace_named_group(&s, cmd);
    }
    // v13.2.1 R7: accent / 罕见字符命令
    s = crate::normalize::replace_command_arg(&s, "c", |inner| inner.to_string());
    s = crate::normalize::replace_command_arg(&s, "rjrare", |inner| inner.to_string());
    // v13.2.1 R7: LaTeX 非断行空格
    s = s.replace('~', " ");
    // 一般清洗：\textbf \textit 等保留内容
    s = s.replace("``", "\u{201C}").replace("''", "\u{201D}");
    s = s.replace("---", "\u{2014}").replace("--", "\u{2013}");
    // 剥 \bibitem 行内 \bibitem 残余（极少出现）
    s = crate::normalize::strip_comments(&s);
    // 通用兜底剥未知命令
    s = crate::normalize::strip_unknown_commands_inline(&s);
    // 缩空白
    s = crate::normalize::collapse_whitespace_pub(&s);
    s.trim().to_string()
}

fn strip_outer_env(text: &str, env: &str) -> String {
    let begin = format!("\\begin{{{env}}}");
    let end = format!("\\end{{{env}}}");
    let mut s = text.to_string();
    if let Some(pos) = s.find(&begin) {
        s = s[pos + begin.len()..].to_string();
    }
    if let Some(pos) = s.find(&end) {
        s.truncate(pos);
    }
    s
}

fn replace_named_group(text: &str, cmd: &str) -> String {
    let token = format!("\\{cmd}");
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + token.len() <= bytes.len() && text[i..i + token.len()] == token {
            // {…} 跟随
            let mut p = i + token.len();
            while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
                p += 1;
            }
            if p < bytes.len() && bytes[p] == b'{' {
                if let Some(end) = crate::normalize::find_matching_brace(text, p) {
                    let inner = &text[p + 1..end];
                    out.push_str(inner);
                    i = end + 1;
                    continue;
                }
            }
        }
        if let Some(ch) = text[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }
    out
}

// ─── parse_newcommands ─────────────────────────────────────────────────────

/// `\newcommand{\X}{body}` 提取 → `HashMap<name, body>`。
///
/// 复用 `normalize::command_arg`，但只对 `newcommand` / `providecommand` /
/// `renewcommand` 三种命令头做扫描。
pub fn parse_newcommands(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for cmd in ["newcommand", "providecommand", "renewcommand"] {
        let mut search_from = 0;
        while let Some(hit) = crate::normalize::command_arg(text, cmd, search_from) {
            // hit.inner = 命令名之后到第一个大括号的内容（不含命令名与外层大括号）
            // hit.cmd_start = 命令名起点
            // hit.after    = 第一个 `}` 之后的位置
            // 所以第一个 `{` 位置 = hit.cmd_start + 命令名长度 + 空白 + ... → 不好算
            // 用 inner 长度反推：open_brace_pos = hit.after - 2 - hit.inner.len()
            let open_rel = match hit.after.checked_sub(2 + hit.inner.len()) {
                Some(v) => v,
                None => {
                    search_from = hit.after;
                    continue;
                }
            };
            // 取 `text[open_rel..]` 一整段（包含 name + [n] + body）
            let s = match text.get(open_rel..) {
                Some(s) => s,
                None => {
                    search_from = hit.after;
                    continue;
                }
            };
            let bytes = s.as_bytes();
            if bytes.first() != Some(&b'{') {
                search_from = hit.after;
                continue;
            }
            // 找匹配的 `}`（name 部分）
            if let Some(end_name_rel) = crate::normalize::find_matching_brace(s, 0) {
                let raw_name = &s[1..end_name_rel];
                let name = raw_name.trim_start_matches('\\').to_string();
                // 跳过空白 + 可选 [n]
                let mut p = end_name_rel + 1;
                while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
                    p += 1;
                }
                if p < bytes.len() && bytes[p] == b'[' {
                    if let Some(close) = s[p..].find(']') {
                        p += close + 1;
                    } else {
                        search_from = hit.after;
                        continue;
                    }
                }
                while p < bytes.len()
                    && (bytes[p] == b' ' || bytes[p] == b'\t' || bytes[p] == b'\n')
                {
                    p += 1;
                }
                if p >= bytes.len() || bytes[p] != b'{' {
                    search_from = hit.after;
                    continue;
                }
                // 找 body 的 `{…}`（body 可跨行，% 注释在 depth=0 时跳过）
                if let Some(end_body_rel) = find_body_end(s, p) {
                    let raw_body = &s[p + 1..end_body_rel];
                    // 跳过 body 起始的 `%` 行续行符：\newcommand{\X}{% \n  body \n}
                    let body = strip_leading_line_continuation(raw_body);
                    out.entry(name).or_insert(body);
                }
            }
            search_from = hit.after;
        }
    }
    out
}

/// 找 body 的 `}` 位置，body 可跨多行 + 行尾 `%` 注释（仅在 depth=0 生效）。
fn find_body_end(s: &str, p: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = p;
    let mut depth = 0i32;
    while i < len {
        let b = bytes[i];
        if b == b'%' && depth == 0 {
            // 跳到行末
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            i += 1;
            continue;
        }
        if b == b'{' {
            depth += 1;
        }
        if b == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// 剥掉 body 起始的 `%` + 换行（LaTeX 的 `\\newcommand{\X}{%\n...` 行续行）。
fn strip_leading_line_continuation(s: &str) -> String {
    let s = s.trim_start();
    if let Some(rest) = s.strip_prefix('%') {
        // 跳到下一个换行
        if let Some(nl) = rest.find('\n') {
            return rest[nl + 1..].to_string();
        }
    }
    s.to_string()
}

// ─── Front matter ──────────────────────────────────────────────────────────

/// 中英文 front matter 抽取结果。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrontMatter {
    pub title_zh: String,
    pub authors_zh: String,
    pub institute_lines: Vec<String>,
    pub abstract_zh: String,
    pub keywords_zh: String,
    pub category: String,
    pub title_en: String,
    pub authors_en: String,
    pub institute_en: String,
    pub abstract_en: String,
    pub keywords_en: String,
    pub running_header: String,
    pub first_footer_text: String,
    pub citation_zh: String,
    pub citation_en: String,
    /// 作者简介条目（"作者简介" section 内的 \item 内容）。
    pub author_bio: Vec<String>,
}

/// 从 `main-jos.tex` 全量抽出 front matter。
///
/// 严格按 `docs/to-docx/02` §2.12 顺序：
/// 1. command_arg: rjtitle / rjauthor / rjinfor / rjcategory / rjhead
/// 2. footnotetext → first_footer_text
/// 3. extract_english_front_matter
/// 4. macros（来自 `parse_newcommands`）→ AbstractContentZh / KeywordsZh / AbstractContentEn / KeywordsEn
/// 5. citation 字段（rjcitation / rjbibstyle）
/// 6. author_bio（`\begin{list}...\end{list}` 内的 \item）
pub fn extract_front_matter(
    main_tex: &str,
    expanded_main: &str,
    macros: &HashMap<String, String>,
) -> FrontMatter {
    let mut fm = FrontMatter {
        title_zh: command_arg_pure(main_tex, "rjtitle").unwrap_or_default(),
        authors_zh: command_arg_pure(main_tex, "rjauthor").unwrap_or_default(),
        ..Default::default()
    };
    let infor = command_arg_pure(main_tex, "rjinfor").unwrap_or_default();
    // V2：latex_to_text 把 `\\` 替换为空格，所以按 "通讯" 关键字分两行
    // 兼容两种格式：`\\` 分行 / 原始换行 / "通讯作者" 作为行 2 起点
    let lines: Vec<String> = if infor.contains("通讯作者") {
        // 形如 "(太原理工大学...) 通讯作者: 石洪雷, E-mail: ..."
        // 按 "通讯作者" 拆：第一段是单位 + 地址，第二段是通讯信息
        let mut iter = infor.splitn(2, "通讯作者");
        let first = iter.next().unwrap_or("").trim().to_string();
        let second = iter.next().unwrap_or("");
        let second = format!("通讯作者{}", second.trim());
        vec![first, second]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        infor
            .split('\n')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    };
    fm.institute_lines = lines;
    fm.category = command_arg_pure(main_tex, "rjcategory").unwrap_or_default();
    fm.running_header = command_arg_pure(main_tex, "rjhead").unwrap_or_default();
    fm.first_footer_text = extract_footnotetext(main_tex);

    // 摘要 / 关键词（先 strip 掉 AbstractContentZh 末尾的 LaTeX 注释 %）
    let strip_pct = |s: String| -> String {
        s.trim()
            .trim_start_matches('%')
            .trim_end_matches('%')
            .trim()
            .to_string()
    };
    fm.abstract_zh = normalize_extracted_text(&strip_pct(
        macros
            .get("\\AbstractContentZh")
            .or_else(|| macros.get("AbstractContentZh"))
            .cloned()
            .unwrap_or_default(),
    ));
    fm.keywords_zh = normalize_extracted_text(&strip_pct(
        macros
            .get("\\KeywordsZh")
            .or_else(|| macros.get("KeywordsZh"))
            .cloned()
            .unwrap_or_default(),
    ));
    fm.abstract_en = normalize_extracted_text(&strip_pct(
        macros
            .get("\\AbstractContentEn")
            .or_else(|| macros.get("AbstractContentEn"))
            .cloned()
            .unwrap_or_default(),
    ));
    fm.keywords_en = normalize_extracted_text(&strip_pct(
        macros
            .get("\\KeywordsEn")
            .or_else(|| macros.get("KeywordsEn"))
            .cloned()
            .unwrap_or_default(),
    ));

    // 英文标题 / 作者 / 机构
    let (title_en, authors_en, institute_en) = extract_english_front_matter(expanded_main);
    fm.title_en = title_en;
    fm.authors_en = authors_en;
    fm.institute_en = institute_en;

    // 引用格式（中英文）
    fm.citation_zh = command_arg_pure(main_tex, "rjcitation").unwrap_or_default();
    fm.citation_en = command_arg_pure(main_tex, "rjbibstyle").unwrap_or_default();
    // 有些模板把中文引用格式放在 rjbibliography
    if fm.citation_zh.is_empty() {
        fm.citation_zh = command_arg_pure(main_tex, "rjbibliography").unwrap_or_default();
    }

    // 作者简介
    fm.author_bio = extract_author_bio(expanded_main);

    fm
}

/// 从 `\foo{...}` 抽出 inner，并做基础 normalize（剥宏、剥花括号外的空白）。
///
/// V2：调用 `latex_to_text` 完整管线，确保 `\hspace{1em}` `\textbf{}` `$math$`
/// 等都被处理（cite/label 用空 map 即可）。
fn command_arg_pure(text: &str, command: &str) -> Option<String> {
    crate::normalize::command_arg(text, command, 0).map(|h| {
        let cite: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let label: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let normalized = crate::normalize::latex_to_text(&h.inner, &cite, &label);
        normalized
            .runs
            .into_iter()
            .map(|r| r.text)
            .collect::<String>()
    })
}

/// 提取 `\footnotetext{...}` 第一处。
fn extract_footnotetext(text: &str) -> String {
    if let Some(hit) = crate::normalize::command_arg(text, "footnotetext", 0) {
        // 剥 \xiaowuhao\song 等字体宏
        let s = crate::normalize::strip_unknown_commands_inline(&hit.inner);
        return crate::normalize::collapse_whitespace_pub(&s)
            .trim()
            .to_string();
    }
    String::new()
}

/// 抽取英文标题/作者/机构。
///
/// 与 `docs/to-docx/02` §2.12.1 对齐：
///   起点 = 第一个 `% ---- 英文标题/作者/机构` 注释的下一行
///   终点 = 下一个 `% ---- 英文摘要` 注释之前
///
/// 区间内：
/// 归一化抽取出来的 front matter 文本：剥宏、合并空白、剥 \hspace/\textbf 等。
fn normalize_extracted_text(s: &str) -> String {
    let cite: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let label: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let normalized = crate::normalize::latex_to_text(s, &cite, &label);
    let joined: String = normalized.runs.into_iter().map(|r| r.text).collect();
    crate::normalize::collapse_whitespace_pub(&joined)
        .trim()
        .to_string()
}

///   title_en   = \textbf{...}
///   authors_en = \vspace{...}{... \xiaowuhao ...} 的内层
///   institute_en = 第一个匹配 `( ... China ... )` 的小括号整体
fn extract_english_front_matter(text: &str) -> (String, String, String) {
    let start_marker = "英文标题";
    let end_marker = "英文摘要";
    let bytes = text.as_bytes();
    let len = bytes.len();
    // 找第一个含 "英文标题" 的注释行
    let mut i = 0;
    let mut start: Option<usize> = None;
    let mut end: Option<usize> = None;
    while i < len {
        if bytes[i] == b'%' {
            // 注释行
            let line_end = text[i..].find('\n').map(|p| i + p).unwrap_or(len);
            let line = &text[i..line_end];
            if start.is_none() && line.contains(start_marker) {
                // 下一行
                let next = text[line_end + 1..]
                    .find('\n')
                    .map(|p| line_end + 1 + p)
                    .unwrap_or(len);
                start = Some(next);
                i = line_end + 1;
                continue;
            }
            if start.is_some() && line.contains(end_marker) {
                end = Some(line_end);
                break;
            }
            i = line_end + 1;
            continue;
        }
        i += 1;
    }
    let s = start.unwrap_or(0);
    let e = end.unwrap_or(len);
    let section = &text[s..e];

    // title_en: \textbf{...}（首个）
    let mut title_en = String::new();
    if let Some(hit) = crate::normalize::command_arg(section, "textbf", 0) {
        title_en = crate::normalize::collapse_whitespace_pub(&hit.inner)
            .trim()
            .to_string();
    }

    // authors_en: \vspace{...}{...} 或 \xiaowuhao{...}（在 title_en 之后的内容）
    let mut authors_en = String::new();
    if let Some(t_pos) = section.find(&title_en) {
        let rest = &section[t_pos + title_en.len()..];
        // 找 \vspace{...}{...} 或 \xiaowuhao{...}
        if let Some(hit) = crate::normalize::command_arg(rest, "vspace", 0) {
            // hit.inner = "{...}{...}"
            let s2 = hit.inner.as_str();
            let sb = s2.as_bytes();
            if sb.first() == Some(&b'{') {
                if let Some(end1) = crate::normalize::find_matching_brace(s2, 0) {
                    // 跳到第二对
                    let mut p = end1 + 1;
                    while p < sb.len() && (sb[p] == b' ' || sb[p] == b'\t') {
                        p += 1;
                    }
                    if p < sb.len() && sb[p] == b'{' {
                        if let Some(end2) = crate::normalize::find_matching_brace(s2, p) {
                            authors_en = s2[p + 1..end2].to_string();
                        }
                    }
                }
            }
        }
        if authors_en.is_empty() {
            // 兜底：找 \xiaowuhao 所在的最外层 {...}，取 inner 文本
            // 适配 {\xiaowuhao\nSHI Hong-Lei, ~ZHAO Juan-Juan} 模式
            if let Some(xpos) = rest.find("\\xiaowuhao") {
                // 向前找最近的 `{`（一定是包含 \xiaowuhao 的 {xxx}）
                if let Some(open_rel) = rest[..xpos].rfind('{') {
                    let open = open_rel;
                    if let Some(close) = crate::normalize::find_matching_brace(rest, open) {
                        let inner = &rest[open + 1..close];
                        // 剥掉前导 \xiaowuhao 宏名
                        let stripped = if let Some(stripped_pos) = inner.find("\\xiaowuhao") {
                            inner[stripped_pos + "\\xiaowuhao".len()..]
                                .trim_start_matches(|c: char| c.is_whitespace())
                        } else {
                            inner
                        };
                        authors_en = stripped.to_string();
                    }
                }
            }
        }
    }

    // institute_en: 第一个匹配 `( ... China ... )` 的小括号整体
    let mut institute_en = String::new();
    if let Some(start_idx) = section.find('(') {
        // 找配对 ')'
        let bytes_rest = section.as_bytes();
        let mut depth = 0i32;
        let mut end_idx = None;
        for (i, &b) in bytes_rest.iter().enumerate().skip(start_idx) {
            match b {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        end_idx = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(end_idx) = end_idx {
            let inner = &section[start_idx + 1..end_idx];
            if inner.contains("China") || inner.contains("Chinese") {
                institute_en = inner.to_string();
            }
        }
    }

    // V2：对三个字段统一走 normalize_extracted_text
    //     去除 ~ (不间断空格)、\fontsize{...}\selectfont、\song 等残留。
    (
        normalize_extracted_text(&title_en),
        normalize_author_en(&authors_en),
        normalize_extracted_text(&institute_en),
    )
}

/// 英文作者行：保留逗号，仅将 `~` 转为空格（不走完整 latex_to_text）。
fn normalize_author_en(s: &str) -> String {
    let s = s.replace(['~', '\n'], " ");
    crate::normalize::collapse_whitespace_pub(s.trim())
}

/// 抽取 `\begin{list}…\end{list}` 内的 `\item`。
fn extract_author_bio(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let begin = "\\begin{list}";
    let end = "\\end{list}";
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        // 防御：保证 i 始终落在 char 边界上
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if let Some(rel) = text[i..].find(begin) {
            let abs = i + rel;
            if let Some(end_rel) = text[abs..].find(end) {
                let body = &text[abs + begin.len()..abs + end_rel];
                // v13.2 F15: 跳过 list body 开头、第一个 `\item` 之前的所有
                //   「参数残渗行」(`\leftmargin=2em \itemindent=-2em ...`)。
                //   之前直接传 body 给 clean_bio_item → latex_to_text →
                //   strip_command("item") 会把 `\itemindent` 误剥为
                //   `indent=-2em` 字面输出。
                let body = skip_list_param_lines(body);
                // 按 \item 切（跳过 list 环境参数 preamble）
                let mut last = 0usize;
                let mut seen_item = false;
                let ci_pairs: Vec<(usize, char)> = body.char_indices().collect();
                let mut ci = 0usize;
                while ci < ci_pairs.len() {
                    let p = ci_pairs[ci].0;
                    if p + 5 <= body.len() && body[p..].starts_with("\\item") {
                        if seen_item && p > last {
                            let seg = &body[last..p];
                            let cleaned = clean_bio_item(seg);
                            if !cleaned.is_empty() {
                                out.push(cleaned);
                            }
                        }
                        seen_item = true;
                        let advance_to = p + 5;
                        let next_ci = match ci_pairs.binary_search_by_key(&advance_to, |&(i, _)| i)
                        {
                            Ok(idx) => idx,
                            Err(idx) => idx,
                        };
                        if next_ci < ci_pairs.len() {
                            last = ci_pairs[next_ci].0;
                            ci = next_ci;
                        } else {
                            last = body.len();
                            ci = ci_pairs.len();
                        }
                        continue;
                    }
                    ci += 1;
                }
                if seen_item && last < body.len() {
                    let seg = &body[last..];
                    let cleaned = clean_bio_item(seg);
                    if !cleaned.is_empty() {
                        out.push(cleaned);
                    }
                }
                i = abs + end_rel + end.len();
                continue;
            }
        }
        i += 1;
    }
    out
}

/// 跳过 list body 开头、第一个 `\item` 之前的「参数残渗行」。
///
/// 用于 `extract_author_bio` 在传给 `latex_to_text` 之前调用，避免
/// `\leftmargin=2em \itemindent=-2em \labelwidth=0pt ...` 里的 `\itemindent`
/// 被 `strip_command("item")` 误剥为 `indent=-2em` 字面输出。
fn skip_list_param_lines(body: &str) -> &str {
    let bytes = body.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        let mut eol = i;
        while eol < len && bytes[eol] != b'\n' {
            eol += 1;
        }
        let line = &body[i..eol];
        let trimmed = line.trim_start();
        if let Some(after) = trimmed.strip_prefix("\\item") {
            if after.is_empty()
                || after.starts_with(' ')
                || after.starts_with('\t')
                || after.starts_with('{')
                || after.starts_with('[')
                || after.starts_with('\r')
            {
                return &body[i..];
            }
        }
        i = eol + 1;
    }
    body
}

fn clean_bio_item(s: &str) -> String {
    let mut raw = s.trim().to_string();
    raw = raw.replace("\\par", " ");
    raw = raw.replace("\\noindent", " ");
    let item_tex = format!(r"\item {raw}");
    let empty_cite = std::collections::HashMap::new();
    let empty_label = std::collections::HashMap::new();
    let mut text =
        crate::normalize::latex_to_text(&item_tex, &empty_cite, &empty_label).join_plain();

    text = text.replace(['{', '}'], "");
    crate::normalize::collapse_whitespace_pub(&text)
        .trim()
        .to_string()
}

// ─── unit tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_numbers_basic() {
        assert_eq!(compress_numbers(vec![1]), "1");
        assert_eq!(compress_numbers(vec![1, 2, 3]), "1-3");
        assert_eq!(compress_numbers(vec![1, 2, 4, 5, 7]), "1-2,4-5,7");
        assert_eq!(compress_numbers(vec![3, 1, 2]), "1-3");
        assert_eq!(compress_numbers(vec![]), "");
    }

    #[test]
    fn parse_bbl_two_items() {
        let raw = r#"\begin{thebibliography}{2}
\bibitem{a}
A. Author, Title A. 2020.
\bibitem{b}
B. Author, Title B. 2021.
\end{thebibliography}"#;
        let (cite, refs) = parse_bbl(raw);
        assert_eq!(cite.get("a"), Some(&1));
        assert_eq!(cite.get("b"), Some(&2));
        assert_eq!(refs.len(), 2);
        assert!(refs[0].text.contains("Author, Title A"));
    }

    #[test]
    fn parse_newcommands_one() {
        let raw = r"\newcommand{\X}{hello}";
        let m = parse_newcommands(raw);
        assert_eq!(m.get("X"), Some(&"hello".to_string()));
    }

    #[test]
    fn parse_newcommands_multi_line() {
        let raw = "before\n\\newcommand{\\Body}{%\n  Body text\n}\nafter";
        let m = parse_newcommands(raw);
        let body = m.get("Body").unwrap();
        assert!(body.contains("Body text"));
    }

    #[test]
    fn clean_bio_item_normalizes_latex() {
        let raw = r"{\hei 石洪雷}，博士，CCF专业会员，主要研究领域为微服务架构开发。
\par\noindent E-mail: \nolinkurl{shihonglei0042@link.tyut.edu.cn}";
        let s = clean_bio_item(raw);
        assert!(s.contains("石洪雷"));
        assert!(s.contains("CCF专业会员"));
        assert!(s.contains("shihonglei0042@link.tyut.edu.cn"));
        assert!(!s.contains("\\hei"));
        assert!(!s.contains("{"));
    }

    #[test]
    fn bibitem_removes_tilde() {
        let body = r"Nabor~C. Smith";
        let s = clean_bibitem_body(body);
        assert!(s.contains("Nabor C."));
        assert!(!s.contains('~'));
    }

    #[test]
    fn bibitem_removes_cedilla() {
        let body = r"Fran{\c{c}}ois Journal";
        let s = clean_bibitem_body(body);
        assert!(s.contains("Francois") || s.contains("Fran c ois") || s.contains("Fran"));
        assert!(!s.contains(r"\c{"));
    }
}
