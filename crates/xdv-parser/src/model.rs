//! Data model for parsed XDV/DVI documents.
//!
//! This module defines the intermediate representation produced by the parser.

use serde::{Deserialize, Serialize};

/// A fully parsed XDV/DVI document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct XdvDocument {
    /// Document preamble (numerator, denominator, magnification, comment).
    pub preamble: Option<XdvPreamble>,

    /// All pages in the document, in order.
    pub pages: Vec<XdvPage>,

    /// All font definitions encountered in the document.
    pub fonts: Vec<FontDef>,

    /// Top-level commands that are not part of any page
    /// (e.g., font defs that appear between postamble sections).
    pub commands: Vec<XdvCommand>,

    /// Total number of raw opcodes processed.
    pub opcode_count: usize,
}

impl XdvDocument {
    /// Returns true if the document has at least one page.
    pub fn has_pages(&self) -> bool {
        !self.pages.is_empty()
    }

    /// Returns the total number of font definitions.
    pub fn font_count(&self) -> usize {
        self.fonts.len()
    }
}

/// XDV/DVI preamble — the fixed-point parameters at the start of a DVI file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XdvPreamble {
    /// Format version / magic byte (247 for standard DVI, 254 for XDV).
    pub id: u8,
    /// Numerator of the design size ratio (e.g., 2540000 for TeX's DVI unit).
    pub numerator: i32,
    /// Denominator (e.g., 473628672 for 72.27 pt/inch resolution).
    pub denominator: i32,
    /// Magnification factor (e.g., 1000 = no magnification).
    pub magnification: i32,
    /// Comment string from the preamble (usually empty or a filename).
    pub comment: String,
}

/// A single page within the document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct XdvPage {
    /// Zero-based page number.
    pub number: i32,
    /// All commands on this page.
    pub commands: Vec<XdvCommand>,
}

impl XdvPage {
    /// Returns the number of glyph set commands on this page.
    pub fn glyph_count(&self) -> usize {
        self.commands
            .iter()
            .filter(|c| matches!(c, XdvCommand::SetChar { .. }))
            .count()
    }

    /// Returns the number of rule commands on this page.
    pub fn rule_count(&self) -> usize {
        self.commands
            .iter()
            .filter(|c| matches!(c, XdvCommand::SetRule { .. } | XdvCommand::PutRule { .. }))
            .count()
    }
}

/// A single command in the XDV/DVI instruction stream.
///
/// These correspond 1:1 with opcodes in the bytecode, augmented
/// with decoded parameter values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum XdvCommand {
    /// A character was typeset.
    SetChar {
        /// The Unicode code point (or glyph index for native fonts).
        code: u32,
    },

    /// A rule was typeset and cursor was advanced.
    SetRule {
        /// Rule height in DVI units.
        height: i32,
        /// Rule width in DVI units.
        width: i32,
    },

    /// A rule was placed without advancing cursor.
    PutRule {
        height: i32,
        width: i32,
    },

    /// Push current position/state onto the stack.
    Push,

    /// Pop state from the stack.
    Pop,

    /// Move the cursor right by the given amount (in signed DVI units).
    MoveRight(i32),

    /// Move the cursor down by the given amount (in signed DVI units).
    MoveDown(i32),

    /// Move right and set the w register (short form).
    W(i32),

    /// Move right and set the x register (short form).
    X(i32),

    /// Move down and set the y register.
    Y(i32),

    /// Move down and set the z register.
    Z(i32),

    /// Select a font by its font number.
    SelectFont {
        /// The font number as assigned by font_def.
        font_num: u32,
    },

    /// A font was defined.
    FontDef(FontDef),

    /// An XDV/XeTeX native glyph (native font, extended encoding).
    NativeGlyph {
        /// Font number.
        font_id: u32,
        /// Glyph ID within the font.
        glyph_id: u32,
        /// Width in DVI units.
        width: i32,
        /// The raw special bytes (e.g., XDV-specific font info).
        special: Vec<u8>,
    },

    /// An XDV/XeTeX native node (e.g., whitespace, boundary, etc.).
    NativeNode {
        /// Node type discriminator from XDV stream.
        node_type: u8,
        /// Width in DVI units.
        width: i32,
        /// Special data.
        special: Vec<u8>,
    },

    /// A XeTeX extended font definition (with native font data).
    FontDefExt(FontDefExt),

    /// A special command (raw bytes, usually for PDF annotations, color, etc.).
    Special {
        /// The special payload.
        data: Vec<u8>,
    },

    /// Begin page marker.
    Bop,

    /// End page marker.
    Eop,

    /// A raw unknown opcode that was skipped.
    Unknown {
        /// The raw opcode byte.
        opcode: u8,
        /// Byte offset in the input stream.
        offset: usize,
    },
}

/// Font definition (standard DVI format).
///
/// Note: XeTeX's native fonts use `FontDefExt` instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontDef {
    /// Font number (assigned sequentially, referenced by set_char / fnt).
    pub id: u32,
    /// Checksum (from TFM file).
    pub checksum: u32,
    /// Scale factor (scaled design size).
    pub scale: i32,
    /// Design size in DVI units (e.g., 10pt = 10485760).
    pub design_size: i32,
    /// Area/directory part of the font name.
    pub area: String,
    /// Font name (the name used in the DVI file).
    pub name: String,
}

/// Extended font definition for XeTeX native fonts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontDefExt {
    /// Font number.
    pub id: u32,
    /// Checksum.
    pub checksum: u32,
    /// Scale factor.
    pub scale: i32,
    /// Design size.
    pub design_size: i32,
    /// Area name.
    pub area: String,
    /// Font name.
    pub name: String,
    /// Flags (XeTeX-specific font properties).
    pub flags: u32,
    /// The number of characters in this font.
    pub char_count: u32,
    /// Native font data (e.g., font file name or embedded font).
    pub native_data: Vec<u8>,
}
