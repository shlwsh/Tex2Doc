//! AST → OOXML 元素序列化（V2 重构）
//!
//! 关键变化：
//! - 序列化层现在能完整表达 `TextStyle::Bold/Italic/BoldItalic/Code/MathInline/Superscript/Subscript`
//! - 算法/代码块使用 `JOSCode` 样式 + Courier 字体
//! - 段落支持 `keep_next` / `keep_lines`（算法块、表格不跨页）
//! - 公式块走 `JOSCode` 样式 + OMML
//! - 21 个 JOS 样式由 `styles.rs` 单一来源生成

use doc_mathml::{parse_latex_math, to_omml};
use doc_semantic_ast::{AlgLine, Block, Document, TextRun, TextStyle};
use doc_utils::ImageAssets;
use image::GenericImageView;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use crate::model::{Paragraph, Run};
use crate::page_setup::PageSetup;
use crate::styles::{
    STYLE_ABSTRACT_EN, STYLE_ABSTRACT_ZH, STYLE_AUTHOR_ZH, STYLE_BODY, STYLE_BODY_NO_INDENT,
    STYLE_CAPTION, STYLE_CITATION, STYLE_CODE, STYLE_ENGLISH_TITLE, STYLE_HEADING1, STYLE_HEADING2,
    STYLE_HEADING3, STYLE_IMAGE, STYLE_INSTITUTE_ZH, STYLE_KEYWORDS, STYLE_LIST_BULLET,
    STYLE_LIST_NUMBER, STYLE_REFERENCE, STYLE_REFERENCE_HEADING, STYLE_TABLE_TEXT, STYLE_TITLE_ZH,
};

/// 一张已嵌入的图片：packer 阶段把它写到 `word/media/imageN.<ext>` 并生成 rel。
///
/// `fig_id` 来自 serializer 内的 fig 计数器（与 `r:embed` 编号对齐）。
/// `ext` 是不带点的小写扩展名（png / jpg / jpeg）。
/// `bytes` 是原始文件内容。
#[derive(Debug, Clone)]
pub struct EmbeddedImage {
    pub fig_id: u32,
    pub ext: String,
    pub bytes: Vec<u8>,
}

/// 把 semantic-ast 的 `TextRun` 映射到 docx-writer 的 `Run`。
///
/// 规则：
/// - `TextStyle::Plain` → Plain run（仍可被 bold/italic 字段 override）
/// - `Bold / BoldItalic` → 对应 bold 字段
/// - `Italic / MathInline` → italic 字段（MathInline 在 serializer 里走 OMML）
/// - `Code` → 用 Courier New 字体 + 9pt
/// - `Superscript / Subscript` → 由 serializer 输出 `<w:vertAlign>`
fn from_text_run(r: &TextRun) -> Run {
    Run {
        text: r.text.clone(),
        style_id: None,
        style: r.style,
        bold: matches!(r.style, TextStyle::Bold | TextStyle::BoldItalic),
        italic: matches!(
            r.style,
            TextStyle::Italic | TextStyle::BoldItalic | TextStyle::MathInline
        ),
        font_ascii: None,
        font_east: None,
    }
}

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
    embedded_images: &mut Vec<EmbeddedImage>,
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
        "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
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
    let mut equation_counter: u32 = 0;

    // V2：先写 front matter（中文标题 / 作者 / 单位 / 摘要 / 关键词 / 引用 / 英文标题块）
    write_front_matter(&mut w, &doc.metadata);

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
                    _ => STYLE_HEADING1,
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
                        style: TextStyle::Bold,
                        bold: true,
                        italic: false,
                        font_ascii: None,
                        font_east: None,
                    }],
                    jc: None,
                    keep_next: false,
                    keep_lines: false,
                };
                write_paragraph(&mut w, &para);
            }
            Block::Paragraph { runs, .. } => {
                let paragraph_text = runs.iter().map(|r| r.text.as_str()).collect::<String>();
                let paragraph_text = paragraph_text.trim();
                if matches!(paragraph_text, "{" | "}") {
                    continue;
                }
                // 启发：参考文献样式 — 段落以 `[数字]` 开头时用 JOSReference
                // （悬挂缩进，西文 Times New Roman，中文宋体）。
                // 这样在 paper3 这类「顶层 \item 不在 list env 内」的
                // 场景也能拿到正确的样式。
                let is_jos_ref = paragraph_starts_with_reference_marker(runs);
                let is_jos_ref_heading = matches!(
                    paragraph_text.trim_end_matches(':'),
                    "References" | "附中文参考文献" | "作者简介"
                );
                let para = Paragraph {
                    style_id: Some(if is_jos_ref_heading {
                        STYLE_REFERENCE_HEADING.to_string()
                    } else if is_jos_ref {
                        STYLE_REFERENCE.to_string()
                    } else {
                        STYLE_BODY.to_string()
                    }),
                    runs: runs.iter().map(from_text_run).collect(),
                    jc: None,
                    keep_next: false,
                    keep_lines: false,
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
                        && s.chars().any(|c| c.is_ascii_digit())
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
                        runs: vec![Run::plain(text)],
                        jc: None,
                        keep_next: false,
                        keep_lines: false,
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
                            basename
                                .trim_end_matches(".pdf")
                                .trim_end_matches(".PDF")
                                .to_string()
                                + ".png"
                        } else {
                            basename.to_string()
                        };
                        let bytes_opt = assets
                            .get(fig_key)
                            .or_else(|| assets.get(basename))
                            .or_else(|| assets.get(png_basename.as_str()));
                        if let Some(bytes) = bytes_opt {
                            // 将图片统一转换为 JPEG（RGBA 先转 RGB）再嵌入；
                            // JPEG 在 OOXML 中兼容性更好（soffice/LibreOffice 渲染更稳定）。
                            let rgb_jpg = {
                                use image::{ImageReader, RgbImage};
                                if let Ok(img) = ImageReader::new(std::io::Cursor::new(bytes))
                                    .with_guessed_format()
                                    .map(|r| r.decode())
                                {
                                    if let Ok(img) = img {
                                        let rgb: RgbImage = img.to_rgb8();
                                        let mut buf = std::io::Cursor::new(Vec::new());
                                        if rgb.write_to(&mut buf, image::ImageFormat::Jpeg).is_ok()
                                        {
                                            Some(buf.into_inner())
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            };
                            let (ext, final_bytes) = if let Some(jpg) = rgb_jpg {
                                ("jpg".to_string(), jpg)
                            } else {
                                // 回退原格式（PNG）
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
                                (ext.to_string(), bytes.to_vec())
                            };
                            let media_name = format!("image{}.{}", fig_id, ext);

                            // 计算图片尺寸（EMU：914400 = 1 英寸）
                            let (cx, cy) = calc_image_emu(&final_bytes, 4572000, 3429000);

                            // 记录到 embedded_images 让 packer 把字节写入 word/media/ 并生成 rel
                            embedded_images.push(EmbeddedImage {
                                fig_id,
                                ext,
                                bytes: final_bytes,
                            });

                            // 组装 inline drawing XML（使用 r:embed 引用 rIdImg{fig_id}）
                            // 关键修复：
                            // 1. <wp:cNvGraphicFramePr> + <a:graphicFrameLocks> 是 soffice 正确渲染的必要条件
                            // 2. xmlns:a / xmlns:pic 必须声明在 wp:inline 上（即使 w:document 已声明，
                            //    soffice 24.x 严格按「首次声明优先」处理 inline XML 片段）
                            let drawing = format!(
                                r#"<w:drawing><wp:inline xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture" distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{}" cy="{}"/><wp:docPr id="{}" name="Picture {}" descr="{}"/><wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect="1"/></wp:cNvGraphicFramePr><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="{}" name="{}"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rIdImg{}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="rect"/></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing>"#,
                                cx,
                                cy,
                                fig_id,
                                fig_id,
                                xml_escape(fig_key),
                                fig_id,
                                xml_escape(&media_name),
                                fig_id,
                                cx,
                                cy
                            );

                            write_drawing_paragraph(&mut w, &drawing);

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
                                        style: TextStyle::Plain,
                                        bold: false,
                                        italic: true,
                                        font_ascii: None,
                                        font_east: None,
                                    }],
                                    jc: None,
                                    keep_next: false,
                                    keep_lines: true,
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
                    style: TextStyle::Plain,
                    bold: false,
                    italic: true,
                    font_ascii: None,
                    font_east: None,
                }];
                let para = Paragraph {
                    style_id: Some(STYLE_BODY_NO_INDENT.to_string()),
                    runs,
                    jc: None,
                    keep_next: false,
                    keep_lines: false,
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
                            style: TextStyle::Plain,
                            bold: false,
                            italic: true,
                            font_ascii: None,
                            font_east: None,
                        }],
                        jc: None,
                        keep_next: false,
                        keep_lines: false,
                    };
                    write_paragraph(&mut w, &cap_para);
                }
            }
            Block::Equation {
                latex, is_block, ..
            } => {
                if *is_block {
                    equation_counter += 1;
                }
                write_equation(&mut w, latex, *is_block, equation_counter);
            }
            Block::TheoremLike {
                kind, title, body, ..
            } => {
                let title_suffix = title
                    .as_ref()
                    .map(|title| format!("（{title}）"))
                    .unwrap_or_default();
                let para = Paragraph {
                    style_id: Some(STYLE_BODY_NO_INDENT.to_string()),
                    runs: vec![
                        Run {
                            text: format!("{}{} ", kind.display_name(), title_suffix),
                            style_id: None,
                            style: TextStyle::Bold,
                            bold: true,
                            italic: false,
                            font_ascii: None,
                            font_east: None,
                        },
                        Run::plain(body.clone()),
                    ],
                    jc: None,
                    keep_next: false,
                    keep_lines: false,
                };
                write_paragraph(&mut w, &para);
            }
            Block::Bibliography { entries } => {
                let para = Paragraph {
                    style_id: Some(STYLE_HEADING2.to_string()),
                    runs: vec![Run {
                        text: "参考文献".into(),
                        style_id: Some(STYLE_HEADING2.to_string()),
                        style: TextStyle::Bold,
                        bold: true,
                        italic: false,
                        font_ascii: None,
                        font_east: None,
                    }],
                    jc: None,
                    keep_next: false,
                    keep_lines: false,
                };
                write_paragraph(&mut w, &para);
                for e in entries {
                    let line = format!("[{}] {} ({})", e.key, e.title, e.year);
                    let para = Paragraph {
                        style_id: Some(STYLE_BODY_NO_INDENT.to_string()),
                        runs: vec![Run::plain(line)],
                        jc: None,
                        keep_next: false,
                        keep_lines: false,
                    };
                    write_paragraph(&mut w, &para);
                }
            }
            Block::RawFallback { text, .. } => {
                // V2：空 RawFallback（来自 rjabstract/rjkeywords 等已提取到 metadata 的
                //     front matter 容器）直接跳过，不写出空段落。
                if text.is_empty() {
                    // 跳过
                } else {
                    let para = Paragraph {
                        style_id: Some(STYLE_BODY_NO_INDENT.to_string()),
                        runs: vec![Run {
                            text: text.clone(),
                            style_id: None,
                            style: TextStyle::Plain,
                            bold: false,
                            italic: false,
                            font_ascii: None,
                            font_east: None,
                        }],
                        jc: None,
                        keep_next: false,
                        keep_lines: false,
                    };
                    write_paragraph(&mut w, &para);
                }
            }
            Block::Algorithm {
                lines,
                io,
                caption,
                number,
                ..
            } => {
                write_algorithm_table(&mut w, lines, io, caption.as_deref(), number.as_deref());
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

    // pgMar：仅在 page_setup 显式提供时写
    if ps.margin_top.is_some()
        || ps.margin_right.is_some()
        || ps.margin_bottom.is_some()
        || ps.margin_left.is_some()
        || ps.margin_header.is_some()
        || ps.margin_footer.is_some()
    {
        let mut pg_mar = BytesStart::new("w:pgMar");
        if let Some(t) = ps.margin_top {
            pg_mar.push_attribute(("w:top", t.to_string().as_str()));
        }
        if let Some(r) = ps.margin_right {
            pg_mar.push_attribute(("w:right", r.to_string().as_str()));
        }
        if let Some(b) = ps.margin_bottom {
            pg_mar.push_attribute(("w:bottom", b.to_string().as_str()));
        }
        if let Some(l) = ps.margin_left {
            pg_mar.push_attribute(("w:left", l.to_string().as_str()));
        }
        if let Some(h) = ps.margin_header {
            pg_mar.push_attribute(("w:header", h.to_string().as_str()));
        }
        if let Some(f) = ps.margin_footer {
            pg_mar.push_attribute(("w:footer", f.to_string().as_str()));
        }
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

fn paragraph_starts_with_reference_marker(runs: &[TextRun]) -> bool {
    let Some(first) = runs.iter().find(|r| !r.text.trim().is_empty()) else {
        return false;
    };
    let mut chars = first.text.trim_start().chars();
    matches!(chars.next(), Some('[')) && matches!(chars.next(), Some(c) if c.is_ascii_digit())
}

/// 从图片字节计算 EMU 尺寸（最大宽度 5 英寸 = 4572000 EMU，保持宽高比）。
/// 计算图片在 docx 内显示的 EMU 尺寸（914400 EMU = 1 英寸）。
///
/// `bytes`：图片原始字节（PNG / JPEG）。
/// `max_cx_emu`：水平方向最大允许 EMU（默认 4572000 = 5 英寸）。
/// `default_cy_emu`：解析失败时的回退高度 EMU。
///
/// 算法：以 96 DPI 为基准（OOXML 渲染常用），将原始像素换算成 EMU。
/// 等比缩放保证 `cx <= max_cx_emu`。
fn calc_image_emu(bytes: &[u8], max_cx_emu: u64, default_cy_emu: u64) -> (u64, u64) {
    const EMU_PER_INCH: u64 = 914_400;
    const ASSUMED_DPI: u32 = 96;
    if let Ok(img) = image::load_from_memory(bytes) {
        let (w, h) = img.dimensions();
        if w == 0 || h == 0 {
            return (max_cx_emu, default_cy_emu);
        }
        // 像素 → 96DPI EMU
        let cx_emu = (w as u64) * EMU_PER_INCH / ASSUMED_DPI as u64;
        let scale = if cx_emu > max_cx_emu {
            max_cx_emu as f64 / cx_emu as f64
        } else {
            1.0
        };
        let cx = ((cx_emu as f64) * scale) as u64;
        let cy = ((cx_emu as f64) * scale * (h as f64 / w as f64)) as u64;
        (cx.max(1), cy.max(1))
    } else {
        (max_cx_emu, default_cy_emu)
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

fn write_drawing_paragraph(w: &mut Writer<Vec<u8>>, drawing: &str) {
    use std::io::Write;

    let prefix = format!(
        r#"<w:p><w:pPr><w:pStyle w:val="{}"/><w:jc w:val="center"/><w:keepNext/><w:keepLines/></w:pPr><w:r>"#,
        STYLE_IMAGE
    );
    let _ = w.get_mut().write_all(prefix.as_bytes());
    let _ = w.get_mut().write_all(drawing.as_bytes());
    let _ = w.get_mut().write_all(b"</w:r></w:p>");
}

fn write_equation(w: &mut Writer<Vec<u8>>, latex: &str, is_block: bool, number: u32) {
    if is_block {
        let mut runs = formula_runs(latex);
        runs.push(Run {
            text: format!("({number})"),
            style_id: None,
            style: TextStyle::Plain,
            bold: false,
            italic: false,
            font_ascii: None,
            font_east: None,
        });
        let para = Paragraph {
            style_id: Some(STYLE_CODE.to_string()),
            runs,
            jc: Some("center".to_string()),
            keep_next: false,
            keep_lines: true,
        };
        write_paragraph(w, &para);
    }

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

fn formula_runs(latex: &str) -> Vec<Run> {
    let cleaned = clean_formula_latex(latex);
    let mut runs = Vec::new();
    let bytes = cleaned.as_bytes();
    let mut plain = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let ch = cleaned[i..].chars().next().unwrap_or_default();
        if ch == '_' || ch == '^' {
            if !plain.is_empty() {
                runs.push(Run::plain(std::mem::take(&mut plain)));
            }
            let (arg, next) = read_script_arg(&cleaned, i + ch.len_utf8());
            let text = clean_formula_latex(&arg);
            runs.push(Run {
                text,
                style_id: None,
                style: if ch == '_' {
                    TextStyle::Subscript
                } else {
                    TextStyle::Superscript
                },
                bold: false,
                italic: false,
                font_ascii: None,
                font_east: None,
            });
            i = next;
            continue;
        }
        plain.push(ch);
        i += ch.len_utf8();
    }
    if !plain.trim().is_empty() {
        runs.push(Run::plain(plain));
    }
    runs
}

fn read_script_arg(text: &str, start: usize) -> (String, usize) {
    let bytes = text.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'{' {
        let mut depth = 0usize;
        for (off, ch) in text[i..].char_indices() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let end = i + off;
                    return (text[i + 1..end].to_string(), end + 1);
                }
            }
        }
    }
    let mut end = i;
    while end < bytes.len() {
        let ch = text[end..].chars().next().unwrap_or_default();
        if ch.is_alphanumeric() || ch == '\\' {
            end += ch.len_utf8();
            if ch == '\\' {
                while end < bytes.len() {
                    let c = text[end..].chars().next().unwrap_or_default();
                    if !c.is_alphabetic() {
                        break;
                    }
                    end += c.len_utf8();
                }
            }
            break;
        }
        break;
    }
    (text[i..end].to_string(), end.max(i + 1))
}

fn clean_formula_latex(latex: &str) -> String {
    let mut s = latex.to_string();
    s = strip_latex_command_arg(&s, "label");
    for cmd in ["mathrm", "mathcal", "mathbb", "operatorname"] {
        s = unwrap_latex_command_arg(&s, cmd);
    }
    for (from, to) in [
        ("\\alpha", "α"),
        ("\\beta", "β"),
        ("\\gamma", "γ"),
        ("\\delta", "δ"),
        ("\\lambda", "λ"),
        ("\\rho", "ρ"),
        ("\\xi", "ξ"),
        ("\\sum", "Σ"),
        ("\\min", "min"),
        ("\\max", "max"),
        ("\\exp", "exp"),
        ("\\cdot", "·"),
        ("\\,", " "),
        ("\\!", ""),
        ("\\bigl", ""),
        ("\\bigr", ""),
        ("\\left", ""),
        ("\\right", ""),
    ] {
        s = s.replace(from, to);
    }
    s.replace(['{', '}'], "")
}

fn strip_latex_command_arg(text: &str, command: &str) -> String {
    replace_latex_command_arg(text, command, false)
}

fn unwrap_latex_command_arg(text: &str, command: &str) -> String {
    replace_latex_command_arg(text, command, true)
}

fn replace_latex_command_arg(text: &str, command: &str, keep_inner: bool) -> String {
    let token = format!("\\{command}");
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if text[i..].starts_with(&token) {
            let mut p = i + token.len();
            while p < text.len() && text.as_bytes()[p].is_ascii_whitespace() {
                p += 1;
            }
            if p < text.len() && text.as_bytes()[p] == b'{' {
                let (arg, next) = read_script_arg(text, p);
                if keep_inner {
                    out.push_str(&arg);
                }
                i = next;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap_or_default();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
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
            Block::TheoremLike {
                kind, title, body, ..
            } => {
                out.push_str(kind.display_name());
                if let Some(title) = title {
                    out.push_str(title);
                }
                out.push_str(body);
                out.push(' ');
            }
            _ => {}
        }
    }
    out.trim().to_string()
}

fn write_algorithm_table(
    w: &mut Writer<Vec<u8>>,
    lines: &[AlgLine],
    io: &[(String, String)],
    caption: Option<&str>,
    number: Option<&str>,
) {
    let cap = match (number, caption) {
        (Some(n), Some(c)) if !c.is_empty() => format!("{n}: {c}"),
        (Some(n), _) => n.to_string(),
        (_, Some(c)) => format!("Algorithm: {c}"),
        _ => "Algorithm".to_string(),
    };
    let has_input = io
        .iter()
        .any(|(kind, _)| normalize_io_label(kind) == "Input");
    let has_output = io
        .iter()
        .any(|(kind, _)| normalize_io_label(kind) == "Output");
    let para = Paragraph {
        style_id: Some(STYLE_CAPTION.to_string()),
        runs: vec![Run {
            text: cap.clone(),
            style_id: None,
            style: TextStyle::Bold,
            bold: true,
            italic: false,
            font_ascii: None,
            font_east: None,
        }],
        jc: Some("center".into()),
        keep_next: true,
        keep_lines: true,
    };
    write_paragraph(w, &para);

    w.write_event(Event::Start(BytesStart::new("w:tbl")))
        .unwrap();
    w.write_event(Event::Start(BytesStart::new("w:tblPr")))
        .unwrap();
    let mut tbl_w = BytesStart::new("w:tblW");
    tbl_w.push_attribute(("w:w", "9000"));
    tbl_w.push_attribute(("w:type", "dxa"));
    w.write_event(Event::Empty(tbl_w)).unwrap();
    w.write_event(Event::Start(BytesStart::new("w:tblBorders")))
        .unwrap();
    for side in ["top", "bottom"] {
        let name = format!("w:{side}");
        let mut b = BytesStart::new(name.as_str());
        b.push_attribute(("w:val", "single"));
        b.push_attribute(("w:sz", "8"));
        b.push_attribute(("w:space", "0"));
        b.push_attribute(("w:color", "000000"));
        w.write_event(Event::Empty(b)).unwrap();
    }
    for side in ["left", "right", "insideH", "insideV"] {
        let name = format!("w:{side}");
        let mut b = BytesStart::new(name.as_str());
        b.push_attribute(("w:val", "nil"));
        w.write_event(Event::Empty(b)).unwrap();
    }
    w.write_event(Event::End(BytesEnd::new("w:tblBorders")))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("w:tblPr"))).unwrap();

    w.write_event(Event::Start(BytesStart::new("w:tblGrid")))
        .unwrap();
    for col_w in ["650", "6350", "2000"] {
        let mut grid_col = BytesStart::new("w:gridCol");
        grid_col.push_attribute(("w:w", col_w));
        w.write_event(Event::Empty(grid_col)).unwrap();
    }
    w.write_event(Event::End(BytesEnd::new("w:tblGrid")))
        .unwrap();

    write_algorithm_caption_row(w, &cap);
    for (kind, content) in io {
        let code = format!("{}: {content}", normalize_io_label(kind));
        write_algorithm_row(w, "", &code, "", false);
    }
    if !has_input {
        write_algorithm_row(w, "", "Input:", "", false);
    }
    if !has_output {
        write_algorithm_row(w, "", "Output:", "", false);
    }
    for (idx, line) in lines.iter().enumerate() {
        let line_no = (idx + 1).to_string();
        let code = algorithm_code_text(line);
        let comment = if line.comment.trim().is_empty() {
            String::new()
        } else {
            format!("// {}", line.comment.trim())
        };
        write_algorithm_row(w, &line_no, &code, &comment, false);
    }

    w.write_event(Event::End(BytesEnd::new("w:tbl"))).unwrap();
}

fn write_algorithm_caption_row(w: &mut Writer<Vec<u8>>, caption: &str) {
    w.write_event(Event::Start(BytesStart::new("w:tr")))
        .unwrap();
    w.write_event(Event::Start(BytesStart::new("w:trPr")))
        .unwrap();
    w.write_event(Event::Empty(BytesStart::new("w:cantSplit")))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("w:trPr"))).unwrap();
    w.write_event(Event::Start(BytesStart::new("w:tc")))
        .unwrap();
    w.write_event(Event::Start(BytesStart::new("w:tcPr")))
        .unwrap();
    let mut grid_span = BytesStart::new("w:gridSpan");
    grid_span.push_attribute(("w:val", "3"));
    w.write_event(Event::Empty(grid_span)).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:tcPr"))).unwrap();
    let para = Paragraph {
        style_id: Some(STYLE_CODE.to_string()),
        runs: vec![Run {
            text: caption.to_string(),
            style_id: None,
            style: TextStyle::Code,
            bold: true,
            italic: false,
            font_ascii: None,
            font_east: None,
        }],
        jc: Some("center".to_string()),
        keep_next: false,
        keep_lines: true,
    };
    write_paragraph(w, &para);
    w.write_event(Event::End(BytesEnd::new("w:tc"))).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:tr"))).unwrap();
}

fn normalize_io_label(kind: &str) -> &'static str {
    let lower = kind.trim().to_ascii_lowercase();
    if lower.contains("out") || lower.contains("输出") {
        "Output"
    } else {
        "Input"
    }
}

fn algorithm_code_text(line: &AlgLine) -> String {
    let guide_slots = line
        .guides
        .iter()
        .chain(line.end_guides.iter())
        .copied()
        .max()
        .map(|level| level.saturating_add(1))
        .unwrap_or(line.indent);
    let mut out = String::new();
    for level in 0..guide_slots {
        if line.guides.contains(&level) || line.end_guides.contains(&level) {
            out.push_str("| ");
        } else {
            out.push_str("  ");
        }
    }
    if guide_slots < line.indent {
        out.push_str(&"  ".repeat((line.indent - guide_slots) as usize));
    }
    out.push_str(line.code.trim());
    out
}

fn write_algorithm_row(
    w: &mut Writer<Vec<u8>>,
    line_no: &str,
    code: &str,
    comment: &str,
    is_header: bool,
) {
    w.write_event(Event::Start(BytesStart::new("w:tr")))
        .unwrap();
    w.write_event(Event::Start(BytesStart::new("w:trPr")))
        .unwrap();
    w.write_event(Event::Empty(BytesStart::new("w:cantSplit")))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("w:trPr"))).unwrap();
    write_algorithm_cell(w, line_no, "650", Some("center"), is_header);
    write_algorithm_cell(w, code, "6350", None, is_header);
    write_algorithm_cell(w, comment, "2000", None, is_header);
    w.write_event(Event::End(BytesEnd::new("w:tr"))).unwrap();
}

fn write_algorithm_cell(
    w: &mut Writer<Vec<u8>>,
    text: &str,
    width: &str,
    jc: Option<&str>,
    bold: bool,
) {
    w.write_event(Event::Start(BytesStart::new("w:tc")))
        .unwrap();
    w.write_event(Event::Start(BytesStart::new("w:tcPr")))
        .unwrap();
    let mut tc_w = BytesStart::new("w:tcW");
    tc_w.push_attribute(("w:w", width));
    tc_w.push_attribute(("w:type", "dxa"));
    w.write_event(Event::Empty(tc_w)).unwrap();
    let mut valign = BytesStart::new("w:vAlign");
    valign.push_attribute(("w:val", "top"));
    w.write_event(Event::Empty(valign)).unwrap();
    w.write_event(Event::Start(BytesStart::new("w:tcBorders")))
        .unwrap();
    for side in ["left", "bottom"] {
        let name = format!("w:{side}");
        let mut border = BytesStart::new(name.as_str());
        border.push_attribute(("w:val", "single"));
        border.push_attribute(("w:sz", "4"));
        border.push_attribute(("w:space", "0"));
        border.push_attribute(("w:color", "000000"));
        w.write_event(Event::Empty(border)).unwrap();
    }
    w.write_event(Event::End(BytesEnd::new("w:tcBorders")))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("w:tcPr"))).unwrap();

    let para = Paragraph {
        style_id: Some(STYLE_CODE.to_string()),
        runs: vec![Run {
            text: text.to_string(),
            style_id: None,
            style: TextStyle::Code,
            bold,
            italic: false,
            font_ascii: None,
            font_east: None,
        }],
        jc: jc.map(str::to_string),
        keep_next: false,
        keep_lines: true,
    };
    write_paragraph(w, &para);
    w.write_event(Event::End(BytesEnd::new("w:tc"))).unwrap();
}

fn write_table(
    w: &mut Writer<Vec<u8>>,
    rows: &[doc_semantic_ast::TableRow],
    caption: Option<&str>,
    number: Option<&str>,
) {
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
                style: TextStyle::Plain,
                bold: false,
                italic: true,
                font_ascii: None,
                font_east: None,
            }],
            jc: Some("center".to_string()),
            keep_next: true,
            keep_lines: true,
        };
        write_paragraph(w, &p);
    }

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
    let total_w: i64 = 9000i64; // 6.25 inches — 适合 A4 双栏
    let mut w_attr = BytesStart::new("w:w");
    w_attr.push_attribute(("w:w", total_w.to_string().as_str()));
    w_attr.push_attribute(("w:type", "dxa"));
    w.write_event(Event::Empty(w_attr)).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:tblW"))).unwrap();
    w.write_event(Event::Start(BytesStart::new("w:tblBorders")))
        .unwrap();
    for side in ["top", "bottom", "insideH", "insideV"] {
        let name = format!("w:{side}");
        let mut b = BytesStart::new(name.as_str());
        b.push_attribute(("w:val", "single"));
        b.push_attribute(("w:sz", "4"));
        b.push_attribute(("w:color", "auto"));
        w.write_event(Event::Empty(b)).unwrap();
    }
    for side in ["left", "right"] {
        let name = format!("w:{side}");
        let mut b = BytesStart::new(name.as_str());
        b.push_attribute(("w:val", "nil"));
        w.write_event(Event::Empty(b)).unwrap();
    }
    w.write_event(Event::End(BytesEnd::new("w:tblBorders")))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("w:tblPr"))).unwrap();

    let ncols = rows.iter().map(|r| r.cells.len()).max().unwrap_or(1);
    w.write_event(Event::Start(BytesStart::new("w:tblGrid")))
        .unwrap();
    // V2：均分表格宽度（避免某些 cell 被挤到 1 字符宽导致 wrap）
    let col_w: i64 = total_w / ncols.max(1) as i64;
    let col_w_str = col_w.to_string();
    for _ in 0..ncols {
        let mut gc = BytesStart::new("w:gridCol");
        gc.push_attribute(("w:w", col_w_str.as_str()));
        w.write_event(Event::Empty(gc)).unwrap();
    }
    w.write_event(Event::End(BytesEnd::new("w:tblGrid")))
        .unwrap();

    for (i, row) in rows.iter().enumerate() {
        let is_header = i == 0;
        w.write_event(Event::Start(BytesStart::new("w:tr")))
            .unwrap();
        w.write_event(Event::Start(BytesStart::new("w:trPr")))
            .unwrap();
        w.write_event(Event::Empty(BytesStart::new("w:cantSplit")))
            .unwrap();
        w.write_event(Event::End(BytesEnd::new("w:trPr"))).unwrap();
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
                style_id: Some(if is_header {
                    STYLE_TABLE_TEXT.to_string()
                } else {
                    STYLE_TABLE_TEXT.to_string()
                }),
                runs: if cell.runs.is_empty() {
                    vec![Run::plain(String::new())]
                } else {
                    // 表格首行粗体（表头）；其余按 text style 走
                    cell.runs
                        .iter()
                        .map(|r| {
                            let mut run = from_text_run(r);
                            run.style_id = Some(STYLE_TABLE_TEXT.to_string());
                            run.font_ascii = Some("Times New Roman".to_string());
                            run.font_east = Some("宋体".to_string());
                            run.bold = is_header;
                            run.italic = false;
                            run
                        })
                        .collect()
                },
                jc: None,
                keep_next: false,
                keep_lines: false,
            };
            write_paragraph(w, &p);
            w.write_event(Event::End(BytesEnd::new("w:tc"))).unwrap();
        }
        w.write_event(Event::End(BytesEnd::new("w:tr"))).unwrap();
    }

    w.write_event(Event::End(BytesEnd::new("w:tbl"))).unwrap();
}

fn write_paragraph(w: &mut Writer<Vec<u8>>, p: &Paragraph) {
    w.write_event(Event::Start(BytesStart::new("w:p"))).unwrap();

    // pPr：style / jc / keepNext / keepLines
    let need_ppr = p.style_id.is_some() || p.jc.is_some() || p.keep_next || p.keep_lines;
    if need_ppr {
        w.write_event(Event::Start(BytesStart::new("w:pPr")))
            .unwrap();
        if let Some(s) = &p.style_id {
            let mut pstyle = BytesStart::new("w:pStyle");
            pstyle.push_attribute(("w:val", s.as_str()));
            w.write_event(Event::Empty(pstyle)).unwrap();
        }
        if let Some(j) = &p.jc {
            let mut jc = BytesStart::new("w:jc");
            jc.push_attribute(("w:val", j.as_str()));
            w.write_event(Event::Empty(jc)).unwrap();
        }
        if p.keep_next {
            w.write_event(Event::Empty(BytesStart::new("w:keepNext")))
                .unwrap();
        }
        if p.keep_lines {
            w.write_event(Event::Empty(BytesStart::new("w:keepLines")))
                .unwrap();
        }
        w.write_event(Event::End(BytesEnd::new("w:pPr"))).unwrap();
    }

    for run in &p.runs {
        write_run(w, run);
    }
    w.write_event(Event::End(BytesEnd::new("w:p"))).unwrap();
}

/// 写单个 `<w:r>`，根据 `Run.style` 产生正确的 `<w:rPr>`。
fn write_run(w: &mut Writer<Vec<u8>>, run: &Run) {
    // 空 run 跳过（避免空 text 触发 Word 警告）
    if run.text.is_empty() && run.style == TextStyle::Plain {
        return;
    }

    w.write_event(Event::Start(BytesStart::new("w:r"))).unwrap();

    // rPr 是否非空
    let has_rpr = run.style_id.is_some()
        || run.bold
        || run.italic
        || run.font_ascii.is_some()
        || run.font_east.is_some()
        || !matches!(run.style, TextStyle::Plain);
    if has_rpr {
        w.write_event(Event::Start(BytesStart::new("w:rPr")))
            .unwrap();
        if let Some(s) = &run.style_id {
            let mut rstyle = BytesStart::new("w:rStyle");
            rstyle.push_attribute(("w:val", s.as_str()));
            w.write_event(Event::Empty(rstyle)).unwrap();
        }
        if run.font_ascii.is_some() || run.font_east.is_some() {
            let mut rfonts = BytesStart::new("w:rFonts");
            if let Some(a) = &run.font_ascii {
                rfonts.push_attribute(("w:ascii", a.as_str()));
                rfonts.push_attribute(("w:hAnsi", a.as_str()));
                rfonts.push_attribute(("w:cs", a.as_str()));
            }
            if let Some(e) = &run.font_east {
                rfonts.push_attribute(("w:eastAsia", e.as_str()));
            }
            w.write_event(Event::Empty(rfonts)).unwrap();
        } else {
            // Code 样式自动用 Courier New
            if matches!(run.style, TextStyle::Code) {
                let mut rfonts = BytesStart::new("w:rFonts");
                rfonts.push_attribute(("w:ascii", "Courier New"));
                rfonts.push_attribute(("w:hAnsi", "Courier New"));
                rfonts.push_attribute(("w:eastAsia", "宋体"));
                rfonts.push_attribute(("w:cs", "Courier New"));
                w.write_event(Event::Empty(rfonts)).unwrap();
            }
        }
        if run.bold {
            w.write_event(Event::Empty(BytesStart::new("w:b"))).unwrap();
            w.write_event(Event::Empty(BytesStart::new("w:bCs")))
                .unwrap();
        }
        if run.italic {
            w.write_event(Event::Empty(BytesStart::new("w:i"))).unwrap();
            w.write_event(Event::Empty(BytesStart::new("w:iCs")))
                .unwrap();
        }
        if matches!(run.style, TextStyle::Superscript) {
            let mut va = BytesStart::new("w:vertAlign");
            va.push_attribute(("w:val", "superscript"));
            w.write_event(Event::Empty(va)).unwrap();
        } else if matches!(run.style, TextStyle::Subscript) {
            let mut va = BytesStart::new("w:vertAlign");
            va.push_attribute(("w:val", "subscript"));
            w.write_event(Event::Empty(va)).unwrap();
        }
        if run.style_id.as_deref() == Some(STYLE_TABLE_TEXT) {
            let mut sz = BytesStart::new("w:sz");
            sz.push_attribute(("w:val", "15"));
            w.write_event(Event::Empty(sz)).unwrap();
            let mut sz_cs = BytesStart::new("w:szCs");
            sz_cs.push_attribute(("w:val", "15"));
            w.write_event(Event::Empty(sz_cs)).unwrap();
        }
        w.write_event(Event::End(BytesEnd::new("w:rPr"))).unwrap();
    }

    // text：quick-xml owns XML escaping for text nodes. Pre-escaping here
    // would serialize visible "&lt;" as "&amp;lt;" in Word.
    w.write_event(Event::Start(BytesStart::new("w:t"))).unwrap();
    let mut t = BytesStart::new("w:t");
    t.push_attribute(("xml:space", "preserve"));
    // quick-xml 的 BytesText + Start 组合用 Text 事件
    let _ = t; // 不需要单独的 start 标记
    w.write_event(Event::Text(quick_xml::events::BytesText::new(&run.text)))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("w:t"))).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:r"))).unwrap();
}

fn split_first_http_url(text: &str) -> (&str, Option<&str>) {
    let Some(start) = text.find("http://").or_else(|| text.find("https://")) else {
        return (text, None);
    };
    let rest = &text[start..];
    let end = rest
        .find(|c: char| c.is_whitespace() || matches!(c, ')' | '）' | ',' | '，' | ';' | '；'))
        .unwrap_or(rest.len());
    (text[..start].trim_end(), Some(&rest[..end]))
}

fn collapse_cjk_internal_spaces(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (idx, ch) in chars.iter().enumerate() {
        if ch.is_whitespace() {
            let prev = idx.checked_sub(1).and_then(|i| chars.get(i)).copied();
            let next = chars.get(idx + 1).copied();
            if prev.map_or(false, is_cjk_char) && next.map_or(false, is_cjk_char) {
                continue;
            }
        }
        out.push(*ch);
    }
    out
}

fn is_cjk_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF
    )
}

// ════════════════════════════════════════════════════════════════════
//  Front matter
// ════════════════════════════════════════════════════════════════════

/// 写出 doc.metadata 中的全部 front matter 块。
///
/// 输出顺序（对齐 oracle PDF）：
/// 1. 中文标题 (JOSTitleZh)
/// 2. 中文作者 (JOSAuthorZh)
/// 3. 中文单位 (JOSInstituteZh) — 每行一段
/// 4. 中文摘要 (JOSAbstractZh) — 标签 "摘   要:" + 正文
/// 5. 中文关键词 (JOSKeywords) — 标签 "关键词:" + 列表
/// 6. 中图法分类号 (JOSBodyNoIndent) — 标签 "中图法分类号:" + 内容
/// 7. 中文引用格式 + 英文引用格式 (JOSBodyNoIndent)
/// 8. 英文标题 (JOSEnglishTitle) + 粗体
/// 9. 英文作者 (JOSBodyNoIndent)
/// 10. 英文单位 (JOSBodyNoIndent)
/// 11. 英文摘要 (JOSAbstractEn) — 标签 "Abstract:" + 正文
/// 12. 英文关键词 (JOSKeywords) — 标签 "Key words:" + 列表
fn write_front_matter(w: &mut Writer<Vec<u8>>, meta: &doc_semantic_ast::MetaData) {
    use doc_semantic_ast::MetaData;

    // ── 中文标题 ──
    if let Some(title) = &meta.title {
        let p = Paragraph {
            style_id: Some(STYLE_TITLE_ZH.to_string()),
            runs: vec![Run {
                text: title.clone(),
                style_id: None,
                style: TextStyle::Bold,
                bold: true,
                italic: false,
                font_ascii: None,
                font_east: None,
            }],
            jc: Some("center".into()),
            keep_next: false,
            keep_lines: true,
        };
        write_paragraph(w, &p);
    }

    // ── 中文作者 ──
    if !meta.authors.is_empty() {
        let mut runs: Vec<Run> = Vec::new();
        for (i, a) in meta.authors.iter().enumerate() {
            if i > 0 {
                runs.push(Run::plain(", ".to_string()));
            }
            runs.push(Run::plain(a.clone()));
        }
        let p = Paragraph {
            style_id: Some(STYLE_AUTHOR_ZH.to_string()),
            runs,
            jc: Some("center".into()),
            keep_next: false,
            keep_lines: false,
        };
        write_paragraph(w, &p);
    }

    // ── 中文单位（每行一段）──
    for line in &meta.institute_lines {
        let p = Paragraph {
            style_id: Some(STYLE_INSTITUTE_ZH.to_string()),
            runs: vec![Run::plain(collapse_cjk_internal_spaces(line))],
            jc: Some("center".into()),
            keep_next: false,
            keep_lines: false,
        };
        write_paragraph(w, &p);
    }

    // ── 中文摘要（标签 + 正文，同段）──
    if let Some(abstract_zh) = &meta.abstract_text {
        if !abstract_zh.is_empty() {
            let p = Paragraph {
                style_id: Some(STYLE_ABSTRACT_ZH.to_string()),
                runs: vec![
                    Run {
                        text: "摘   要: ".to_string(),
                        style_id: None,
                        style: TextStyle::Bold,
                        bold: true,
                        italic: false,
                        font_ascii: None,
                        font_east: None,
                    },
                    Run::plain(abstract_zh.clone()),
                ],
                jc: None,
                keep_next: false,
                keep_lines: false,
            };
            write_paragraph(w, &p);
        }
    }

    // ── 中文关键词（标签 + 列表）──
    if !meta.keywords.is_empty() {
        let joined = meta.keywords.join("; ");
        let p = Paragraph {
            style_id: Some(STYLE_KEYWORDS.to_string()),
            runs: vec![
                Run {
                    text: "关键词: ".to_string(),
                    style_id: None,
                    style: TextStyle::Bold,
                    bold: true,
                    italic: false,
                    font_ascii: None,
                    font_east: None,
                },
                Run::plain(joined),
            ],
            jc: None,
            keep_next: false,
            keep_lines: false,
        };
        write_paragraph(w, &p);
    }

    // ── 中图法分类号 ──
    if let Some(cat) = &meta.category {
        let p = Paragraph {
            style_id: Some(STYLE_BODY_NO_INDENT.to_string()),
            runs: vec![
                Run {
                    text: "中图法分类号: ".to_string(),
                    style_id: None,
                    style: TextStyle::Bold,
                    bold: true,
                    italic: false,
                    font_ascii: None,
                    font_east: None,
                },
                Run::plain(cat.clone()),
            ],
            jc: None,
            keep_next: false,
            keep_lines: false,
        };
        write_paragraph(w, &p);
    }

    // ── 中文引用格式 ──
    if let Some(cz) = &meta.citation_zh {
        if !cz.is_empty() {
            let (citation, url) = split_first_http_url(cz);
            let p = Paragraph {
                style_id: Some(STYLE_CITATION.to_string()),
                runs: vec![
                    Run {
                        text: "中文引用格式: ".to_string(),
                        style_id: None,
                        style: TextStyle::Bold,
                        bold: true,
                        italic: false,
                        font_ascii: None,
                        font_east: None,
                    },
                    Run::plain(citation.to_string()),
                ],
                jc: None,
                keep_next: false,
                keep_lines: false,
            };
            write_paragraph(w, &p);
            if let Some(url) = url {
                let p = Paragraph {
                    style_id: Some(STYLE_CITATION.to_string()),
                    runs: vec![Run::plain(url.to_string())],
                    jc: None,
                    keep_next: false,
                    keep_lines: false,
                };
                write_paragraph(w, &p);
            }
        }
    }

    // ── 英文引用格式 ──
    if let Some(ce) = &meta.citation_en {
        if !ce.is_empty() {
            let (citation, url) = split_first_http_url(ce);
            let p = Paragraph {
                style_id: Some(STYLE_CITATION.to_string()),
                runs: vec![
                    Run {
                        text: "英文引用格式: ".to_string(),
                        style_id: None,
                        style: TextStyle::Bold,
                        bold: true,
                        italic: false,
                        font_ascii: None,
                        font_east: None,
                    },
                    Run::plain(citation.to_string()),
                ],
                jc: None,
                keep_next: false,
                keep_lines: false,
            };
            write_paragraph(w, &p);
            if let Some(url) = url {
                let p = Paragraph {
                    style_id: Some(STYLE_CITATION.to_string()),
                    runs: vec![Run::plain(url.to_string())],
                    jc: None,
                    keep_next: false,
                    keep_lines: false,
                };
                write_paragraph(w, &p);
            }
        }
    }

    // ── 英文标题 ──
    if let Some(title_en) = &meta.title_en {
        let p = Paragraph {
            style_id: Some(STYLE_ENGLISH_TITLE.to_string()),
            runs: vec![Run {
                text: title_en.clone(),
                style_id: None,
                style: TextStyle::Bold,
                bold: true,
                italic: false,
                font_ascii: None,
                font_east: None,
            }],
            jc: None,
            keep_next: false,
            keep_lines: true,
        };
        write_paragraph(w, &p);
    }

    // ── 英文作者 ──
    if !meta.authors_en.is_empty() {
        let mut runs: Vec<Run> = Vec::new();
        for (i, a) in meta.authors_en.iter().enumerate() {
            if i > 0 {
                runs.push(Run::plain(", ".to_string()));
            }
            runs.push(Run::plain(a.clone()));
        }
        let p = Paragraph {
            style_id: Some(STYLE_CITATION.to_string()),
            runs,
            jc: None,
            keep_next: false,
            keep_lines: false,
        };
        write_paragraph(w, &p);
    }

    // ── 英文单位 ──
    if let Some(inst_en) = &meta.institute_en {
        if !inst_en.is_empty() {
            let inst_text = if inst_en.trim_start().starts_with('(') {
                inst_en.clone()
            } else {
                format!("({inst_en})")
            };
            let p = Paragraph {
                style_id: Some(STYLE_CITATION.to_string()),
                runs: vec![Run::plain(inst_text)],
                jc: None,
                keep_next: false,
                keep_lines: false,
            };
            write_paragraph(w, &p);
        }
    }

    // ── 英文摘要（标签 + 正文，同段）──
    if let Some(abstract_en) = &meta.abstract_en {
        if !abstract_en.is_empty() {
            let p = Paragraph {
                style_id: Some(STYLE_ABSTRACT_EN.to_string()),
                runs: vec![
                    Run {
                        text: "Abstract:   ".to_string(),
                        style_id: None,
                        style: TextStyle::Bold,
                        bold: true,
                        italic: false,
                        font_ascii: None,
                        font_east: None,
                    },
                    Run::plain(abstract_en.clone()),
                ],
                jc: None,
                keep_next: false,
                keep_lines: false,
            };
            write_paragraph(w, &p);
        }
    }

    // ── 英文关键词 ──
    if !meta.keywords_en.is_empty() {
        let joined = meta.keywords_en.join("; ");
        let p = Paragraph {
            style_id: Some(STYLE_KEYWORDS.to_string()),
            runs: vec![
                Run {
                    text: "Key words: ".to_string(),
                    style_id: None,
                    style: TextStyle::Bold,
                    bold: true,
                    italic: false,
                    font_ascii: None,
                    font_east: None,
                },
                Run::plain(joined),
            ],
            jc: None,
            keep_next: false,
            keep_lines: false,
        };
        write_paragraph(w, &p);
    }

    // 兜底：避免编译器认为 MetaData 未使用
    let _ = MetaData::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use doc_semantic_ast::{AlgLine, Block, Document, Span, TextRun, TextStyle};

    #[test]
    fn cjk_internal_spaces_are_collapsed_for_institutes() {
        assert_eq!(
            collapse_cjk_internal_spaces("（太原理工大学，山西 太原 030024）"),
            "（太原理工大学，山西太原 030024）"
        );
    }

    #[test]
    fn paragraph_with_inline_citation_keeps_body_style() {
        let doc = Document {
            metadata: Default::default(),
            blocks: vec![Block::Paragraph {
                runs: vec![
                    TextRun {
                        text: "正文包含引用".to_string(),
                        style: TextStyle::Plain,
                        span: Span::default(),
                    },
                    TextRun {
                        text: "[1]".to_string(),
                        style: TextStyle::Superscript,
                        span: Span::default(),
                    },
                ],
                span: Span::default(),
            }],
        };
        let mut embedded = Vec::new();
        let xml = serialize_document(&doc, None, None, &mut embedded);
        let xml = String::from_utf8(xml).expect("document xml utf8");

        assert!(xml.contains(r#"<w:pStyle w:val="JOSBody"/>"#));
        assert!(!xml.contains(r#"<w:pStyle w:val="JOSReference"/>"#));
    }

    #[test]
    fn text_nodes_are_not_double_escaped() {
        let doc = Document {
            metadata: Default::default(),
            blocks: vec![Block::Paragraph {
                runs: vec![TextRun {
                    text: "p<0.001 & x>0".to_string(),
                    style: TextStyle::Plain,
                    span: Span::default(),
                }],
                span: Span::default(),
            }],
        };
        let mut embedded = Vec::new();
        let xml = serialize_document(&doc, None, None, &mut embedded);
        let xml = String::from_utf8(xml).expect("document xml utf8");

        assert!(xml.contains("p&lt;0.001 &amp; x&gt;0"), "got: {xml}");
        assert!(!xml.contains("&amp;lt;"), "text was double-escaped: {xml}");
        assert!(!xml.contains("&amp;gt;"), "text was double-escaped: {xml}");
    }

    #[test]
    fn algorithm_serializes_as_three_column_table() {
        let doc = Document {
            metadata: Default::default(),
            blocks: vec![Block::Algorithm {
                lines: vec![
                    AlgLine {
                        indent: 0,
                        guides: vec![],
                        end_guides: vec![],
                        code: "init H".to_string(),
                        comment: String::new(),
                        keyword: None,
                    },
                    AlgLine {
                        indent: 1,
                        guides: vec![0],
                        end_guides: vec![],
                        code: "collect p".to_string(),
                        comment: "hot path".to_string(),
                        keyword: Some("ForEach".to_string()),
                    },
                ],
                io: vec![("Input".to_string(), "logs".to_string())],
                caption: Some("Attention list".to_string()),
                number: Some("Algorithm 1".to_string()),
                span: Span::default(),
            }],
        };
        let mut embedded = Vec::new();
        let xml = serialize_document(&doc, None, None, &mut embedded);
        let xml = String::from_utf8(xml).expect("document xml utf8");

        assert!(xml.contains("<w:tbl>"));
        assert!(xml.contains(r#"<w:gridCol w:w="650"/>"#));
        assert!(xml.contains(r#"<w:gridCol w:w="6350"/>"#));
        assert!(xml.contains(r#"<w:gridCol w:w="2000"/>"#));
        assert!(xml.contains("Algorithm 1: Attention list"));
        assert!(xml.contains("Input: logs"));
        assert!(xml.contains(">1<"));
        assert!(xml.contains(">2<"));
        assert!(xml.contains("| collect p"));
        assert!(xml.contains("// hot path"));
    }
}
