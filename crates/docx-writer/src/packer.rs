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
    header_xml: Option<String>,
    footer_xml: Option<String>,
    first_header_xml: Option<String>,
    first_footer_xml: Option<String>,
}

/// 渲染 `PageSetup` 里的 header/footer 文本到 OOXML。
///
/// 支持的占位符：
/// - `{{PAGE}}`     → w:fldChar + w:instrText " PAGE "
/// - `{{NUMPAGES}}` → w:fldChar + w:instrText " NUMPAGES "
///
/// 返回 `(header_xml, footer_xml, first_header_xml, first_footer_xml)`，None
/// 表示不写对应 part。
fn build_header_footer(ps: &PageSetup) -> HeaderFooterParts {
    fn body(template: &str, style_id: &str) -> String {
        // 拆行：每行单独一个 paragraph；空行不写
        let mut para = String::new();
        para.push_str(&format!(
            r#"<w:p><w:pPr><w:pStyle w:val="{style_id}"/><w:jc w:val="center"/></w:pPr>"#,
        ));
        // 单段（不拆行）；后续如需多行可按 \n 切分
        for (i, seg) in template.split('\n').enumerate() {
            if i > 0 {
                para.push_str(&format!(
                    r#"</w:p><w:p><w:pPr><w:pStyle w:val="{style_id}"/><w:jc w:val="center"/></w:pPr>"#,
                ));
            }
            let escaped = xml_escape(seg);
            render_runs(&mut para, &escaped);
        }
        para.push_str("</w:p>");
        para
    }

    fn xml_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }

    let header_xml = ps.header_text.as_deref().and_then(|t| {
        if t.trim().is_empty() {
            None
        } else {
            Some(wrap_header(body(t, "JOSHeader")))
        }
    });
    let footer_xml = ps.footer_text.as_deref().and_then(|t| {
        if t.trim().is_empty() {
            None
        } else {
            Some(wrap_footer(body(t, "Footer")))
        }
    });
    let first_header_xml = ps.first_header_text.as_deref().and_then(|t| {
        if t.trim().is_empty() {
            None
        } else {
            Some(wrap_header(body(t, "JOSHeader")))
        }
    });
    let first_footer_xml = ps.first_footer_text.as_deref().and_then(|t| {
        if t.trim().is_empty() {
            None
        } else {
            Some(wrap_footer(body(t, "Footer")))
        }
    });
    HeaderFooterParts {
        header_xml,
        footer_xml,
        first_header_xml,
        first_footer_xml,
    }
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
            header_xml: None,
            footer_xml: None,
            first_header_xml: None,
            first_footer_xml: None,
        });
    let has_h = parts.header_xml.is_some();
    let has_f = parts.footer_xml.is_some();
    let has_fh = parts.first_header_xml.is_some();
    let has_ff = parts.first_footer_xml.is_some();
    let has_any_hdr_ftr = has_h || has_f || has_fh || has_ff;

    // document.xml 内的 sectPr 必须在引用 rId 前知道，所以 document.xml
    // 改由 packer 内联拼接：先调 serialize_document 拿到 body，再 append sectPr。
    let mut embedded_images: Vec<EmbeddedImage> = Vec::new();
    let body_xml = serialize_document(doc, image_assets, page_setup, &mut embedded_images);
    let body_xml = if has_any_hdr_ftr {
        inject_sectpr_refs(
            &body_xml,
            has_h,
            has_f,
            has_fh,
            has_ff,
        )
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
    let content_types = build_content_types(has_h, has_f, has_fh, has_ff);
    write_zip(&mut zip, "[Content_Types].xml", content_types.as_bytes(), opts)?;
    write_zip(&mut zip, "_rels/.rels", ROOT_RELS, opts)?;
    // document.xml.rels：动态追加 header/footer + 图片 relationship
    let doc_rels = build_doc_rels(has_h, has_f, has_fh, has_ff, &embedded_images);
    write_zip(
        &mut zip,
        "word/_rels/document.xml.rels",
        doc_rels.as_bytes(),
        opts,
    )?;
    write_zip(&mut zip, "word/document.xml", &body_xml, opts)?;
    write_zip(&mut zip, "word/styles.xml", &styles_xml, opts)?;

    // 真正把图片字节写入 word/media/，让 soffice/Word 能正确解析 r:embed
    for img in &embedded_images {
        let path = format!("word/media/image{}.{}", img.fig_id, img.ext.to_lowercase());
        write_zip(&mut zip, &path, &img.bytes, opts)?;
    }

    if let Some(h) = &parts.header_xml {
        write_zip(&mut zip, "word/header1.xml", h.as_bytes(), opts)?;
    }
    if let Some(f) = &parts.footer_xml {
        write_zip(&mut zip, "word/footer1.xml", f.as_bytes(), opts)?;
    }
    if let Some(h) = &parts.first_header_xml {
        write_zip(&mut zip, "word/header2.xml", h.as_bytes(), opts)?;
    }
    if let Some(f) = &parts.first_footer_xml {
        write_zip(&mut zip, "word/footer2.xml", f.as_bytes(), opts)?;
    }

    let cursor = zip.finish().map_err(|e| DocxWriteError(e.to_string()))?;
    Ok(cursor.into_inner())
}

/// 把 serialize_document 生成的 sectPr 里追加 header/footer 引用。
///
/// 策略：找到最后一个 `</w:sectPr>`，在 `</w:sectPr>` 之前按
/// `titlePg → first_header → first_footer → header → footer → pgSz → pgMar → cols`
/// 的 OOXML 规范顺序插入 reference。
fn inject_sectpr_refs(
    body_xml: &[u8],
    has_h: bool,
    has_f: bool,
    has_fh: bool,
    has_ff: bool,
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
    if has_fh || has_ff {
        injection.push_str(r#"<w:titlePg/>"#);
    }
    if has_h {
        injection.push_str(
            r#"<w:headerReference w:type="default" r:id="rIdH1"/>"#,
        );
    }
    if has_f {
        injection.push_str(
            r#"<w:footerReference w:type="default" r:id="rIdF1"/>"#,
        );
    }
    if has_fh {
        injection.push_str(
            r#"<w:headerReference w:type="first" r:id="rIdH2"/>"#,
        );
    }
    if has_ff {
        injection.push_str(
            r#"<w:footerReference w:type="first" r:id="rIdF2"/>"#,
        );
    }
    let mut out = String::with_capacity(s.len() + injection.len());
    out.push_str(&s[..sectpr_end]);
    out.push_str(&injection);
    out.push_str(&s[sectpr_end..]);
    out.into_bytes()
}

/// 构造 [Content_Types].xml（始终包含 styles/document；可选追加 header/footer）。
fn build_content_types(
    has_h: bool,
    has_f: bool,
    has_fh: bool,
    has_ff: bool,
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
    if has_h {
        s.push_str(
            "<Override PartName=\"/word/header1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml\"/>",
        );
    }
    if has_f {
        s.push_str(
            "<Override PartName=\"/word/footer1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml\"/>",
        );
    }
    if has_fh {
        s.push_str(
            "<Override PartName=\"/word/header2.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml\"/>",
        );
    }
    if has_ff {
        s.push_str(
            "<Override PartName=\"/word/footer2.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml\"/>",
        );
    }
    s.push_str("</Types>");
    s
}

/// 构造 word/_rels/document.xml.rels（必须包含 rId1→styles，可选追加 header/footer）。
///
/// 使用 `rIdH1`/`rIdF1`/`rIdH2`/`rIdF2` 作为 part id，与 `inject_sectpr_refs` 对应。
/// 图片的 rId 形如 `rIdImgN`，由调用方传入已嵌入的图片清单。
fn build_doc_rels(
    has_h: bool,
    has_f: bool,
    has_fh: bool,
    has_ff: bool,
    embedded_images: &[EmbeddedImage],
) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
  <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>",
    );
    if has_h {
        s.push_str(
            "<Relationship Id=\"rIdH1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/header\" Target=\"header1.xml\"/>",
        );
    }
    if has_f {
        s.push_str(
            "<Relationship Id=\"rIdF1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer\" Target=\"footer1.xml\"/>",
        );
    }
    if has_fh {
        s.push_str(
            "<Relationship Id=\"rIdH2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/header\" Target=\"header2.xml\"/>",
        );
    }
    if has_ff {
        s.push_str(
            "<Relationship Id=\"rIdF2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer\" Target=\"footer2.xml\"/>",
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
            footer_text: Some("第 {{PAGE}} / {{NUMPAGES}} 页".to_string()),
            first_header_text: None,
            first_footer_text: Some("首页脚".to_string()),
        };
        let bytes = pack_with_page_setup(&doc, None, None, Some(&ps)).unwrap();
        // 必须含 header1.xml / footer1.xml / footer2.xml
        let mut r = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
        let names: Vec<String> = (0..r.len())
            .map(|i| r.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "word/header1.xml"), "{names:?}");
        assert!(names.iter().any(|n| n == "word/footer1.xml"), "{names:?}");
        assert!(names.iter().any(|n| n == "word/footer2.xml"), "{names:?}");
        // 验证 footer1.xml 含 PAGE 字段
        let mut f = r.by_name("word/footer1.xml").unwrap();
        let mut s = String::new();
        std::io::Read::read_to_string(&mut f, &mut s).unwrap();
        assert!(s.contains("PAGE"), "footer1.xml must contain PAGE field: {s}");
        assert!(s.contains("NUMPAGES"), "footer1.xml must contain NUMPAGES field: {s}");
    }
}
