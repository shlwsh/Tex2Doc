//! Rule output types: what a rule declares about how to process a macro.

use serde::{Deserialize, Serialize};

/// Output semantics of a matched rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuleOutput {
    /// The macro starts a heading at the given level.
    Heading {
        level: u8,
        /// Which argument (0-indexed) contains the heading text.
        text_arg: usize,
    },
    /// The macro's argument should be treated as a paragraph.
    Paragraph {
        /// Which argument (0-indexed) contains the paragraph body.
        body_arg: usize,
    },
    /// The macro is inline text (e.g., `\textit`, `\textbf`).
    InlineText {
        /// Which argument (0-indexed) contains the inline content.
        content_arg: usize,
    },
    /// The macro should be ignored entirely (no output).
    Ignore,
    /// The macro's argument should be treated as a table.
    Table {
        /// Which argument (0-indexed) contains table data.
        body_arg: usize,
    },
    /// The macro's argument should be treated as a figure.
    Figure {
        /// Which argument (0-indexed) contains the figure path/caption.
        body_arg: usize,
    },
    /// The macro should be kept as-is (verbatim).
    Verbatim,
    // === Journal-specific output variants ===
    /// A citation macro (e.g., `\citet`, `\citep`, `\citealp`).
    Citation {
        /// Which argument (0-indexed) contains the citation keys.
        keys_arg: usize,
        /// Citation style hint: "textual", "parenthetical", etc.
        style: String,
    },
    /// A metadata field macro (e.g., `\IEEEkeywords`, `\shorttitle`, `\confName`).
    MetadataField {
        /// The field name (e.g., "keywords", "title", "confName").
        key: String,
        /// Which argument (0-indexed) contains the value.
        content_arg: usize,
    },
    /// An author list macro (e.g., `\IEEEauthorblockN`, `\author`).
    AuthorList {
        /// Which argument (0-indexed) contains the author block.
        content_arg: usize,
    },
    /// An affiliation/address macro (e.g., `\IEEEauthorblockA`, `\affiliation`, `\institute`).
    Affiliation {
        /// Which argument (0-indexed) contains the affiliation.
        content_arg: usize,
    },
    /// A keyword list macro (e.g., `\IEEEkeywords`, `\keywords`).
    KeywordList {
        /// Which argument (0-indexed) contains the keyword text.
        content_arg: usize,
        /// Separator between keywords (default: comma).
        separator: String,
    },
}

impl RuleOutput {
    /// Human-readable description of this output type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Heading { .. } => "heading",
            Self::Paragraph { .. } => "paragraph",
            Self::InlineText { .. } => "inline_text",
            Self::Ignore => "ignore",
            Self::Table { .. } => "table",
            Self::Figure { .. } => "figure",
            Self::Verbatim => "verbatim",
            Self::Citation { .. } => "citation",
            Self::MetadataField { .. } => "metadata_field",
            Self::AuthorList { .. } => "author_list",
            Self::Affiliation { .. } => "affiliation",
            Self::KeywordList { .. } => "keyword_list",
        }
    }
}
