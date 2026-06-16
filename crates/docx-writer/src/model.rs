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
}
