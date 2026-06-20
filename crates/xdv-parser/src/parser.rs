//! XDV/DVI bytecode parser.
//!
//! This module implements the state machine that consumes a byte stream
//! and produces a structured `XdvDocument`.

use std::io::Cursor;

use crate::error::XdvError;
use crate::model::{
    FontDef, FontDefExt, XdvCommand, XdvDocument, XdvPage, XdvPreamble,
};
use crate::reader::ByteReader;

/// XDV/DVI parser.
#[derive(Debug, Default)]
pub struct XdvParser {
    /// Assembled document (built as we parse).
    doc: XdvDocument,

    /// Flag: are we inside a page?
    in_page: bool,

    /// Current page being built.
    current_page: Option<XdvPage>,

    /// Current page number.
    page_count: i32,

    /// Total opcodes processed.
    opcode_count: usize,
}

impl XdvParser {
    /// Create a new parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse XDV/DVI data from a byte slice.
    pub fn parse_bytes(&mut self, data: &[u8]) -> Result<XdvDocument, XdvError> {
        self.reset();
        let mut r = ByteReader::new(Cursor::new(data));
        self.parse_stream(&mut r)
    }

    /// Parse XDV/DVI from a reader.
    pub fn parse<R: std::io::Read>(&mut self, reader: R) -> Result<XdvDocument, XdvError> {
        self.reset();
        let mut r = ByteReader::new(reader);
        self.parse_stream(&mut r)
    }

    fn reset(&mut self) {
        self.doc = XdvDocument::default();
        self.in_page = false;
        self.current_page = None;
        self.page_count = 0;
        self.opcode_count = 0;
    }

    fn parse_stream<R: std::io::Read>(&mut self, r: &mut ByteReader<R>) -> Result<XdvDocument, XdvError> {
        loop {
            match r.read_u8() {
                Ok(opcode) => {
                    self.opcode_count += 1;
                    self.dispatch_opcode(opcode, r)?;
                }
                Err(XdvError::UnexpectedEof { .. }) => {
                    // Flush any pending page at EOF
                    if let Some(page) = self.current_page.take() {
                        self.doc.pages.push(page);
                    }
                    break;
                }
                Err(e) => return Err(e),
            }
        }
        self.doc.opcode_count = self.opcode_count;
        Ok(std::mem::take(&mut self.doc))
    }

    fn dispatch_opcode<R: std::io::Read>(
        &mut self,
        opcode: u8,
        r: &mut ByteReader<R>,
    ) -> Result<(), XdvError> {
        const DVI_MAGIC: u8 = 247;
        const XDV_MAGIC: u8 = 254;

        match opcode {
            // ─── Preamble ───────────────────────────────────────────────────────
            239 => {
                let id = r.read_u1()?;
                if self.doc.preamble.is_some() {
                    return Err(XdvError::InvalidFormat {
                        offset: r.offset() - 1,
                        message: "duplicate preamble".to_string(),
                    });
                }
                if id != DVI_MAGIC && id != XDV_MAGIC {
                    return Err(XdvError::BadPreamble {
                        offset: r.offset() - 1,
                        message: format!(
                            "unknown DVI format ID {id} (expected {DVI_MAGIC} for DVI, {XDV_MAGIC} for XDV)"
                        ),
                    });
                }
                let numerator = r.read_i4()?;
                let denominator = r.read_i4()?;
                let magnification = r.read_i4()?;
                let comment = r.read_pascal_string()?;

                self.doc.preamble = Some(XdvPreamble {
                    id,
                    numerator,
                    denominator,
                    magnification,
                    comment,
                });
            }

            // ─── Page boundaries ───────────────────────────────────────────────
            136 => {
                // bop — flush any pending page
                if let Some(page) = self.current_page.take() {
                    self.doc.pages.push(page);
                }
                let mut buf = [0u8; 40];
                r.read_exact(&mut buf)?;
                self.page_count += 1;
                self.in_page = true;
                self.current_page = Some(XdvPage {
                    number: self.page_count - 1,
                    commands: Vec::new(),
                });
                self.push_cmd(XdvCommand::Bop);
            }

            137 => {
                // eop
                self.push_cmd(XdvCommand::Eop);
                self.in_page = false;
            }

            // ─── Stack ─────────────────────────────────────────────────────────
            138 => self.push_cmd(XdvCommand::Push),   // push
            139 => self.push_cmd(XdvCommand::Pop),    // pop

            // ─── Horizontal movement ───────────────────────────────────────────
            140 => {
                let v = r.read_i1()? as i32;
                self.push_cmd(XdvCommand::MoveRight(v));
            }
            141 => {
                let v = r.read_i2()?;
                self.push_cmd(XdvCommand::MoveRight(v));
            }
            142 => {
                let v = r.read_i3()?;
                self.push_cmd(XdvCommand::MoveRight(v));
            }
            143 => {
                let v = r.read_i4()?;
                self.push_cmd(XdvCommand::MoveRight(v));
            }

            // ─── Vertical movement ─────────────────────────────────────────────
            156 => {
                let v = r.read_i1()? as i32;
                self.push_cmd(XdvCommand::MoveDown(v));
            }
            157 => {
                let v = r.read_i2()?;
                self.push_cmd(XdvCommand::MoveDown(v));
            }
            158 => {
                let v = r.read_i3()?;
                self.push_cmd(XdvCommand::MoveDown(v));
            }
            159 => {
                let v = r.read_i4()?;
                self.push_cmd(XdvCommand::MoveDown(v));
            }

            // ─── w register ────────────────────────────────────────────────────
            144 => self.push_cmd(XdvCommand::W(0)),
            145 => {
                let v = r.read_i1()? as i32;
                self.push_cmd(XdvCommand::W(v));
            }
            146 => {
                let v = r.read_i2()?;
                self.push_cmd(XdvCommand::W(v));
            }
            147 => {
                let v = r.read_i3()?;
                self.push_cmd(XdvCommand::W(v));
            }
            148 => {
                let v = r.read_i4()?;
                self.push_cmd(XdvCommand::W(v));
            }

            // ─── x register ────────────────────────────────────────────────────
            150 => self.push_cmd(XdvCommand::X(0)),
            151 => {
                let v = r.read_i1()? as i32;
                self.push_cmd(XdvCommand::X(v));
            }
            152 => {
                let v = r.read_i2()?;
                self.push_cmd(XdvCommand::X(v));
            }
            153 => {
                let v = r.read_i3()?;
                self.push_cmd(XdvCommand::X(v));
            }
            154 => {
                let v = r.read_i4()?;
                self.push_cmd(XdvCommand::X(v));
            }

            // ─── y register ────────────────────────────────────────────────────
            160 => self.push_cmd(XdvCommand::Y(0)),
            161 => {
                let v = r.read_i1()? as i32;
                self.push_cmd(XdvCommand::Y(v));
            }
            162 => {
                let v = r.read_i2()?;
                self.push_cmd(XdvCommand::Y(v));
            }
            163 => {
                let v = r.read_i3()?;
                self.push_cmd(XdvCommand::Y(v));
            }
            164 => {
                let v = r.read_i4()?;
                self.push_cmd(XdvCommand::Y(v));
            }

            // ─── z register ────────────────────────────────────────────────────
            166 => self.push_cmd(XdvCommand::Z(0)),
            167 => {
                let v = r.read_i1()? as i32;
                self.push_cmd(XdvCommand::Z(v));
            }
            168 => {
                let v = r.read_i2()?;
                self.push_cmd(XdvCommand::Z(v));
            }
            169 => {
                let v = r.read_i3()?;
                self.push_cmd(XdvCommand::Z(v));
            }
            170 => {
                let v = r.read_i4()?;
                self.push_cmd(XdvCommand::Z(v));
            }

            // ─── Set character (short form: 0–127) ───────────────────────────
            0..=127 => {
                self.push_cmd(XdvCommand::SetChar { code: opcode as u32 });
            }

            // ─── Variable-width set_char ─────────────────────────────────────
            128 => {
                // set1
                let code = r.read_u1()? as u32;
                self.push_cmd(XdvCommand::SetChar { code });
            }
            129 => {
                // set2
                let code = r.read_u2()? as u32;
                self.push_cmd(XdvCommand::SetChar { code });
            }
            130 => {
                // set3
                let code = r.read_u3()?;
                self.push_cmd(XdvCommand::SetChar { code });
            }
            131 => {
                // set4
                let code = r.read_u4()?;
                self.push_cmd(XdvCommand::SetChar { code });
            }
            132 => {
                // set5
                let code = r.read_u4()?;
                self.push_cmd(XdvCommand::SetChar { code });
            }

            // ─── Rules ────────────────────────────────────────────────────────
            133 => {
                // set_rule
                let height = r.read_i4()?;
                let width = r.read_i4()?;
                self.push_cmd(XdvCommand::SetRule { height, width });
            }
            134 => {
                // put_rule
                let height = r.read_i4()?;
                let width = r.read_i4()?;
                self.push_cmd(XdvCommand::PutRule { height, width });
            }

            // ─── Font selection (fnt_num_0 … fnt_num_62 = 172–234) ───────────
            172..=234 => {
                let font_num = (opcode as u32) - 172;
                self.push_cmd(XdvCommand::SelectFont { font_num });
            }

            // ─── Font definition ──────────────────────────────────────────────
            235 => {
                let font = self.read_font_def_impl(r, 1)?;
                self.doc.fonts.push(font.clone());
                self.push_cmd(XdvCommand::FontDef(font));
            }
            236 => {
                let font = self.read_font_def_impl(r, 2)?;
                self.doc.fonts.push(font.clone());
                self.push_cmd(XdvCommand::FontDef(font));
            }
            237 => {
                let font = self.read_font_def_impl(r, 3)?;
                self.doc.fonts.push(font.clone());
                self.push_cmd(XdvCommand::FontDef(font));
            }
            238 => {
                let font = self.read_font_def_impl(r, 4)?;
                self.doc.fonts.push(font.clone());
                self.push_cmd(XdvCommand::FontDef(font));
            }

            // ─── Specials (xxx1–xxx4) ────────────────────────────────────────
            244 => {
                let len = r.read_u1()? as usize;
                let data = r.read_bytes(len)?;
                self.push_cmd(XdvCommand::Special { data });
            }
            245 => {
                let len = r.read_u2()? as usize;
                let data = r.read_bytes(len)?;
                self.push_cmd(XdvCommand::Special { data });
            }
            246 => {
                let len = r.read_u3()? as usize;
                let data = r.read_bytes(len)?;
                self.push_cmd(XdvCommand::Special { data });
            }
            247 => {
                let len = r.read_u4()? as usize;
                let data = r.read_bytes(len)?;
                self.push_cmd(XdvCommand::Special { data });
            }

            // ─── Postamble ───────────────────────────────────────────────────
            240 => {
                // post — flush pending page
                if let Some(page) = self.current_page.take() {
                    self.doc.pages.push(page);
                }
                // Skip postamble body + backpointer
                let mut skip_buf = [0u8; 44];
                let _ = r.read_exact(&mut skip_buf);
                let _ = r.read_u4();
            }

            241 => {
                // post_post — consumed by EOF
            }

            // ─── XeTeX extended opcodes ──────────────────────────────────────
            242 => {
                // XeTeXChar
                let len = r.read_u1()? as usize;
                let data = r.read_bytes(len)?;
                if data.len() >= 8 {
                    let font_id = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                    let glyph_id = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
                    self.push_cmd(XdvCommand::NativeGlyph {
                        font_id,
                        glyph_id,
                        width: 0,
                        special: data[8..].to_vec(),
                    });
                } else {
                    self.push_cmd(XdvCommand::Special { data });
                }
            }

            243 => {
                // XeTeXExt1
                let len = r.read_u2()? as usize;
                let data = r.read_bytes(len)?;
                self.push_cmd(XdvCommand::Special { data });
            }

            251 => {
                // XeTeX native glyph
                let font_id = r.read_u4()?;
                let glyph_id = r.read_u4()?;
                let width = r.read_i4()?;
                let len = r.read_u4()? as usize;
                let special = r.read_bytes(len)?;
                self.push_cmd(XdvCommand::NativeGlyph {
                    font_id,
                    glyph_id,
                    width,
                    special,
                });
            }

            252 => {
                // XeTeXNativeNode
                let node_type = r.read_u1()?;
                let width = r.read_i4()?;
                let len = r.read_u4()? as usize;
                let special = r.read_bytes(len)?;
                self.push_cmd(XdvCommand::NativeNode {
                    node_type,
                    width,
                    special,
                });
            }

            253 => {
                // XeTeXFntDefExt
                let ext = self.read_font_def_ext(r)?;
                self.push_cmd(XdvCommand::FontDefExt(ext));
            }

            // ─── No-op ────────────────────────────────────────────────────────
            135 => {}

            // ─── Unknown opcode — record and continue ──────────────────────────
            _ => {
                self.push_cmd(XdvCommand::Unknown {
                    opcode,
                    offset: r.offset() - 1,
                });
            }
        }

        Ok(())
    }

    fn read_font_def_impl<R: std::io::Read>(
        &self,
        r: &mut ByteReader<R>,
        n_bytes: usize,
    ) -> Result<FontDef, XdvError> {
        let font_num = match n_bytes {
            1 => r.read_u1()? as u32,
            2 => r.read_u2()? as u32,
            3 => r.read_u3()? as u32,
            4 => r.read_u4()?,
            _ => unreachable!(),
        };
        let checksum = r.read_u4()?;
        let scale = r.read_i4()?;
        let design_size = r.read_i4()?;
        let area_len = r.read_u1()? as usize;
        let name_len = r.read_u1()? as usize;

        let area = {
            let mut buf = vec![0u8; area_len];
            r.read_exact(&mut buf)?;
            String::from_utf8(buf).map_err(|_| XdvError::InvalidUtf8 {
                offset: r.offset() - area_len,
            })?
        };

        let name = {
            let mut buf = vec![0u8; name_len];
            r.read_exact(&mut buf)?;
            String::from_utf8(buf).map_err(|_| XdvError::InvalidUtf8 {
                offset: r.offset() - name_len,
            })?
        };

        Ok(FontDef {
            id: font_num,
            checksum,
            scale,
            design_size,
            area,
            name,
        })
    }

    fn read_font_def_ext<R: std::io::Read>(&self, r: &mut ByteReader<R>) -> Result<FontDefExt, XdvError> {
        let id = r.read_u4()?;
        let checksum = r.read_u4()?;
        let scale = r.read_i4()?;
        let design_size = r.read_i4()?;

        let area_len = r.read_u1()? as usize;
        let name_len = r.read_u1()? as usize;

        let area = {
            let mut buf = vec![0u8; area_len];
            r.read_exact(&mut buf)?;
            String::from_utf8(buf).map_err(|_| XdvError::InvalidUtf8 {
                offset: r.offset() - area_len,
            })?
        };
        let name = {
            let mut buf = vec![0u8; name_len];
            r.read_exact(&mut buf)?;
            String::from_utf8(buf).map_err(|_| XdvError::InvalidUtf8 {
                offset: r.offset() - name_len,
            })?
        };

        let flags = r.read_u4()?;
        let char_count = r.read_u4()?;
        let native_len = r.read_u4()? as usize;
        let native_data = r.read_bytes(native_len)?;

        Ok(FontDefExt {
            id,
            checksum,
            scale,
            design_size,
            area,
            name,
            flags,
            char_count,
            native_data,
        })
    }

    /// Push a command into the current page or document top-level.
    fn push_cmd(&mut self, cmd: XdvCommand) {
        if self.in_page {
            if let Some(ref mut page) = self.current_page {
                page.commands.push(cmd);
            }
        } else {
            self.doc.commands.push(cmd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preamble_only(id: u8, numerator: i32, denominator: i32, magnification: i32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(239);
        data.push(id);
        data.extend_from_slice(&numerator.to_be_bytes());
        data.extend_from_slice(&denominator.to_be_bytes());
        data.extend_from_slice(&magnification.to_be_bytes());
        data.push(4);
        data.extend_from_slice(b"test");
        data
    }

    #[test]
    fn parse_minimal_dvi_preamble() {
        let data = preamble_only(247, 25400000, 473628672, 1000);
        let doc = XdvParser::new().parse_bytes(&data).unwrap();

        let pre = doc.preamble.as_ref().unwrap();
        assert_eq!(pre.id, 247);
        assert_eq!(pre.numerator, 25400000);
        assert_eq!(pre.denominator, 473628672);
        assert_eq!(pre.magnification, 1000);
        assert_eq!(pre.comment, "test");
    }

    #[test]
    fn parse_minimal_xdv_preamble() {
        let data = preamble_only(254, 25400000, 473628672, 1000);
        let doc = XdvParser::new().parse_bytes(&data).unwrap();

        let pre = doc.preamble.as_ref().unwrap();
        assert_eq!(pre.id, 254);
    }

    #[test]
    fn parse_set_char_short() {
        let mut data = preamble_only(247, 25400000, 473628672, 1000);
        // bop to create a page
        data.push(136); // bop
        data.extend_from_slice(&[0u8; 40]);
        data.push(65);  // set_char_65 ('A')
        data.push(137); // eop

        let doc = XdvParser::new().parse_bytes(&data).unwrap();
        assert_eq!(doc.pages.len(), 1);
        let glyphs: Vec<_> = doc.pages[0]
            .commands
            .iter()
            .filter_map(|c| match c {
                XdvCommand::SetChar { code } => Some(*code),
                _ => None,
            })
            .collect();
        assert_eq!(glyphs, &[65]);
    }

    #[test]
    fn parse_page_with_push_pop_and_font() {
        let mut data = preamble_only(247, 25400000, 473628672, 1000);
        data.push(136); // bop
        data.extend_from_slice(&[0u8; 40]);

        // font_def1: font 0 = CMR10
        data.push(235);
        data.push(0);
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&1048576i32.to_be_bytes());
        data.extend_from_slice(&1048576i32.to_be_bytes());
        data.push(0);   // area len
        data.push(5);  // name len
        data.extend_from_slice(b"CMR10");

        // select font 0
        data.push(172); // fnt_num_0
        // push
        data.push(138);
        // right1
        data.push(140);
        data.push(10u8);
        // set_char 'X'
        data.push(88);
        // pop
        data.push(139);
        // eop
        data.push(137);

        let doc = XdvParser::new().parse_bytes(&data).unwrap();
        assert_eq!(doc.pages.len(), 1);
        assert_eq!(doc.font_count(), 1);
        assert_eq!(doc.pages[0].glyph_count(), 1);
    }

    #[test]
    fn parse_set_rule() {
        let mut data = preamble_only(247, 25400000, 473628672, 1000);
        data.push(136); // bop
        data.extend_from_slice(&[0u8; 40]);
        data.push(133); // set_rule
        data.extend_from_slice(&100i32.to_be_bytes());
        data.extend_from_slice(&200i32.to_be_bytes());
        data.push(137); // eop

        let doc = XdvParser::new().parse_bytes(&data).unwrap();
        assert_eq!(doc.pages[0].rule_count(), 1);
    }

    #[test]
    fn parse_special() {
        let mut data = preamble_only(247, 25400000, 473628672, 1000);
        data.push(136);
        data.extend_from_slice(&[0u8; 40]);
        data.push(244); // xxx1
        data.push(5);   // length
        data.extend_from_slice(b"hello");
        data.push(137); // eop

        let doc = XdvParser::new().parse_bytes(&data).unwrap();
        let specials: Vec<_> = doc.pages[0]
            .commands
            .iter()
            .filter_map(|c| match c {
                XdvCommand::Special { data: d } => Some(d.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(specials.len(), 1);
        assert_eq!(specials[0], b"hello");
    }

    #[test]
    fn bad_preamble_id() {
        let data = preamble_only(99, 25400000, 473628672, 1000);
        let err = XdvParser::new().parse_bytes(&data).unwrap_err();
        assert!(matches!(err, XdvError::BadPreamble { .. }));
    }
}
