//! algorithm2e 环境的状态机解析（V2 引入，对齐 `docs/to-docx/02-tex-parsing.md` §2.13）
//!
//! 输入：`\begin{algorithm}...\end{algorithm}` 内部的源码。
//! 输出：`Vec<AlgLine>`，每行含缩进级数 / 引导线 / 代码 / 注释 / 关键字。
//!
//! ## 状态机说明
//!
//! - `\ForEach{cond}{body}` → 关键字 `ForEach`，递归下降 body
//! - `\If{cond}{body}` → 关键字 `If`，递归下降 body
//! - `\Return{value}` → 关键字 `Return`，追加 `;`（若原行含 `\;`）
//! - `\;` → 语句分隔符
//! - `\tcp*{...}` → 行尾注释
//!
//! ## Rust 简化版
//!
//! 与 Python 完整版不同，Rust 当前实现仅做"扁平切行 + 关键字识别"，不渲染竖线。
//! guides / end_guides 字段保留以备扩展。
//!
//! 输入示例：
//! ```latex
//! \KwIn{logs}
//! \KwOut{filtered}
//! \ForEach{$l \in L$}{
//!     \If{$l.status = 5xx$}{
//!         keep($l$)\;
//!     }
//! }
//! \Return{filt}
//! ```

use doc_semantic_ast::AlgLine;
use std::collections::HashMap;

/// 算法内 LaTeX 数学符号 Unicode 化。
fn normalize_alg_math(s: &str) -> String {
    let mut r = s.to_string();
    // v13.2 F17d: 先把 \\; 转为 ;（algorithm2e 行终止符）
    r = r.replace("\\;", ";");
    // 多重替换：先长后短
    r = r.replace("\\leftarrow", "←");
    r = r.replace("\\Leftarrow", "⇐");
    r = r.replace("\\rightarrow", "→");
    r = r.replace("\\Rightarrow", "⇒");
    r = r.replace("\\geq", "≥");
    r = r.replace("\\leq", "≤");
    r = r.replace("\\neq", "≠");
    r = r.replace("\\alpha", "α");
    r = r.replace("\\beta", "β");
    r = r.replace("\\gamma", "γ");
    r = r.replace("\\delta", "δ");
    r = r.replace("\\epsilon", "ε");
    r = r.replace("\\lambda", "λ");
    r = r.replace("\\mu", "μ");
    r = r.replace("\\pi", "π");
    r = r.replace("\\sigma", "σ");
    r = r.replace("\\tau", "τ");
    r = r.replace("\\phi", "φ");
    r = r.replace("\\psi", "ψ");
    r = r.replace("\\omega", "ω");
    r = r.replace("\\infty", "∞");
    r = r.replace("\\times", "×");
    r = r.replace("\\cdot", "·");
    r = r.replace("\\ge", "≥");
    r = r.replace("\\le", "≤");
    r = r.replace("\\mod", " mod ");
    r = r.replace("\\bmod", " mod ");
    // 剥残留的 `\name{...}` 宏（\mathrm 等）
    r = strip_remaining_math_macros(&r);
    r.trim().to_string()
}

/// 剥残留的 `\mathrm` `\mathbf` `\mathsf` `\mathsf*` 等。
fn strip_remaining_math_macros(s: &str) -> String {
    let tokens = [
        "mathrm", "mathbf", "mathsf", "mathsf*", "mathtt", "mathcal", "mathit", "mathbb", "prem",
        "bmod",
    ];
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !s.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let mut matched = false;
        for tok in &tokens {
            let token = format!("\\{tok}");
            if i + token.len() + 1 <= bytes.len() && s[i..].starts_with(&token) {
                let after = i + token.len();
                if bytes[after] == b'{' {
                    // 找配对的 }
                    let mut depth = 0i32;
                    let mut j = after;
                    while j < bytes.len() {
                        if bytes[j] == b'{' {
                            depth += 1;
                        } else if bytes[j] == b'}' {
                            depth -= 1;
                            if depth == 0 {
                                i = j + 1;
                                matched = true;
                                break;
                            }
                        }
                        j += 1;
                    }
                    if !matched {
                        i += token.len();
                        matched = true;
                    }
                    break;
                }
            }
        }
        if !matched {
            if let Some(ch) = s[i..].chars().next() {
                out.push(ch);
                i += ch.len_utf8();
            } else {
                i += 1;
            }
        }
    }
    out
}

/// 用 `latex_to_text` 归一化算法代码行。
fn normalize_alg_code(raw: &str) -> String {
    let cite: HashMap<String, usize> = HashMap::new();
    let label: HashMap<String, String> = HashMap::new();
    let mut s = raw.to_string();
    // 剥元数据命令
    for cmd in [
        "KwIn", "KwOut", "KwData", "KwResult", "KwBody", "caption", "label",
    ] {
        s = strip_cmd(&s, cmd);
    }
    // 先走通用 normalizer
    let normalized = crate::normalize::latex_to_text(&s, &cite, &label);
    let text = normalized.join_plain();
    // 再补数学符号 Unicode 化（algorithm2e 常用符号）
    normalize_alg_math(&text)
}

/// 剥掉 `\cmd{...}` 命令。
fn strip_cmd(s: &str, cmd: &str) -> String {
    let prefix = format!("\\{cmd}"); // e.g. "\KwIn"
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !s.is_char_boundary(i) {
            i += 1;
            continue;
        }
        // 检查前缀 "\cmd{"
        if i + prefix.len() + 1 <= bytes.len()
            && s[i..].starts_with(&prefix)
            && bytes[i + prefix.len()] == b'{'
        {
            let brace_pos = i + prefix.len(); // position of '{'
                                              // 找配对的 }
            let mut depth = 0i32;
            let mut j = brace_pos;
            while j < bytes.len() {
                if bytes[j] == b'{' {
                    depth += 1;
                } else if bytes[j] == b'}' {
                    depth -= 1;
                    if depth == 0 {
                        i = j + 1;
                        break;
                    }
                }
                j += 1;
            }
            if depth != 0 {
                // 未匹配，i 推进到末尾
                i = bytes.len();
            }
            continue;
        }
        if let Some(ch) = s[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }
    out
}

/// 从 algorithm body 提取 caption / label / io（`\KwIn` / `\KwOut`）。
pub fn extract_algorithm_io(body: &str) -> (Vec<(String, String)>, Option<String>, String) {
    let mut io: Vec<(String, String)> = Vec::new();
    let mut caption: Option<String> = None;
    let mut label = String::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(inner) = extract_io_arg(trimmed, "KwIn") {
            io.push(("Input".to_string(), normalize_alg_code(&inner)));
        } else if let Some(inner) = extract_io_arg(trimmed, "KwOut") {
            io.push(("Output".to_string(), normalize_alg_code(&inner)));
        } else if let Some(inner) = extract_io_arg(trimmed, "KwData") {
            io.push(("Data".to_string(), normalize_alg_code(&inner)));
        } else if let Some(inner) = extract_io_arg(trimmed, "KwResult") {
            io.push(("Result".to_string(), normalize_alg_code(&inner)));
        } else if let Some(inner) = extract_brace_arg_inline(trimmed, "caption") {
            caption = Some(normalize_alg_code(&inner));
        } else if let Some(inner) = extract_brace_arg_inline(trimmed, "label") {
            label = inner;
        }
    }
    (io, caption, label)
}

/// 抽 `\cmd{...}` 的内容（无 `\caption` 之外的 \label 处理；保持独立）。
fn extract_io_arg(line: &str, cmd: &str) -> Option<String> {
    extract_brace_arg_inline(line, cmd)
}

fn extract_brace_arg_inline(line: &str, cmd: &str) -> Option<String> {
    let rest = line.strip_prefix(&format!("\\{cmd}"))?;
    let rest = rest.trim_start();
    if !rest.starts_with('{') {
        return None;
    }
    let mut depth = 0;
    for (i, ch) in rest.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(rest[1..i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// 主入口：把 algorithm body 解析为 `Vec<AlgLine>`。
///
/// 实现策略（简化版）：
/// 1. 剥 `\caption` `\label` `\KwIn` `\KwOut` 等元数据
/// 2. 把 body 切成"逻辑行"（`\;` 或换行作为行终止）
/// 3. 每行识别前缀关键字：`\ForEach` / `\If` / `\Return` / `else` 等
/// 剥掉算法行内的元数据命令（`\KwIn` `\KwOut` 等）。
fn strip_algorithm_line_meta(line: LogicLine) -> LogicLine {
    let mut s = line.text;
    for cmd in [
        "KwIn", "KwOut", "KwData", "KwResult", "KwBody", "caption", "label",
    ] {
        s = strip_cmd(&s, cmd);
    }
    LogicLine {
        text: s,
        comment: line.comment,
    }
}

/// 4. 计算缩进级数（嵌套 ForEach/If +1）
pub fn parse_algorithm_rows(body: &str) -> Vec<AlgLine> {
    let mut out: Vec<AlgLine> = Vec::new();
    let mut iter = LogicLineIter::new(body);
    let mut indent_stack: Vec<u8> = vec![0];
    while let Some(line) = iter.next() {
        // V2 归一化：剥算法元数据命令
        let line = strip_algorithm_line_meta(line);
        let keyword = detect_keyword(&line.text);
        if let Some(kw) = &keyword {
            match kw.as_str() {
                "ForEach" | "If" | "For" | "While" => {
                    // 找到 cond 与 body
                    let (cond, body) = match split_two_braces(&line.text) {
                        Some(pair) => pair,
                        None => ("".to_string(), String::new()),
                    };
                    let indent = *indent_stack.last().unwrap_or(&0);
                    indent_stack.push(indent + 1);
                    let guides: Vec<u8> = indent_stack
                        .iter()
                        .take(indent_stack.len() - 1)
                        .copied()
                        .collect();
                    let end_guides = guides.clone();
                    out.push(AlgLine {
                        indent,
                        guides,
                        end_guides,
                        code: format!("{kw} ({})", normalize_alg_code(&cond)),
                        comment: normalize_alg_code(&line.comment),
                        keyword: Some(kw.clone()),
                    });
                    // 递归处理 body
                    let mut sub = parse_algorithm_rows(&body);
                    for s in &mut sub {
                        s.indent += 1;
                    }
                    out.extend(sub);
                    // 关闭：end 行
                    let close_indent = indent;
                    indent_stack.pop();
                    out.push(AlgLine {
                        indent: close_indent,
                        guides: indent_stack
                            .iter()
                            .take(indent_stack.len())
                            .copied()
                            .collect(),
                        end_guides: vec![],
                        code: "end".to_string(),
                        comment: String::new(),
                        keyword: Some("End".to_string()),
                    });
                    continue;
                }
                "Return" => {
                    let raw_val = extract_single_brace(&line.text).unwrap_or_default();
                    let has_semi = line.text.contains("\\;");
                    let val = normalize_alg_code(&raw_val);
                    let code = if has_semi {
                        format!("return {val};")
                    } else {
                        format!("return {val}")
                    };
                    let indent = *indent_stack.last().unwrap_or(&0);
                    out.push(AlgLine {
                        indent,
                        guides: indent_stack
                            .iter()
                            .take(indent_stack.len())
                            .copied()
                            .collect(),
                        end_guides: vec![],
                        code,
                        comment: normalize_alg_code(&line.comment),
                        keyword: Some("Return".to_string()),
                    });
                    continue;
                }
                _ => {}
            }
        }
        // 普通语句
        // v13.2 F17d: 保留 `\;` 在 line.text，由 normalize_alg_code → normalize_alg_math
        //   转为 `;`（之前 replace 为空格丢掉了语句末尾的 `;`）。
        let mut code = normalize_alg_code(&line.text);
        // v13.2 F20: 跳过空文本行（strip_caption/KwIn/KwOut/label 后空），避免被下面的
        //   `!ends_with(';')` 分支加上 `;` 输出空 `;` 行。
        if code.is_empty() {
            continue;
        }
        // sh 版行为：每行后默认加 `;`（即使源码没有 `\;`）。
        // v13.2 F17e: 如果 text 已含 `;`（来自 `\;`）则不再加。
        if !code.trim_end().ends_with(';') && !code.trim_end().ends_with(',') {
            code.push(';');
        }
        let indent = *indent_stack.last().unwrap_or(&0);
        out.push(AlgLine {
            indent,
            guides: indent_stack
                .iter()
                .take(indent_stack.len())
                .copied()
                .collect(),
            end_guides: vec![],
            code,
            comment: normalize_alg_code(&line.comment),
            keyword: None,
        });
    }
    out
}

/// 一行逻辑行：包含代码正文与可选的 `\tcp*` 注释。
#[derive(Debug, Clone, Default)]
struct LogicLine {
    text: String,
    comment: String,
}

/// 把 body 切成逻辑行（`\;` 作为语句分隔符）。
///
/// 同时把每行内的 `\tcp*{...}` 抽出来作为 comment。
struct LogicLineIter<'a> {
    body: &'a str,
    pos: usize,
}

impl<'a> LogicLineIter<'a> {
    fn new(body: &'a str) -> Self {
        Self { body, pos: 0 }
    }
}

impl<'a> Iterator for LogicLineIter<'a> {
    type Item = LogicLine;

    fn next(&mut self) -> Option<Self::Item> {
        let body = self.body;
        let bytes = body.as_bytes();
        let len = bytes.len();
        while self.pos < len {
            // 防御 char boundary
            if !body.is_char_boundary(self.pos) {
                self.pos += 1;
                continue;
            }
            let b = bytes[self.pos];
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                self.pos += 1;
                continue;
            }
            // 读取到下一个 `\;` 或行尾或 `}`
            let start = self.pos;
            let mut i = start;
            let mut comment: Option<String> = None;
            while i < len {
                let cb = bytes[i];
                if cb == b'\\' && i + 1 < len {
                    let next = bytes[i + 1];
                    if next == b';' {
                        // v13.2 F17d: 保留 `\;` 在 line.text（让 normalize_alg_code
                        //   后处理转为 `;`），仅终止本行——之前直接 i+=2 跳过 `\;`
                        //   丢掉了语句末尾的 `;`。
                        i += 2;
                        break;
                    }
                    if next == b'%' {
                        // 转义的百分号，原样保留
                        i += 2;
                        continue;
                    }
                    // 检查 \tcp*{...}
                    if i + 5 <= len && &body[i..i + 5] == "\\tcp*" {
                        // 跳到下一个 {
                        let mut p = i + 5;
                        while p < len && (bytes[p] == b' ' || bytes[p] == b'\t') {
                            p += 1;
                        }
                        if p < len && bytes[p] == b'{' {
                            if let Some(end) = find_matching_brace_at(body, p) {
                                let inner = &body[p + 1..end];
                                comment = Some(inner.to_string());
                                i = end + 1;
                                continue;
                            }
                        }
                    }
                    // 其它命令：吞到下一个空白或 \;
                    i += 1;
                    while i < len && bytes[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    continue;
                }
                if cb == b'{' {
                    // v13.2 F17: 跳到配对 `}`，不当作行结束
                    //   （避免 `\mathrm{count}` 内的 `}` 误切本行）。
                    if let Some(end) = find_matching_brace_at(body, i) {
                        i = end + 1;
                    } else {
                        // 无配对则吞掉这一个字符
                        i += 1;
                    }
                    continue;
                }
                if cb == b'}' {
                    // 块的右括号作为当前层的结束
                    i += 1;
                    break;
                }
                if cb == b'\n' {
                    i += 1;
                    break;
                }
                // 多字节字符
                let ch = body[i..].chars().next().unwrap();
                i += ch.len_utf8();
            }
            self.pos = i;
            // v13.2 F17c: 不再 trim_end_matches('}')——F17 在 LogicLineIter 跳过 {...}
            //   后 line.text 含完整 `{...}` 内容；之前 trim 末尾的 `}` 破坏 brace 结构
            //   （嵌套 \If{...}{...} 的 cond/body 都被剥掉）。
            let raw = body[start..i].trim();
            if raw.is_empty() && comment.is_none() {
                continue;
            }
            let text = strip_tcp_marker(raw);
            return Some(LogicLine {
                text,
                comment: comment.unwrap_or_default(),
            });
        }
        None
    }
}

fn strip_tcp_marker(s: &str) -> String {
    // 去掉 \tcp*{...} 标记
    let mut out = String::with_capacity(s.len());
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut ci = 0;
    while ci < chars.len() {
        let i = chars[ci].0;
        if i + 5 <= s.len() && s.is_char_boundary(i) && s[i..].starts_with("\\tcp*") {
            // 跳到下一个 {
            let mut p = i + 5;
            while p < s.len() && (s.as_bytes()[p] == b' ' || s.as_bytes()[p] == b'\t') {
                p += 1;
            }
            if p < s.len() && s.as_bytes()[p] == b'{' {
                if let Some(end) = find_matching_brace_at(s, p) {
                    // 跳到 end 之后，下一个 char
                    ci = match chars.binary_search_by_key(&end, |&(b, _)| b) {
                        Ok(idx) => idx + 1,
                        Err(idx) => idx,
                    };
                    continue;
                }
            }
            // 找不到 `{`，跳过 "\\tcp*" 这 5 字节
            ci = match chars.binary_search_by_key(&(i + 5), |&(b, _)| b) {
                Ok(idx) => idx,
                Err(idx) => idx,
            };
            continue;
        }
        // 普通字符
        out.push(chars[ci].1);
        ci += 1;
    }
    out.trim().to_string()
}

/// 找与 `s[p] == '{'` 配对的 `}` 的字节位置（绝对）。
fn find_matching_brace_at(s: &str, p: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.get(p) != Some(&b'{') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = p;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn detect_keyword(line: &str) -> Option<String> {
    // 跳过前导空白
    let s = line.trim_start();
    for kw in ["ForEach", "If", "For", "While", "Return", "Else"] {
        let token = format!("\\{kw}");
        if let Some(rest) = s.strip_prefix(&token) {
            // 必须后接空白 / { / （关键字
            if rest.is_empty()
                || rest.starts_with('{')
                || rest.starts_with(' ')
                || rest.starts_with('\t')
            {
                return Some(kw.to_string());
            }
        }
    }
    None
}

/// 把 `\cmd{cond}{body}` 拆成 `(cond, body)`。
fn split_two_braces(s: &str) -> Option<(String, String)> {
    let p = s.find('{')?;
    let end1 = find_matching_brace_at(s, p)?;
    let cond = s[p + 1..end1].to_string();
    let mut q = end1 + 1;
    // v13.2 F17b: 跳所有 ASCII 空白（space/tab/换行）—— 之前只跳 space/tab，
    //   在 body 起始 `{` 前有 `\n` 时漏掉第二对 brace，导致 cond 正确但 body 为空。
    while q < s.len() && (s.as_bytes()[q] as char).is_ascii_whitespace() {
        q += 1;
    }
    if q >= s.len() || s.as_bytes()[q] != b'{' {
        return Some((cond, String::new()));
    }
    let end2 = find_matching_brace_at(s, q)?;
    let body = s[q + 1..end2].to_string();
    Some((cond, body))
}

fn extract_single_brace(s: &str) -> Option<String> {
    let p = s.find('{')?;
    let end = find_matching_brace_at(s, p)?;
    Some(s[p + 1..end].to_string())
}

// ─── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_algorithm_simple_return() {
        let body = "\\KwIn{x}\n\\KwOut{y}\n\\Return{x + 1}";
        let rows = parse_algorithm_rows(body);
        assert!(rows.iter().any(|r| r.keyword.as_deref() == Some("Return")));
    }

    #[test]
    fn parse_algorithm_foreach() {
        let body = "\\KwIn{L}\n\\ForEach{$l \\in L$}{$a = $l\\;\\Return{$a$}\\}";
        let rows = parse_algorithm_rows(body);
        // 至少有一个 ForEach
        assert!(rows.iter().any(|r| r.keyword.as_deref() == Some("ForEach")));
        // ForEach 行 indent=0
        let fe = rows
            .iter()
            .find(|r| r.keyword.as_deref() == Some("ForEach"))
            .unwrap();
        assert_eq!(fe.indent, 0);
    }

    #[test]
    fn parse_algorithm_tcp_comment() {
        let body = "a = b \\tcp*{test comment}";
        let rows = parse_algorithm_rows(body);
        assert!(rows[0].comment.contains("test comment"));
        assert!(!rows[0].code.contains("\\tcp*"));
    }

    #[test]
    fn parse_algorithm_comment_normalizes_math() {
        let body = "A \\leftarrow TopK(H, K) \\tcp*{按 $w_p$ 降序取前 $K$ 项}";
        let rows = parse_algorithm_rows(body);
        // v13.2.7a: subscript run **不带** `_` 字面前缀；
        // plain 拼接 = plain `w` + sub `p` = `wp`
        assert!(rows[0].comment.contains("wp"), "got: {}", rows[0].comment);
        assert!(rows[0].comment.contains("p"), "got: {}", rows[0].comment);
        assert!(!rows[0].comment.contains('$'), "got: {}", rows[0].comment);
    }
}
