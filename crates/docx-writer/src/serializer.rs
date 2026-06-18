//! AST → OOXML 元素序列化（V2 重构）
//!
//! 关键变化：
//! - 序列化层现在能完整表达 `TextStyle::Bold/Italic/BoldItalic/Code/MathInline/Superscript/Subscript`
//! - 算法/代码块使用 `JOSCode` 样式 + Courier 字体
//! - 段落支持 `keep_next` / `keep_lines`（算法块、表格不跨页）
//! - 公式块走 `JOSCode` 居中纯文本（对齐 sh `clean_math`，非 OMML）
//! - 21 个 JOS 样式由 `styles.rs` 单一来源生成

use doc_semantic_ast::{AlgLine, Block, Document, Span, TextRun, TextStyle, TheoremLikeKind};
use doc_utils::ImageAssets;
use image::GenericImageView;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use crate::model::{merge_adjacent_runs, Paragraph, Run};
use crate::page_setup::PageSetup;
use crate::styles::{
    STYLE_ABSTRACT_EN, STYLE_ABSTRACT_ZH, STYLE_AUTHOR_ZH, STYLE_BODY, STYLE_BODY_NO_INDENT,
    STYLE_CAPTION, STYLE_CITATION, STYLE_CODE, STYLE_ENGLISH_TITLE, STYLE_HEADING1, STYLE_HEADING2,
    STYLE_HEADING3, STYLE_IMAGE, STYLE_INSTITUTE_ZH, STYLE_KEYWORDS, STYLE_REFERENCE,
    STYLE_REFERENCE_HEADING, STYLE_TABLE_TEXT, STYLE_TITLE_ZH,
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

    let body_blocks = coalesce_theorem_like_blocks(&doc.blocks);
    for block in &body_blocks {
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
                // v13 P1/P2: 不再把 paragraph style 复制到 run.style_id,
                // 也不再强制 bold: true (sh oracle 由 paragraph style 提供 bold)
                let para = Paragraph {
                    style_id: Some(style.to_string()),
                    runs: vec![Run::plain(display_text)],
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
                // v13.2.5 R6: metadata 已含作者简介时跳过正文中的重复标题
                if paragraph_text == "作者简介" && !doc.metadata.author_bio.is_empty() {
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
                let is_jos_ref = is_jos_reference_list(items);
                // v13 P0: 仿 sh oracle 行为——itemize/enumerate 全部用
                // JOSBody 样式 + 手写序号"1. …"。之前用 ListBullet/ListNumber
                // 与 sh 完全错位，导致 v12 段 #147-#159 大量 deleted/inserted。
                let style = if is_jos_ref {
                    "JOSReference"
                } else {
                    STYLE_BODY
                };
                // v13.2.7: sh 将 itemize 留在 chunk 内，latex_to_text 后合并为单段
                // （前缀 "itemize " + 各项空格拼接）；enumerate 仍逐 item 独立段。
                if !is_jos_ref && !*is_ordered {
                    let merged = itemize_merged_text(items);
                    if merged.len() > "itemize ".len() {
                        let para = Paragraph {
                            style_id: Some(style.to_string()),
                            runs: vec![Run::plain(merged)],
                            jc: None,
                            keep_next: false,
                            keep_lines: false,
                        };
                        write_paragraph(&mut w, &para);
                    }
                    continue;
                }
                for (idx, sub) in items.iter().enumerate() {
                    // v12 保留子结构：当 item 只有一个 Paragraph 块时直接用其 runs
                    if sub.len() == 1 {
                        if let Block::Paragraph { runs, .. } = &sub[0] {
                            let mut docx_runs: Vec<Run> =
                                runs.iter().map(from_text_run).collect();
                            // v13: 手写序号作为前缀 plain run
                            if !is_jos_ref {
                                let prefix = if *is_ordered {
                                    format!("{}. ", idx + 1)
                                } else {
                                    "• ".to_string()
                                };
                                docx_runs.insert(0, Run::plain(prefix));
                            }
                            let para = Paragraph {
                                style_id: Some(style.to_string()),
                                runs: merge_adjacent_runs(docx_runs),
                                jc: None,
                                keep_next: false,
                                keep_lines: false,
                            };
                            write_paragraph(&mut w, &para);
                            continue;
                        }
                    }
                    // 多个块或非 Paragraph 块：拼接 summarize 文本为单 run 段落
                    let text = summarize(sub);
                    let runs = if is_jos_ref {
                        vec![Run::plain(text)]
                    } else {
                        let prefix = if *is_ordered {
                            format!("{}. ", idx + 1)
                        } else {
                            "• ".to_string()
                        };
                        vec![Run::plain(prefix + &text)]
                    };
                    let para = Paragraph {
                        style_id: Some(style.to_string()),
                        runs: merge_adjacent_runs(runs),
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
                                    runs: vec![Run::plain(cap_text)],
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
                        runs: vec![Run::plain(cap_text)],
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
            Block::TheoremLike { kind, body, .. } => {
                let text = if body.starts_with("theorem")
                    || body.starts_with("proposition")
                    || body.starts_with("lemma")
                {
                    body.clone()
                } else {
                    format_theorem_like_sh(kind, body)
                };
                let para = Paragraph {
                    style_id: Some(STYLE_BODY.to_string()),
                    runs: vec![Run::plain(text)],
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
                let text_width = page_setup
                    .map(|ps| ps.text_width_twips())
                    .unwrap_or_else(|| PageSetup::jos_paper3().text_width_twips());
                write_algorithm_table(
                    &mut w,
                    lines,
                    io,
                    caption.as_deref(),
                    number.as_deref(),
                    text_width,
                );
            }
        }
    }

    // v13.2.5 R6: 作者简介（文档末尾，参考文献之后）
    write_author_bio(&mut w, &doc.metadata);

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
    if !is_block {
        let display = clean_equation_display_oracle(latex);
        let para = Paragraph {
            style_id: Some(STYLE_BODY.to_string()),
            runs: equation_jos_runs(&display),
            jc: None,
            keep_next: false,
            keep_lines: false,
        };
        write_paragraph(w, &para);
        return;
    }
    let normalized = latex.replace("\\\\", " ").replace('\n', " ");
    let display = clean_equation_display_oracle(&normalized);
    let mut runs = equation_jos_runs(&display);
    runs.push(equation_jos_run(&format!("    ({number})"), false));
    let para = Paragraph {
        style_id: Some(STYLE_CODE.to_string()),
        runs,
        jc: Some("center".to_string()),
        keep_next: false,
        keep_lines: true,
    };
    write_paragraph(w, &para);
}

/// 对齐 build_jos_docx.py `clean_math`（块级公式走 JOSCode 纯文本，非 OMML）。
fn clean_equation_display_oracle(text: &str) -> String {
    const LBRACE: char = '\u{FFF0}';
    const RBRACE: char = '\u{FFF1}';
    let mut s = strip_latex_command_arg(text, "label");
    s = s.replace("\\{", &LBRACE.to_string());
    s = s.replace("\\}", &RBRACE.to_string());
    s = s.replace("\\,", " ");
    s = s.replace('~', " ");
    for cmd in ["mathrm", "textbf", "textit"] {
        s = unwrap_latex_command_arg(&s, cmd);
    }
    for (from, to) in [
        ("\\pm", "±"),
        ("\\%", "%"),
        ("\\rightarrow", "→"),
        ("\\leftarrow", "←"),
        ("\\infty", "∞"),
        ("\\leq", "≤"),
        ("\\geq", "≥"),
        ("\\ll", "≪"),
        ("\\times", "×"),
        ("\\cdot", "·"),
        ("\\emptyset", "∅"),
        ("\\alpha", "α"),
        ("\\beta", "β"),
        ("\\rho", "ρ"),
        ("\\xi", "ξ"),
        ("\\ldots", "…"),
        ("\\log", " log "),
        ("\\min", "min"),
        ("\\max", "max"),
        ("\\in", "∈"),
        ("\\!", ""),
        ("\\sum", "sum"),
        ("\\exp", "exp"),
        ("\\bigl", ""),
        ("\\bigr", ""),
        ("\\left", ""),
        ("\\right", ""),
    ] {
        s = s.replace(from, to);
    }
    for _ in 0..6 {
        s = strip_one_brace_level(&s);
    }
    s = strip_latex_command_names(&s);
    s = s.replace(LBRACE, "{").replace(RBRACE, "}");
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_one_brace_level(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s.as_bytes()[i] == b'{' {
            if let Some((inner, next)) = read_braced_content(s, i) {
                out.push_str(&inner);
                i = next;
                continue;
            }
        }
        let ch = s[i..].chars().next().unwrap_or_default();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn read_braced_content(s: &str, open: usize) -> Option<(String, usize)> {
    if s.as_bytes().get(open) != Some(&b'{') {
        return None;
    }
    let (inner, next) = read_script_arg(s, open);
    Some((inner, next))
}

fn strip_latex_command_names(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s.as_bytes()[i] == b'\\' {
            let start = i + 1;
            let mut end = start;
            while end < s.len() {
                let ch = s[end..].chars().next().unwrap_or_default();
                if ch.is_ascii_alphabetic() {
                    end += ch.len_utf8();
                } else {
                    break;
                }
            }
            if end > start {
                out.push_str(&s[start..end]);
                i = end;
                continue;
            }
        }
        let ch = s[i..].chars().next().unwrap_or_default();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn equation_jos_run(text: &str, bold: bool) -> Run {
    Run {
        text: text.to_string(),
        style_id: None,
        style: TextStyle::Plain,
        bold,
        italic: false,
        font_ascii: None,
        font_east: None,
    }
}

fn equation_jos_runs(text: &str) -> Vec<Run> {
    let mut runs = Vec::new();
    let mut plain = String::new();
    let mut i = 0;
    while i < text.len() {
        let ch = text[i..].chars().next().unwrap_or_default();
        if ch == '_' || ch == '^' {
            if !plain.is_empty() {
                runs.push(equation_jos_run(&plain, false));
                plain.clear();
            }
            let (arg, next) = read_script_arg(text, i + ch.len_utf8());
            runs.push(Run {
                text: arg,
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
    if !plain.is_empty() {
        runs.push(equation_jos_run(&plain, false));
    }
    runs
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

fn is_jos_reference_list(items: &[Vec<Block>]) -> bool {
    items.iter().any(|sub| {
        let s = summarize(sub);
        s.contains('[')
            && s.chars().any(|c| c.is_ascii_digit())
            && (s.contains('—') || s.contains("--"))
    })
}

fn itemize_merged_text(items: &[Vec<Block>]) -> String {
    let mut merged = String::from("itemize ");
    let mut first = true;
    for sub in items {
        let text = summarize(sub);
        if text.is_empty() {
            continue;
        }
        if !first {
            merged.push(' ');
        }
        merged.push_str(&text);
        first = false;
    }
    merged
}

/// 合并连续 TheoremLike 为单段（对齐 sh chunk flatten）。
fn coalesce_theorem_like_blocks(blocks: &[Block]) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::with_capacity(blocks.len());
    let mut buf: Vec<(TheoremLikeKind, String)> = Vec::new();

    let flush = |buf: &mut Vec<(TheoremLikeKind, String)>, out: &mut Vec<Block>| {
        if buf.is_empty() {
            return;
        }
        if buf.len() == 1 {
            let (kind, body) = buf.pop().unwrap();
            out.push(Block::TheoremLike {
                kind,
                title: None,
                body,
                span: Span::default(),
            });
            return;
        }
        let mut merged = String::new();
        for (kind, body) in buf.drain(..) {
            let part = format_theorem_like_sh(&kind, &body);
            if !merged.is_empty() {
                merged.push(' ');
            }
            merged.push_str(&part);
        }
        out.push(Block::TheoremLike {
            kind: TheoremLikeKind::Theorem,
            title: None,
            body: merged,
            span: Span::default(),
        });
    };

    for block in blocks {
        if let Block::TheoremLike { kind, body, .. } = block {
            buf.push((kind.clone(), body.clone()));
        } else {
            flush(&mut buf, &mut out);
            out.push(block.clone());
        }
    }
    flush(&mut buf, &mut out);
    out
}

fn format_theorem_like_sh(kind: &TheoremLikeKind, body: &str) -> String {
    match kind {
        TheoremLikeKind::Proof => format!("theorem proof {body}"),
        TheoremLikeKind::Proposition => format!("proposition {body} proposition"),
        TheoremLikeKind::Theorem => format!("theorem {body} theorem"),
        TheoremLikeKind::Lemma => format!("lemma {body} lemma"),
        TheoremLikeKind::Corollary => format!("corollary {body} corollary"),
        TheoremLikeKind::Definition => format!("definition {body} definition"),
        TheoremLikeKind::Remark => format!("remark {body} remark"),
        TheoremLikeKind::Example => format!("example {body} example"),
    }
}

fn write_algorithm_table(
    w: &mut Writer<Vec<u8>>,
    lines: &[AlgLine],
    io: &[(String, String)],
    caption: Option<&str>,
    number: Option<&str>,
    text_width: u32,
) {
    let cap = build_algorithm_caption(number, caption);
    if lines.is_empty() {
        if !cap.is_empty() {
            write_paragraph(
                w,
                &Paragraph {
                    style_id: Some(STYLE_CAPTION.to_string()),
                    runs: vec![Run::plain(cap)],
                    jc: Some("center".to_string()),
                    keep_next: true,
                    keep_lines: true,
                },
            );
        }
        return;
    }

    let max_depth = lines.iter().map(|l| u32::from(l.indent)).max().unwrap_or(0);
    let line_width = 560u32;
    let guide_width = 280u32;
    let comment_width = 3050u32;
    let code_width = 2200u32.max(
        text_width
            .saturating_sub(line_width)
            .saturating_sub(guide_width.saturating_mul(max_depth))
            .saturating_sub(comment_width),
    );
    let mut widths = vec![line_width];
    widths.extend(std::iter::repeat_n(guide_width, max_depth as usize));
    widths.push(code_width);
    widths.push(comment_width);
    let total_cols = widths.len();

    let mut tbl = BytesStart::new("w:tbl");
    tbl.push_attribute((
        "xmlns:w",
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    ));
    w.write_event(Event::Start(tbl)).unwrap();
    w.write_event(Event::Start(BytesStart::new("w:tblPr")))
        .unwrap();
    let mut tbl_w = BytesStart::new("w:tblW");
    tbl_w.push_attribute(("w:w", "5000"));
    tbl_w.push_attribute(("w:type", "pct"));
    w.write_event(Event::Empty(tbl_w)).unwrap();
    w.write_event(Event::Start(BytesStart::new("w:tblBorders")))
        .unwrap();
    for (side, val, sz) in [
        ("top", "single", "8"),
        ("left", "nil", "4"),
        ("bottom", "single", "8"),
        ("right", "nil", "4"),
        ("insideH", "nil", "4"),
        ("insideV", "nil", "4"),
    ] {
        let mut b = BytesStart::new(format!("w:{side}"));
        b.push_attribute(("w:val", val));
        if val != "nil" {
            b.push_attribute(("w:sz", sz));
            b.push_attribute(("w:space", "0"));
            b.push_attribute(("w:color", "000000"));
        }
        w.write_event(Event::Empty(b)).unwrap();
    }
    w.write_event(Event::End(BytesEnd::new("w:tblBorders")))
        .unwrap();
    let mut layout = BytesStart::new("w:tblLayout");
    layout.push_attribute(("w:type", "fixed"));
    w.write_event(Event::Empty(layout)).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:tblPr"))).unwrap();

    w.write_event(Event::Start(BytesStart::new("w:tblGrid")))
        .unwrap();
    for width in &widths {
        let w_str = width.to_string();
        let mut gc = BytesStart::new("w:gridCol");
        gc.push_attribute(("w:w", w_str.as_str()));
        w.write_event(Event::Empty(gc)).unwrap();
    }
    w.write_event(Event::End(BytesEnd::new("w:tblGrid")))
        .unwrap();

    let title_width: u32 = widths.iter().sum();
    begin_algorithm_table_row(w);
    write_algorithm_cell(
        w,
        algorithm_title_runs(&cap),
        title_width,
        total_cols as u32,
        None,
        AlgoCellBorders {
            top: true,
            bottom: true,
            left: false,
        },
        true,
        true,
        false,
        Some(80),
    );
    end_algorithm_table_row(w);

    let mut io_lines: Vec<(String, String)> = io.to_vec();
    let has_input = io_lines
        .iter()
        .any(|(k, _)| normalize_io_label(k) == "Input");
    let has_output = io_lines
        .iter()
        .any(|(k, _)| normalize_io_label(k) == "Output");
    if !has_input {
        io_lines.insert(0, ("Input".to_string(), String::new()));
    }
    if !has_output {
        io_lines.push(("Output".to_string(), String::new()));
    }
    for (kind, content) in &io_lines {
        let label = normalize_io_label(kind);
        let mut runs = algorithm_inline_runs(&format!("{label}: "), true);
        runs.extend(algorithm_inline_runs(content, false));
        begin_algorithm_table_row(w);
        write_algorithm_cell(
            w,
            vec![],
            line_width,
            1,
            Some("right"),
            AlgoCellBorders::default(),
            true,
            true,
            false,
            None,
        );
        write_algorithm_cell(
            w,
            runs,
            widths[1..].iter().sum(),
            (total_cols - 1) as u32,
            None,
            AlgoCellBorders::default(),
            true,
            true,
            false,
            None,
        );
        end_algorithm_table_row(w);
    }

    let mut line_no = 0u32;
    for (idx, line) in lines.iter().enumerate() {
        line_no += 1;
        let keep_next = idx + 1 < lines.len();
        let indent = u32::from(line.indent);
        let guides: std::collections::HashSet<u8> = line.guides.iter().copied().collect();
        let end_guides: std::collections::HashSet<u8> = line.end_guides.iter().copied().collect();

        begin_algorithm_table_row(w);
        write_algorithm_cell(
            w,
            algorithm_inline_runs(&line_no.to_string(), false),
            line_width,
            1,
            Some("right"),
            AlgoCellBorders::default(),
            keep_next,
            true,
            true,
            None,
        );
        for level in 0..max_depth {
            write_algorithm_cell(
                w,
                vec![],
                guide_width,
                1,
                None,
                AlgoCellBorders {
                    top: false,
                    bottom: end_guides.contains(&(level as u8)),
                    left: guides.contains(&(level as u8)),
                },
                keep_next,
                true,
                false,
                None,
            );
        }
        let span = max_depth - indent + 1;
        let code_span_width: u32 = widths[1 + indent as usize..1 + indent as usize + span as usize]
            .iter()
            .sum();
        write_algorithm_cell(
            w,
            algorithm_code_runs(&format_algline_display_code(line)),
            code_span_width,
            span,
            None,
            AlgoCellBorders::default(),
            keep_next,
            true,
            false,
            None,
        );
        let comment = if line.comment.is_empty() {
            vec![]
        } else {
            algorithm_inline_runs(&format!("// {}", line.comment), false)
        };
        write_algorithm_cell(
            w,
            comment,
            comment_width,
            1,
            Some("right"),
            AlgoCellBorders::default(),
            keep_next,
            true,
            false,
            None,
        );
        end_algorithm_table_row(w);
    }

    w.write_event(Event::End(BytesEnd::new("w:tbl"))).unwrap();
}

#[derive(Default)]
struct AlgoCellBorders {
    top: bool,
    bottom: bool,
    left: bool,
}

fn build_algorithm_caption(number: Option<&str>, caption: Option<&str>) -> String {
    match (number, caption) {
        (Some(n), Some(c)) if !c.is_empty() => {
            if let Some(no) = n.strip_prefix("Algorithm ") {
                format!("算法 {no}  {c}")
            } else {
                format!("{n}  {c}")
            }
        }
        (Some(n), _) => n.to_string(),
        (_, Some(c)) => c.to_string(),
        _ => String::new(),
    }
}

fn begin_algorithm_table_row(w: &mut Writer<Vec<u8>>) {
    w.write_event(Event::Start(BytesStart::new("w:tr"))).unwrap();
    w.write_event(Event::Start(BytesStart::new("w:trPr")))
        .unwrap();
    w.write_event(Event::Empty(BytesStart::new("w:cantSplit")))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("w:trPr"))).unwrap();
}

fn end_algorithm_table_row(w: &mut Writer<Vec<u8>>) {
    w.write_event(Event::End(BytesEnd::new("w:tr"))).unwrap();
}

fn write_algorithm_cell(
    w: &mut Writer<Vec<u8>>,
    runs: Vec<Run>,
    width: u32,
    grid_span: u32,
    align: Option<&str>,
    borders: AlgoCellBorders,
    keep_next: bool,
    keep_lines: bool,
    no_wrap: bool,
    left_margin: Option<u32>,
) {
    w.write_event(Event::Start(BytesStart::new("w:tc"))).unwrap();
    w.write_event(Event::Start(BytesStart::new("w:tcPr"))).unwrap();
    let w_str = width.to_string();
    let mut tc_w = BytesStart::new("w:tcW");
    tc_w.push_attribute(("w:w", w_str.as_str()));
    tc_w.push_attribute(("w:type", "dxa"));
    w.write_event(Event::Empty(tc_w)).unwrap();
    if grid_span > 1 {
        let mut gs = BytesStart::new("w:gridSpan");
        gs.push_attribute(("w:val", grid_span.to_string().as_str()));
        w.write_event(Event::Empty(gs)).unwrap();
    }
    if no_wrap {
        w.write_event(Event::Empty(BytesStart::new("w:noWrap"))).unwrap();
    }
    if borders.top || borders.bottom || borders.left {
        w.write_event(Event::Start(BytesStart::new("w:tcBorders")))
            .unwrap();
        if borders.top {
            let mut b = BytesStart::new("w:top");
            b.push_attribute(("w:val", "single"));
            b.push_attribute(("w:sz", "8"));
            b.push_attribute(("w:space", "0"));
            b.push_attribute(("w:color", "000000"));
            w.write_event(Event::Empty(b)).unwrap();
        }
        if borders.bottom {
            let mut b = BytesStart::new("w:bottom");
            b.push_attribute(("w:val", "single"));
            b.push_attribute(("w:sz", "8"));
            b.push_attribute(("w:space", "0"));
            b.push_attribute(("w:color", "000000"));
            w.write_event(Event::Empty(b)).unwrap();
        }
        if borders.left {
            let mut b = BytesStart::new("w:left");
            b.push_attribute(("w:val", "single"));
            b.push_attribute(("w:sz", "4"));
            b.push_attribute(("w:space", "0"));
            b.push_attribute(("w:color", "000000"));
            w.write_event(Event::Empty(b)).unwrap();
        }
        w.write_event(Event::End(BytesEnd::new("w:tcBorders"))).unwrap();
    }
    if let Some(m) = left_margin {
        w.write_event(Event::Start(BytesStart::new("w:tcMar")))
            .unwrap();
        let m_str = m.to_string();
        let mut left = BytesStart::new("w:left");
        left.push_attribute(("w:w", m_str.as_str()));
        left.push_attribute(("w:type", "dxa"));
        w.write_event(Event::Empty(left)).unwrap();
        w.write_event(Event::End(BytesEnd::new("w:tcMar"))).unwrap();
    }
    w.write_event(Event::End(BytesEnd::new("w:tcPr"))).unwrap();

    let p = Paragraph {
        style_id: None,
        runs: if runs.is_empty() {
            vec![Run::plain(String::new())]
        } else {
            runs
        },
        jc: align.map(str::to_string),
        keep_next,
        keep_lines,
    };
    write_paragraph_with_opts(w, &p, true, 18);
    w.write_event(Event::End(BytesEnd::new("w:tc"))).unwrap();
}

fn algorithm_title_runs(caption: &str) -> Vec<Run> {
    if let Some(caps) = regex_simple_algorithm_title(caption) {
        return vec![
            algo_run(&caps.0, true),
            algo_run(&caps.1, false),
        ];
    }
    vec![algo_run(caption, false)]
}

fn regex_simple_algorithm_title(caption: &str) -> Option<(String, String)> {
    let caption = caption.trim();
    if let Some(rest) = caption.strip_prefix("算法") {
        let rest = rest.trim_start();
        if let Some((num, body)) = rest.split_once(|c: char| c.is_whitespace()) {
            let body = body.trim();
            if !num.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            return Some((format!("Algorithm {num}: "), body.to_string()));
        }
    }
    None
}

fn algo_run(text: &str, bold: bool) -> Run {
    Run {
        text: text.to_string(),
        style_id: None,
        style: if bold {
            TextStyle::Bold
        } else {
            TextStyle::Plain
        },
        bold,
        italic: false,
        font_ascii: Some("Times New Roman".to_string()),
        font_east: Some("宋体".to_string()),
    }
}

fn algorithm_inline_runs(text: &str, bold: bool) -> Vec<Run> {
    if text.is_empty() {
        return vec![];
    }
    vec![algo_run(text, bold)]
}

fn algorithm_code_runs(text: &str) -> Vec<Run> {
    let lower = text.to_ascii_lowercase();
    if lower.starts_with("foreach ") && lower.ends_with(" do") {
        let body = &text[8..text.len().saturating_sub(3)];
        return vec![
            algo_run("foreach", true),
            algo_run(&format!(" {body} "), false),
            algo_run("do", true),
        ];
    }
    if lower.starts_with("if ") && lower.ends_with(" then") {
        let body = &text[3..text.len().saturating_sub(5)];
        return vec![
            algo_run("if", true),
            algo_run(&format!(" {body} "), false),
            algo_run("then", true),
        ];
    }
    if lower.starts_with("return") {
        return vec![
            algo_run("return", true),
            algo_run(&text[6..], false),
        ];
    }
    algorithm_inline_runs(text, false)
}

fn format_algline_display_code(line: &AlgLine) -> String {
    if let Some(kw) = line.keyword.as_deref() {
        match kw {
            "ForEach" => {
                let cond = line
                    .code
                    .strip_prefix("ForEach (")
                    .or_else(|| line.code.strip_prefix("foreach ("))
                    .and_then(|s| s.strip_suffix(')'))
                    .unwrap_or(line.code.as_str());
                format!("foreach {cond} do")
            }
            "If" => {
                let cond = line
                    .code
                    .strip_prefix("If (")
                    .or_else(|| line.code.strip_prefix("if ("))
                    .and_then(|s| s.strip_suffix(')'))
                    .unwrap_or(line.code.as_str());
                format!("if {cond} then")
            }
            "Return" => {
                if line.code.starts_with("return") {
                    line.code.clone()
                } else {
                    format!("return {}", line.code.trim())
                }
            }
            "End" => "end".to_string(),
            _ => line.code.clone(),
        }
    } else {
        line.code.clone()
    }
}

fn normalize_io_label(kind: &str) -> &'static str {
    let lower = kind.trim().to_ascii_lowercase();
    if lower.contains("out") || lower.contains("输出") {
        "Output"
    } else {
        "Input"
    }
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
        // v13 P6: 不再在 run 上加 italic:true,
        // JOSCaption 样式自身在 pPr/rPr 中提供 italic
        let p = Paragraph {
            style_id: Some(STYLE_CAPTION.to_string()),
            runs: vec![Run::plain(cap_text)],
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
    let total_w: i64 = 9000i64; // 6.25 inches — 适合 A4 双栏
    // v13.2.4 R5: 正确写入 tblW（属性在 w:tblW 元素上）
    let mut tbl_w = BytesStart::new("w:tblW");
    tbl_w.push_attribute(("w:w", total_w.to_string().as_str()));
    tbl_w.push_attribute(("w:type", "dxa"));
    w.write_event(Event::Empty(tbl_w)).unwrap();
    // v13.2.4 R5: 表格居中
    let mut jc = BytesStart::new("w:jc");
    jc.push_attribute(("w:val", "center"));
    w.write_event(Event::Empty(jc)).unwrap();
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
            // v13.2.4 R5: cell 宽度
            let mut tc_w = BytesStart::new("w:tcW");
            tc_w.push_attribute(("w:w", col_w_str.as_str()));
            tc_w.push_attribute(("w:type", "dxa"));
            w.write_event(Event::Empty(tc_w)).unwrap();
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
                    // v13.1 P2: cell run 应用 collapse_cjk_internal_spaces 清理
                    // CJK-标点 (如 *) 之间的空格
                    cell.runs
                        .iter()
                        .map(|r| {
                            let mut run = from_text_run(r);
                            run.text = collapse_cjk_internal_spaces(&run.text);
                            // v13.1 P2': data cell 不强制 run.bold=false (保持原 style, 避免丢失 sup 等)
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
            write_paragraph_with_opts(w, &p, true, 15);
            w.write_event(Event::End(BytesEnd::new("w:tc"))).unwrap();
        }
        w.write_event(Event::End(BytesEnd::new("w:tr"))).unwrap();
    }

    w.write_event(Event::End(BytesEnd::new("w:tbl"))).unwrap();
}

fn write_paragraph(w: &mut Writer<Vec<u8>>, p: &Paragraph) {
    write_paragraph_with_opts(w, p, false, 0)
}

/// v13.2.7: 正文段落 run 归一化——合并 plain run 并清理 CJK 内部空格（对齐 sh latex_to_text）。
fn normalize_body_runs(runs: Vec<Run>) -> Vec<Run> {
    let has_script = runs
        .iter()
        .any(|r| matches!(r.style, TextStyle::Superscript | TextStyle::Subscript));
    if has_script {
        let runs = merge_adjacent_runs(runs);
        return runs
            .into_iter()
            .map(|mut r| {
                r.text = collapse_cjk_internal_spaces(&r.text);
                r
            })
            .collect();
    }
    let flat: String = runs.iter().map(|r| r.text.as_str()).collect();
    let flat = collapse_cjk_internal_spaces(flat.trim());
    if flat.is_empty() {
        return vec![];
    }
    vec![Run::plain(flat)]
}

fn is_body_style_for_cjk(style_id: &Option<String>) -> bool {
    style_id.as_deref().is_some_and(|s| {
        matches!(s, "JOSBody" | "JOSReference")
    })
}

/// v13.2.1 R1: JOSBody 等正文段落内联 Bold/Italic/Code 降级为 Plain（对齐 sh oracle）。
fn downgrade_body_inline_styles(p: Paragraph) -> Paragraph {
    let is_body = p.style_id.as_deref().is_some_and(|s| {
        matches!(
            s,
            "JOSBody"
                | "JOSAbstractEn"
                | "JOSAbstractZh"
                | "JOSKeywords"
                | "JOSReference"
        )
    });
    if !is_body {
        return p;
    }
    Paragraph {
        runs: p
            .runs
            .into_iter()
            .map(|mut r| {
                if matches!(
                    r.style,
                    TextStyle::Bold | TextStyle::Italic | TextStyle::BoldItalic | TextStyle::Code
                ) {
                    r.style = TextStyle::Plain;
                    r.bold = false;
                    r.italic = false;
                }
                r
            })
            .collect(),
        ..p
    }
}

/// v13 P1/P7: 增加 force_table_cell_font + cell_font_half_points 参数。
/// table cell 内部用 force_table_cell_font=true 让 write_run 自动获得字体；
/// cell_font_half_points > 0 时替代默认 15 (e.g. algorithm cell 用 18)。
fn write_paragraph_with_opts(
    w: &mut Writer<Vec<u8>>,
    p: &Paragraph,
    force_table_cell_font: bool,
    cell_font_half_points: u16,
) {
    let p = if force_table_cell_font {
        p.clone()
    } else {
        downgrade_body_inline_styles(p.clone())
    };
    let runs = if force_table_cell_font {
        merge_adjacent_runs(p.runs.clone())
    } else if is_body_style_for_cjk(&p.style_id) {
        normalize_body_runs(p.runs.clone())
    } else {
        merge_adjacent_runs(p.runs.clone())
    };
    let p = Paragraph {
        style_id: p.style_id.clone(),
        jc: p.jc.clone(),
        runs,
        keep_next: p.keep_next,
        keep_lines: p.keep_lines,
    };
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
        write_run(w, run, force_table_cell_font, cell_font_half_points);
    }
    w.write_event(Event::End(BytesEnd::new("w:p"))).unwrap();
}

/// 写单个 `<w:r>`，根据 `Run.style` 产生正确的 `<w:rPr>`。
///
/// v13: `force_table_cell_font` + `cell_font_half_points`
/// - 普通 table cell: `force_table_cell_font=true, half_points=15` (Times/宋体 + 15)
/// - algorithm cell: `force_table_cell_font=true, half_points=18`
/// - 普通段: `force_table_cell_font=false, half_points=0`
fn write_run(
    w: &mut Writer<Vec<u8>>,
    run: &Run,
    force_table_cell_font: bool,
    cell_font_half_points: u16,
) {
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
        || force_table_cell_font
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
        } else if force_table_cell_font {
            // v13 P1: cell run 默认 sz=15 + Times New Roman/宋体
            let mut rfonts = BytesStart::new("w:rFonts");
            rfonts.push_attribute(("w:ascii", "Times New Roman"));
            rfonts.push_attribute(("w:hAnsi", "Times New Roman"));
            rfonts.push_attribute(("w:cs", "Times New Roman"));
            rfonts.push_attribute(("w:eastAsia", "宋体"));
            w.write_event(Event::Empty(rfonts)).unwrap();
        } else if matches!(run.style, TextStyle::Code) {
            // Code 样式自动用 Courier New
            let mut rfonts = BytesStart::new("w:rFonts");
            rfonts.push_attribute(("w:ascii", "Courier New"));
            rfonts.push_attribute(("w:hAnsi", "Courier New"));
            rfonts.push_attribute(("w:eastAsia", "宋体"));
            rfonts.push_attribute(("w:cs", "Courier New"));
            w.write_event(Event::Empty(rfonts)).unwrap();
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
        // v13 P1/P7: cell run 的 sz 由 cell_font_half_points 提供
        if force_table_cell_font && cell_font_half_points > 0 {
            let sz_str = cell_font_half_points.to_string();
            let mut sz = BytesStart::new("w:sz");
            sz.push_attribute(("w:val", sz_str.as_str()));
            w.write_event(Event::Empty(sz)).unwrap();
            let mut sz_cs = BytesStart::new("w:szCs");
            sz_cs.push_attribute(("w:val", sz_str.as_str()));
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


fn collapse_cjk_internal_spaces(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch.is_whitespace() {
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let next = chars.get(i + 1).copied();
            // v13.1 P2: 清理以下模式
            // (a) CJK-非字母 (CJK-CJK / CJK-标点)
            // (b) 非字母-非字母 (标点-标点)
            // (c) CJK-CJK 字母
            let left_collapse = prev.map_or(false, |p| {
                is_cjk_char(p) || (!p.is_alphanumeric() && !p.is_whitespace())
            });
            let right_collapse = next.map_or(false, |n| {
                is_cjk_char(n) || (!n.is_alphanumeric() && !n.is_whitespace())
            });
            if left_collapse && right_collapse {
                i += 1;
                continue;
            }
        }
        out.push(ch);
        i += 1;
    }
    // v13.1 P2 尾步: 去掉 "字母+空格+尾标点" 模式中的空格 (e.g. "CPU *" → "CPU*")
    let trimmed = strip_trailing_space_before_punct(&out);
    trimmed
}

/// 去除形如 "字母/数字+空格+尾标点" 的多余空格。
/// 仅在标点是 footnote 类符号 (* † ‡ § ¶ #) 时启用,避免误清 "0.05% CPU" 这种 % 与 C 之间合法的空格。
fn strip_trailing_space_before_punct(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch.is_whitespace() && i + 1 < chars.len() {
            let next = chars[i + 1];
            if matches!(next, '*' | '†' | '‡' | '§' | '¶' | '#') {
                // 跳过空格
                i += 1;
                continue;
            }
        }
        out.push(ch);
        i += 1;
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
fn write_spacer(w: &mut Writer<Vec<u8>>, height_twips: u32) {
    w.write_event(Event::Start(BytesStart::new("w:p")))
        .unwrap();
    w.write_event(Event::Start(BytesStart::new("w:pPr")))
        .unwrap();
    let mut spacing = BytesStart::new("w:spacing");
    let line = height_twips.to_string();
    spacing.push_attribute(("w:line", line.as_str()));
    spacing.push_attribute(("w:lineRule", "exact"));
    w.write_event(Event::Empty(spacing)).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:pPr"))).unwrap();
    w.write_event(Event::End(BytesEnd::new("w:p"))).unwrap();
}

fn token_width_units(token: &str) -> f64 {
    let mut total = 0.0;
    for ch in token.chars() {
        if ch.is_whitespace() {
            total += 0.35;
        } else if ('\u{4E00}'..='\u{9FFF}').contains(&ch) {
            total += 1.0;
        } else if ch.is_uppercase() {
            total += 0.62;
        } else if ch.is_lowercase() || ch.is_ascii_digit() {
            total += 0.52;
        } else if matches!(ch, '-' | '/' | '.') {
            total += 0.28;
        } else {
            total += 0.35;
        }
    }
    total
}

fn wrap_text_units(text: &str, max_units: f64) -> Vec<String> {
    let tokens = tokenize_citation_text(text);
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut width = 0.0;
    for token in tokens {
        let mut token = token;
        let mut token_width = token_width_units(&token);
        if !current.is_empty() && width + token_width > max_units {
            lines.push(current.trim().to_string());
            current.clear();
            width = 0.0;
            token = token.trim_start().to_string();
            token_width = token_width_units(&token);
        }
        if !token.is_empty() || !current.is_empty() {
            current.push_str(&token);
            width += token_width;
        }
    }
    if !current.is_empty() {
        lines.push(current.trim().to_string());
    }
    lines.into_iter().filter(|l| !l.is_empty()).collect()
}

fn tokenize_citation_text(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if text[i..].starts_with("http://") || text[i..].starts_with("https://") {
            let start = i;
            while i < text.len() {
                let ch = text[i..].chars().next().unwrap();
                if ch.is_whitespace() {
                    break;
                }
                i += ch.len_utf8();
            }
            tokens.push(text[start..i].to_string());
            continue;
        }
        let ch = text[i..].chars().next().unwrap();
        let start = i;
        i += ch.len_utf8();
        if ch.is_whitespace() {
            tokens.push(" ".to_string());
            continue;
        }
        if ch.is_ascii_alphanumeric() {
            while i < text.len() {
                let c = text[i..].chars().next().unwrap();
                if c.is_ascii_alphanumeric() || matches!(c, '-' | '/') {
                    i += c.len_utf8();
                } else {
                    break;
                }
            }
            tokens.push(text[start..i].to_string());
            continue;
        }
        if ('\u{4E00}'..='\u{9FFF}').contains(&ch) {
            tokens.push(ch.to_string());
            continue;
        }
        tokens.push(ch.to_string());
    }
    tokens
}

fn split_citation_text(text: &str, max_units: f64) -> Vec<String> {
    let text = text.trim();
    if text.is_empty() {
        return vec![];
    }
    wrap_text_units(text, max_units)
}

fn normalize_institute_line(text: &str) -> String {
    let mut s = collapse_cjk_internal_spaces(text);
    // "通讯作者:石洪雷" → "通讯作者: 石洪雷"
    if let Some(idx) = s.find(':') {
        let after = &s[idx + 1..];
        if after.starts_with(' ') {
            return s;
        }
        if !after.is_empty() {
            s.insert(idx + 1, ' ');
        }
    }
    s
}

fn spaced_keywords(keywords: &[String]) -> String {
    keywords
        .join("; ")
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("; ")
}

fn write_front_matter(w: &mut Writer<Vec<u8>>, meta: &doc_semantic_ast::MetaData) {
    use doc_semantic_ast::MetaData;

    // ── 中文标题 ──
    if let Some(title) = &meta.title {
        let p = Paragraph {
            style_id: Some(STYLE_TITLE_ZH.to_string()),
            // v13 P2: 不再在 run 上加 bold:true,JOSTitleZh 样式内部已含 bold
            runs: vec![Run::plain(title.clone())],
            jc: Some("left".to_string()),
            keep_next: false,
            keep_lines: true,
        };
        write_paragraph(w, &p);
    }

    // ── 中文作者（整段单 run，对齐 sh populate）──
    if !meta.authors.is_empty() {
        let authors = meta.authors.join("");
        let p = Paragraph {
            style_id: Some(STYLE_AUTHOR_ZH.to_string()),
            runs: vec![Run::plain(authors)],
            jc: Some("left".to_string()),
            keep_next: false,
            keep_lines: false,
        };
        write_paragraph(w, &p);
    }

    // ── 中文单位（每行一段）──
    for line in &meta.institute_lines {
        let p = Paragraph {
            style_id: Some(STYLE_INSTITUTE_ZH.to_string()),
            runs: vec![Run::plain(normalize_institute_line(line))],
            jc: Some("left".to_string()),
            keep_next: false,
            keep_lines: false,
        };
        write_paragraph(w, &p);
    }

    if !meta.institute_lines.is_empty() {
        write_spacer(w, 300);
    }

    // ── 中文摘要（标签 + 正文，单 run）──
    if let Some(abstract_zh) = &meta.abstract_text {
        if !abstract_zh.is_empty() {
            let p = Paragraph {
                style_id: Some(STYLE_ABSTRACT_ZH.to_string()),
                runs: vec![Run::plain(format!("摘   要: {abstract_zh}"))],
                jc: None,
                keep_next: false,
                keep_lines: false,
            };
            write_paragraph(w, &p);
        }
    }

    // ── 中文关键词（标签 + 列表，单 run）──
    if !meta.keywords.is_empty() {
        let joined = spaced_keywords(&meta.keywords);
        let p = Paragraph {
            style_id: Some(STYLE_KEYWORDS.to_string()),
            runs: vec![Run::plain(format!("关键词: {joined}"))],
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
            runs: vec![Run::plain(format!("中图法分类号: {cat}"))],
            jc: None,
            keep_next: false,
            keep_lines: false,
        };
        write_paragraph(w, &p);
    }

    write_spacer(w, 300);

    // ── 引用格式（按宽度换行，无额外标签）──
    if let Some(cz) = &meta.citation_zh {
        for line in split_citation_text(cz, 52.0) {
            let p = Paragraph {
                style_id: Some(STYLE_CITATION.to_string()),
                runs: vec![Run::plain(collapse_cjk_internal_spaces(&line))],
                jc: None,
                keep_next: false,
                keep_lines: false,
            };
            write_paragraph(w, &p);
        }
    }
    if let Some(ce) = &meta.citation_en {
        for line in split_citation_text(ce, 52.0) {
            let p = Paragraph {
                style_id: Some(STYLE_CITATION.to_string()),
                runs: vec![Run::plain(collapse_cjk_internal_spaces(&line))],
                jc: None,
                keep_next: false,
                keep_lines: false,
            };
            write_paragraph(w, &p);
        }
    }

    write_spacer(w, 220);
    // ── 英文标题 ──
    if let Some(title_en) = &meta.title_en {
        let p = Paragraph {
            style_id: Some(STYLE_ENGLISH_TITLE.to_string()),
            // v13 P2: 不再 bold:true,JOSEnglishTitle 样式内部已含 bold
            runs: vec![Run::plain(title_en.clone())],
            jc: None,
            keep_next: false,
            keep_lines: true,
        };
        write_paragraph(w, &p);
    }

    // ── 英文作者（整段单 run）──
    if !meta.authors_en.is_empty() {
        let authors = meta.authors_en.join("");
        let p = Paragraph {
            style_id: Some(STYLE_CITATION.to_string()),
            runs: vec![Run::plain(authors)],
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

    write_spacer(w, 340);

    // ── 英文摘要（标签 + 正文，单 run）──
    if let Some(abstract_en) = &meta.abstract_en {
        if !abstract_en.is_empty() {
            let p = Paragraph {
                style_id: Some(STYLE_ABSTRACT_EN.to_string()),
                runs: vec![Run::plain(format!("Abstract:   {abstract_en}"))],
                jc: None,
                keep_next: false,
                keep_lines: false,
            };
            write_paragraph(w, &p);
        }
    }

    // ── 英文关键词（单 run）──
    if !meta.keywords_en.is_empty() {
        let joined = spaced_keywords(&meta.keywords_en);
        let p = Paragraph {
            style_id: Some(STYLE_KEYWORDS.to_string()),
            runs: vec![Run::plain(format!("Key words: {joined}"))],
            jc: None,
            keep_next: false,
            keep_lines: false,
        };
        write_paragraph(w, &p);
    }

    // 兜底：避免编译器认为 MetaData 未使用
    let _ = MetaData::default();
}

/// v13.2.5 R6: 输出作者简介（文档末尾）。
fn write_author_bio(w: &mut Writer<Vec<u8>>, meta: &doc_semantic_ast::MetaData) {
    if meta.author_bio.is_empty() {
        return;
    }
    write_paragraph(
        w,
        &Paragraph {
            style_id: Some(STYLE_REFERENCE_HEADING.to_string()),
            runs: vec![Run::plain("作者简介".to_string())],
            jc: None,
            keep_next: false,
            keep_lines: false,
        },
    );
    for bio in &meta.author_bio {
        write_paragraph(
            w,
            &Paragraph {
                style_id: Some(STYLE_REFERENCE.to_string()),
                runs: vec![Run::plain(bio.clone())],
                jc: None,
                keep_next: false,
                keep_lines: false,
            },
        );
    }
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

    // v13.1 P2: CJK-标点(*) 之间的空格也应清理
    #[test]
    fn cjk_to_punct_spaces_are_collapsed() {
        // "条 *" → "条*"
        assert_eq!(
            collapse_cjk_internal_spaces("72 vs 4388 条 *"),
            "72 vs 4388 条*"
        );
        // "0.05% CPU *" — CPU 与 * 之间的空格保留 (字母-标点 不动)
        assert_eq!(
            collapse_cjk_internal_spaces("0.05% CPU *"),
            "0.05% CPU*"
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
    fn block_equation_uses_jos_code_plain_text() {
        let doc = Document {
            metadata: Default::default(),
            blocks: vec![Block::Equation {
                latex: r"\mathrm{Score}(u,t)=\alpha\,\mathrm{Freq}(u,t)+\beta\,\mathrm{Err}(u,t)+\gamma\,\mathrm{Delay}(u,t)+\delta\,\mathrm{Trend}(u,t)".to_string(),
                is_block: true,
                span: Span::default(),
            }],
        };
        let mut embedded = Vec::new();
        let xml = serialize_document(&doc, None, None, &mut embedded);
        let xml = String::from_utf8(xml).expect("document xml utf8");
        assert!(xml.contains(r#"<w:pStyle w:val="JOSCode"/>"#));
        assert!(xml.contains("Score(u,t)"));
        assert!(xml.contains("gamma"));
        assert!(!xml.contains("m:oMath"), "block equation should not use OMML");
    }

    #[test]
    fn algorithm_serializes_as_table() {
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
                        code: "ForEach (p)".to_string(),
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

        assert!(xml.contains("w:tbl"), "algorithm should use table");
        assert!(xml.contains("Algorithm 1:"));
        assert!(xml.contains("Attention list"));
        assert!(xml.contains("Input:"));
        assert!(xml.contains("logs"));
        assert!(xml.contains("init H"));
        assert!(xml.contains("foreach"));
        assert!(!xml.contains(" | "), "algorithm lines should not merge with | ");
    }

    #[test]
    fn normalize_body_runs_collapses_cjk_spaces() {
        let runs = normalize_body_runs(vec![
            Run::plain("中心 ".to_string()),
            Run::plain("之前".to_string()),
        ]);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "中心之前");
    }

    #[test]
    fn downgrade_body_inline_styles_drops_bold() {
        let p = downgrade_body_inline_styles(Paragraph {
            style_id: Some(STYLE_BODY.to_string()),
            runs: vec![Run::bold("bold text")],
            ..Default::default()
        });
        assert_eq!(p.runs[0].style, TextStyle::Plain);
        assert!(!p.runs[0].bold);
    }

    #[test]
    fn downgrade_body_inline_styles_keeps_sup() {
        let p = downgrade_body_inline_styles(Paragraph {
            style_id: Some(STYLE_BODY.to_string()),
            runs: vec![Run {
                text: "x".to_string(),
                style: TextStyle::Superscript,
                ..Default::default()
            }],
            ..Default::default()
        });
        assert_eq!(p.runs[0].style, TextStyle::Superscript);
    }

    #[test]
    fn downgrade_body_inline_styles_keeps_heading() {
        let p = downgrade_body_inline_styles(Paragraph {
            style_id: Some(STYLE_HEADING1.to_string()),
            runs: vec![Run::bold("heading")],
            ..Default::default()
        });
        assert_eq!(p.runs[0].style, TextStyle::Bold);
    }

    #[test]
    fn table_has_center_alignment_and_tcW() {
        use doc_semantic_ast::{TableCell, TableRow};
        let doc = Document {
            metadata: Default::default(),
            blocks: vec![Block::Table {
                rows: vec![TableRow {
                    cells: vec![
                        TableCell {
                            runs: vec![TextRun {
                                text: "A".to_string(),
                                style: TextStyle::Plain,
                                span: Span::default(),
                            }],
                            colspan: 1,
                            rowspan: 1,
                            bg_color: None,
                        },
                        TableCell {
                            runs: vec![TextRun {
                                text: "B".to_string(),
                                style: TextStyle::Plain,
                                span: Span::default(),
                            }],
                            colspan: 1,
                            rowspan: 1,
                            bg_color: None,
                        },
                    ],
                }],
                caption: None,
                number: None,
                span: Span::default(),
            }],
        };
        let mut embedded = Vec::new();
        let xml = serialize_document(&doc, None, None, &mut embedded);
        let xml = String::from_utf8(xml).expect("document xml utf8");
        assert!(xml.contains(r#"<w:jc w:val="center"/>"#));
        assert!(xml.contains("<w:tcW"));
    }
}
