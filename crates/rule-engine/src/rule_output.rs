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
        }
    }
}
