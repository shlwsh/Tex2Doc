//! styles.xml 序列化（V1 默认样式表）
//!
//! 详见方案 §4.3.2 样式 ID 命名规范。

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

pub const STYLE_TITLE: &str = "Title";
pub const STYLE_HEADING1: &str = "Heading1";
pub const STYLE_HEADING2: &str = "Heading2";
pub const STYLE_HEADING3: &str = "Heading3";
pub const STYLE_BODY: &str = "BodyText";
pub const STYLE_LIST_BULLET: &str = "ListBullet";
pub const STYLE_LIST_NUMBER: &str = "ListNumber";
pub const STYLE_CAPTION: &str = "Caption";
pub const STYLE_TABLE_HEADER: &str = "TableHeader";

/// 写出 `styles.xml` 字节流。
pub fn write_styles() -> Vec<u8> {
    let mut w = Writer::new(Vec::new());
    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .unwrap();

    let mut styles = BytesStart::new("w:styles");
    styles.push_attribute(("xmlns:w", "http://schemas.openxmlformats.org/wordprocessingml/2006/main"));
    w.write_event(Event::Start(styles)).unwrap();

    write_default(&mut w, STYLE_TITLE, "Title", "Calibri", 32, true);
    write_default(&mut w, STYLE_HEADING1, "heading 1", "Calibri", 28, true);
    write_default(&mut w, STYLE_HEADING2, "heading 2", "Calibri", 24, true);
    write_default(&mut w, STYLE_HEADING3, "heading 3", "Calibri", 22, true);
    write_default(&mut w, STYLE_BODY, "Normal", "Calibri", 22, false);
    write_default(&mut w, STYLE_LIST_BULLET, "List Bullet", "Calibri", 22, false);
    write_default(&mut w, STYLE_LIST_NUMBER, "List Number", "Calibri", 22, false);
    write_default(&mut w, STYLE_CAPTION, "Caption", "Calibri", 20, false);
    write_default(&mut w, STYLE_TABLE_HEADER, "TableHeader", "Calibri", 22, true);

    w.write_event(Event::End(BytesEnd::new("w:styles"))).unwrap();
    w.into_inner()
}

fn write_default(
    w: &mut Writer<Vec<u8>>,
    id: &str,
    name: &str,
    font: &str,
    size_half_pt: u32,
    bold: bool,
) {
    let mut s = BytesStart::new("w:style");
    s.push_attribute(("w:type", "paragraph"));
    s.push_attribute(("w:styleId", id));
    w.write_event(Event::Start(s.clone())).unwrap();

    w.write_event(Event::Start(BytesStart::new("w:name"))).unwrap();
    w.write_event(Event::Text(quick_xml::events::BytesText::new(name))).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:name"))).unwrap();

    let mut rpr = BytesStart::new("w:rPr");
    let mut rfonts = BytesStart::new("w:rFonts");
    rfonts.push_attribute(("w:ascii", font));
    rfonts.push_attribute(("w:hAnsi", font));
    w.write_event(Event::Start(rpr.clone())).unwrap();
    w.write_event(Event::Start(rfonts)).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:rFonts"))).unwrap();
    if bold {
        w.write_event(Event::Empty(BytesStart::new("w:b"))).unwrap();
    }
    w.write_event(Event::Empty(BytesStart::new("w:szCs"))).unwrap();
    let mut sz = BytesStart::new("w:sz");
    sz.push_attribute(("w:val", size_half_pt.to_string().as_str()));
    w.write_event(Event::Empty(sz)).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:rPr"))).unwrap();

    w.write_event(Event::End(BytesEnd::new("w:style"))).unwrap();
}
