//! LaTeX 解析器入口

#![forbid(unsafe_code)]

pub mod expand;
pub mod green;
pub mod include;
pub mod lexer;
pub mod lower;
pub mod parser;

pub use expand::{expand_macros, MacroMap};
pub use green::GreenNode;
pub use green::SyntaxKind;
pub use green::SyntaxNode;
pub use include::{IncludeGraph, JoinedStream};
pub use lower::{lower_to_document, lower_with_macros};
pub use parser::parse as parse_tex;
pub use parser::Parse;
