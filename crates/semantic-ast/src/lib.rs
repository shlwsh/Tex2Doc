//! Doc-engine 语义块模型（V1）
//!
//! 该模块是 Reader / Writer 完全解耦的核心长期资产。
//! 任何 LaTeX 语法特性都在此被消融为「标准强类型 Enum」。

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

pub mod span;
pub mod visit;

pub use span::{SourceId, Span};

/// 文档元数据。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MetaData {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub abstract_text: Option<String>,
    pub keywords: Vec<String>,
}

/// 文档主体。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub metadata: MetaData,
    pub blocks: Vec<Block>,
}

impl Document {
    /// 创建空文档。
    pub fn new() -> Self {
        Self::default()
    }

    /// 推入一个块。
    pub fn push(&mut self, block: Block) {
        self.blocks.push(block);
    }
}

/// 顶层块枚举（V1 锁定）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        text: String,
        /// Auto-generated number prefix (e.g., "1", "1.1", "1.1.1").
        /// When None, no auto-numbering is used.
        number: Option<String>,
        span: Span,
    },
    Paragraph {
        runs: Vec<TextRun>,
        span: Span,
    },
    List {
        is_ordered: bool,
        items: Vec<Vec<Block>>,
        span: Span,
    },
    Table {
        rows: Vec<TableRow>,
        caption: Option<String>,
        /// Auto-generated table number (e.g., "表 1").
        number: Option<String>,
        span: Span,
    },
    Figure {
        path: String,
        caption: Option<String>,
        scale: f32,
        /// Auto-generated figure number (e.g., "图 1").
        number: Option<String>,
        span: Span,
    },
    Equation {
        latex: String,
        is_block: bool,
        span: Span,
    },
    Bibliography {
        entries: Vec<BibEntry>,
    },
    /// V2：算法块（algorithm2e 环境）
    /// `lines` 是按行解析后的 {indent, code, comment, keyword} 序列
    /// `io` 是 \KwIn / \KwOut 等元数据
    /// `caption` / `number` 来自 \caption{...} 和自动计数
    Algorithm {
        lines: Vec<AlgLine>,
        io: Vec<(String, String)>,
        caption: Option<String>,
        number: Option<String>,
        span: Span,
    },
    /// 错误降级：解析失败但仍保留原文
    RawFallback {
        text: String,
        span: Span,
    },
}

/// 算法块的一行。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlgLine {
    /// 缩进级数（0 = 顶层）
    pub indent: u8,
    /// 此行**上方**的悬挂缩进竖线位置（缩进级数列表）
    pub guides: Vec<u8>,
    /// 此行**下方**的竖线位置（用于 "end" 收尾）
    pub end_guides: Vec<u8>,
    /// 清洗后的代码文本
    pub code: String,
    /// 行尾注释（`\tcp*{...}`）
    pub comment: String,
    /// 关键字标记：`ForEach` / `If` / `Return` / `For` / `While`
    pub keyword: Option<String>,
}

/// 表格行。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

/// 表格单元格。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableCell {
    pub runs: Vec<TextRun>,
    pub colspan: u32,
    pub rowspan: u32,
    /// Background color as hex string (e.g., "#FF0000") or None
    pub bg_color: Option<String>,
}

/// 文本运行（带样式）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub style: TextStyle,
    pub span: Span,
}

impl TextRun {
    /// 创建纯文本运行。
    pub fn plain(text: impl Into<String>, span: Span) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::Plain,
            span,
        }
    }
}

/// 文本样式（V1 锁定）。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TextStyle {
    #[default]
    Plain,
    Bold,
    Italic,
    BoldItalic,
    Code,
    MathInline,
    /// V2：上标（来自 `[N]` / `^X` / `^{XYZ}`）
    Superscript,
    /// V2：下标（来自 `_X` / `_{XYZ}`）
    Subscript,
}

/// BibLaTeX 渲染条目（V1 最小集）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BibEntry {
    pub key: String,
    pub authors: Vec<String>,
    pub title: String,
    pub year: String,
    pub venue: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip() {
        let mut doc = Document::new();
        doc.metadata.title = Some("Hello".into());
        doc.push(Block::Heading {
            level: 1,
            text: "Intro".into(),
            number: None,
            span: Span::new(0, 6, SourceId(0)),
        });
        let json = serde_json::to_string(&doc).unwrap();
        let back: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, back);
    }

    #[test]
    fn text_run_plain_helper() {
        let r = TextRun::plain("hi", Span::new(0, 2, SourceId(0)));
        assert_eq!(r.text, "hi");
        assert_eq!(r.style, TextStyle::Plain);
    }
}
