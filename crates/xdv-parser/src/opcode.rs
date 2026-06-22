//! DVI/XDV opcodes.
//!
//! The DVI format uses a stack-based virtual machine. XeTeX's XDV format
//! extends the standard DVI opcodes with native font support.
//!
//! ## Opcode ranges
//!
//! | Range     | Meaning                        |
//! |-----------|--------------------------------|
//! | 0–127     | set_char_0 … set_char_127      |
//! | 128       | set1                           |
//! | 129       | set2                           |
//! | 130       | set3                           |
//! | 131       | set4                           |
//! | 132       | set5                           |
//! | 133       | set_rule                       |
//! | 134       | put_rule                       |
//! | 135       | nop                            |
//! | 136       | bop                            |
//! | 137       | eop                            |
//! | 138       | push                           |
//! | 139       | pop                            |
//! | 140–143   | right1–right4                  |
//! | 144–148   | w0–w4                         |
//! | 150–154   | x0–x4                         |
//! | 156–159   | down1–down4                   |
//! | 160–164   | y0–y4                         |
//! | 166–170   | z0–z4                         |
//! | 172–234   | fnt_num_0 … fnt_num_62        |
//! | 235–238   | font_def1–font_def4           |
//! | 239       | pre                            |
//! | 240       | post                           |
//! | 241       | post_post                      |
//! | 242       | XeTeX char (extended)         |
//! | 243       | XeTeX ext1                     |
//! | 244–247   | xxx1–xxx4 (special)           |
//! | 251       | XeTeX native glyph             |
//! | 252       | XeTeX native node              |
//! | 253       | XeTeX font_def_ext             |

/// DVI opcodes (standard DVI format).
///
/// Note: variants 0–127 are set_char_0 through set_char_127.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DviOpcode {
    // Variable-width set_char forms
    Set1 = 128,
    Set2 = 129,
    Set3 = 130,
    Set4 = 131,
    Set5 = 132,

    /// Set a rule (height, width) and advance cursor.
    SetRule = 133,
    /// Put a rule at current position without advancing.
    PutRule = 134,

    /// No-op (alignment spacing, etc.)
    Nop = 135,

    /// Begin page.
    Bop = 136,
    /// End page.
    Eop = 137,

    /// Push state onto stack.
    Push = 138,
    /// Pop state from stack.
    Pop = 139,

    /// Move right by 1-byte signed integer.
    Right1 = 140,
    /// Move right by 2-byte signed integer.
    Right2 = 141,
    /// Move right by 3-byte signed integer.
    Right3 = 142,
    /// Move right by 4-byte signed integer.
    Right4 = 143,

    /// Set w register to 0 (no movement)
    W0 = 144,
    /// Set w and move right by 1-byte.
    W1 = 145,
    W2 = 146,
    W3 = 147,
    W4 = 148,

    /// Set x register to 0
    X0 = 150,
    /// Set x and move right by 1-byte.
    X1 = 151,
    X2 = 152,
    X3 = 153,
    X4 = 154,

    /// Move down by 1-byte signed integer.
    Down1 = 156,
    Down2 = 157,
    Down3 = 158,
    Down4 = 159,

    /// Set y register to 0
    Y0 = 160,
    Y1 = 161,
    Y2 = 162,
    Y3 = 163,
    Y4 = 164,

    /// Set z register to 0
    Z0 = 166,
    Z1 = 167,
    Z2 = 168,
    Z3 = 169,
    Z4 = 170,

    // font_num_0 is 172

    // font_def opcodes
    FontDef1 = 235,
    FontDef2 = 236,
    FontDef3 = 237,
    FontDef4 = 238,

    // DVI header
    Pre = 239,
    Post = 240,
    PostPost = 241,

    // XeTeX extended opcodes
    XeTeXChar = 242,
    XeTeXExt1 = 243,

    // special opcodes
    Xxx1 = 244,
    Xxx2 = 245,
    Xxx3 = 246,
    Xxx4 = 247,

    // XeTeX native font / node opcodes
    XeTeXNative = 251,
    XeTeXNativeNode = 252,
    XeTeXFntDefExt = 253,
}

impl DviOpcode {
    /// Number of parameter bytes for this opcode (0 if not applicable).
    pub fn param_bytes(&self) -> usize {
        match self {
            Self::Set1 => 1,
            Self::Set2 => 2,
            Self::Set3 => 3,
            Self::Set4 => 4,
            Self::Set5 => 5,
            Self::SetRule | Self::PutRule => 8,
            Self::Right1
            | Self::W1
            | Self::X1
            | Self::Down1
            | Self::Y1
            | Self::Z1 => 1,
            Self::Right2
            | Self::W2
            | Self::X2
            | Self::Down2
            | Self::Y2
            | Self::Z2 => 2,
            Self::Right3
            | Self::W3
            | Self::X3
            | Self::Down3
            | Self::Y3
            | Self::Z3 => 3,
            Self::Right4
            | Self::W4
            | Self::X4
            | Self::Down4
            | Self::Y4
            | Self::Z4 => 4,
            Self::Xxx1 | Self::XeTeXChar => 1,
            Self::Xxx2 | Self::XeTeXExt1 => 2,
            Self::Xxx3 => 3,
            Self::Xxx4 => 4,
            _ => 0,
        }
    }
}
