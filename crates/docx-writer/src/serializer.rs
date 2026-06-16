//! AST → OOXML 元素序列化
//!
//! 支持的块（V1）：
//! - Heading / Paragraph
//! - List（itemize / enumerate）
//! - Table（简单网格）
//! - Figure（PNG/JPEG 嵌入 via base64 inline）
//! - Equation（OMML `<m:oMath>`）
//! - Bibliography
//! - RawFallback

use doc_mathml::{parse_latex_math, to_omml};
use doc_semantic_ast::{Block, Document, TextStyle};
use doc_utils::ImageAssets;
use image::GenericImageView;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use crate::model::{Paragraph, Run};
use crate::page_setup::PageSetup;
use crate::styles::{
    STYLE_BODY, STYLE_CAPTION, STYLE_HEADING1, STYLE_HEADING2, STYLE_HEADING3, STYLE_LIST_BULLET,
    STYLE_LIST_NUMBER, STYLE_TABLE_HEADER, STYLE_TITLE,
};

/// 写出 `document.xml` 字节流。
///
/// `image_assets` 提供图片字节（来自 VFS）；若 `Block::Figure` 的路径命中，
/// 则以 OOXML inline drawing + base64 嵌入形式输出；否则回退到占位文本。
///
/// `page_setup`：Some → 写自定义 `pgSz / pgMar / cols`；None → 12240×15840 + 默认 margins。
pub fn serialize_document(
    doc: &Document,
    image_assets: Option<&ImageAssets>,
    page_setup: Option<&PageSetup>,
) -> Vec<u8> {
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
    root.push_attribute((
        "xmlns:wp",
        "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing/inline",
    ));
    root.push_attribute((
        "xmlns:a",
        "http://schemas.openxmlformats.org/drawingml/2006/main",
    ));
    root.push_attribute((
        "xmlns:pic",
        "http://schemas.openxmlformats.org/drawingml/2006/picture",
    ));
    w.write_event(Event::Start(root)).unwrap();
    w.write_event(Event::Start(BytesStart::new("w:body")))
        .unwrap();

    let mut fig_counter: u32 = 0;

    for block in &doc.blocks {
        match block {
            Block::Heading {
                level,
                text,
                number,
                ..
            } => {
                let style = match level {
                    1 => STYLE_HEADING1,
                    2 => STYLE_HEADING2,
                    3 => STYLE_HEADING3,
                    _ => STYLE_TITLE,
                };
                let display_text = match number {
                    Some(n) => format!("{} {}", n, text),
                    None => text.clone(),
                };
                let para = Paragraph {
                    style_id: Some(style.to_string()),
                    runs: vec![Run {
                        text: display_text,
                        style_id: Some(style.to_string()),
                        bold: true,
                        italic: false,
                    }],
                };
                write_paragraph(&mut w, &para);
            }
            Block::Paragraph { runs, .. } => {
                // 启发：参考文献样式 — 段落以 `[数字]` 开头时用 JOSReference
                // （悬挂缩进，西文 Times New Roman，中文宋体）。
                // 这样在 paper3 这类「顶层 \item 不在 list env 内」的
                // 场景也能拿到正确的样式。
                let is_jos_ref = runs.iter().any(|r| {
                    let t = r.text.trim_start();
                    t.starts_with('[')
                        && t.chars()
                            .nth(1)
                            .map(|c| c.is_ascii_digit())
                            .unwrap_or(false)
                });
                let para = Paragraph {
                    style_id: Some(if is_jos_ref {
                        "JOSReference".to_string()
                    } else {
                        STYLE_BODY.to_string()
                    }),
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
                // JOS 参考文献模式：item 文本含 `[N] —` 形式
                // （来自 `\item[{[N]}]`，lower_list 加了 `{` 包装）。
                // 使用 `JOSReference` 样式（悬挂缩进 + Times New Roman + SimSun）。
                let is_jos_ref = items.iter().any(|sub| {
                    let s = summarize(sub);
                    s.contains('[')
                        && s.chars()
                            .any(|c| c.is_ascii_digit())
                        && (s.contains('—') || s.contains("--"))
                });
                let style = if is_jos_ref {
                    "JOSReference"
                } else if *is_ordered {
                    STYLE_LIST_NUMBER
                } else {
                    STYLE_LIST_BULLET
                };
                for sub in items.iter() {
                    let text = summarize(sub);
                    let para = Paragraph {
                        style_id: Some(style.to_string()),
                        runs: vec![Run {
                            text,
                            style_id: None,
                            bold: false,
                            italic: false,
                        }],
                    };
                    write_paragraph(&mut w, &para);
                }
            }
            Block::Table {
                rows,
                caption,
                number,
                ..
            } => {
                write_table(&mut w, rows, caption.as_deref(), number.as_deref());
            }
            Block::Figure {
                path,
                caption,
                number,
                ..
            } => {
                fig_counter += 1;
                let fig_id = fig_counter;
                let fig_key = path.trim();

                // 尝试嵌入图片
                if let Some(assets) = image_assets {
                    if !fig_key.is_empty() {
                        // 尝试多个候选 key：
                        // 1) 原始 fig_key
                        // 2) fig_key 去掉目录前缀（basename）
                        // 3) fig_key 把 .pdf/.PDF 换成 .png（设计稿 §4.5.1 PDF→PNG fallback）
                        let basename = std::path::Path::new(fig_key)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(fig_key);
                        let png_basename = if basename.to_lowercase().ends_with(".pdf") {
                            basename.trim_end_matches(".pdf").trim_end_matches(".PDF").to_string() + ".png"
                        } else {
                            basename.to_string()
                        };
                        let bytes_opt = assets
                            .get(fig_key)
                            .or_else(|| assets.get(basename))
                            .or_else(|| assets.get(png_basename.as_str()));
                        if let Some(bytes) = bytes_opt {
                            // 探测格式
                            let ext = if bytes.len() >= 8
                                && bytes[0] == 0x89
                                && bytes[1] == b'P'
                                && bytes[2] == b'N'
                                && bytes[3] == b'G'
                            {
                                "png"
                            } else {
                                "jpg"
                            };
                            let media_name = format!("image{}.{}", fig_id, ext);

                            // 计算图片尺寸（EMU：914400 = 1 英寸）
                            let (cx, cy) = calc_image_emu(bytes, 4572000, 3429000);

                            // base64 编码
                            use base64::{engine::general_purpose::STANDARD, Engine};
                            let b64 = STANDARD.encode(bytes);

                            // 组装 inline drawing XML
                            let drawing = format!(
                                r#"<w:drawing><wp:inline dist="0"><wp:extent cx="{}" cy="{}"/><wp:docPr id="{}" name="Picture {}" descr="{}"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="{}" name="{}"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip><w:binData w:name="word/media/{}">{}</w:binData></a:blip><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing>"#,
                                cx,
                                cy,
                                fig_id,
                                fig_id,
                                xml_escape(fig_key),
                                fig_id,
                                xml_escape(&media_name),
                                xml_escape(&format!("word/media/{}", media_name)),
                                b64,
                                cx,
                                cy
                            );

                            use std::io::Write;
                            let _ = w.get_mut().write_all(drawing.as_bytes());

                            // caption 单独写一行
                            if let Some(cap) = caption {
                                let cap_text = match number {
                                    Some(n) => format!("{} {}", n, cap),
                                    None => cap.clone(),
                                };
                                let cap_para = Paragraph {
                                    style_id: Some(STYLE_CAPTION.to_string()),
                                    runs: vec![Run {
                                        text: cap_text,
                                        style_id: None,
                                        bold: false,
                                        italic: true,
                                    }],
                                };
                                write_paragraph(&mut w, &cap_para);
                            }
                            continue;
                        }
                    }
                }

                // 回退占位文本
                let runs = vec![Run {
                    text: format!(
                        "[图片：{}]",
                        if fig_key.is_empty() {
                            "（未提供）"
                        } else {
                            fig_key
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
                    let cap_text = match number {
                        Some(n) => format!("{} {}", n, cap),
                        None => cap.clone(),
                    };
                    let cap_para = Paragraph {
                        style_id: Some(STYLE_CAPTION.to_string()),
                        runs: vec![Run {
                            text: cap_text,
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
            Block::Algorithm {
                lines,
                io,
                caption,
                number,
                ..
            } => {
                // 算法块：先写 "算法 N  caption" 标题，再写 I/O + 代码行
                let cap = format!(
                    "{}  {}",
                    number.as_deref().unwrap_or("算法"),
                    caption.as_deref().unwrap_or("")
                );
                let para = Paragraph {
                    style_id: Some(STYLE_HEADING2.to_string()),
                    runs: vec![Run {
                        text: cap,
                        style_id: Some(STYLE_HEADING2.to_string()),
                        bold: true,
                        italic: false,
                    }],
                };
                write_paragraph(&mut w, &para);
                for (kind, content) in io {
                    let line = format!("{kind}: {content}");
                    let para = Paragraph {
                        style_id: Some(STYLE_BODY.to_string()),
                        runs: vec![Run {
                            text: line,
                            style_id: None,
                            bold: false,
                            italic: true,
                        }],
                    };
                    write_paragraph(&mut w, &para);
                }
                for alg in lines {
                    let indent_spaces = "  ".repeat(alg.indent as usize);
                    let comment = if alg.comment.is_empty() {
                        String::new()
                    } else {
                        format!(" /* {} */", alg.comment)
                    };
                    let code = format!("{indent_spaces}{}{comment}", alg.code);
                    let para = Paragraph {
                        style_id: Some(STYLE_BODY.to_string()),
                        runs: vec![Run {
                            text: code,
                            style_id: Some(STYLE_BODY.to_string()),
                            bold: false,
                            italic: false,
                        }],
                    };
                    write_paragraph(&mut w, &para);
                }
            }
        }
    }

    let sect = BytesStart::new("w:sectPr");
    w.write_event(Event::Start(sect)).unwrap();

    // pgSz：纸张尺寸（twips）
    let ps = page_setup.cloned().unwrap_or_default();
    let mut pg_sz = BytesStart::new("w:pgSz");
    pg_sz.push_attribute(("w:w", ps.width_twips.to_string().as_str()));
    pg_sz.push_attribute(("w:h", ps.height_twips.to_string().as_str()));
    w.write_event(Event::Empty(pg_sz)).unwrap();

    // pgMar：页边距（仅在 page_setup 显式提供时写）
    if let (Some(t), Some(r), Some(b), Some(l)) =
        (ps.margin_top, ps.margin_right, ps.margin_bottom, ps.margin_left)
    {
        let mut pg_mar = BytesStart::new("w:pgMar");
        pg_mar.push_attribute(("w:top", t.to_string().as_str()));
        pg_mar.push_attribute(("w:right", r.to_string().as_str()));
        pg_mar.push_attribute(("w:bottom", b.to_string().as_str()));
        pg_mar.push_attribute(("w:left", l.to_string().as_str()));
        pg_mar.push_attribute(("w:header", "0"));
        pg_mar.push_attribute(("w:footer", "0"));
        pg_mar.push_attribute(("w:gutter", "0"));
        w.write_event(Event::Empty(pg_mar)).unwrap();
    }

    // cols：分栏（仅在 page_setup 显式提供 num 时写）
    if let Some(num) = ps.cols_num {
        let mut cols = BytesStart::new("w:cols");
        cols.push_attribute(("w:num", num.to_string().as_str()));
        if let Some(space) = ps.cols_space {
            cols.push_attribute(("w:space", space.to_string().as_str()));
        }
        w.write_event(Event::Empty(cols)).unwrap();
    }

    w.write_event(Event::End(BytesEnd::new("w:sectPr")))
        .unwrap();

    w.write_event(Event::End(BytesEnd::new("w:body"))).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:document")))
        .unwrap();
    w.into_inner()
}

/// 从图片字节计算 EMU 尺寸（最大宽度 5 英寸 = 4572000 EMU，保持宽高比）。
fn calc_image_emu(bytes: &[u8], max_cx: u64, default_cy: u64) -> (u64, u64) {
    if let Ok(img) = image::load_from_memory(bytes) {
        let (w, h) = img.dimensions();
        let scale = if w as u64 > max_cx {
            max_cx as f64 / w as f64
        } else {
            1.0
        };
        let cx = (w as f64 * scale) as u64;
        let cy = (h as f64 * scale) as u64;
        (cx.max(1), cy.max(1))
    } else {
        (max_cx, default_cy)
    }
}

/// 最小 XML 转义（仅处理 & < > "）。
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

fn write_equation(w: &mut Writer<Vec<u8>>, latex: &str, is_block: bool) {
    w.write_event(Event::Start(BytesStart::new("w:p"))).unwrap();
    if is_block {
        let ppr = BytesStart::new("w:pPr");
        let mut jc = BytesStart::new("w:jc");
        jc.push_attribute(("w:val", "center"));
        w.write_event(Event::Start(ppr.clone())).unwrap();
        w.write_event(Event::Empty(jc)).unwrap();
        w.write_event(Event::End(BytesEnd::new("w:pPr"))).unwrap();
    }
    let expr = parse_latex_math(latex);
    let omml = to_omml(&expr);
    let omml_str = String::from_utf8_lossy(&omml);
    if let Some(start) = omml_str.find("<m:oMath") {
        if let Some(end) = omml_str[start..].find("</m:oMath>") {
            let inner = &omml_str[start..start + end + "</m:oMath>".len()];
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
    number: Option<&str>,
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

            // Cell properties
            w.write_event(Event::Start(BytesStart::new("w:tcPr")))
                .unwrap();
            // gridSpan for colspan
            if cell.colspan > 1 {
                let mut gs = BytesStart::new("w:gridSpan");
                gs.push_attribute(("w:val", cell.colspan.to_string().as_str()));
                w.write_event(Event::Empty(gs)).unwrap();
            }
            // Background color
            if let Some(ref color) = cell.bg_color {
                let mut shd = BytesStart::new("w:shd");
                shd.push_attribute(("w:val", "clear"));
                shd.push_attribute(("w:color", "auto"));
                // Normalize hex color (remove # if present)
                let fill = color.trim_start_matches('#').to_uppercase();
                shd.push_attribute(("w:fill", fill.as_str()));
                w.write_event(Event::Empty(shd)).unwrap();
            }
            w.write_event(Event::End(BytesEnd::new("w:tcPr"))).unwrap();

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
                            bold: matches!(r.style, TextStyle::Bold | TextStyle::BoldItalic),
                            italic: matches!(
                                r.style,
                                TextStyle::Italic | TextStyle::BoldItalic | TextStyle::MathInline
                            ),
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
        let cap_text = match number {
            Some(n) => format!("{} {}", n, cap),
            None => cap.to_string(),
        };
        let p = Paragraph {
            style_id: Some(STYLE_CAPTION.to_string()),
            runs: vec![Run {
                text: cap_text,
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
