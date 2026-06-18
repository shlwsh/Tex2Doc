//! LaTeX 文本归一化（V2 引入，对齐 `docs/to-docx/03-syntax-normalization.md`）
//!
//! 这是连接 raw LaTeX ↔ docx 文本的关键环节。当前 V1.5 直接把 `\\textbf{...}`、
//! `\\cite{...}`、`\\ref{...}`、`\\rightarrow` 等原样塞进 docx，导致 LibreOffice
//! 转 PDF 后字面残留大量 LaTeX 痕迹。本模块提供一个**纯函数** `latex_to_text`，
//! 对任意 LaTeX 片段（章节正文、表格 cell、图题、算法内联）做归一化，输出
//! "保留富文本语义" 的近 plain 字符串；上标/下标通过 `TextRun::style` 标记。
//!
//! ## 设计原则
//!
//! 1. **彻底性**：先按 `docs/to-docx/03` §3.2 的 26 步流水线复刻，输出与 Python
//!    `latex_to_text` **字节级等价**（同一份输入产生同一份归一结果）。
//! 2. **可观测**：归一后的富文本结构（`Runs`）可观察，便于 docx-writer 把上标/下标
//!    切到 `vertAlign="superscript"/"subscript"`。
//! 3. **不递归到 normalizer 自身**：`clean_math` 处理行内数学时不调 `latex_to_text`，
//!    防止 stack overflow；它处理的语法子集（命令/上下标）比 normalizer 窄。
//! 4. **错误降级**：找不到匹配的命令走"通用兜底" `\\<name>` → 删除；不 panic。
//!
//! ## 与 V1.5 的区别
//!
//! V1.5 的 `lower.rs` 把每个段落当成字符串原样塞进 `TextRun::text`；V2 改为
//! 先把段落切成 `Vec<TextRun>`，再交给 docx-writer 按 run 写 `<w:r>`。

use std::collections::HashMap;

use doc_semantic_ast::TextStyle;

/// LaTeX 文本归一化结果（**带样式 run**）。
///
/// 与 V1.5 兼容：`runs` 的拼接就是最终纯文本（`runs.iter().map(|r| &r.text).collect()`）。
/// 上标/下标通过 `style = TextStyle::Superscript / Subscript` 标记。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NormalizedText {
    pub runs: Vec<NormalizedRun>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedRun {
    pub text: String,
    pub style: TextStyle,
}

impl NormalizedText {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            runs: vec![NormalizedRun {
                text: text.into(),
                style: TextStyle::Plain,
            }],
        }
    }

    pub fn join_plain(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
    }
}

/// 整体归一化入口（章节正文、表格 cell、图题、算法内联都可以用）。
///
/// `cite_map`：`\cite{key}` → 编号（N）。`label_map`：`\ref{label}` → 占位（"1" / "(1)"）。
///
/// 输出：`NormalizedText` 含若干 run；每个 run 的 `style` 决定 docx-writer 是否切上/下标。
pub fn latex_to_text(
    text: &str,
    cite_map: &HashMap<String, usize>,
    label_map: &HashMap<String, String>,
) -> NormalizedText {
    let mut s = text.to_string();
    // 1. 剥注释（strip_comments 内部已能识别奇数个 `\` 转义的 `%`）
    s = strip_comments(&s);
    // 2. CR → LF
    s = s.replace('\r', "\n");
    // 3. \\ → 空格
    s = s.replace("\\\\", " ");
    // 4. \, → 空格（窄不可断空格）
    s = s.replace("\\,", " ");
    // 5. 保护 \{ \}
    s = s.replace("\\{", "\u{FFF0}");
    s = s.replace("\\}", "\u{FFF1}");
    // 6. \cite{...} → [N,M-N]
    s = replace_cite(&s, cite_map);
    // 7. \ref{...} → label_map
    s = replace_ref(&s, label_map);
    // 8. \label{...} → ""
    s = replace_command_arg(&s, "label", |_| String::new());
    // 9. 行内数学 $...$ 与 \(...\) → clean_math(content)
    s = replace_inline_math(&s);
    // 10. \footnote{...} → "（注：内容）"
    s = replace_command_arg(&s, "footnote", |inner| {
        format!(
            "（注：{}）",
            latex_to_text(inner, cite_map, label_map).join_plain()
        )
    });
    // 11. 文本装饰命令：\textbf/\textit/\emph/\url/\nolinkurl/\texttt/\mathrm/\rjrare
    // V2：用 sentinel（\u{0001}B/\u{0001}I/\u{0001}T）包裹，split_runs_with_sup_sub 之后
    // 由 split_styled_runs 切分成多 run（plain + bold + italic + code）。
    s = wrap_styled_command(&s, "textbf", '\u{0001}', 'B');
    s = wrap_styled_command(&s, "textit", '\u{0001}', 'I');
    s = wrap_styled_command(&s, "emph", '\u{0001}', 'E');
    s = wrap_styled_command(&s, "texttt", '\u{0001}', 'T');
    s = wrap_styled_command(&s, "mathrm", '\u{0001}', 'M');
    // url/nolinkurl/rjrare 仍保持原样（plain 即可）
    s = replace_command_arg(&s, "url", |inner| inner.to_string());
    s = replace_command_arg(&s, "nolinkurl", |inner| inner.to_string());
    s = replace_command_arg(&s, "rjrare", |inner| inner.to_string());
    // 12. \item[LABEL] → "LABEL "（label 部分保留）
    s = replace_item_with_label(&s);
    // 13. \item → ""（列表项标记删除）
    s = strip_command(&s, "item");
    // 14. 引号
    s = s.replace("``", "\u{201C}").replace("''", "\u{201D}");
    // 15. 破折号
    s = s.replace("---", "\u{2014}").replace("--", "\u{2013}");
    // 16. 转义（\% 由 strip_comments 保留到此处；strip_inline 已经写成 `\%` 形式）
    s = s
        .replace("\\%", "%")
        .replace("\\&", "&")
        .replace("\\_", "_")
        .replace("\\#", "#")
        .replace("\\$", "$");
    // 17. ~ → " "
    s = s.replace('~', " ");
    // 18. 字体/字号宏删除
    for cmd in [
        "xiaowuhao",
        "wuhao",
        "xiaosihao",
        "sihao",
        "small",
        "centering",
        "noindent",
        "song",
        "kai",
        "hei",
        "fs",
        "par",
        "allowbreak",
        "songti",
        "kaishu",
        "fangsong",
        "heiti",
        "lishu",
    ] {
        s = strip_command(&s, cmd);
    }
    // 19. \fontsize{...}{...}\selectfont → ""
    s = strip_fontsize_selectfont(&s);
    // 20. \hspace \vspace → " "
    s = strip_command_with_braces(&s, "hspace", " ");
    s = strip_command_with_braces(&s, "vspace", " ");
    // 21. 通用兜底：\\[A-Za-z]+\*?(?:\[[^\]]*\])? → ""
    s = strip_unknown_commands(&s);
    // 22. 删外层 { }（保留内层）
    s = strip_outer_braces(&s);
    // 23. 占位符还原
    s = s.replace('\u{FFF0}', "{").replace('\u{FFF1}', "}");
    // 24. 多个连续空白 → 1 个
    s = collapse_whitespace(&s);
    // 25. strip
    s = s.trim().to_string();

    // 26. 切 run：识别 [N] 上标 / ^[X] 上标 / _[X] 下标
    split_runs_with_sup_sub(&s, true, true)
}

// ─── helper：strip_comments ──────────────────────────────────────────────────

/// `docs/to-docx/03` §3.5 — 整行 `%` 注释删除 + 行内 `(?<!\\)%` 删除。
pub fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let (content, nl) = if let Some(stripped) = line.strip_suffix('\n') {
            (stripped, "\n")
        } else {
            (line, "")
        };
        let trimmed_start = content.trim_start();
        if trimmed_start.starts_with('%') {
            out.push_str(nl);
            continue;
        }
        // 行内注释：找首个未被奇数个 `\` 转义的 `%`。
        // 必须以 char 边界扫描，避免在多字节字符内部切片。
        let bytes = content.as_bytes();
        let mut cut: Option<usize> = None;
        let mut i = 0;
        while i < bytes.len() {
            if !content.is_char_boundary(i) {
                i += 1;
                continue;
            }
            if bytes[i] == b'%' {
                // 计数前面的连续 `\`，奇数才算被转义
                let mut backslashes = 0usize;
                let mut k = i;
                while k > 0 {
                    k -= 1;
                    if bytes[k] == b'\\' {
                        backslashes += 1;
                    } else {
                        break;
                    }
                }
                if backslashes % 2 == 0 {
                    cut = Some(i);
                    break;
                }
            }
            // 前进一个 char
            let ch = content[i..].chars().next().unwrap();
            i += ch.len_utf8();
        }
        if let Some(c) = cut {
            out.push_str(&content[..c]);
        } else {
            out.push_str(content);
        }
        out.push_str(nl);
    }
    out
}

// ─── helper：find_matching_brace ─────────────────────────────────────────────

/// 与 `text[open_index] == b'{'` 配对的 `}` 位置。失败返回 None。
///
/// 转义判定：连续 `\` 计数为奇数才算"被转义"。`\\\\{` 视为 `\\` + `\\` + `{`，
/// 最后这个 `{` 是字面量（被 2 个反斜杠保护，偶数）；`\\{` 视为 `\\` + `{`，
/// 这个 `{` 也被保护；`\{` 才视为转义。
pub fn find_matching_brace(text: &str, open_index: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(open_index) != Some(&b'{') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = open_index;
    while i < bytes.len() {
        let b = bytes[i];
        let escaped = {
            let mut count = 0u32;
            let mut k = i;
            while k > 0 && bytes[k - 1] == b'\\' {
                count += 1;
                k -= 1;
            }
            count % 2 == 1
        };
        if b == b'{' && !escaped {
            depth += 1;
        } else if b == b'}' && !escaped {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// 与 `text[open_index] == b'['` 配对的 `]` 位置。失败返回 None。
pub fn find_matching_bracket(text: &str, open_index: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(open_index) != Some(&b'[') {
        return None;
    }
    let mut depth: i32 = 0;
    let mut i = open_index;
    while i < bytes.len() {
        match bytes[i] {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

// ─── helper：command_arg ─────────────────────────────────────────────────────

/// `\foo{bar}` 的 (inner_text, after_end_position)。
pub fn command_arg(text: &str, command: &str, start: usize) -> Option<CommandArgHit> {
    let token = format!("\\{command}");
    // 防御：start 必须落在 char 边界上
    let safe_start = if text.is_char_boundary(start) {
        start
    } else {
        // 向前找最近的 char 边界
        let mut p = start;
        while p > 0 && !text.is_char_boundary(p) {
            p -= 1;
        }
        p
    };
    let rel = text[safe_start..].find(&token)?;
    let cmd_start = safe_start + rel;
    let after_token = cmd_start + token.len();
    // 跳过空白
    let bytes = text.as_bytes();
    let mut p = after_token;
    while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
        p += 1;
    }
    if p >= bytes.len() || bytes[p] != b'{' {
        return None;
    }
    let end = find_matching_brace(text, p)?;
    let inner = text[p + 1..end].to_string();
    Some(CommandArgHit {
        inner,
        cmd_start,
        after: end + 1,
    })
}

pub struct CommandArgHit {
    pub inner: String,
    pub cmd_start: usize,
    pub after: usize,
}

/// 把所有 `\foo{inner}` 替换为 `<sentinel><style>inner<sentinel><style>`。
///
/// `sentinel` 字符与 `style` 字符配合使用：`<sentinel><style>` 是开始，
/// 内部是 raw text（不再做替换），结束也是 `<sentinel><style>`。
/// 调用方需要在最终 split_runs 阶段把这些 sentinel 序列切分成对应 style 的 run。
pub fn wrap_styled_command(text: &str, command: &str, sentinel: char, style: char) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut i = 0;
    let bytes = text.as_bytes();
    let token = format!("\\{command}");
    while i < bytes.len() {
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if i + token.len() <= bytes.len()
            && text.is_char_boundary(i + token.len())
            && &text[i..i + token.len()] == token
        {
            let mut p = i + token.len();
            while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
                p += 1;
            }
            if p < bytes.len() && bytes[p] == b'{' {
                if let Some(end) = find_matching_brace(text, p) {
                    let inner = &text[p + 1..end];
                    out.push(sentinel);
                    out.push(style);
                    out.push_str(inner);
                    out.push(sentinel);
                    out.push(style);
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

/// 对所有 `\foo{inner}` 命中，把 `inner` 喂给 `f`，回写结果。
///
/// `start = pos`（而非 `pos + len(repl)`）以保证嵌套命令不漏；这与 Python 版一致。
pub fn replace_command_arg<F: Fn(&str) -> String>(text: &str, command: &str, f: F) -> String {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let bytes = text.as_bytes();
    let token = format!("\\{command}");
    while i < bytes.len() {
        // 必须落在 char 边界上才能安全 slice
        if !text.is_char_boundary(i) {
            // 防御：跳到下一个边界（CJK 可能让某些路径产生非边界 offset）
            i += 1;
            continue;
        }
        if i + token.len() <= bytes.len() && text.is_char_boundary(i + token.len()) {
            if &text[i..i + token.len()] == token {
                // 检查命令名后是空白 + `{`
                let mut p = i + token.len();
                while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
                    p += 1;
                }
                if p < bytes.len() && bytes[p] == b'{' {
                    if let Some(end) = find_matching_brace(text, p) {
                        let inner = &text[p + 1..end];
                        out.push_str(&f(inner));
                        i = end + 1;
                        continue;
                    }
                }
            }
        }
        // 单字符推进（按 char 边界）
        if let Some(ch) = text[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }
    out
}

// ─── helper：cite / ref / label ─────────────────────────────────────────────

fn replace_cite(text: &str, cite_map: &HashMap<String, usize>) -> String {
    replace_command_arg(text, "cite", |inner| {
        let keys: Vec<&str> = inner
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let numbers: Vec<usize> = keys
            .iter()
            .filter_map(|k| cite_map.get(*k).copied())
            .collect();
        if numbers.is_empty() {
            return "[?]".to_string();
        }
        format!("[{}]", compress_numbers(numbers))
    })
}

fn replace_ref(text: &str, label_map: &HashMap<String, String>) -> String {
    replace_command_arg(text, "ref", |inner| {
        label_map
            .get(inner)
            .cloned()
            .unwrap_or_else(|| "??".to_string())
    })
}

/// 数字列表压缩：`[1,2,3] → "1-3"`、`[1,2,4,5,7] → "1-2,4-5,7"`。
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

// ─── helper：math ───────────────────────────────────────────────────────────

/// 行内数学 `$...$` / `\(...\)` → `clean_math(content)` 替换。`$$...$$` / `\[...\]` 跳过。
pub fn replace_inline_math(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        // $$ / \[ → 原样保留
        if i + 1 < len && bytes[i] == b'$' && bytes[i + 1] == b'$' {
            out.push('$');
            out.push('$');
            i += 2;
            continue;
        }
        if i + 1 < len && bytes[i] == b'\\' && bytes[i + 1] == b'[' {
            out.push_str("\\[");
            i += 2;
            continue;
        }
        if bytes[i] == b'$' {
            // 找下一个 `$`（不跨越 $$）
            let mut j = i + 1;
            while j < len {
                if bytes[j] == b'$' {
                    if j + 1 < len && bytes[j + 1] == b'$' {
                        j += 1;
                        continue;
                    }
                    break;
                }
                j += 1;
            }
            if j >= len {
                out.push('$');
                i += 1;
                continue;
            }
            let math = &text[i + 1..j];
            out.push_str(&clean_math(math));
            i = j + 1;
            continue;
        }
        if bytes[i] == b'\\' && i + 1 < len && bytes[i + 1] == b'(' {
            // \( ... \)
            if let Some(end) = find_substring(&text[i + 2..], "\\)") {
                let math = &text[i + 2..i + 2 + end];
                out.push_str(&clean_math(math));
                i = i + 2 + end + 2;
                continue;
            } else {
                out.push_str("\\(");
                i += 2;
                continue;
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

fn find_substring(haystack: &str, needle: &str) -> Option<usize> {
    haystack.find(needle)
}

/// `clean_math` —— 把 LaTeX 数学内容降级为含 Unicode 符号的纯文本。
///
/// 保留 `_`（下标在 `inline_runs_xml` 阶段切）、保留 `^`（上标同样后续切）。
pub fn clean_math(text: &str) -> String {
    let mut s = text.to_string();
    // 1. 保护 \{ \}
    s = s.replace("\\{", "\u{FFF0}").replace("\\}", "\u{FFF1}");
    s = s.replace("\\,", " ");
    s = s.replace('~', " ");
    // v13.1 P3: \mathcal{X} → Script X (U+210B ℋ, U+1D49C 𝒜, etc.)
    // 必须在 \mathcal 剥外壳前替换
    s = s.replace("\\mathcal{H}", "\u{210B}");
    s = s.replace("\\mathcal{L}", "\u{2112}");
    s = s.replace("\\mathcal{P}", "\u{2118}");
    // 2. \mathrm/\textbf/\textit 内的内容原样保留
    for cmd in [
        "mathrm", "textbf", "textit", "text", "mathbf", "mathit", "mathcal", "mathbb", "mathsf",
        "mathtt",
    ] {
        s = replace_command_arg(&s, cmd, |inner| inner.to_string());
    }
    // 3. 标准符号替换（Unicode 化，源仍是纯 ASCII，s.replace 安全）
    s = s.replace("\\pm", "\u{00B1}");
    s = s.replace("\\rightarrow", "\u{2192}");
    s = s.replace("\\leftarrow", "\u{2190}");
    s = s.replace("\\infty", "\u{221E}");
    s = s.replace("\\leq", "\u{2264}");
    s = s.replace("\\geq", "\u{2265}");
    s = s.replace("\\ll", "\u{226A}");
    s = s.replace("\\gg", "\u{226B}");
    s = s.replace("\\times", "\u{00D7}");
    s = s.replace("\\cdot", "\u{00B7}");
    s = s.replace("\\emptyset", "\u{2205}");
    s = s.replace("\\alpha", "\u{03B1}");
    s = s.replace("\\beta", "\u{03B2}");
    s = s.replace("\\gamma", "\u{03B3}");
    s = s.replace("\\delta", "\u{03B4}");
    s = s.replace("\\epsilon", "\u{03B5}");
    s = s.replace("\\varepsilon", "\u{03B5}");
    s = s.replace("\\lambda", "\u{03BB}");
    s = s.replace("\\theta", "\u{03B8}");
    s = s.replace("\\mu", "\u{03BC}");
    s = s.replace("\\pi", "\u{03C0}");
    s = s.replace("\\rho", "\u{03C1}");
    s = s.replace("\\sigma", "\u{03C3}");
    s = s.replace("\\tau", "\u{03C4}");
    s = s.replace("\\phi", "\u{03C6}");
    s = s.replace("\\varphi", "\u{03C6}");
    s = s.replace("\\xi", "\u{03BE}");
    s = s.replace("\\omega", "\u{03C9}");
    s = s.replace("\\ldots", "\u{2026}");
    s = s.replace("\\dots", "\u{2026}");
    s = s.replace("\\in", "\u{2208}");
    s = s.replace("\\notin", "\u{2209}");
    s = s.replace("\\subset", "\u{2282}");
    s = s.replace("\\supset", "\u{2283}");
    s = s.replace("\\cup", "\u{222A}");
    s = s.replace("\\cap", "\u{2229}");
    s = s.replace("\\to", "\u{2192}");
    s = s.replace("\\Rightarrow", "\u{21D2}");
    s = s.replace("\\Leftarrow", "\u{21D0}");
    s = s.replace("\\Leftrightarrow", "\u{21D4}");
    s = s.replace("\\sum", "\u{2211}");
    s = s.replace("\\prod", "\u{220F}");
    s = s.replace("\\int", "\u{222B}");
    s = s.replace("\\partial", "\u{2202}");
    s = s.replace("\\nabla", "\u{2207}");
    s = s.replace("\\forall", "\u{2200}");
    s = s.replace("\\exists", "\u{2203}");
    s = s.replace("\\neg", "\u{00AC}");
    s = s.replace("\\neq", "\u{2260}");
    s = s.replace("\\approx", "\u{2248}");
    s = s.replace("\\equiv", "\u{2261}");
    s = s.replace("\\sim", "\u{223C}");
    s = s.replace("\\propto", "\u{221D}");
    s = s.replace("\\mapsto", "\u{21A6}");
    s = s.replace("\\langle", "\u{27E8}");
    s = s.replace("\\rangle", "\u{27E9}");
    s = s.replace("\\lfloor", "\u{230A}");
    s = s.replace("\\rfloor", "\u{230B}");
    s = s.replace("\\lceil", "\u{2308}");
    s = s.replace("\\rceil", "\u{2309}");
    s = s.replace("\\bigl", "");
    s = s.replace("\\bigr", "");
    s = s.replace("\\bigl", "");
    s = s.replace("\\bigr", "");
    s = s.replace("\\left", "");
    s = s.replace("\\right", "");
    // 4. 把剩余的 \[A-Za-z]+ 命令名 → 字母（现在文本可能含多字节字符，函数内部按 char 边界扫描）
    s = strip_math_command_names(&s);
    // 5. 反复剥外层 {}（6 次）
    for _ in 0..6 {
        let prev = s.clone();
        // {([^{}]*)}  → \1（按字符扫描，不引入 regex 依赖）
        s = strip_balanced_braces(&s);
        if s == prev {
            break;
        }
    }
    // 6. 占位符还原
    s = s.replace('\u{FFF0}', "{").replace('\u{FFF1}', "}");
    // 7. 多个空白 → 1
    s = collapse_whitespace(&s);
    s.trim().to_string()
}

fn strip_balanced_braces(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        // v13.1 P1: 跳过 ^{...} 和 _{...} 模式, 不剥外层 {}
        if (bytes[i] == b'^' || bytes[i] == b'_')
            && i + 1 < bytes.len()
            && bytes[i + 1] == b'{'
        {
            if let Some(end) = find_matching_brace(text, i + 1) {
                out.push_str(&text[i..=end]);
                i = end + 1;
                continue;
            }
        }
        if bytes[i] == b'{' && (i == 0 || bytes[i - 1] != b'\\') {
            // 找匹配 `}`，要求内部不嵌套 `{}`（prev 已被剥过）
            if let Some(end) = find_matching_brace(text, i) {
                let inner = &text[i + 1..end];
                // 仅当内部不嵌套 {} 时剥
                if !inner.contains('{') {
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

fn strip_math_command_names(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let exceptions = [
        "log", "min", "max", "exp", "sin", "cos", "tan", "ln", "gcd", "mod",
    ];
    while i < text.len() {
        // 关键：扫描 UTF-8 多字节字符串时必须以 char 边界为单位
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if let Some(rest) = text[i..].strip_prefix('\\') {
            // 跳过 `\` 后的字母
            let name_len = rest
                .as_bytes()
                .iter()
                .take_while(|b| b.is_ascii_alphabetic() || **b == b'@')
                .count();
            if name_len > 0 {
                let name = &rest[..name_len];
                if exceptions.contains(&name) {
                    if math_function_needs_leading_space(&out) {
                        out.push(' ');
                    }
                    out.push_str(name);
                } else {
                    // 普通命令：保留字母
                    out.push_str(name);
                }
                i += 1 + name_len;
                continue;
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

fn math_function_needs_leading_space(out: &str) -> bool {
    out.chars()
        .next_back()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ')' | ']' | '}'))
}

// ─── helper：item[label] / item 去除 ────────────────────────────────────────

fn replace_item_with_label(text: &str) -> String {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < len {
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if i + 5 <= len && text.is_char_boundary(i + 5) && &text[i..i + 5] == "\\item" {
            // 边界判断：\item 后是空白 / [ / { / 行尾
            let next = if i + 5 < len { bytes[i + 5] } else { b' ' };
            if next == b' ' || next == b'\t' || next == b'\n' || next == b'\r' || i + 5 == len {
                out.push_str("\\item");
                i += 5;
                continue;
            }
            if next == b'[' {
                // \item[LABEL]
                if let Some(end) = find_matching_bracket(text, i + 5) {
                    let label = &text[i + 6..end];
                    out.push_str(label);
                    out.push(' ');
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

fn strip_command(text: &str, command: &str) -> String {
    let token = format!("\\{command}");
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if i + token.len() <= bytes.len() && text.is_char_boundary(i + token.len()) {
            if &text[i..i + token.len()] == token {
                let next = if i + token.len() < bytes.len() {
                    bytes[i + token.len()]
                } else {
                    b' '
                };
                if !next.is_ascii_alphabetic() && next != b'@' {
                    i += token.len();
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

fn strip_command_with_braces(text: &str, command: &str, replacement: &str) -> String {
    let mut s = text.to_string();
    let new_s = replace_command_arg(&s, command, |_| replacement.to_string());
    s = new_s;
    // 还可能存在 \command （无大括号）
    s = strip_command(&s, command);
    s
}

fn strip_fontsize_selectfont(text: &str) -> String {
    let mut s = text.to_string();
    // \fontsize{...}{...}\selectfont → ""
    let token1 = "\\selectfont";
    while let Some(pos) = s.find(token1) {
        // 找前一个 \fontsize{...}{...}
        if let Some(fs_pos) = s[..pos].rfind("\\fontsize") {
            s = format!("{}{}", &s[..fs_pos], &s[pos + token1.len()..]);
        } else {
            s = format!("{}{}", &s[..pos], &s[pos + token1.len()..]);
        }
    }
    // 单独 \fontsize{...}{...}（无 \selectfont 跟随）
    s = strip_command_with_braces(&s, "fontsize", "");
    s
}

fn strip_unknown_commands(text: &str) -> String {
    strip_unknown_commands_inline(text)
}

/// 与 `strip_unknown_commands` 同语义，作为 `pub` 暴露给 `latex_to_text::*` 调用。
///
/// 与 `latex_to_text` 主体流水线里的 `strip_unknown_commands` 区别仅在于：本函数
/// 不会保护 `\,` `\%` `\{` `}` 等前序已被替换的标点形式——调用方应自己先做转义替换。
pub fn strip_unknown_commands_inline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_alphabetic() {
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j].is_ascii_alphabetic() || bytes[j] == b'@') {
                j += 1;
            }
            // 可选 * 后缀
            if j < bytes.len() && bytes[j] == b'*' {
                j += 1;
            }
            // 可选 [opt]
            if j < bytes.len() && bytes[j] == b'[' {
                if let Some(close) = find_matching_bracket(text, j) {
                    j = close + 1;
                }
            }
            // 关键差异：还吃掉紧随其后的可选 `{…}` / `{…}{…}` / `{…}{…}{…}`，
            // 这样 `\pkgname{foo}` → "foo"（不留花括号）。这是与 `latex_to_text`
            // 主流水线里 26 步算法的"第 21 步通用兜底"语义一致——目标是清除 LaTeX
            // 语法痕迹；如果调用方想保留某个命令的内容，应在前序已知的 `replace_command_arg`
            // 步骤里显式处理（不要走到这里）。
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            while j < bytes.len() && bytes[j] == b'{' {
                if let Some(end) = find_matching_brace(text, j) {
                    let inner = &text[j + 1..end];
                    out.push_str(inner);
                    j = end + 1;
                    while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                        j += 1;
                    }
                } else {
                    break;
                }
            }
            i = j;
            continue;
        }
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
            out.push_str("\\\\");
            i += 2;
            continue;
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

fn strip_outer_braces(text: &str) -> String {
    let s = text.trim();
    if s.starts_with('{') && s.ends_with('}') {
        // 仅当最外层 {} 是配对且内部不再有顶层 {} 时剥
        if let Some(end) = find_matching_brace(s, 0) {
            if end == s.len() - 1 {
                return strip_outer_braces(&s[1..s.len() - 1]);
            }
        }
    }
    text.to_string()
}

fn collapse_whitespace(text: &str) -> String {
    collapse_whitespace_pub(text)
}

/// 公开版 collapse_whitespace：被 `latex_to_text::parse_bbl` / `extract_*` 等调用。
pub fn collapse_whitespace_pub(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_was_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        } else {
            out.push(ch);
            last_was_space = false;
        }
    }
    out
}

// ─── run 切分：上标 / 下标 ──────────────────────────────────────────────────

/// 把归一化后的字符串切为 NormalizedRun 序列，识别：
///   - `[N]` / `[N-M]` / `[N,M]` → 上标
///   - `^[X]` / `^{XYZ}` → 上标
///   - `_[X]` / `_{XYZ}` → 下标（仅当 enable_subscript）
pub fn split_runs_with_sup_sub(
    text: &str,
    enable_superscript: bool,
    enable_subscript: bool,
) -> NormalizedText {
    if text.is_empty() {
        return NormalizedText::default();
    }
    let mut runs: Vec<NormalizedRun> = Vec::new();
    let mut buf = String::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    let flush = |buf: &mut String, runs: &mut Vec<NormalizedRun>| {
        if !buf.is_empty() {
            runs.push(NormalizedRun {
                text: std::mem::take(buf),
                style: TextStyle::Plain,
            });
        }
    };

    while i < len {
        // V2: 检测 sentinel（\u{0001}<style>...）
        if bytes[i] == 0x01 {
            // sentinel 必须是 char 边界
            if !text.is_char_boundary(i) {
                i += 1;
                continue;
            }
            if i + 1 < len && text.is_char_boundary(i + 1) {
                let style_char = text[i + 1..].chars().next().unwrap();
                // 找匹配的 sentinel+style（end marker）
                let end_marker: String = std::iter::once(0x01 as char)
                    .chain(std::iter::once(style_char))
                    .collect();
                if let Some(end_pos) = text[i + 2..].find(&end_marker) {
                    let end_abs = i + 2 + end_pos;
                    flush(&mut buf, &mut runs);
                    let inner = &text[i + 2..end_abs];
                    let target_style = match style_char {
                        'B' => TextStyle::Bold,
                        'I' => TextStyle::Italic,
                        'E' => TextStyle::Italic, // \emph 渲染为 italic
                        'T' => TextStyle::Code,
                        'M' => TextStyle::Plain, // \mathrm 视为 plain
                        _ => TextStyle::Plain,
                    };
                    // 内部可能含 sub/sup，进一步 split
                    let inner_runs =
                        split_runs_with_sup_sub(inner, enable_superscript, enable_subscript);
                    for mut r in inner_runs.runs {
                        r.style = combine_styles(r.style, target_style);
                        runs.push(r);
                    }
                    i = end_abs + 2;
                    continue;
                }
            }
        }
        if enable_superscript {
            // [N] / [N-M] / [N,M,...]
            if bytes[i] == b'[' {
                if let Some(end) = find_matching_bracket(text, i) {
                    let inner = &text[i + 1..end];
                    if is_citation_or_index(inner) {
                        flush(&mut buf, &mut runs);
                        runs.push(NormalizedRun {
                            text: text[i..=end].to_string(),
                            style: TextStyle::Superscript,
                        });
                        i = end + 1;
                        continue;
                    }
                }
            }
            // ^[X] / ^{XYZ}
            if bytes[i] == b'^' && i + 1 < len {
                if bytes[i + 1] == b'{' {
                    if let Some(end) = find_matching_brace(text, i + 1) {
                        let inner = &text[i + 2..end];
                        flush(&mut buf, &mut runs);
                        runs.push(NormalizedRun {
                            text: inner.to_string(),
                            style: TextStyle::Superscript,
                        });
                        i = end + 1;
                        continue;
                    }
                } else {
                    // 单字符上标 ^[A-Za-z0-9*]
                    let ch = bytes[i + 1];
                    if ch.is_ascii_alphanumeric() || ch == b'*' {
                        flush(&mut buf, &mut runs);
                        runs.push(NormalizedRun {
                            text: (ch as char).to_string(),
                            style: TextStyle::Superscript,
                        });
                        i += 2;
                        continue;
                    }
                }
            }
        }
        if enable_subscript && bytes[i] == b'_' && i + 1 < len {
            if bytes[i + 1] == b'{' {
                if let Some(end) = find_matching_brace(text, i + 1) {
                    let inner = &text[i + 2..end];
                    flush(&mut buf, &mut runs);
                    runs.push(NormalizedRun {
                        text: inner.to_string(),
                        style: TextStyle::Subscript,
                    });
                    i = end + 1;
                    continue;
                }
            } else {
                let ch = bytes[i + 1];
                if ch.is_ascii_alphabetic() {
                    let mut end = i + 1;
                    while end < len && bytes[end].is_ascii_alphanumeric() {
                        end += 1;
                    }
                    let word = &text[i + 1..end];
                    if matches!(word, "max" | "min") {
                        flush(&mut buf, &mut runs);
                        runs.push(NormalizedRun {
                            text: word.to_string(),
                            style: TextStyle::Subscript,
                        });
                        i = end;
                        continue;
                    }
                }
                if ch.is_ascii_alphanumeric() {
                    // V2 启发式：若下标候选字符后紧跟另一个 ASCII 字母/数字，
                    // 这通常是 snake_case 代码（`network_mode`），不是下标。
                    // 例外：末尾的 `_X`（X 后是空格/标点/中文/行尾）算下标。
                    if i + 2 < len {
                        let next = bytes[i + 2];
                        if next.is_ascii_alphabetic() || next == b'_' {
                            // 视为代码下划线，原样保留
                            buf.push('_');
                            i += 1;
                            continue;
                        }
                    }
                    flush(&mut buf, &mut runs);
                    runs.push(NormalizedRun {
                        text: (ch as char).to_string(),
                        style: TextStyle::Subscript,
                    });
                    i += 2;
                    continue;
                }
            }
        }
        // 普通字符
        if let Some(ch) = text[i..].chars().next() {
            buf.push(ch);
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }
    flush(&mut buf, &mut runs);
    NormalizedText { runs }
}

/// 把 inner run 样式与外层 wrapper（\\textbf/\\textit 等）合并。
/// 上/下标是位置属性，优先于粗/斜体 wrapper 保留。
fn combine_styles(inner: TextStyle, wrapper: TextStyle) -> TextStyle {
    use TextStyle::*;
    if matches!(inner, Superscript | Subscript) {
        return inner;
    }
    if matches!(wrapper, Superscript | Subscript) {
        return wrapper;
    }
    match (inner, wrapper) {
        (Code, _) | (_, Code) => Code,
        (Bold, Italic) | (Italic, Bold) => BoldItalic,
        (_, Bold) => Bold,
        (_, Italic) => Italic,
        (BoldItalic, _) | (_, BoldItalic) => BoldItalic,
        (MathInline, MathInline) => MathInline,
        (MathInline, _) | (_, MathInline) => MathInline,
        (Plain, x) | (x, Plain) => x,
        (inner, _) => inner,
    }
}

fn is_citation_or_index(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_digit() {
        return false;
    }
    for c in chars {
        if !(c.is_ascii_digit() || c == ',' || c == '-' || c == ' ') {
            return false;
        }
    }
    true
}

// ─── unit tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> (HashMap<String, usize>, HashMap<String, String>) {
        (HashMap::new(), HashMap::new())
    }

    #[test]
    fn strip_comments_basic() {
        let s = "hello % comment\nworld\n% full line\nbody";
        let out = strip_comments(s);
        assert_eq!(out, "hello \nworld\n\nbody");
    }

    #[test]
    fn strip_comments_escaped_percent() {
        let s = "50\\% off";
        let out = strip_comments(s);
        assert_eq!(out, "50\\% off");
    }

    #[test]
    fn find_matching_brace_nested() {
        let text = "{a{b}c}";
        assert_eq!(find_matching_brace(text, 0), Some(6));
        assert_eq!(find_matching_brace(text, 2), Some(4));
    }

    #[test]
    fn find_matching_brace_skip_escape() {
        // String content: `\\{a\}b}` = 8 bytes
        //   pos 0:'\\' 1:'\\' 2:'{' 3:'a' 4:'\\' 5:'}' 6:'b' 7:'}'
        // The first `\\` is an escaped backslash (literal `\`).
        // The `{` at pos 2 is the opening brace.
        // Inside, `a`, then `\}b` (escaped `}` then `b`), then closing `}` at pos 7.
        // find_matching_brace(text, 2) → Some(7)
        let text = "\\\\{a\\}b}";
        assert_eq!(find_matching_brace(text, 2), Some(7));
    }

    #[test]
    fn command_arg_basic() {
        let hit = command_arg("\\title{Hello}", "title", 0).unwrap();
        assert_eq!(hit.inner, "Hello");
    }

    #[test]
    fn compress_numbers_basic() {
        assert_eq!(compress_numbers(vec![1]), "1");
        assert_eq!(compress_numbers(vec![1, 2, 3]), "1-3");
        assert_eq!(compress_numbers(vec![1, 2, 4, 5, 7]), "1-2,4-5,7");
        assert_eq!(compress_numbers(vec![3, 1, 2]), "1-3");
        assert_eq!(compress_numbers(vec![]), "");
    }

    #[test]
    fn clean_math_simple() {
        let out = clean_math("\\pm");
        assert_eq!(out, "\u{00B1}");
        let out = clean_math("\\rightarrow");
        assert_eq!(out, "\u{2192}");
        // `\\{` 是 LaTeX 对字面 `{` 的转义，clean_math 保留其语义
        let out = clean_math("L=\\{l_1,\\ldots,l_N\\}");
        assert_eq!(out, "L={l_1,…,l_N}");
    }

    #[test]
    fn clean_math_strip_braces() {
        // {\alpha} 的外层 {} 是分组（无内容保护），被剥
        let out = clean_math("{\\alpha}");
        assert_eq!(out, "α");
    }

    #[test]
    fn clean_math_common_greek_and_fonts() {
        let out = clean_math("\\mathrm{Score}+\\gamma+\\delta+\\lambda+\\theta+\\mathcal{H}");
        // v13.1 P3: \mathcal{H} 现在映射为 ℋ (U+210B) 而非 H
        assert_eq!(out, "Score+γ+δ+λ+θ+\u{210B}");
    }

    #[test]
    fn clean_math_function_keeps_space_after_variable() {
        let out = clean_math("O(N\\log N)+O(N+M\\log M)+O(M\\log K)");
        assert_eq!(out, "O(N log N)+O(N+M log M)+O(M log K)");
    }

    #[test]
    fn latex_to_text_math_function_subscript() {
        let (cite, label) = empty();
        let n = latex_to_text("$d_{\\max}+d_0$", &cite, &label);
        let plain = n.join_plain();
        assert_eq!(plain, "dmax+d0");
        assert!(!plain.contains('_'), "got: {plain}");
        assert!(n
            .runs
            .iter()
            .any(|r| r.text == "max" && r.style == TextStyle::Subscript));
    }

    #[test]
    fn latex_to_text_cite() {
        let (mut cite, label) = empty();
        cite.insert("a".into(), 1);
        cite.insert("b".into(), 2);
        cite.insert("c".into(), 3);
        let n = latex_to_text("see \\cite{a,b,c} for details", &cite, &label);
        let plain = n.join_plain();
        assert!(plain.contains("[1-3]"), "got: {plain}");
    }

    #[test]
    fn latex_to_text_strip_textbf() {
        let (cite, label) = empty();
        let n = latex_to_text("\\textbf{摘要}", &cite, &label);
        let plain = n.join_plain();
        assert_eq!(plain, "摘要");
    }

    #[test]
    fn latex_to_text_math_unicode() {
        let (cite, label) = empty();
        let n = latex_to_text("$L=\\{l_1\\}$", &cite, &label);
        // `\\{` 是 LaTeX 对字面 `{` 的转义，clean_math 保留其语义 → 输出含 `{` `}`
        // `_1` 在 `split_runs_with_sup_sub` 阶段切为下标 run，"_" 自身是分隔符
        // 不出现在 plain 文本中。
        assert!(n
            .runs
            .iter()
            .any(|r| r.text == "1" && r.style == TextStyle::Subscript));
        let plain = n.join_plain();
        assert!(plain.contains("L={l"), "got: {plain}");
        assert!(!plain.contains("\\"), "leaked backslash: {plain}");
    }

    #[test]
    fn latex_to_text_quotes_dash() {
        let (cite, label) = empty();
        let n = latex_to_text("``hello'' --- world -- foo", &cite, &label);
        let plain = n.join_plain();
        assert!(plain.contains('\u{201C}'));
        assert!(plain.contains('\u{2014}'));
        assert!(plain.contains('\u{2013}'));
    }

    #[test]
    fn latex_to_text_fontsize() {
        let (cite, label) = empty();
        let n = latex_to_text("\\fontsize{7.5pt}{12pt}\\selectfont Body", &cite, &label);
        let plain = n.join_plain();
        assert!(!plain.contains("\\fontsize"), "leak: {plain}");
        assert!(!plain.contains("\\selectfont"), "leak: {plain}");
    }

    #[test]
    fn latex_to_text_escapes() {
        let (cite, label) = empty();
        let n = latex_to_text("50\\% off \\& more", &cite, &label);
        let plain = n.join_plain();
        assert_eq!(plain, "50% off & more");
    }

    #[test]
    fn latex_to_text_escaped_percent_in_long_text() {
        let (cite, label) = empty();
        let n = latex_to_text(
            "降幅 98.4\\%，策略过滤独立贡献 67.8\\%，Agent CPU 与内存开销分别降低 37.5\\% 和 2.0\\%。",
            &cite,
            &label,
        );
        let plain = n.join_plain();
        // 应当保留 4 个字面 %
        let count = plain.matches('%').count();
        assert_eq!(count, 4, "got: {plain:?} ({} %)", count);
    }

    #[test]
    fn latex_to_text_strip_unknown_commands() {
        let (cite, label) = empty();
        let n = latex_to_text("\\pkgname{foo} bar", &cite, &label);
        let plain = n.join_plain();
        // strip_unknown_commands_inline 会把 \pkgname{foo} 整段替换为 `foo`（不带尾随空格），
        // 这是 V1 26 步算法的第 21 步通用兜底语义；调用方应在 replace_command_arg 前序步骤
        // 处理需要保留的命令。
        assert_eq!(plain, "foobar");
    }

    #[test]
    fn latex_to_text_footnote_star_in_table_cell() {
        let cite = HashMap::new();
        let label = HashMap::new();
        let n = latex_to_text("72 vs 4388 条$^*$", &cite, &label);
        assert!(
            n.runs
                .iter()
                .any(|r| r.style == TextStyle::Superscript && r.text == "*"),
            "runs: {:?}",
            n.runs
                .iter()
                .map(|r| (&r.text, r.style))
                .collect::<Vec<_>>()
        );
        let n2 = latex_to_text(r"\textbf{72 vs 4388 条$^*$}", &cite, &label);
        assert!(
            n2.runs
                .iter()
                .any(|r| r.style == TextStyle::Superscript && r.text == "*"),
            "textbf runs: {:?}",
            n2.runs
                .iter()
                .map(|r| (&r.text, r.style))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn split_runs_sup() {
        let n = split_runs_with_sup_sub("Top-[1-2] and ^2", true, false);
        assert_eq!(n.runs.len(), 4);
        assert_eq!(n.runs[0].text, "Top-");
        assert_eq!(n.runs[0].style, TextStyle::Plain);
        assert_eq!(n.runs[1].text, "[1-2]");
        assert_eq!(n.runs[1].style, TextStyle::Superscript);
        assert_eq!(n.runs[2].text, " and ");
        assert_eq!(n.runs[3].text, "2");
        assert_eq!(n.runs[3].style, TextStyle::Superscript);
    }

    #[test]
    fn split_runs_sub() {
        let n = split_runs_with_sup_sub("l_1 and ^{10}", true, true);
        assert!(n
            .runs
            .iter()
            .any(|r| r.text == "1" && r.style == TextStyle::Subscript));
        assert!(n
            .runs
            .iter()
            .any(|r| r.text == "10" && r.style == TextStyle::Superscript));
    }

    // v13.1 P1 regression: clean_math 不应剥 ^{...} _{...} 的外层 {}
    #[test]
    fn clean_math_preserves_sup_sub_braces() {
        // 关键回归: ^{**} 之前被 clean_math 剥成 ^**^** 然后切成 sup+plain+sup+plain
        // 现在 strip_balanced_braces 跳过 ^{...} 模式, 保持 ^{**} 完整让 split_runs 切为 sup
        let n = split_runs_with_sup_sub("5.06e-03$^{**}$", true, false);
        // 期望 2 个 run: plain "5.06e-03" + sup "**" (不要把 ** 拆开)
        let sup_runs: Vec<&str> = n
            .runs
            .iter()
            .filter(|r| r.style == TextStyle::Superscript)
            .map(|r| r.text.as_str())
            .collect();
        assert_eq!(sup_runs, vec!["**"], "** must stay as one sup run, not split");
    }
}
