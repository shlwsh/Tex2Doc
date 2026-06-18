//! OOXML 扁平结构体
//!
//! V2 重构：Run 现在直接对应 `TextStyle`，write_paragraph 根据 style 生成正确的 `<w:rPr>`。

use doc_semantic_ast::TextStyle;
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Paragraph {
    pub style_id: Option<String>,
    /// 段落级对齐（center/left/both/right），优先级低于 pStyle 内部 jc。
    pub jc: Option<String>,
    pub runs: Vec<Run>,
    /// `<w:keepNext/>`：段与下一段不分开（避免算法标题与首行代码分页）。
    pub keep_next: bool,
    /// `<w:keepLines/>`：段内行不分开（避免标题/算法行的分页切断）。
    pub keep_lines: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Run {
    pub text: String,
    /// 段落样式（极少数情况下 override）。
    pub style_id: Option<String>,
    pub style: TextStyle,
    /// 强制粗体（即使 TextStyle 不要求）。
    pub bold: bool,
    /// 强制斜体。
    pub italic: bool,
    /// 直接指定 rFonts ascii（覆盖样式）；None → 用样式自带的。
    pub font_ascii: Option<String>,
    /// 直接指定 rFonts eastAsia。
    pub font_east: Option<String>,
}

impl Default for Run {
    fn default() -> Self {
        Self {
            text: String::new(),
            style_id: None,
            style: TextStyle::Plain,
            bold: false,
            italic: false,
            font_ascii: None,
            font_east: None,
        }
    }
}

impl Run {
    /// 便捷构造：纯文本 + Plain
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::Plain,
            ..Default::default()
        }
    }

    /// 便捷构造：Bold
    pub fn bold(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::Bold,
            ..Default::default()
        }
    }

    /// 便捷构造：Italic
    pub fn italic(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::Italic,
            ..Default::default()
        }
    }

    /// 便捷构造：Code (Courier)
    pub fn code(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::Code,
            ..Default::default()
        }
    }

    /// 便捷构造：MathInline（用 italic 渲染，正文里附 "math" 标记 → OMML 暂不在 Run 层处理）
    pub fn math(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::MathInline,
            ..Default::default()
        }
    }

    /// 格式签名：用于合并相邻同格式 run。
    /// 两个 run 拥有相同 `signature()` 才能合并。
    /// 关键：合并发生在序列化前的 Paragraph 层。
    pub fn signature(&self) -> RunSignature {
        RunSignature {
            style: self.style,
            bold: self.bold,
            italic: self.italic,
            style_id: self.style_id.clone(),
            font_ascii: self.font_ascii.clone(),
            font_east: self.font_east.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunSignature {
    pub style: TextStyle,
    pub bold: bool,
    pub italic: bool,
    pub style_id: Option<String>,
    pub font_ascii: Option<String>,
    pub font_east: Option<String>,
}

/// 合并相邻的、格式签名完全一致的 run（v12 run 规范化）。
///
/// 仅合并相邻 run，避免跨段或跨子结构合并。
/// 文本用空格分隔（与 serializer 现有处理一致），但 footnote/superscript 类的尾标点
/// (e.g. `*`, `†`, `‡`) 不加空格，避免 `5.06e-03 *` → `5.06e-03*` 出现多余空格。
pub fn merge_adjacent_runs(runs: Vec<Run>) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::with_capacity(runs.len());
    for run in runs {
        if run.text.is_empty() {
            // 空文本 run 直接丢弃
            continue;
        }
        if let Some(last) = out.last_mut() {
            if last.signature() == run.signature() {
                if last.text.is_empty() {
                    last.text = run.text;
                } else {
                    // v13.1 P2: footnote 标点 (* † ‡ § ¶) 不加前导空格
                    let is_footnote = is_footnote_symbol(&run.text);
                    if !is_footnote {
                        last.text.push(' ');
                    }
                    last.text.push_str(&run.text);
                }
                continue;
            }
        }
        out.push(run);
    }
    out
}

fn is_footnote_symbol(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed.chars().all(|c| matches!(c, '*' | '†' | '‡' | '§' | '¶' | '#'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use doc_semantic_ast::TextStyle;

    #[test]
    fn merge_adjacent_runs_joins_same_format() {
        let runs = vec![
            Run::plain("hello"),
            Run::plain("world"),
        ];
        let out = merge_adjacent_runs(runs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "hello world");
    }

    #[test]
    fn merge_adjacent_runs_keeps_different_format() {
        // 构造一个 plain + bold(true) 的 pair,plain 用 "hello " 结尾避免被合并
        let runs = vec![
            Run {
                text: "hello ".into(),
                style_id: None,
                style: TextStyle::Plain,
                bold: false,
                italic: false,
                font_ascii: None,
                font_east: None,
            },
            Run {
                text: "world".into(),
                style_id: None,
                style: TextStyle::Plain,
                bold: true, // bold 是关键差别
                italic: false,
                font_ascii: None,
                font_east: None,
            },
        ];
        let out = merge_adjacent_runs(runs);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].text, "hello ");
        assert!(out[1].bold);
        assert_eq!(out[1].text, "world");
    }

    #[test]
    fn merge_adjacent_runs_three_same_format() {
        let runs = vec![
            Run::plain("a"),
            Run::plain("b"),
            Run::plain("c"),
        ];
        let out = merge_adjacent_runs(runs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "a b c");
    }

    #[test]
    fn merge_adjacent_runs_drops_empty() {
        let runs = vec![
            Run::plain(""),
            Run::plain("hello"),
        ];
        let out = merge_adjacent_runs(runs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "hello");
    }

    // v13.1 P2: footnote 标点 (* † ‡) 与前一 run 合并时不应插入空格
    #[test]
    fn merge_adjacent_runs_footnote_no_space() {
        let runs = vec![
            Run::plain("5.06e-03"),
            Run::plain("*"),
        ];
        let out = merge_adjacent_runs(runs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "5.06e-03*");
    }

    #[test]
    fn merge_adjacent_runs_style_differs_not_merged() {
        let runs = vec![
            Run::plain("a"),
            Run {
                text: "b".into(),
                style_id: None,
                style: TextStyle::Plain,
                bold: true, // bold 是关键差别
                italic: false,
                font_ascii: None,
                font_east: None,
            },
        ];
        let out = merge_adjacent_runs(runs);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn merge_preserves_signature_when_font_differs() {
        let runs = vec![
            Run {
                text: "a".into(),
                style_id: None,
                style: TextStyle::Plain,
                bold: false,
                italic: false,
                font_ascii: Some("Courier New".into()),
                font_east: None,
            },
            Run {
                text: "b".into(),
                style_id: None,
                style: TextStyle::Plain,
                bold: false,
                italic: false,
                font_ascii: Some("Times New Roman".into()),
                font_east: None,
            },
        ];
        let out = merge_adjacent_runs(runs);
        assert_eq!(out.len(), 2);
    }
}
