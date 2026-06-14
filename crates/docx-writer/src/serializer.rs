//! AST → OOXML 元素序列化
//!
//! 支持的块（V1）：
//! - Heading / Paragraph
//! - List（itemize / enumerate）
//! - Table（简单网格）
//! - Figure（占位）
//! - Equation（OMML `<m:oMath>`，由 doc-mathml 转换）
//! - Bibliography
//! - RawFallback

use doc_mathml::{parse_latex_math, to_omml};
use doc_semantic_ast::{Block, Document, TextStyle};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use crate::model::{Paragraph, Run};
use crate::styles::{
    STYLE_BODY, STYLE_CAPTION, STYLE_HEADING1, STYLE_HEADING2, STYLE_HEADING3, STYLE_LIST_BULLET,
    STYLE_LIST_NUMBER, STYLE_TABLE_HEADER, STYLE_TITLE,
};

/// 写出 `document.xml` 字节流。
pub fn serialize_document(doc: &Document) -> Vec<u8> {
    let mut w = Writer::new(Vec::new());
    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .unwrap();

    let mut root = BytesStart::new("w:document");
    root.push_attribute((
        "xmlns:w",
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    ));
    root.push_attribute((
        "xmlns:r",
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    ));
    root.push_attribute((
        "xmlns:m",
        "http://schemas.openxmlformats.org/officeDocument/2006/math",
    ));
    w.write_event(Event::Start(root)).unwrap();
    w.write_event(Event::Start(BytesStart::new("w:body")))
        .unwrap();

    for block in &doc.blocks {
        match block {
            Block::Heading { level, text, .. } => {
                let style = match level {
                    1 => STYLE_HEADING1,
                    2 => STYLE_HEADING2,
                    3 => STYLE_HEADING3,
                    _ => STYLE_TITLE,
                };
                let para = Paragraph {
                    style_id: Some(style.to_string()),
                    runs: vec![Run {
                        text: text.clone(),
                        style_id: Some(style.to_string()),
                        bold: true,
                        italic: false,
                    }],
                };
                write_paragraph(&mut w, &para);
            }
            Block::Paragraph { runs, .. } => {
                let para = Paragraph {
                    style_id: Some(STYLE_BODY.to_string()),
                    runs: runs
                        .iter()
                        .map(|r| Run {
                            text: r.text.clone(),
                            style_id: None,
                            bold: matches!(r.style, TextStyle::Bold | TextStyle::BoldItalic),
                            italic: matches!(
                                r.style,
                                TextStyle::Italic | TextStyle::BoldItalic | TextStyle::MathInline
                            ),
                        })
                        .collect(),
                };
                write_paragraph(&mut w, &para);
            }
            Block::List {
                is_ordered, items, ..
            } => {
                let style = if *is_ordered {
                    STYLE_LIST_NUMBER
                } else {
                    STYLE_LIST_BULLET
                };
                for (idx, sub) in items.iter().enumerate() {
                    let label = if *is_ordered {
                        format!("{}.", idx + 1)
                    } else {
                        "•".to_string()
                    };
                    let para = Paragraph {
                        style_id: Some(style.to_string()),
                        runs: vec![Run {
                            text: format!("{} {}", label, summarize(sub)),
                            style_id: None,
                            bold: false,
                            italic: false,
                        }],
                    };
                    write_paragraph(&mut w, &para);
                }
            }
            Block::Table { rows, caption, .. } => {
                write_table(&mut w, rows, caption.as_deref());
            }
            Block::Figure { path, caption, .. } => {
                let runs = vec![Run {
                    text: format!(
                        "[图片：{}]",
                        if path.is_empty() {
                            "（未提供）"
                        } else {
                            path
                        }
                    ),
                    style_id: None,
                    bold: false,
                    italic: true,
                }];
                let para = Paragraph {
                    style_id: Some(STYLE_BODY.to_string()),
                    runs,
                };
                write_paragraph(&mut w, &para);
                if let Some(cap) = caption {
                    let cap_para = Paragraph {
                        style_id: Some(STYLE_CAPTION.to_string()),
                        runs: vec![Run {
                            text: cap.clone(),
                            style_id: None,
                            bold: false,
                            italic: true,
                        }],
                    };
                    write_paragraph(&mut w, &cap_para);
                }
            }
            Block::Equation {
                latex, is_block, ..
            } => {
                write_equation(&mut w, latex, *is_block);
            }
            Block::Bibliography { entries } => {
                let para = Paragraph {
                    style_id: Some(STYLE_HEADING2.to_string()),
                    runs: vec![Run {
                        text: "参考文献".into(),
                        style_id: Some(STYLE_HEADING2.to_string()),
                        bold: true,
                        italic: false,
                    }],
                };
                write_paragraph(&mut w, &para);
                for e in entries {
                    let line = format!("[{}] {} ({})", e.key, e.title, e.year);
                    let para = Paragraph {
                        style_id: Some(STYLE_BODY.to_string()),
                        runs: vec![Run {
                            text: line,
                            style_id: None,
                            bold: false,
                            italic: false,
                        }],
                    };
                    write_paragraph(&mut w, &para);
                }
            }
            Block::RawFallback { text, .. } => {
                let para = Paragraph {
                    style_id: Some(STYLE_BODY.to_string()),
                    runs: vec![Run {
                        text: text.clone(),
                        style_id: None,
                        bold: false,
                        italic: false,
                    }],
                };
                write_paragraph(&mut w, &para);
            }
        }
    }

    let sect = BytesStart::new("w:sectPr");
    w.write_event(Event::Start(sect)).unwrap();
    let mut pg_sz = BytesStart::new("w:pgSz");
    pg_sz.push_attribute(("w:w", "12240"));
    pg_sz.push_attribute(("w:h", "15840"));
    w.write_event(Event::Empty(pg_sz)).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:sectPr")))
        .unwrap();

    w.write_event(Event::End(BytesEnd::new("w:body"))).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:document")))
        .unwrap();
    w.into_inner()
}

fn write_equation(w: &mut Writer<Vec<u8>>, latex: &str, is_block: bool) {
    // 1) 写一个 w:p
    w.write_event(Event::Start(BytesStart::new("w:p"))).unwrap();
    if is_block {
        let ppr = BytesStart::new("w:pPr");
        let mut jc = BytesStart::new("w:jc");
        jc.push_attribute(("w:val", "center"));
        w.write_event(Event::Start(ppr.clone())).unwrap();
        w.write_event(Event::Empty(jc)).unwrap();
        w.write_event(Event::End(BytesEnd::new("w:pPr"))).unwrap();
    }

    // 2) 解析 → OMML
    let expr = parse_latex_math(latex);
    let omml = to_omml(&expr);

    // 3) 直接把 OMML 字符串嵌入（去 BOM/声明）
    let omml_str = String::from_utf8_lossy(&omml);
    // 提取 `<m:oMath ...>...</m:oMath>` 部分
    if let Some(start) = omml_str.find("<m:oMath") {
        if let Some(end) = omml_str[start..].find("</m:oMath>") {
            let inner = &omml_str[start..start + end + "</m:oMath>".len()];
            // 用 raw bytes 写入，避免二次 XML 编码
            use std::io::Write;
            let _ = w.get_mut().write_all(inner.as_bytes());
        }
    }

    w.write_event(Event::End(BytesEnd::new("w:p"))).unwrap();
}

fn summarize(blocks: &[Block]) -> String {
    let mut out = String::new();
    for b in blocks {
        match b {
            Block::Paragraph { runs, .. } => {
                for r in runs {
                    out.push_str(&r.text);
                    out.push(' ');
                }
            }
            Block::List { items, .. } => {
                for it in items {
                    out.push_str(&summarize(it));
                }
            }
            Block::Heading { text, .. } => {
                out.push_str(text);
                out.push(' ');
            }
            _ => {}
        }
    }
    out.trim().to_string()
}

fn write_table(
    w: &mut Writer<Vec<u8>>,
    rows: &[doc_semantic_ast::TableRow],
    caption: Option<&str>,
) {
    let mut tbl = BytesStart::new("w:tbl");
    tbl.push_attribute((
        "xmlns:w",
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    ));
    w.write_event(Event::Start(tbl.clone())).unwrap();

    w.write_event(Event::Start(BytesStart::new("w:tblPr")))
        .unwrap();
    w.write_event(Event::Start(BytesStart::new("w:tblW")))
        .unwrap();
    let mut w_attr = BytesStart::new("w:w");
    w_attr.push_attribute(("w:w", "0"));
    w_attr.push_attribute(("w:type", "auto"));
    w.write_event(Event::Empty(w_attr)).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:tblW"))).unwrap();
    w.write_event(Event::Start(BytesStart::new("w:tblBorders")))
        .unwrap();
    for side in ["top", "left", "bottom", "right", "insideH", "insideV"] {
        let name = format!("w:{side}");
        let mut b = BytesStart::new(name.as_str());
        b.push_attribute(("w:val", "single"));
        b.push_attribute(("w:sz", "4"));
        b.push_attribute(("w:color", "auto"));
        w.write_event(Event::Empty(b)).unwrap();
    }
    w.write_event(Event::End(BytesEnd::new("w:tblBorders")))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("w:tblPr"))).unwrap();

    let ncols = rows.iter().map(|r| r.cells.len()).max().unwrap_or(1);
    w.write_event(Event::Start(BytesStart::new("w:tblGrid")))
        .unwrap();
    for _ in 0..ncols {
        let mut gc = BytesStart::new("w:gridCol");
        gc.push_attribute(("w:w", "2000"));
        w.write_event(Event::Empty(gc)).unwrap();
    }
    w.write_event(Event::End(BytesEnd::new("w:tblGrid")))
        .unwrap();

    for (i, row) in rows.iter().enumerate() {
        let is_header = i == 0;
        w.write_event(Event::Start(BytesStart::new("w:tr")))
            .unwrap();
        for cell in &row.cells {
            w.write_event(Event::Start(BytesStart::new("w:tc")))
                .unwrap();
            let p = Paragraph {
                style_id: if is_header {
                    Some(STYLE_TABLE_HEADER.to_string())
                } else {
                    Some(STYLE_BODY.to_string())
                },
                runs: if cell.runs.is_empty() {
                    vec![Run {
                        text: String::new(),
                        style_id: None,
                        bold: false,
                        italic: false,
                    }]
                } else {
                    cell.runs
                        .iter()
                        .map(|r| Run {
                            text: r.text.clone(),
                            style_id: None,
                            bold: false,
                            italic: false,
                        })
                        .collect()
                },
            };
            write_paragraph(w, &p);
            w.write_event(Event::End(BytesEnd::new("w:tc"))).unwrap();
        }
        w.write_event(Event::End(BytesEnd::new("w:tr"))).unwrap();
    }

    w.write_event(Event::End(BytesEnd::new("w:tbl"))).unwrap();

    if let Some(cap) = caption {
        let p = Paragraph {
            style_id: Some(STYLE_CAPTION.to_string()),
            runs: vec![Run {
                text: cap.to_string(),
                style_id: None,
                bold: false,
                italic: true,
            }],
        };
        write_paragraph(w, &p);
    }
}

fn write_paragraph(w: &mut Writer<Vec<u8>>, p: &Paragraph) {
    w.write_event(Event::Start(BytesStart::new("w:p"))).unwrap();
    if let Some(s) = &p.style_id {
        let ppr = BytesStart::new("w:pPr");
        let mut pstyle = BytesStart::new("w:pStyle");
        pstyle.push_attribute(("w:val", s.as_str()));
        w.write_event(Event::Start(ppr)).unwrap();
        w.write_event(Event::Empty(pstyle)).unwrap();
        w.write_event(Event::End(BytesEnd::new("w:pPr"))).unwrap();
    }
    for run in &p.runs {
        w.write_event(Event::Start(BytesStart::new("w:r"))).unwrap();
        if let Some(s) = &run.style_id {
            let rpr = BytesStart::new("w:rPr");
            let mut rstyle = BytesStart::new("w:rStyle");
            rstyle.push_attribute(("w:val", s.as_str()));
            w.write_event(Event::Start(rpr)).unwrap();
            w.write_event(Event::Empty(rstyle)).unwrap();
            w.write_event(Event::End(BytesEnd::new("w:rPr"))).unwrap();
        } else if run.bold || run.italic {
            let rpr = BytesStart::new("w:rPr");
            w.write_event(Event::Start(rpr)).unwrap();
            if run.bold {
                w.write_event(Event::Empty(BytesStart::new("w:b"))).unwrap();
            }
            if run.italic {
                w.write_event(Event::Empty(BytesStart::new("w:i"))).unwrap();
            }
            w.write_event(Event::End(BytesEnd::new("w:rPr"))).unwrap();
        }
        w.write_event(Event::Start(BytesStart::new("w:t"))).unwrap();
        w.write_event(Event::Text(quick_xml::events::BytesText::new(&run.text)))
            .unwrap();
        w.write_event(Event::End(BytesEnd::new("w:t"))).unwrap();
        w.write_event(Event::End(BytesEnd::new("w:r"))).unwrap();
    }
    w.write_event(Event::End(BytesEnd::new("w:p"))).unwrap();
}
