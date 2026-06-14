//! styles.xml 序列化（V1 默认样式表）
//!
//! 详见方案 §4.3.2 样式 ID 命名规范。

use doc_utils::{FontProbe, FontStatus};
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
    styles.push_attribute((
        "xmlns:w",
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    ));
    w.write_event(Event::Start(styles)).unwrap();

    write_default(&mut w, STYLE_TITLE, "Title", "Calibri", 32, true);
    write_default(&mut w, STYLE_HEADING1, "heading 1", "Calibri", 28, true);
    write_default(&mut w, STYLE_HEADING2, "heading 2", "Calibri", 24, true);
    write_default(&mut w, STYLE_HEADING3, "heading 3", "Calibri", 22, true);
    write_default(&mut w, STYLE_BODY, "Normal", "Calibri", 22, false);
    write_default(
        &mut w,
        STYLE_LIST_BULLET,
        "List Bullet",
        "Calibri",
        22,
        false,
    );
    write_default(
        &mut w,
        STYLE_LIST_NUMBER,
        "List Number",
        "Calibri",
        22,
        false,
    );
    write_default(&mut w, STYLE_CAPTION, "Caption", "Calibri", 20, false);
    write_default(
        &mut w,
        STYLE_TABLE_HEADER,
        "TableHeader",
        "Calibri",
        22,
        true,
    );

    w.write_event(Event::End(BytesEnd::new("w:styles")))
        .unwrap();
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

    w.write_event(Event::Start(BytesStart::new("w:name")))
        .unwrap();
    w.write_event(Event::Text(quick_xml::events::BytesText::new(name)))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("w:name"))).unwrap();

    let mut rpr = BytesStart::new("w:rPr");
    let mut rfonts = BytesStart::new("w:rFonts");
    rfonts.push_attribute(("w:ascii", font));
    rfonts.push_attribute(("w:hAnsi", font));
    w.write_event(Event::Start(rpr.clone())).unwrap();
    w.write_event(Event::Start(rfonts)).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:rFonts")))
        .unwrap();
    if bold {
        w.write_event(Event::Empty(BytesStart::new("w:b"))).unwrap();
    }
    w.write_event(Event::Empty(BytesStart::new("w:szCs")))
        .unwrap();
    let mut sz = BytesStart::new("w:sz");
    sz.push_attribute(("w:val", size_half_pt.to_string().as_str()));
    w.write_event(Event::Empty(sz)).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:rPr"))).unwrap();

    w.write_event(Event::End(BytesEnd::new("w:style"))).unwrap();
}

/// 根据字体探测结果修改 styles_xml 字节流。
///
/// 对于标记为 Embed 的字体，在样式中嵌入字体回退声明。
/// 对于标记为 Fallback 的字体，将字体名替换为推荐字体。
pub fn apply_font_probes(styles_xml: &mut Vec<u8>, probes: &[FontProbe]) {
    if probes.is_empty() {
        return;
    }
    let xml_str = String::from_utf8_lossy(styles_xml).to_string();
    let mut modified = xml_str;
    for probe in probes {
        if probe.needs_fallback() {
            // 替换字体引用：将 w:ascii/w:hAnsi/w:eastAsia 等属性替换为 recommended
            for attr in &["w:ascii", "w:hAnsi", "w:eastAsia", "w:cs"] {
                let pattern = format!("{}=\"{}\"", attr, probe.name);
                let replacement = format!("{}=\"{}\"", attr, probe.recommended);
                if modified.contains(&pattern) {
                    modified = modified.replace(&pattern, &replacement);
                }
            }
        }
    }
    *styles_xml = modified.into_bytes();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_font_probes_no_change_when_empty() {
        let mut xml = b"<w:styles><w:style w:type=\"paragraph\" w:styleId=\"BodyText\"/></w:styles>".to_vec();
        apply_font_probes(&mut xml, &[]);
        assert_eq!(xml, b"<w:styles><w:style w:type=\"paragraph\" w:styleId=\"BodyText\"/></w:styles>".to_vec());
    }

    #[test]
    fn apply_font_probes_fallback_replaces() {
        let mut xml = b"<w:style w:type=\"paragraph\" w:styleId=\"Test\"><w:rPr><w:rFonts w:ascii=\"OldFont\" w:hAnsi=\"OldFont\"/></w:rPr></w:style>".to_vec();
        let probe = FontProbe {
            name: "OldFont".to_string(),
            status: FontStatus::Fallback,
            recommended: "SimSun".to_string(),
            system_path: None,
        };
        apply_font_probes(&mut xml, &[probe]);
        let s = String::from_utf8_lossy(&xml);
        assert!(s.contains("SimSun"));
        assert!(!s.contains("OldFont"));
    }
}
