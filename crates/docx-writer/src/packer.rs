//! `.docx` 打包
//!
//! 写出最小可工作的 OOXML 包：
//! - `[Content_Types].xml`
//! - `_rels/.rels`
//! - `word/_rels/document.xml.rels`
//! - `word/document.xml`
//! - `word/styles.xml`

use std::io::Write;

use doc_semantic_ast::Document;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::serializer::serialize_document;
use crate::styles::write_styles;
use crate::template::{merge_styles, parse_template, TemplateStyles};

/// 序列化 + 打包（无模板）。
pub fn pack(doc: &Document) -> Result<Vec<u8>, DocxWriteError> {
    pack_with_template(doc, None)
}

/// 序列化 + 打包 + 模板样式合并。
///
/// `template_bytes` 是 `reference.docx` 完整字节流；从中提取 `word/styles.xml`，
/// 把模板中**未在默认样式表出现**的样式补到 `styles.xml` 末尾。
pub fn pack_with_template(
    doc: &Document,
    template_bytes: Option<&[u8]>,
) -> Result<Vec<u8>, DocxWriteError> {
    let document_xml = serialize_document(doc);
    let mut styles_xml = write_styles();

    // 解析模板并合并
    let template_styles: Option<TemplateStyles> = template_bytes
        .and_then(|b| parse_template(b).ok());
    if let Some(ts) = &template_styles {
        merge_styles(&mut styles_xml, ts);
    }

    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    write_zip(&mut zip, "[Content_Types].xml", CONTENT_TYPES, opts)?;
    write_zip(&mut zip, "_rels/.rels", ROOT_RELS, opts)?;
    write_zip(&mut zip, "word/_rels/document.xml.rels", DOC_RELS, opts)?;
    write_zip(&mut zip, "word/document.xml", &document_xml, opts)?;
    write_zip(&mut zip, "word/styles.xml", &styles_xml, opts)?;

    let cursor = zip.finish().map_err(|e| DocxWriteError(e.to_string()))?;
    Ok(cursor.into_inner())
}

fn write_zip(
    zip: &mut zip::ZipWriter<std::io::Cursor<Vec<u8>>>,
    name: &str,
    content: &[u8],
    opts: SimpleFileOptions,
) -> Result<(), DocxWriteError> {
    zip.start_file(name, opts)
        .map_err(|e| DocxWriteError(e.to_string()))?;
    zip.write_all(content).map_err(|e| DocxWriteError(e.to_string()))?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[error("docx write error: {0}")]
pub struct DocxWriteError(pub String);

const CONTENT_TYPES: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
</Types>
"#;

const ROOT_RELS: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>
"#;

const DOC_RELS: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
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
        // docx 是 ZIP；magic = 0x04034b50（PK\x03\x04）
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        assert!(bytes.len() > 100);
    }
}
