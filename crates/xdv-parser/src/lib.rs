//! XDV/DVI bytecode parser for XeLaTeX layout extraction.
//!
//! This crate parses the XDV (Extended DVI) bytecode format produced by XeTeX/XeLaTeX,
//! extracting glyphs, font definitions, rules, specials, and positioning information
//! needed for semantic layout reconstruction.
//!
//! # Architecture
//!
//! ```text
//! ByteReader  -> OpcodeParser  -> XdvDocument
//! (byte I/O)    (state machine)  (document model)
//! ```
//!
//! # Supported opcodes (Phase 1)
//!
//! - Preamble / postamble
//! - bop / eop (begin/end of page)
//! - push / pop (stack)
//! - set_char (short + long forms)
//! - set_rule / put_rule
//! - right / down (movement)
//! - select_font / font_def
//! - special
//!
//! # Not covered in Phase 1
//!
//! - XDV-native font extensions
//! - OpenType glyph mapping
//! - Line/paragraph clustering
//! - Integration with DOCX renderer

pub mod error;
pub mod model;
pub mod opcode;
pub mod parser;
pub mod reader;

pub use error::XdvError;
pub use model::{FontDef, XdvCommand, XdvDocument, XdvPage, XdvPreamble};
pub use parser::XdvParser;
