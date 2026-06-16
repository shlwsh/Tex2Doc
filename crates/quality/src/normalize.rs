//! 文本 normalize：沿用 `to-docx/08-verification.md §8.7` 的归一规则。
//!
//! 设计见 `docs/study/08-pdf-pipeline/04-quality-comparison.md` §4.6.1。

/// 把原文做归一：
/// 1. `nbsp` → 普通空格
/// 2. 全部空白 → 删除
/// 3. en/em dash → `-`
/// 4. 中文左右引号 → `"` / `'`
pub fn normalize(text: &str) -> String {
    text.replace('\u{00a0}', " ")
        .replace(['–', '—'], "-")
        .replace(['“', '”'], "\"")
        .replace(['‘', '’'], "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_removes_whitespace_and_normalizes_punct() {
        let s = "摘  要\u{00a0}网 络 — 1 引言\u{201c}Hello\u{201d}";
        let n = normalize(s);
        assert!(n.contains("摘要网络-1引言\"Hello\""));
    }
}
