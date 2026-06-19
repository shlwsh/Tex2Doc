//! LaTeX 解析器入口

#![forbid(unsafe_code)]

pub mod algorithm;
pub mod expand;
pub mod green;
pub mod include;
pub mod latex_to_text;
pub mod lexer;
pub mod lower;
pub mod normalize;
pub mod parser;

pub use expand::{expand_macros, MacroMap};
pub use green::GreenNode;
pub use green::SyntaxKind;
pub use green::SyntaxNode;
pub use include::{IncludeGraph, JoinedStream};
pub use latex_to_text::{
    compress_numbers as latex_compress_numbers, parse_bbl, parse_bib, parse_newcommands,
};
pub use lower::{
    lower_to_document, lower_to_document_with_cite_map, lower_to_standard_document,
    lower_with_macros, lower_with_macros_to_standard_document,
};
pub use normalize::{latex_to_text, NormalizedRun, NormalizedText};
pub use parser::parse as parse_tex;
pub use parser::Parse;
