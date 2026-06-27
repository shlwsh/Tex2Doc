//! Doc-engine 语义块模型（V1）
//!
//! 该模块是 Reader / Writer 完全解耦的核心长期资产。
//! 任何 LaTeX 语法特性都在此被消融为「标准强类型 Enum」。

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

pub mod docx_render;
pub mod mapping_loader;
pub mod span;
pub mod standard;
pub mod visit;

pub use docx_render::*;
pub use mapping_loader::*;
pub use span::{SourceId, Span};
pub use standard::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FigureSizing {
    pub source_options: Option<String>,
    pub width_expr: Option<String>,
    pub height_expr: Option<String>,
    pub scale_expr: Option<String>,
    pub normalized_width_ratio: Option<f32>,
    pub normalized_height_ratio: Option<f32>,
}

impl FigureSizing {
    pub fn from_options(source_options: Option<String>) -> Option<Self> {
        let source_options = source_options.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        let options = source_options.clone()?;
        let mut sizing = Self {
            source_options,
            ..Self::default()
        };

        for part in split_option_list(&options) {
            let Some((key, value)) = part.split_once('=') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim().trim_matches('{').trim_matches('}').to_string();
            if value.is_empty() {
                continue;
            }
            match key.as_str() {
                "width" => {
                    sizing.normalized_width_ratio = parse_relative_ratio(
                        &value,
                        &["\\textwidth", "\\linewidth", "\\columnwidth"],
                    );
                    sizing.width_expr = Some(value);
                }
                "height" => {
                    sizing.normalized_height_ratio = parse_relative_ratio(
                        &value,
                        &["\\textheight", "\\paperheight", "\\pageheight"],
                    );
                    sizing.height_expr = Some(value);
                }
                "scale" => {
                    sizing.scale_expr = Some(value.clone());
                    if sizing.normalized_width_ratio.is_none() {
                        sizing.normalized_width_ratio = parse_plain_number(&value);
                    }
                    if sizing.normalized_height_ratio.is_none() {
                        sizing.normalized_height_ratio = parse_plain_number(&value);
                    }
                }
                _ => {}
            }
        }

        Some(sizing)
    }
}

fn split_option_list(options: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut brace_depth = 0i32;
    let mut bracket_depth = 0i32;
    for (idx, ch) in options.char_indices() {
        match ch {
            '{' => brace_depth += 1,
            '}' => brace_depth -= 1,
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            ',' if brace_depth == 0 && bracket_depth == 0 => {
                parts.push(options[start..idx].trim());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    if start <= options.len() {
        parts.push(options[start..].trim());
    }
    parts
}

fn parse_relative_ratio(value: &str, bases: &[&str]) -> Option<f32> {
    let compact = value.split_whitespace().collect::<String>();
    for base in bases {
        if let Some(prefix) = compact.strip_suffix(base) {
            if prefix.is_empty() {
                return Some(1.0);
            }
            if let Some(number) = parse_plain_number(prefix) {
                return Some(number);
            }
        }
    }
    parse_plain_number(&compact).filter(|ratio| (0.0..=10.0).contains(ratio))
}

fn parse_plain_number(value: &str) -> Option<f32> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut end = 0usize;
    let mut seen_digit = false;
    for (idx, ch) in trimmed.char_indices() {
        if ch.is_ascii_digit() {
            seen_digit = true;
            end = idx + ch.len_utf8();
        } else if ch == '.' || ch == '+' {
            end = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    if !seen_digit {
        return None;
    }
    trimmed[..end].parse::<f32>().ok()
}

/// 文档元数据（V2：包含 JOS 期刊投稿所需的全部 front matter）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MetaData {
    // ── 中文 ──
    pub title: Option<String>,
    pub authors: Vec<String>,
    /// 单位 / 通讯地址（按 `\\` 拆分行）
    pub institute_lines: Vec<String>,
    pub abstract_text: Option<String>,
    pub keywords: Vec<String>,
    /// 中图法分类号
    pub category: Option<String>,
    // ── 英文 ──
    pub title_en: Option<String>,
    pub authors_en: Vec<String>,
    pub institute_en: Option<String>,
    pub abstract_en: Option<String>,
    pub keywords_en: Vec<String>,
    // ── 引用格式 ──
    pub citation_zh: Option<String>,
    pub citation_en: Option<String>,
    // ── 页眉页脚 ──
    pub running_header: Option<String>,
    pub first_footer_text: Option<String>,
    /// 作者简介条目（每条一段）
    pub author_bio: Vec<String>,
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
        sizing: Option<FigureSizing>,
        /// Auto-generated figure number (e.g., "图 1").
        number: Option<String>,
        /// LaTeX label for cross-referencing (e.g., "fig:main").
        label: Option<String>,
        /// Text direction: left-to-right, right-to-left, etc.
        #[serde(default)]
        text_direction: Option<TextDirection>,
        span: Span,
    },
    Equation {
        latex: String,
        is_block: bool,
        span: Span,
    },
    /// Theorem-like environments such as theorem/proof/proposition.
    TheoremLike {
        kind: TheoremLikeKind,
        title: Option<String>,
        body: String,
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
    /// 代码块：从 minted / listings / verbatim 环境提取的源代码。
    CodeBlock {
        language: Option<String>,
        code: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TheoremLikeKind {
    Theorem,
    Proof,
    Proposition,
    Lemma,
    Corollary,
    Definition,
    Remark,
    Example,
}

impl TheoremLikeKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Theorem => "定理",
            Self::Proof => "证明",
            Self::Proposition => "命题",
            Self::Lemma => "引理",
            Self::Corollary => "推论",
            Self::Definition => "定义",
            Self::Remark => "注",
            Self::Example => "例",
        }
    }
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
    /// Vertical alignment within cell: top / center / bottom / auto
    #[serde(default)]
    pub vertical_align: Option<VerticalAlign>,
    /// Text direction within cell
    #[serde(default)]
    pub text_direction: Option<TextDirection>,
}

/// 单元格垂直对齐方式。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlign {
    #[default]
    Top,
    Center,
    Bottom,
}

/// 单元格文本方向。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TextDirection {
    #[default]
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
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
    /// DOI identifier
    #[serde(default)]
    pub doi: Option<String>,
    /// URL field
    #[serde(default)]
    pub url: Option<String>,
    /// Pages range (e.g., "1-10")
    #[serde(default)]
    pub pages: Option<String>,
    /// Volume number
    #[serde(default)]
    pub volume: Option<String>,
    /// Issue number
    #[serde(default)]
    pub number: Option<String>,
    /// Publisher name
    #[serde(default)]
    pub publisher: Option<String>,
    /// Raw BibTeX entry type (article, inproceedings, book, etc.)
    #[serde(default)]
    pub entry_type: Option<String>,
    /// Raw BibTeX fields for extensibility
    #[serde(default)]
    pub raw_fields: std::collections::HashMap<String, String>,
}

/// 参考文献引用样式。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CitationStyle {
    /// 作者-年份格式: (Smith, 2020)
    #[default]
    AuthorYear,
    /// 数字编号格式: [1]
    Numeric,
    /// 上标数字格式: ^1^
    Superscript,
    /// 混合格式: Smith et al. [1]
    AuthorYearNumeric,
}

impl CitationStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AuthorYear => "author_year",
            Self::Numeric => "numeric",
            Self::Superscript => "superscript",
            Self::AuthorYearNumeric => "author_year_numeric",
        }
    }
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
