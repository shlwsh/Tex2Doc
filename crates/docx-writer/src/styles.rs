//! JOS 2025 21-style 样式表（V2 重构版）
//!
//! 对应 `docs/to-docx/07-format-profiles.md` §7.5 表格。
//! 单一来源：所有 21 个样式的 ID、字体、字号、缩进、行距都在这里。
//!
//! 风格选择：**手写字符串模板**——不借助 `style()` builder。
//! 因为 21 个样式展开后共 ~120 行 XML，模板化反而难以阅读。

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

// ════════════════════════════════════════════════════════════════════
//  21 个样式 ID 常量
// ════════════════════════════════════════════════════════════════════

pub const STYLE_NORMAL: &str = "Normal";
pub const STYLE_MASTHEAD: &str = "JOSMasthead";
pub const STYLE_TITLE_ZH: &str = "JOSTitleZh";
pub const STYLE_AUTHOR_ZH: &str = "JOSAuthorZh";
pub const STYLE_INSTITUTE_ZH: &str = "JOSInstituteZh";
pub const STYLE_ABSTRACT_ZH: &str = "JOSAbstractZh";
pub const STYLE_ABSTRACT_EN: &str = "JOSAbstractEn";
pub const STYLE_KEYWORDS: &str = "JOSKeywords";
pub const STYLE_CITATION: &str = "JOSCitation";
pub const STYLE_ENGLISH_TITLE: &str = "JOSEnglishTitle";
pub const STYLE_BODY: &str = "JOSBody";
pub const STYLE_BODY_NO_INDENT: &str = "JOSBodyNoIndent";
pub const STYLE_HEADING1: &str = "JOSHeading1";
pub const STYLE_HEADING2: &str = "JOSHeading2";
pub const STYLE_HEADING3: &str = "JOSHeading3";
pub const STYLE_CAPTION: &str = "JOSCaption";
pub const STYLE_IMAGE: &str = "JOSImage";
pub const STYLE_TABLE_TEXT: &str = "JOSTableText";
pub const STYLE_CODE: &str = "JOSCode";
pub const STYLE_REFERENCE_HEADING: &str = "JOSReferenceHeading";
pub const STYLE_REFERENCE: &str = "JOSReference";

/// 简单 List 段落样式（V1 兼容）：保留 V1 的 list_bullet / list_number ID。
pub const STYLE_LIST_BULLET: &str = "ListBullet";
pub const STYLE_LIST_NUMBER: &str = "ListNumber";
pub const STYLE_TABLE_HEADER: &str = "TableHeader";

/// 写出 JOS 21-style `styles.xml` 字节流。
pub fn write_styles() -> Vec<u8> {
    let mut w = Writer::new(Vec::new());
    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .unwrap();

    let mut root = BytesStart::new("w:styles");
    root.push_attribute((
        "xmlns:w",
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    ));
    root.push_attribute((
        "xmlns:r",
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    ));
    w.write_event(Event::Start(root)).unwrap();

    // docDefaults
    write_doc_defaults(&mut w);

    // 1) Normal
    write_style(
        &mut w,
        STYLE_NORMAL,
        "Normal",
        9.0,
        "宋体",
        "Times New Roman",
        false,
        Some("both"),
        None,
        None,
        None,
        None,
        Some(260),
    );

    // 2) JOSMasthead
    write_style(
        &mut w,
        STYLE_MASTHEAD,
        "JOS masthead from sample body style 4",
        7.5,
        "宋体",
        "Times New Roman",
        false,
        None,
        None,
        None,
        None,
        None,
        Some(180),
    );

    // 3) JOSTitleZh
    write_style(
        &mut w,
        STYLE_TITLE_ZH,
        "JOS Chinese title from sample style 64",
        14.0,
        "黑体",
        "Times New Roman",
        true,
        Some("left"),
        None,
        None,
        Some(0),
        Some(120),
        None,
    );

    // 4) JOSAuthorZh
    write_style(
        &mut w,
        STYLE_AUTHOR_ZH,
        "JOS Chinese author from sample style 65",
        12.0,
        "仿宋_GB2312",
        "Times New Roman",
        false,
        Some("left"),
        None,
        None,
        Some(120),
        Some(120),
        None,
    );

    // 5) JOSInstituteZh
    write_style(
        &mut w,
        STYLE_INSTITUTE_ZH,
        "JOS institute from sample style 66",
        8.0,
        "宋体",
        "Times New Roman",
        false,
        Some("left"),
        None,
        None,
        None,
        None,
        Some(220),
    );

    // 6) JOSAbstractZh
    write_style(
        &mut w,
        STYLE_ABSTRACT_ZH,
        "JOS abstract from sample style 117",
        9.0,
        "楷体_GB2312",
        "Times New Roman",
        false,
        Some("both"),
        None,
        None,
        None,
        None,
        Some(240),
    );

    // 7) JOSAbstractEn
    write_style(
        &mut w,
        STYLE_ABSTRACT_EN,
        "JOS English abstract from sample first page",
        10.0,
        "宋体",
        "Times New Roman",
        false,
        Some("left"),
        None,
        None,
        None,
        None,
        Some(240),
    );

    // 8) JOSKeywords  (left=430 + hanging=430)
    write_style_with_ind(
        &mut w,
        STYLE_KEYWORDS,
        "JOS keywords from sample style 118",
        9.0,
        "宋体",
        "Times New Roman",
        false,
        None,
        None,
        Some(430),
        Some(430),
        None,
        None,
        Some(240),
    );

    // 9) JOSCitation
    write_style(
        &mut w,
        STYLE_CITATION,
        "JOS citation from sample style 121",
        9.0,
        "宋体",
        "Times New Roman",
        false,
        Some("both"),
        None,
        None,
        None,
        None,
        Some(220),
    );

    // 10) JOSEnglishTitle
    write_style(
        &mut w,
        STYLE_ENGLISH_TITLE,
        "JOS English title from sample style 120",
        12.0,
        "黑体",
        "Times New Roman",
        true,
        None,
        None,
        None,
        Some(120),
        Some(100),
        None,
    );

    // 11) JOSBody (firstLine=420)
    write_style(
        &mut w,
        STYLE_BODY,
        "JOS body from sample style 145",
        9.0,
        "宋体",
        "Times New Roman",
        false,
        Some("both"),
        Some(420),
        None,
        None,
        None,
        Some(260),
    );

    // 12) JOSBodyNoIndent
    write_style(
        &mut w,
        STYLE_BODY_NO_INDENT,
        "JOS body without first-line indent",
        9.0,
        "宋体",
        "Times New Roman",
        false,
        Some("both"),
        None,
        None,
        None,
        None,
        Some(260),
    );

    // 13) JOSHeading1
    write_style(
        &mut w,
        STYLE_HEADING1,
        "JOS heading 1 from sample style 213",
        10.5,
        "黑体",
        "Times New Roman",
        true,
        None,
        None,
        None,
        Some(160),
        Some(160),
        None,
    );

    // 14) JOSHeading2
    write_style(
        &mut w,
        STYLE_HEADING2,
        "JOS heading 2 from sample style 215",
        9.0,
        "黑体",
        "Times New Roman",
        true,
        None,
        None,
        None,
        Some(25),
        Some(25),
        None,
    );

    // 15) JOSHeading3
    write_style(
        &mut w,
        STYLE_HEADING3,
        "JOS heading 3 from sample style 217",
        9.0,
        "黑体",
        "Times New Roman",
        true,
        None,
        None,
        None,
        Some(20),
        Some(20),
        None,
    );

    // 16) JOSCaption
    write_style(
        &mut w,
        STYLE_CAPTION,
        "JOS caption from sample figure/table captions",
        9.0,
        "宋体",
        "Times New Roman",
        false,
        Some("center"),
        None,
        None,
        None,
        Some(120),
        None,
    );

    // 17) JOSImage
    write_style(
        &mut w,
        STYLE_IMAGE,
        "JOS image paragraph with automatic line height",
        9.0,
        "宋体",
        "Times New Roman",
        false,
        Some("center"),
        None,
        None,
        Some(80),
        Some(80),
        None,
    );

    // 18) JOSTableText
    write_style(
        &mut w,
        STYLE_TABLE_TEXT,
        "JOS table text",
        7.5,
        "宋体",
        "Times New Roman",
        false,
        Some("center"),
        None,
        None,
        None,
        None,
        Some(220),
    );

    // 19) JOSCode
    write_style(
        &mut w,
        STYLE_CODE,
        "JOS algorithm/code text",
        8.0,
        "宋体",
        "Courier New",
        false,
        None,
        None,
        None,
        None,
        None,
        Some(220),
    );

    // 20) JOSReferenceHeading
    write_style(
        &mut w,
        STYLE_REFERENCE_HEADING,
        "JOS reference heading from sample style 126",
        9.0,
        "黑体",
        "Times New Roman",
        true,
        None,
        None,
        None,
        Some(280),
        None,
        None,
    );

    // 21) JOSReference (left=420 + hanging=420)
    write_style_with_ind(
        &mut w,
        STYLE_REFERENCE,
        "JOS reference text from sample style 129",
        7.5,
        "宋体",
        "Times New Roman",
        false,
        Some("both"),
        None,
        Some(420),
        Some(420),
        None,
        None,
        Some(260),
    );

    // 22) Header — V2 通用页眉样式（居中、宋体 + 西文 Times New Roman、9pt）。
    //     必须先于 header1.xml 里 `<w:pStyle w:val="Header"/>` 出现。
    write_builtin_paragraph_style(
        &mut w,
        "Header",
        "header",
        9.0,
        "宋体",
        "Times New Roman",
        false,
        Some("center"),
    );

    // 23) Footer — V2 通用页脚样式（居中、宋体、9pt）。
    write_builtin_paragraph_style(
        &mut w,
        "Footer",
        "footer",
        9.0,
        "宋体",
        "Times New Roman",
        false,
        Some("center"),
    );

    // 24) JOSHeader — 软件学报风格页眉（楷体 9pt，居中，first page 默认）
    write_builtin_paragraph_style(
        &mut w,
        "JOSHeader",
        "JOS header (kai 9pt centered)",
        9.0,
        "楷体",
        "Times New Roman",
        false,
        Some("center"),
    );

    w.write_event(Event::End(BytesEnd::new("w:styles")))
        .unwrap();
    w.into_inner()
}

fn write_doc_defaults(w: &mut Writer<Vec<u8>>) {
    w.write_event(Event::Start(BytesStart::new("w:docDefaults")))
        .unwrap();
    w.write_event(Event::Start(BytesStart::new("w:rPrDefault")))
        .unwrap();
    w.write_event(Event::Start(BytesStart::new("w:rPr")))
        .unwrap();
    w.write_event(Event::Start(BytesStart::new("w:rFonts")))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("w:rFonts")))
        .unwrap();
    let mut sz = BytesStart::new("w:sz");
    sz.push_attribute(("w:val", "18"));
    w.write_event(Event::Empty(sz)).unwrap();
    let mut szcs = BytesStart::new("w:szCs");
    szcs.push_attribute(("w:val", "18"));
    w.write_event(Event::Empty(szcs)).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:rPr"))).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:rPrDefault")))
        .unwrap();
    w.write_event(Event::Start(BytesStart::new("w:pPrDefault")))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("w:pPrDefault")))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("w:docDefaults")))
        .unwrap();
}

/// 写出单个 `<w:style>`（带 firstLine / spacing / line / jc）。
#[allow(clippy::too_many_arguments)]
fn write_style(
    w: &mut Writer<Vec<u8>>,
    id: &str,
    name: &str,
    size_pt: f32,
    east: &str,
    ascii: &str,
    bold: bool,
    jc: Option<&str>,
    first_line: Option<u32>,
    left: Option<u32>,
    before: Option<u32>,
    after: Option<u32>,
    line: Option<u32>,
) {
    write_style_with_ind(
        w, id, name, size_pt, east, ascii, bold, jc, first_line, left, None, before, after, line,
    );
}

/// 写一个 OOXML 内置 paragraph style（type="paragraph"）。
/// 用于 `Header` / `Footer` 这类保留 id；不输出 spacing/ind（最小可用）。
#[allow(clippy::too_many_arguments)]
fn write_builtin_paragraph_style(
    w: &mut Writer<Vec<u8>>,
    id: &str,
    name: &str,
    size_pt: f32,
    east: &str,
    ascii: &str,
    bold: bool,
    jc: Option<&str>,
) {
    let mut s = BytesStart::new("w:style");
    s.push_attribute(("w:type", "paragraph"));
    s.push_attribute(("w:styleId", id));
    w.write_event(Event::Start(s)).unwrap();

    w.write_event(Event::Start(BytesStart::new("w:name")))
        .unwrap();
    w.write_event(Event::Text(quick_xml::events::BytesText::new(name)))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("w:name"))).unwrap();
    let mut based = BytesStart::new("w:basedOn");
    based.push_attribute(("w:val", STYLE_NORMAL));
    w.write_event(Event::Empty(based)).unwrap();

    if jc.is_some() {
        w.write_event(Event::Start(BytesStart::new("w:pPr")))
            .unwrap();
        if let Some(j) = jc {
            let mut jc_el = BytesStart::new("w:jc");
            jc_el.push_attribute(("w:val", j));
            w.write_event(Event::Empty(jc_el)).unwrap();
        }
        w.write_event(Event::End(BytesEnd::new("w:pPr"))).unwrap();
    }

    w.write_event(Event::Start(BytesStart::new("w:rPr")))
        .unwrap();
    let mut rf = BytesStart::new("w:rFonts");
    rf.push_attribute(("w:ascii", ascii));
    rf.push_attribute(("w:eastAsia", east));
    rf.push_attribute(("w:hAnsi", ascii));
    w.write_event(Event::Empty(rf)).unwrap();
    if bold {
        w.write_event(Event::Empty(BytesStart::new("w:b"))).unwrap();
        w.write_event(Event::Empty(BytesStart::new("w:bCs")))
            .unwrap();
    }
    let sz_val = (size_pt * 2.0).round() as u32;
    let mut sz = BytesStart::new("w:sz");
    sz.push_attribute(("w:val", sz_val.to_string().as_str()));
    w.write_event(Event::Empty(sz)).unwrap();
    let mut szcs = BytesStart::new("w:szCs");
    szcs.push_attribute(("w:val", sz_val.to_string().as_str()));
    w.write_event(Event::Empty(szcs)).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:rPr"))).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:style"))).unwrap();
}

#[allow(clippy::too_many_arguments)]
fn write_style_with_ind(
    w: &mut Writer<Vec<u8>>,
    id: &str,
    name: &str,
    size_pt: f32,
    east: &str,
    ascii: &str,
    bold: bool,
    jc: Option<&str>,
    first_line: Option<u32>,
    left: Option<u32>,
    hanging: Option<u32>,
    before: Option<u32>,
    after: Option<u32>,
    line: Option<u32>,
) {
    let mut s = BytesStart::new("w:style");
    s.push_attribute(("w:type", "paragraph"));
    s.push_attribute(("w:styleId", id));
    w.write_event(Event::Start(s)).unwrap();

    w.write_event(Event::Start(BytesStart::new("w:name")))
        .unwrap();
    w.write_event(Event::Text(quick_xml::events::BytesText::new(name)))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("w:name"))).unwrap();

    // pPr
    let has_ppr = jc.is_some()
        || first_line.is_some()
        || left.is_some()
        || before.is_some()
        || after.is_some()
        || line.is_some();
    if has_ppr {
        w.write_event(Event::Start(BytesStart::new("w:pPr")))
            .unwrap();
        if let Some(j) = jc {
            let mut jc_e = BytesStart::new("w:jc");
            jc_e.push_attribute(("w:val", j));
            w.write_event(Event::Empty(jc_e)).unwrap();
        }
        let sp = build_spacing(before, after, line);
        if !sp.is_empty() {
            w.write_event(Event::Empty(sp)).unwrap();
        }
        let ind = build_ind(first_line, left, hanging);
        if !ind.is_empty() {
            w.write_event(Event::Empty(ind)).unwrap();
        }
        w.write_event(Event::End(BytesEnd::new("w:pPr"))).unwrap();
    }

    // rPr
    w.write_event(Event::Start(BytesStart::new("w:rPr")))
        .unwrap();
    let mut rfonts = BytesStart::new("w:rFonts");
    rfonts.push_attribute(("w:ascii", ascii));
    rfonts.push_attribute(("w:hAnsi", ascii));
    rfonts.push_attribute(("w:eastAsia", east));
    rfonts.push_attribute(("w:cs", ascii));
    w.write_event(Event::Empty(rfonts)).unwrap();
    if bold {
        w.write_event(Event::Empty(BytesStart::new("w:b"))).unwrap();
        w.write_event(Event::Empty(BytesStart::new("w:bCs")))
            .unwrap();
    }
    let half_pt = (size_pt * 2.0).round() as u32;
    let mut sz = BytesStart::new("w:sz");
    sz.push_attribute(("w:val", half_pt.to_string().as_str()));
    w.write_event(Event::Empty(sz)).unwrap();
    let mut szcs = BytesStart::new("w:szCs");
    szcs.push_attribute(("w:val", half_pt.to_string().as_str()));
    w.write_event(Event::Empty(szcs)).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:rPr"))).unwrap();

    w.write_event(Event::End(BytesEnd::new("w:style"))).unwrap();
}

/// 构造 `<w:spacing>` 元素（self-closing）。
fn build_spacing(
    before: Option<u32>,
    after: Option<u32>,
    line: Option<u32>,
) -> BytesStart<'static> {
    let mut sp = BytesStart::new("w:spacing");
    if let Some(b) = before {
        sp.push_attribute(("w:before", b.to_string().as_str()));
    }
    if let Some(a) = after {
        sp.push_attribute(("w:after", a.to_string().as_str()));
    }
    if let Some(l) = line {
        sp.push_attribute(("w:line", l.to_string().as_str()));
        sp.push_attribute(("w:lineRule", "exact"));
    }
    sp
}

/// 构造 `<w:ind>` 元素（self-closing）。
fn build_ind(
    first_line: Option<u32>,
    left: Option<u32>,
    hanging: Option<u32>,
) -> BytesStart<'static> {
    let mut ind = BytesStart::new("w:ind");
    if let Some(fl) = first_line {
        ind.push_attribute(("w:firstLine", fl.to_string().as_str()));
    }
    if let Some(l) = left {
        ind.push_attribute(("w:left", l.to_string().as_str()));
    }
    if let Some(h) = hanging {
        ind.push_attribute(("w:hanging", h.to_string().as_str()));
    }
    ind
}
