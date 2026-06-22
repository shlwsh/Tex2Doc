//! Error types for XDV parsing.
//!
//! All errors carry byte offsets for precise diagnostics.

use std::fmt;

/// Errors that can occur during XDV parsing.
#[derive(Debug)]
pub enum XdvError {
    /// Unexpected end of input.
    UnexpectedEof { offset: usize, needed: usize },

    /// Unknown or invalid opcode.
    InvalidOpcode { offset: usize, opcode: u8 },

    /// Invalid UTF-8 sequence.
    InvalidUtf8 { offset: usize },

    /// Malformed data that doesn't conform to the DVI/XDV format.
    InvalidFormat { offset: usize, message: String },

    /// IO error reading the input.
    Io { offset: usize, message: String },

    /// Preamble validation failed (wrong magic, bad units, etc.).
    BadPreamble { offset: usize, message: String },
}

impl fmt::Display for XdvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof { offset, needed } => {
                write!(
                    f,
                    "unexpected EOF at byte {offset}, needed {needed} more bytes"
                )
            }
            Self::InvalidOpcode { offset, opcode } => {
                write!(f, "invalid opcode 0x{opcode:02X} at byte {offset}")
            }
            Self::InvalidUtf8 { offset } => {
                write!(f, "invalid UTF-8 sequence at byte {offset}")
            }
            Self::InvalidFormat { offset, message } => {
                write!(f, "invalid format at byte {offset}: {message}")
            }
            Self::Io { offset, message } => {
                write!(f, "I/O error at byte {offset}: {message}")
            }
            Self::BadPreamble { offset, message } => {
                write!(f, "bad preamble at byte {offset}: {message}")
            }
        }
    }
}

impl std::error::Error for XdvError {}
