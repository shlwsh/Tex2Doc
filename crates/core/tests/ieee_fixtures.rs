//! IEEE 风格夹具集成测试（端到端）

use doc_core::{convert_sync, ConvertOptions};
use std::path::PathBuf;

fn fixture(name: &str) -> (String, String) {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../tests/fixtures/ieee");
    p.push(name);
    p = p.canonicalize().expect("canonicalize fixture path");
    let main_tex = p.file_name().unwrap().to_string_lossy().to_string();
    let src = std::fs::read_to_string(&p).expect("fixture not found");
    (main_tex, src)
}

#[test]
fn ieee_simple_end_to_end() {
    let (main, src) = fixture("ieee_simple.tex");
    let opts = ConvertOptions::default();
    let r = convert_sync(&main, &src, &opts).expect("convert");
    assert!(!r.docx.is_empty());
    assert_eq!(&r.docx[..4], b"PK\x03\x04");
    let needle = b"word/document.xml";
    assert!(r.docx.windows(needle.len()).any(|w| w == needle));
}

#[test]
fn ieee_nested_round_trip() {
    let (main, src) = fixture("ieee_nested.tex");
    let opts = ConvertOptions::default();
    let r = convert_sync(&main, &src, &opts).expect("convert");
    assert!(!r.docx.is_empty());
}
