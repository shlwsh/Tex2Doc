//! LaTeX 简易宏展开（V1 简化版）
//!
//! 支持：
//! - `\newcommand{\X}{body}` / `\providecommand{\X}{body}` / `\renewcommand{\X}{body}`
//! - 可选参数：`\newcommand{\X}[n]{body}`（V1 忽略参数个数，只记 body）
//! - 调用站点替换：`\X` 出现处替换为 `body` 文本。
//!
//! 不支持（V1 限制）：
//! - 带可选参数 `[default]{...}` 的命令体（`\newcommand{\X}[1][def]{body}`）；
//!   真实 LaTeX 语义下，`\X{arg}` 替换为 `body` 中 `#1` 替换为 `arg` 的结果。
//!   V1 把 `body` 原样替换，调用站点不传参。
//! - `\def`、`\let`、条件宏、嵌套宏定义（宏体里再 `\newcommand`）。
//! - 局部作用域 / group。
//!
//! 典型工作流（在 [`crate::lower`] 之前调用）：
//! ```text
//! let expanded = expand_macros(&joined.text);
//! let parse = parse_tex(&expanded);
//! ```

use std::collections::HashMap;

/// 简易宏表：宏名（含反斜杠）→ 宏体。
#[derive(Debug, Default, Clone)]
pub struct MacroMap {
    defs: HashMap<String, String>,
}

impl MacroMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn define(&mut self, name: &str, body: String) {
        self.defs.insert(format!("\\{name}"), body);
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.defs.get(name).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.defs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }
}

/// 在 `text` 上做一次扫描：
/// 1. 识别 `\newcommand{\X}[n?]{body}` 等宏定义，加入宏表。
/// 2. 把所有「已知名」的宏调用 `\X` 替换为 `body` 文本。
/// 3. 宏定义本身（行级）从输出中**完全删除**，不留下任何痕迹。
///
/// 单 pass 实现：从前向后逐字符扫描，遇到 `\` 启动「命令名识别」：
/// - 若命令名是 `newcommand` / `providecommand` / `renewcommand`，按定义语法整段跳过；
/// - 否则若命令名已在宏表中，按单词边界判定是否替换为宏体。
///
/// **重要**：`macros` 由调用方持有，跨多次 `expand_macros_in` 调用累加。
/// 这样 outer 文档收集的宏表可以在 inner 段（rjabstract / rjtitle 等环境）复用。
pub fn expand_macros_in(text: &str, macros: &mut MacroMap) -> String {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\\' {
            // 命令名
            let cmd_start = i + 1;
            let mut j = cmd_start;
            while j < len && (bytes[j].is_ascii_alphabetic() || bytes[j] == b'@') {
                j += 1;
            }
            let cmd = &text[cmd_start..j];
            // 1) 定义类命令：整段跳过
            if matches!(cmd, "newcommand" | "providecommand" | "renewcommand") {
                let macros_ref = &mut *macros;
                if let Some(end) = parse_definition_end(text, j, macros_ref) {
                    // 启发式：本行剩余是否仅为注释 / 行尾空白？
                    // 若 end 之后到本行末 (`\n` / EOF) 之间全是 ASCII 空白 / `%` 注释，
                    // 则可安全整行跳过；否则就地停留，避免吞掉同一行内跟在定义
                    // 后面的宏调用（典型场景：单行测试用例）。
                    let mut p = end;
                    let mut only_ws_or_comment = true;
                    while p < len && bytes[p] != b'\n' {
                        let b = bytes[p];
                        if !(b == b' ' || b == b'\t' || b == b'%' || b == b'\r') {
                            only_ws_or_comment = false;
                            break;
                        }
                        if b == b'%' {
                            while p < len && bytes[p] != b'\n' {
                                p += 1;
                            }
                            break;
                        }
                        p += 1;
                    }
                    if only_ws_or_comment {
                        // 整段吞到行末（p 此时指向 \n 或 == len）
                        while p < len && bytes[p] != b'\n' {
                            p += 1;
                        }
                        i = p;
                    } else {
                        // 同行内还有「真」内容：就地从 end 开始，
                        // 让主循环 fallthrough 自然写出 end 处的字符。
                        i = end;
                    }
                    continue;
                }
                // 解析失败：把当前 `\` + 命令名原样写入
                out.push('\\');
                out.push_str(cmd);
                i = j;
                continue;
            }
            // 2) 宏调用替换：仅在单词边界成立时替换
            let boundary_ok = j >= len || !(bytes[j].is_ascii_alphabetic() || bytes[j] == b'@');
            let key = format!("\\{cmd}");
            if boundary_ok {
                if let Some(body) = macros.get(&key) {
                    out.push_str(body);
                    i = j;
                    continue;
                }
            }
            // 3) 未知名：原样写入
            out.push('\\');
            out.push_str(cmd);
            i = j;
            continue;
        }
        // fallthrough：避免 mojibake 二次编码（见 latex-reader/lower.rs 同源注释）
        if let Some(ch) = text[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }
    out
}

/// 尝试在 `pos`（命令名结束处）解析宏定义 `{name}` → 可选 `[n]` → `{body}`。
///
/// 解析成功：
/// 1. 把 `name → body` 加入 `macros`（name 不带 `\` 前缀，由 MacroMap 内部统一加）；
/// 2. 返回「`body` 末尾 `}` 之后的位置」。
///
/// 失败返回 None（调用方按普通文本处理）。
fn parse_definition_end(text: &str, pos: usize, macros: &mut MacroMap) -> Option<usize> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut p = pos;
    while p < len && (bytes[p] == b' ' || bytes[p] == b'\t') {
        p += 1;
    }
    if p >= len || bytes[p] != b'{' {
        return None;
    }
    // name 所在的外层 `{...}`，内容是 `\X` 形式——去掉前导 `\` 再做宏表 key
    let name_off = find_matching_brace(text, p)?;
    let raw_name = &text[p + 1..p + 1 + name_off];
    let name = raw_name.trim_start_matches('\\').to_string();
    p = p + 1 + name_off + 1;
    while p < len && (bytes[p] == b' ' || bytes[p] == b'\t') {
        p += 1;
    }
    // 可选 [n]
    if p < len && bytes[p] == b'[' {
        if let Some(close) = text[p..].find(']') {
            p += close + 1;
        } else {
            return None;
        }
        while p < len && (bytes[p] == b' ' || bytes[p] == b'\t') {
            p += 1;
        }
    }
    if p >= len || bytes[p] != b'{' {
        return None;
    }
    if let Some(off) = find_matching_brace(text, p) {
        let body = text[p + 1..p + 1 + off].to_string();
        macros.define(&name, body);
        return Some(p + 1 + off + 1);
    }
    None
}

/// 一次性「自包含」展开：内部新建宏表，适合单元测试。
pub fn expand_macros(text: &str) -> String {
    let mut macros = MacroMap::new();
    expand_macros_in(text, &mut macros)
}

/// 在 `text[pos..]` 上找与 `text[pos] == b'{'` 配对的 `}` 偏移（相对于 pos+1）。
/// 即返回值为 `body.len()`，调用方用 `&text[pos+1..pos+1+返回值]` 取出 body。
/// 失败（未闭合）返回 None。
fn find_matching_brace(text: &str, pos: usize) -> Option<usize> {
    let bytes = text.as_bytes();
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

/// 把已知名宏调用 `\X` 替换为宏体 `body`。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn define_and_expand_basic() {
        let text = "\\newcommand{\\X}{hello} \\X world";
        let out = expand_macros(text);
        assert_eq!(out, " hello world");
    }

    #[test]
    fn word_boundary_protects_partial_match() {
        let text = "\\newcommand{\\X}{hi} \\Xtra there";
        let out = expand_macros(text);
        // \Xtra 不会被展开（因为 \X 后是字母 't'）
        assert_eq!(out, " \\Xtra there");
    }

    #[test]
    fn providecommand_and_renewcommand() {
        // 两个 \Y 之间的「\renewcommand{...}」整段被剥除（不留任何痕迹），
        // 但其前后空格原样保留；展开后是 "y-body" + " " + " " + "y-v2" = "y-body  y-v2"。
        let text = "\\providecommand{\\Y}{y-body} \\Y \\renewcommand{\\Y}{y-v2} \\Y";
        let out = expand_macros(text);
        assert_eq!(out, " y-body  y-v2");
    }

    #[test]
    fn body_with_chinese() {
        let text = "\\newcommand{\\Greeting}{你好，世界} \\Greeting!";
        let out = expand_macros(text);
        assert_eq!(out, " 你好，世界!");
    }
}
