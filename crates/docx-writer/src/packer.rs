//! `.docx` 打包
//!
//! 写出最小可工作的 OOXML 包（含可选模板继承和图片嵌入）。

use std::io::Write;

use doc_semantic_ast::Document;
use doc_utils::ImageAssets;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::page_setup::PageSetup;
use crate::serializer::{serialize_document, EmbeddedImage};
use crate::styles::write_styles;
use crate::template::{merge_styles, parse_template, TemplateStyles};

/// V2：header/footer 部件的字节内容。None 表示不写该 part。
struct HeaderFooterParts {
    /// header0.xml — 首页 masthead（first）
    masthead_header_xml: Option<String>,
    /// header1.xml — 正文奇数页 running header（default）
    default_header_xml: Option<String>,
    /// header2.xml — 偶数页页眉（even）
    even_header_xml: Option<String>,
    /// footer1.xml — 首页页脚（first，带上边框线）
    first_footer_xml: Option<String>,
    /// footer2.xml — 正文奇数页页脚（default，空）
    default_footer_xml: Option<String>,
    /// footer3.xml — 偶数页页脚（even，空）
    even_footer_xml: Option<String>,
}

/// 渲染 `PageSetup` 里的 header/footer 文本到 OOXML。
///
/// 支持的占位符：
/// - `{{PAGE}}`     → w:fldChar + w:instrText " PAGE "
/// - `{{NUMPAGES}}` → w:fldChar + w:instrText " NUMPAGES "
///
/// 返回 JOS 六件套 header/footer 部件 XML；None 表示不写该 part。
fn build_header_footer(ps: &PageSetup) -> HeaderFooterParts {
    let text_width = ps.text_width_twips();
    let jos_mode = ps.even_header_text.is_some()
        || ps.first_footer_indent_twips.is_some()
        || ps.header_text.as_ref().is_some_and(|t| !t.trim().is_empty());

    if !jos_mode {
        return HeaderFooterParts {
            masthead_header_xml: None,
            default_header_xml: None,
            even_header_xml: None,
            first_footer_xml: ps.first_footer_text.as_deref().and_then(|t| {
                if t.trim().is_empty() {
                    None
                } else {
                    Some(wrap_footer(jos_first_footer_body(
                        t,
                        ps.first_footer_indent_twips.unwrap_or(0),
                    )))
                }
            }),
            default_footer_xml: ps.footer_text.as_deref().and_then(|t| {
                if t.trim().is_empty() {
                    None
                } else {
                    Some(wrap_footer(legacy_footer_body(t)))
                }
            }),
            even_footer_xml: None,
        };
    }

    let running = ps.header_text.as_deref().unwrap_or("").trim();
    let even = ps
        .even_header_text
        .as_deref()
        .unwrap_or(PageSetup::JOS_EVEN_HEADER)
        .trim();
    let first_footer = ps
        .first_footer_text
        .as_deref()
        .unwrap_or(PageSetup::JOS_FIRST_FOOTER)
        .trim();
    let indent = ps.first_footer_indent_twips.unwrap_or(330);

    HeaderFooterParts {
        masthead_header_xml: Some(wrap_header(masthead_body(text_width))),
        default_header_xml: Some(wrap_header(header_line_body(running, text_width))),
        even_header_xml: Some(wrap_header(header_line_body(even, text_width))),
        first_footer_xml: Some(wrap_footer(jos_first_footer_body(first_footer, indent))),
        default_footer_xml: Some(wrap_footer(empty_footer_body())),
        even_footer_xml: Some(wrap_footer(empty_footer_body())),
    }
}

fn legacy_footer_body(template: &str) -> String {
    let mut para = String::from(
        r#"<w:p><w:pPr><w:pStyle w:val="Footer"/><w:jc w:val="center"/></w:pPr>"#,
    );
    for (i, seg) in template.split('\n').enumerate() {
        if i > 0 {
            para.push_str(
                r#"</w:p><w:p><w:pPr><w:pStyle w:val="Footer"/><w:jc w:val="center"/></w:pPr>"#,
            );
        }
        render_runs(&mut para, &xml_escape_local(seg));
    }
    para.push_str("</w:p>");
    para
}

fn jos_first_footer_body(text: &str, indent_twips: u32) -> String {
    format!(
        r#"<w:p><w:pPr><w:pStyle w:val="JOSMasthead"/><w:jc w:val="left"/><w:ind w:left="{indent_twips}"/><w:pBdr><w:top w:val="single" w:sz="4" w:space="1" w:color="auto"/></w:pBdr></w:pPr><w:r><w:t xml:space="preserve">{text}</w:t></w:r></w:p>"#,
        text = xml_escape_local(text)
    )
}

fn empty_footer_body() -> String {
    "<w:p/>".to_string()
}

fn header_line_body(text: &str, text_width: u32) -> String {
    // v13.2 F1: 与 sh 的 header_xml() 对齐——
    //   pStyle="JOSMasthead"、run 上无内嵌 rPr（不写 w:rFonts/w:sz，
    //   由 styles.xml 中的 JOSMasthead 样式统一提供字体/字号）。
    //   页码字段用 <w:fldSimple>，与 sh 完全一致。
    let clean = text.lines().next().unwrap_or(text).trim();
    let mut para = format!(
        r#"<w:p><w:pPr><w:pStyle w:val="JOSMasthead"/><w:tabs><w:tab w:val="right" w:pos="{text_width}"/></w:tabs></w:pPr>"#,
    );
    para.push_str(r#"<w:r><w:t xml:space="preserve">"#);
    para.push_str(&xml_escape_local(clean));
    para.push_str("</w:t></w:r><w:r><w:tab/></w:r>");
    para.push_str(r#"<w:fldSimple w:instr=" PAGE "><w:r><w:t>1</w:t></w:r></w:fldSimple>"#);
    para.push_str("</w:p>");
    para
}

fn masthead_body(text_width: u32) -> String {
    let rows = [
        (
            "软件学报 ISSN 1000-9825, CODEN RUXUEW",
            "E-mail: jos@iscas.ac.cn",
        ),
        (
            "Journal of Software, [doi: 10.13328/j.cnki.jos.000000]",
            "http://www.jos.org.cn",
        ),
        (
            "© 中国科学院软件研究所版权所有.",
            "Tel: +86-10-62562563",
        ),
    ];
    let mut out = String::new();
    for (left, right) in rows {
        out.push_str(&format!(
            r#"<w:p><w:pPr><w:pStyle w:val="JOSMasthead"/><w:tabs><w:tab w:val="right" w:pos="{text_width}"/></w:tabs></w:pPr>"#,
        ));
        out.push_str(r#"<w:r><w:t xml:space="preserve">"#);
        out.push_str(&xml_escape_local(left));
        out.push_str("</w:t></w:r><w:r><w:tab/></w:r>");
        out.push_str(r#"<w:r><w:t xml:space="preserve">"#);
        out.push_str(&xml_escape_local(right));
        out.push_str("</w:t></w:r></w:p>");
    }
    out
}

fn xml_escape_local(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// 把 `{{PAGE}}` / `{{NUMPAGES}}` 替换成对应 OOXML 字段，再把普通文本写成 w:r。
fn render_runs(out: &mut String, text: &str) {
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(pos) = rest.find("{{PAGE}}") {
            // 前置文本
            if pos > 0 {
                out.push_str("<w:r><w:rPr><w:rFonts w:ascii=\"Times New Roman\" w:eastAsia=\"SimSun\"/><w:sz w:val=\"18\"/></w:rPr><w:t xml:space=\"preserve\">");
                out.push_str(&rest[..pos]);
                out.push_str("</w:t></w:r>");
            }
            // PAGE 字段
            out.push_str(
                r#"<w:r><w:rPr><w:rFonts w:ascii="Times New Roman" w:eastAsia="SimSun"/><w:sz w:val="18"/></w:rPr><w:fldChar w:fldCharType="begin"/></w:r>"#,
            );
            out.push_str(
                r#"<w:r><w:rPr><w:rFonts w:ascii="Times New Roman" w:eastAsia="SimSun"/><w:sz w:val="18"/></w:rPr><w:instrText xml:space="preserve"> PAGE </w:instrText></w:r>"#,
            );
            out.push_str(
                r#"<w:r><w:rPr><w:rFonts w:ascii="Times New Roman" w:eastAsia="SimSun"/><w:sz w:val="18"/></w:rPr><w:fldChar w:fldCharType="separate"/></w:r>"#,
            );
            out.push_str(
                r#"<w:r><w:rPr><w:rFonts w:ascii="Times New Roman" w:eastAsia="SimSun"/><w:sz w:val="18"/></w:rPr><w:t>1</w:t></w:r>"#,
            );
            out.push_str(
                r#"<w:r><w:rPr><w:rFonts w:ascii="Times New Roman" w:eastAsia="SimSun"/><w:sz w:val="18"/></w:rPr><w:fldChar w:fldCharType="end"/></w:r>"#,
            );
            rest = &rest[pos + "{{PAGE}}".len()..];
        } else if let Some(pos) = rest.find("{{NUMPAGES}}") {
            if pos > 0 {
                out.push_str("<w:r><w:rPr><w:rFonts w:ascii=\"Times New Roman\" w:eastAsia=\"SimSun\"/><w:sz w:val=\"18\"/></w:rPr><w:t xml:space=\"preserve\">");
                out.push_str(&rest[..pos]);
                out.push_str("</w:t></w:r>");
            }
            out.push_str(
                r#"<w:r><w:rPr><w:rFonts w:ascii="Times New Roman" w:eastAsia="SimSun"/><w:sz w:val="18"/></w:rPr><w:fldChar w:fldCharType="begin"/></w:r>"#,
            );
            out.push_str(
                r#"<w:r><w:rPr><w:rFonts w:ascii="Times New Roman" w:eastAsia="SimSun"/><w:sz w:val="18"/></w:rPr><w:instrText xml:space="preserve"> NUMPAGES </w:instrText></w:r>"#,
            );
            out.push_str(
                r#"<w:r><w:rPr><w:rFonts w:ascii="Times New Roman" w:eastAsia="SimSun"/><w:sz w:val="18"/></w:rPr><w:fldChar w:fldCharType="separate"/></w:r>"#,
            );
            out.push_str(
                r#"<w:r><w:rPr><w:rFonts w:ascii="Times New Roman" w:eastAsia="SimSun"/><w:sz w:val="18"/></w:rPr><w:t>1</w:t></w:r>"#,
            );
            out.push_str(
                r#"<w:r><w:rPr><w:rFonts w:ascii="Times New Roman" w:eastAsia="SimSun"/><w:sz w:val="18"/></w:rPr><w:fldChar w:fldCharType="end"/></w:r>"#,
            );
            rest = &rest[pos + "{{NUMPAGES}}".len()..];
        } else {
            // 剩余整段
            out.push_str("<w:r><w:rPr><w:rFonts w:ascii=\"Times New Roman\" w:eastAsia=\"SimSun\"/><w:sz w:val=\"18\"/></w:rPr><w:t xml:space=\"preserve\">");
            out.push_str(rest);
            out.push_str("</w:t></w:r>");
            break;
        }
    }
}

fn wrap_header(body: String) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:hdr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
{body}\
</w:hdr>"
    )
}

fn wrap_footer(body: String) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:ftr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
{body}\
</w:ftr>"
    )
}

/// 序列化 + 打包（无模板、无图片）。
pub fn pack(doc: &Document) -> Result<Vec<u8>, DocxWriteError> {
    pack_with_page_setup(doc, None, None, None)
}

/// 序列化 + 打包 + 模板样式合并（无图片）。
pub fn pack_with_template(
    doc: &Document,
    template_bytes: Option<&[u8]>,
) -> Result<Vec<u8>, DocxWriteError> {
    pack_with_page_setup(doc, template_bytes, None, None)
}
/// 序列化 + 打包 + 模板样式合并 + 图片嵌入。
pub fn pack_with_assets(
    doc: &Document,
    template_bytes: Option<&[u8]>,
    image_assets: Option<&ImageAssets>,
) -> Result<Vec<u8>, DocxWriteError> {
    pack_with_page_setup(doc, template_bytes, image_assets, None)
}

/// V2 新增：序列化 + 打包 + 模板 + 图片 + 自定义页面设置。
///
/// `page_setup`：Some → 写自定义 `pgSz / pgMar / cols`；None → fallback 到
/// `PageSetup::default()`（12240×15840 twips + 1440/1800/1440/1440 margins + 1 col）。
pub fn pack_with_page_setup(
    doc: &Document,
    template_bytes: Option<&[u8]>,
    image_assets: Option<&ImageAssets>,
    page_setup: Option<&PageSetup>,
) -> Result<Vec<u8>, DocxWriteError> {
    // V2：先把 PageSetup 里的 header/footer 渲染成 part
    let parts = page_setup
        .map(|ps| build_header_footer(ps))
        .unwrap_or(HeaderFooterParts {
            masthead_header_xml: None,
            default_header_xml: None,
            even_header_xml: None,
            first_footer_xml: None,
            default_footer_xml: None,
            even_footer_xml: None,
        });
    let has_mh = parts.masthead_header_xml.is_some();
    let has_h = parts.default_header_xml.is_some();
    let has_eh = parts.even_header_xml.is_some();
    let has_ff = parts.first_footer_xml.is_some();
    let has_f = parts.default_footer_xml.is_some();
    let has_ef = parts.even_footer_xml.is_some();
    let has_any_hdr_ftr = has_mh || has_h || has_eh || has_ff || has_f || has_ef;

    // document.xml 内的 sectPr 必须在引用 rId 前知道，所以 document.xml
    // 改由 packer 内联拼接：先调 serialize_document 拿到 body，再 append sectPr。
    let mut embedded_images: Vec<EmbeddedImage> = Vec::new();
    let body_xml = serialize_document(doc, image_assets, page_setup, &mut embedded_images);
    let body_xml = if has_any_hdr_ftr {
        inject_sectpr_refs(&body_xml, has_mh, has_h, has_eh, has_ff, has_f, has_ef)
    } else {
        body_xml
    };
    let mut styles_xml = write_styles();

    // 解析模板并合并
    let template_styles: Option<TemplateStyles> =
        template_bytes.and_then(|b| parse_template(b).ok());
    if let Some(ts) = &template_styles {
        merge_styles(&mut styles_xml, ts);
    }

    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    // [Content_Types].xml：动态追加 header/footer Override
    let content_types = build_content_types(has_mh, has_h, has_eh, has_ff, has_f, has_ef, has_any_hdr_ftr);
    write_zip(
        &mut zip,
        "[Content_Types].xml",
        content_types.as_bytes(),
        opts,
    )?;
    write_zip(&mut zip, "_rels/.rels", ROOT_RELS, opts)?;
    // document.xml.rels：动态追加 header/footer + 图片 relationship
    let doc_rels = build_doc_rels(has_mh, has_h, has_eh, has_ff, has_f, has_ef, has_any_hdr_ftr, &embedded_images);
    write_zip(
        &mut zip,
        "word/_rels/document.xml.rels",
        doc_rels.as_bytes(),
        opts,
    )?;
    write_zip(&mut zip, "word/document.xml", &body_xml, opts)?;
    write_zip(&mut zip, "word/styles.xml", &styles_xml, opts)?;
    if has_any_hdr_ftr {
        write_zip(&mut zip, "word/settings.xml", SETTINGS_XML, opts)?;
    }

    // 真正把图片字节写入 word/media/，让 soffice/Word 能正确解析 r:embed
    for img in &embedded_images {
        let path = format!("word/media/image{}.{}", img.fig_id, img.ext.to_lowercase());
        write_zip(&mut zip, &path, &img.bytes, opts)?;
    }

    if let Some(h) = &parts.masthead_header_xml {
        write_zip(&mut zip, "word/header0.xml", h.as_bytes(), opts)?;
    }
    if let Some(h) = &parts.default_header_xml {
        write_zip(&mut zip, "word/header1.xml", h.as_bytes(), opts)?;
    }
    if let Some(h) = &parts.even_header_xml {
        write_zip(&mut zip, "word/header2.xml", h.as_bytes(), opts)?;
    }
    if let Some(f) = &parts.first_footer_xml {
        write_zip(&mut zip, "word/footer1.xml", f.as_bytes(), opts)?;
    }
    if let Some(f) = &parts.default_footer_xml {
        write_zip(&mut zip, "word/footer2.xml", f.as_bytes(), opts)?;
    }
    if let Some(f) = &parts.even_footer_xml {
        write_zip(&mut zip, "word/footer3.xml", f.as_bytes(), opts)?;
    }

    let cursor = zip.finish().map_err(|e| DocxWriteError(e.to_string()))?;
    Ok(cursor.into_inner())
}

/// 把 serialize_document 生成的 sectPr 里追加 header/footer 引用。
///
/// 对齐 build_jos_docx.py：header0(first) / header1(default) / header2(even) /
/// footer1(first) / footer2(default) / footer3(even) + titlePg。
fn inject_sectpr_refs(
    body_xml: &[u8],
    has_mh: bool,
    has_h: bool,
    has_eh: bool,
    has_ff: bool,
    has_f: bool,
    has_ef: bool,
) -> Vec<u8> {
    let s = match std::str::from_utf8(body_xml) {
        Ok(s) => s,
        Err(_) => return body_xml.to_vec(),
    };
    let sectpr_end = match s.rfind("</w:sectPr>") {
        Some(p) => p,
        None => return body_xml.to_vec(),
    };
    let mut injection = String::new();
    if has_mh {
        injection.push_str(r#"<w:headerReference w:type="first" r:id="rIdH0"/>"#);
    }
    if has_h {
        injection.push_str(r#"<w:headerReference w:type="default" r:id="rIdH1"/>"#);
    }
    if has_eh {
        injection.push_str(r#"<w:headerReference w:type="even" r:id="rIdH2"/>"#);
    }
    if has_ff {
        injection.push_str(r#"<w:footerReference w:type="first" r:id="rIdF1"/>"#);
    }
    if has_f {
        injection.push_str(r#"<w:footerReference w:type="default" r:id="rIdF2"/>"#);
    }
    if has_ef {
        injection.push_str(r#"<w:footerReference w:type="even" r:id="rIdF3"/>"#);
    }
    if has_mh || has_ff {
        injection.push_str(r#"<w:titlePg/>"#);
    }
    let mut out = String::with_capacity(s.len() + injection.len());
    out.push_str(&s[..sectpr_end]);
    out.push_str(&injection);
    out.push_str(&s[sectpr_end..]);
    out.into_bytes()
}

/// 构造 [Content_Types].xml（始终包含 styles/document；可选追加 header/footer）。
fn build_content_types(
    has_mh: bool,
    has_h: bool,
    has_eh: bool,
    has_ff: bool,
    has_f: bool,
    has_ef: bool,
    has_settings: bool,
) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
  <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
  <Default Extension=\"xml\" ContentType=\"application/xml\"/>\
  <Default Extension=\"png\" ContentType=\"image/png\"/>\
  <Default Extension=\"jpg\" ContentType=\"image/jpeg\"/>\
  <Default Extension=\"jpeg\" ContentType=\"image/jpeg\"/>\
  <Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
  <Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>",
    );
    if has_mh {
        s.push_str(
            "<Override PartName=\"/word/header0.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml\"/>",
        );
    }
    if has_h {
        s.push_str(
            "<Override PartName=\"/word/header1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml\"/>",
        );
    }
    if has_eh {
        s.push_str(
            "<Override PartName=\"/word/header2.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml\"/>",
        );
    }
    if has_ff {
        s.push_str(
            "<Override PartName=\"/word/footer1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml\"/>",
        );
    }
    if has_f {
        s.push_str(
            "<Override PartName=\"/word/footer2.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml\"/>",
        );
    }
    if has_ef {
        s.push_str(
            "<Override PartName=\"/word/footer3.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml\"/>",
        );
    }
    if has_settings {
        s.push_str(
            "<Override PartName=\"/word/settings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml\"/>",
        );
    }
    s.push_str("</Types>");
    s
}

/// 构造 word/_rels/document.xml.rels（必须包含 rId1→styles，可选追加 header/footer）。
///
/// rIdH0→header0, rIdH1→header1, rIdH2→header2,
/// rIdF1→footer1, rIdF2→footer2, rIdF3→footer3。
fn build_doc_rels(
    has_mh: bool,
    has_h: bool,
    has_eh: bool,
    has_ff: bool,
    has_f: bool,
    has_ef: bool,
    has_settings: bool,
    embedded_images: &[EmbeddedImage],
) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
  <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>",
    );
    if has_settings {
        s.push_str(
            "<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings\" Target=\"settings.xml\"/>",
        );
    }
    if has_mh {
        s.push_str(
            "<Relationship Id=\"rIdH0\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/header\" Target=\"header0.xml\"/>",
        );
    }
    if has_h {
        s.push_str(
            "<Relationship Id=\"rIdH1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/header\" Target=\"header1.xml\"/>",
        );
    }
    if has_eh {
        s.push_str(
            "<Relationship Id=\"rIdH2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/header\" Target=\"header2.xml\"/>",
        );
    }
    if has_ff {
        s.push_str(
            "<Relationship Id=\"rIdF1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer\" Target=\"footer1.xml\"/>",
        );
    }
    if has_f {
        s.push_str(
            "<Relationship Id=\"rIdF2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer\" Target=\"footer2.xml\"/>",
        );
    }
    if has_ef {
        s.push_str(
            "<Relationship Id=\"rIdF3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer\" Target=\"footer3.xml\"/>",
        );
    }
    // 图片关系：每个 embedded_image 生成 rIdImg{fig_id} → media/image{fig_id}.{ext}
    for img in embedded_images {
        let ext_lower = img.ext.to_lowercase();
        let ctype = match ext_lower.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            _ => "image/png", // 默认 png（避免未知格式挂在主 rels）
        };
        s.push_str(&format!(
            "<Relationship Id=\"rIdImg{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/image{}.{}\"/>",
            img.fig_id, img.fig_id, ext_lower
        ));
        let _ = ctype; // ctype 信息在 [Content_Types].xml 中以 Default Extension 形式处理
    }
    s.push_str("</Relationships>");
    s
}

fn write_zip(
    zip: &mut zip::ZipWriter<std::io::Cursor<Vec<u8>>>,
    name: &str,
    content: &[u8],
    opts: SimpleFileOptions,
) -> Result<(), DocxWriteError> {
    zip.start_file(name, opts)
        .map_err(|e| DocxWriteError(e.to_string()))?;
    zip.write_all(content)
        .map_err(|e| DocxWriteError(e.to_string()))?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[error("docx write error: {0}")]
pub struct DocxWriteError(pub String);

const SETTINGS_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:evenAndOddHeaders/>
  <w:characterSpacingControl w:val="doNotCompress"/>
</w:settings>
"#;

const ROOT_RELS: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use doc_semantic_ast::{Block, Span, TextRun, TextStyle};

    #[test]
    fn pack_minimal() {
        let mut doc = Document::new();
        doc.push(Block::Heading {
            level: 1,
            text: "Title".into(),
            number: None,
            span: Span::default(),
        });
        doc.push(Block::Paragraph {
            runs: vec![TextRun {
                text: "Hello".into(),
                style: TextStyle::Plain,
                span: Span::default(),
            }],
            span: Span::default(),
        });
        let bytes = pack(&doc).unwrap();
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        assert!(bytes.len() > 100);
    }

    #[test]
    fn pack_with_header_footer() {
        use doc_semantic_ast::Document;
        let mut doc = Document::new();
        doc.push(Block::Paragraph {
            runs: vec![TextRun {
                text: "body".into(),
                style: TextStyle::Plain,
                span: Span::default(),
            }],
            span: Span::default(),
        });
        let ps = PageSetup {
            width_twips: 11906,
            height_twips: 16838,
            margin_top: Some(1440),
            margin_right: Some(1440),
            margin_bottom: Some(1440),
            margin_left: Some(1440),
            margin_header: Some(720),
            margin_footer: Some(720),
            cols_space: None,
            cols_num: None,
            header_text: Some("软件学报 ISSN 1000-9825".to_string()),
            footer_text: None,
            first_header_text: None,
            first_footer_text: Some("首页脚".to_string()),
            even_header_text: None,
            first_footer_indent_twips: Some(330),
        };
        let bytes = pack_with_page_setup(&doc, None, None, Some(&ps)).unwrap();
        let mut r = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
        let names: Vec<String> = (0..r.len())
            .map(|i| r.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "word/header0.xml"), "{names:?}");
        assert!(names.iter().any(|n| n == "word/header1.xml"), "{names:?}");
        assert!(names.iter().any(|n| n == "word/header2.xml"), "{names:?}");
        assert!(names.iter().any(|n| n == "word/footer1.xml"), "{names:?}");
        assert!(names.iter().any(|n| n == "word/footer2.xml"), "{names:?}");
        assert!(names.iter().any(|n| n == "word/footer3.xml"), "{names:?}");
        // running header 含 PAGE 字段
        let header1 = {
            let mut h = r.by_name("word/header1.xml").unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut h, &mut s).unwrap();
            s
        };
        assert!(
            header1.contains("PAGE"),
            "header1.xml must contain PAGE field: {header1}"
        );
        // v13.2 F1: header1 必须使用 JOSMasthead 样式且 run 上无内嵌 rPr
        assert!(
            header1.contains(r#"<w:pStyle w:val="JOSMasthead"/>"#),
            "header1 should use JOSMasthead: {header1}"
        );
        assert!(
            !header1.contains("<w:rPr>"),
            "header1 must not embed rPr inside runs: {header1}"
        );
        // v13.2 F1: tab 位置必须等于 text_width（不是硬编码 9000）
        let text_width = ps.text_width_twips();
        assert!(
            header1.contains(&format!(r#"<w:tab w:val="right" w:pos="{text_width}"/>"#)),
            "header1 tab pos should equal text_width {text_width}: {header1}"
        );
        // 首页页脚
        let footer1 = {
            let mut f = r.by_name("word/footer1.xml").unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut f, &mut s).unwrap();
            s
        };
        assert!(
            footer1.contains("首页脚"),
            "footer1.xml must contain first footer: {footer1}"
        );
        // v13.2 F1: footer1 必须使用 JOSMasthead 样式（不是 Footer）
        assert!(
            footer1.contains(r#"<w:pStyle w:val="JOSMasthead"/>"#),
            "footer1 should use JOSMasthead: {footer1}"
        );
    }

    /// v13.2 F1: sectPr 内的 header/footer 引用必须 6 个齐全且类型对得上 rId。
    #[test]
    fn pack_sectpr_has_all_six_header_footer_refs_with_correct_types() {
        use doc_semantic_ast::Document;
        let mut doc = Document::new();
        doc.push(Block::Paragraph {
            runs: vec![TextRun {
                text: "body".into(),
                style: TextStyle::Plain,
                span: Span::default(),
            }],
            span: Span::default(),
        });
        let ps = PageSetup {
            width_twips: 11906,
            height_twips: 16838,
            margin_top: Some(1440),
            margin_right: Some(1440),
            margin_bottom: Some(1440),
            margin_left: Some(1440),
            margin_header: Some(720),
            margin_footer: Some(720),
            cols_space: None,
            cols_num: None,
            header_text: Some("软件学报 ISSN 1000-9825".to_string()),
            footer_text: None,
            first_header_text: None,
            first_footer_text: Some("收稿时间: XXX".to_string()),
            even_header_text: Some("Journal of Software 软件学报".to_string()),
            first_footer_indent_twips: Some(330),
        };
        let bytes = pack_with_page_setup(&doc, None, None, Some(&ps)).unwrap();
        let mut r = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
        let mut doc_xml = String::new();
        std::io::Read::read_to_string(
            &mut r.by_name("word/document.xml").unwrap(),
            &mut doc_xml,
        )
        .unwrap();
        // 必须 6 个 headerReference / footerReference + titlePg
        for typ in ["first", "default", "even"] {
            assert!(
                doc_xml.contains(&format!(r#"<w:headerReference w:type="{typ}"#)),
                "missing headerReference {typ} in sectPr: {doc_xml}"
            );
            assert!(
                doc_xml.contains(&format!(r#"<w:footerReference w:type="{typ}"#)),
                "missing footerReference {typ} in sectPr: {doc_xml}"
            );
        }
        assert!(doc_xml.contains("<w:titlePg/>"), "missing titlePg: {doc_xml}");
        // rIdH0 / rIdH1 / rIdH2 必须是 first / default / even
        assert!(
            doc_xml.contains(r#"<w:headerReference w:type="first" r:id="rIdH0"/>"#),
            "first header should be rIdH0: {doc_xml}"
        );
        assert!(
            doc_xml.contains(r#"<w:headerReference w:type="default" r:id="rIdH1"/>"#),
            "default header should be rIdH1: {doc_xml}"
        );
        assert!(
            doc_xml.contains(r#"<w:headerReference w:type="even" r:id="rIdH2"/>"#),
            "even header should be rIdH2: {doc_xml}"
        );
    }
}
