//! Integration tests using hand-crafted DVI byte sequences.
//!
//! These fixtures are not dependent on external xelatex binaries,
//! ensuring reproducible CI results.

use doc_xdv_parser::{XdvCommand, XdvParser};

/// Builds a DVI preamble.
fn preamble_bytes(id: u8, numerator: i32, denominator: i32, magnification: i32) -> Vec<u8> {
    let mut data = Vec::new();
    data.push(239); // pre
    data.push(id);
    data.extend_from_slice(&numerator.to_be_bytes());
    data.extend_from_slice(&denominator.to_be_bytes());
    data.extend_from_slice(&magnification.to_be_bytes());
    data.extend_from_slice(&[4]); // comment len
    data.extend_from_slice(b"test");
    data
}

/// Builds a bop/eop page wrapping the given commands.
fn page_bytes(commands: &[u8]) -> Vec<u8> {
    let mut d = Vec::new();
    d.push(136); // bop
    d.extend_from_slice(&[0u8; 40]); // 10 count registers
    d.extend_from_slice(commands);
    d.push(137); // eop
    d
}

/// Combines preamble and page content into a full DVI document.
fn build_doc(preamble: &[u8], page_content: &[u8]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(preamble);
    data.extend_from_slice(page_content);
    data
}

/// Returns page commands excluding Bop and Eop markers.
fn page_content<'a>(page: &'a doc_xdv_parser::XdvPage) -> Vec<&'a XdvCommand> {
    page.commands
        .iter()
        .filter(|c| !matches!(c, XdvCommand::Bop | XdvCommand::Eop))
        .collect()
}

#[test]
fn fixture_preamble_dvi_format() {
    let doc = XdvParser::new().parse_bytes(&build_doc(
        &preamble_bytes(247, 25400000, 473628672, 1000),
        &[],
    )).unwrap();

    let pre = doc.preamble.as_ref().unwrap();
    assert_eq!(pre.id, 247);
    assert_eq!(pre.numerator, 25400000);
    assert_eq!(pre.denominator, 473628672);
    assert_eq!(pre.magnification, 1000);
    assert_eq!(pre.comment, "test");
}

#[test]
fn fixture_preamble_xdv_format() {
    let mut pre = preamble_bytes(247, 25400000, 473628672, 1000);
    pre[1] = 254; // Change to XDV format
    let doc = XdvParser::new().parse_bytes(&build_doc(&pre, &[])).unwrap();

    let pre = doc.preamble.as_ref().unwrap();
    assert_eq!(pre.id, 254);
}

#[test]
fn fixture_single_page_empty() {
    let pre = preamble_bytes(247, 25400000, 473628672, 1000);
    let data = build_doc(&pre, &page_bytes(&[]));

    let doc = XdvParser::new().parse_bytes(&data).unwrap();
    assert!(doc.has_pages());
    assert_eq!(doc.pages.len(), 1);
    assert_eq!(doc.pages[0].number, 0);
    assert!(page_content(&doc.pages[0]).is_empty());
}

#[test]
fn fixture_page_with_multiple_bops() {
    let pre = preamble_bytes(247, 25400000, 473628672, 1000);
    let mut data = build_doc(&pre, &[]);
    data.extend_from_slice(&page_bytes(&[])); // page 0
    data.extend_from_slice(&page_bytes(&[])); // page 1

    let doc = XdvParser::new().parse_bytes(&data).unwrap();
    assert_eq!(doc.pages.len(), 2);
    assert_eq!(doc.pages[0].number, 0);
    assert_eq!(doc.pages[1].number, 1);
}

#[test]
fn fixture_font_def_and_select() {
    let pre = preamble_bytes(247, 25400000, 473628672, 1000);
    let mut page = Vec::new();
    // font_def1: font 0 = CMR10
    page.push(235); // font_def1
    page.push(0);   // font number 0
    page.extend_from_slice(&0u32.to_be_bytes());       // checksum
    page.extend_from_slice(&1048576i32.to_be_bytes()); // scale
    page.extend_from_slice(&1048576i32.to_be_bytes()); // design size
    page.push(0);                                  // area len
    page.extend_from_slice(&[5u8]);                // name len
    page.extend_from_slice(b"CMR10");

    let doc = XdvParser::new().parse_bytes(&build_doc(&pre, &page)).unwrap();
    assert_eq!(doc.font_count(), 1);
    assert_eq!(doc.fonts[0].name, "CMR10");
    assert_eq!(doc.fonts[0].id, 0);
}

#[test]
fn fixture_glyph_sequence() {
    let pre = preamble_bytes(247, 25400000, 473628672, 1000);
    let page = page_bytes(&[
        172, // fnt_num_0
        65,  // set_char_65 ('A')
        66,  // set_char_66 ('B')
        67,  // set_char_67 ('C')
    ]);

    let doc = XdvParser::new().parse_bytes(&build_doc(&pre, &page)).unwrap();
    let glyphs: Vec<_> = page_content(&doc.pages[0])
        .iter()
        .filter_map(|c| match c {
            XdvCommand::SetChar { code } => Some(*code),
            _ => None,
        })
        .collect();
    assert_eq!(glyphs, &[65, 66, 67]);
}

#[test]
fn fixture_set_char_long_form() {
    let pre = preamble_bytes(247, 25400000, 473628672, 1000);
    let page = page_bytes(&[
        172,  // fnt_num_0
        128,  // set1
        200,  // char code 200
    ]);

    let doc = XdvParser::new().parse_bytes(&build_doc(&pre, &page)).unwrap();
    let glyphs: Vec<_> = page_content(&doc.pages[0])
        .iter()
        .filter_map(|c| match c {
            XdvCommand::SetChar { code } => Some(*code),
            _ => None,
        })
        .collect();
    assert_eq!(glyphs, &[200]);
}

#[test]
fn fixture_push_pop_stack() {
    let pre = preamble_bytes(247, 25400000, 473628672, 1000);
    let page = page_bytes(&[
        138, // push
        140, // right1
        100, // +100
        65,  // set_char A
        139, // pop
        66,  // set_char B
    ]);

    let doc = XdvParser::new().parse_bytes(&build_doc(&pre, &page)).unwrap();
    let kinds: Vec<_> = page_content(&doc.pages[0])
        .iter()
        .map(|c| match c {
            XdvCommand::Push => "push",
            XdvCommand::Pop => "pop",
            XdvCommand::SetChar { .. } => "glyph",
            XdvCommand::MoveRight(..) => "right",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, &["push", "right", "glyph", "pop", "glyph"]);
}

#[test]
fn fixture_movement_right_down() {
    let pre = preamble_bytes(247, 25400000, 473628672, 1000);
    let page = page_bytes(&[
        140,        // right1
        50,         // +50 DVI units
        156,        // down1
        100,        // +100 DVI units
        142,        // right3
        0, 1, 0,    // +256 DVI units (big-endian 3-byte)
    ]);

    let doc = XdvParser::new().parse_bytes(&build_doc(&pre, &page)).unwrap();
    let movements: Vec<_> = page_content(&doc.pages[0])
        .iter()
        .filter_map(|c| match c {
            XdvCommand::MoveRight(v) => Some(*v),
            XdvCommand::MoveDown(v) => Some(*v),
            _ => None,
        })
        .collect();
    assert_eq!(movements, &[50, 100, 256]);
}

#[test]
fn fixture_set_rule_is_parsed() {
    let pre = preamble_bytes(247, 25400000, 473628672, 1000);
    let mut page = Vec::new();
    page.push(133); // set_rule
    page.extend_from_slice(&1000i32.to_be_bytes());
    page.extend_from_slice(&5000i32.to_be_bytes());

    let data = build_doc(&pre, &page_bytes(&page));

    let doc = XdvParser::new().parse_bytes(&data).unwrap();
    assert!(doc.has_pages(), "should have at least one page");
    let has_set_rule = doc.pages[0]
        .commands
        .iter()
        .any(|c| matches!(c, XdvCommand::SetRule { .. }));
    assert!(has_set_rule, "page should contain a SetRule command");
}

#[test]
fn fixture_special_parsed() {
    // Test that xxx1 (opcode 244) correctly reads its length-prefixed payload.
    // Use page_bytes to properly delimit the page.
    let pre = preamble_bytes(247, 25400000, 473628672, 1000);
    let page = page_bytes(&[
        244,       // xxx1
        4,         // length (fits in 1 byte)
        b'p', b'd', b'f', b'x', // 4 bytes of payload
    ]);

    let doc = XdvParser::new().parse_bytes(&build_doc(&pre, &page)).unwrap();
    let specials: Vec<_> = page_content(&doc.pages[0])
        .iter()
        .filter_map(|c| match c {
            XdvCommand::Special { data } => Some(data.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(specials.len(), 1, "should have one special command");
    assert_eq!(&specials[0], b"pdfx", "special payload should match");
}

#[test]
fn fixture_unknown_opcode_tolerated() {
    let pre = preamble_bytes(247, 25400000, 473628672, 1000);
    // Unknown opcode at top level (between preamble and first page)
    let mut data = build_doc(&pre, &page_bytes(&[]));
    data.push(250); // undefined/reserved opcode

    let doc = XdvParser::new().parse_bytes(&data).unwrap();
    let unknowns: Vec<_> = doc.commands
        .iter()
        .filter(|c| matches!(c, XdvCommand::Unknown { .. }))
        .collect();
    assert_eq!(unknowns.len(), 1, "top-level unknown opcode should be recorded");
}

#[test]
fn fixture_opcode_count_tracked() {
    let pre = preamble_bytes(247, 25400000, 473628672, 1000);
    let page = page_bytes(&[65, 66, 67]); // 3 set_char

    let doc = XdvParser::new().parse_bytes(&build_doc(&pre, &page)).unwrap();
    // Count: preamble(1) + bop(1) + 3 chars + eop(1) = 6
    assert!(doc.opcode_count >= 5, "should track at least 5 opcodes");
}
