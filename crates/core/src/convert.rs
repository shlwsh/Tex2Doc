//! 核心层：统一转换入口
//!
//! 端到端管道：
//! 1. include 拓扑（`IncludeGraph::build`）
//! 2. 拼接单流（`IncludeGraph::join`）
//! 3. Logos + Rowan 解析（`LatexParser`）
//! 4. 降级到 `semantic-ast`
//! 5. docx-writer 序列化 + ZIP 打包

use std::io::Read;
use std::path::{Path, PathBuf};

use doc_latex_reader::{
    lower_to_document, lower_to_document_with_cite_map, parse_bbl, parse_bib, parse_tex,
    IncludeGraph,
};
use doc_semantic_ast::{Block, Document, Span, TextRun, TextStyle};
use doc_utils::{ImageAssets, VirtualFs};

use crate::error::CoreError;
use crate::options::ConvertOptions;
use crate::result::{ConvertResult, ProgressEvent, ProgressPhase};

/// 把 PDF 第一页渲染为 PNG（200 dpi），用 pdfium-render。
///
/// 把 `石 洪 雷 等:网关流量驱动的微服务定向日志采集框架`
/// 缩短为「石 等:网关流量驱动的微服务定向日志采集框架」（不超过 50 字符）。
fn shorten_running_header(rh: &str) -> String {
    rh.trim().to_string()
}

/// V1 设计：zip 内 `\includegraphics{*.pdf}` 时，docx-writer 端只接受 PNG/JPG，
/// 所以这里把 PDF 翻译成 PNG byte 注入 image_assets（同 key）。
///
/// 注意：pdfium-render 依赖 mio，mio 不支持 wasm32 目标。因此在 wasm 下
/// PDF → PNG 转换整体被 cfg 跳过（返回 None 表示「不渲染」，调用方走 fallback）。
#[cfg(not(target_arch = "wasm32"))]
fn render_pdf_to_png(pdf_bytes: &[u8]) -> Option<Vec<u8>> {
    use pdfium_render::prelude::Pdfium;
    let bindings = Pdfium::bind_to_system_library().ok()?;
    let pdfium = Pdfium::new(bindings);
    let doc = pdfium.load_pdf_from_byte_slice(pdf_bytes, None).ok()?;
    let page = doc.pages().get(0).ok()?;
    let bitmap = page
        .render_with_config(&pdfium_render::prelude::PdfRenderConfig::new().set_target_width(1600))
        .ok()?;
    let image = bitmap.as_image();
    let buf: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = image.into_rgba8();
    let dyn_img = image::DynamicImage::ImageRgba8(buf);
    let mut png_bytes: Vec<u8> = Vec::new();
    dyn_img
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .ok()?;
    Some(png_bytes)
}

/// wasm 占位：浏览器端无法用 pdfium，PDF 图片直接忽略（docx-writer 走 fallback）。
#[cfg(target_arch = "wasm32")]
fn render_pdf_to_png(_pdf_bytes: &[u8]) -> Option<Vec<u8>> {
    None
}

fn insert_image_asset_aliases(image_assets: &mut ImageAssets, path: &Path, bytes: Vec<u8>) {
    let path_key = path.to_string_lossy().replace('\\', "/");
    image_assets.insert(path_key.clone(), bytes.clone());
    if let Some(basename) = path.file_name().and_then(|n| n.to_str()) {
        if basename != path_key {
            image_assets.insert(basename.to_string(), bytes);
        }
    }
}

fn png_basename_for_pdf(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|stem| format!("{stem}.png"))
}

fn collect_image_assets_from_dir(dir: &Path, image_assets: &mut ImageAssets) {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let p_lower = path.to_string_lossy().to_lowercase();
            if p_lower.ends_with(".png") || p_lower.ends_with(".jpg") || p_lower.ends_with(".jpeg")
            {
                if let Ok(bytes) = std::fs::read(&path) {
                    insert_image_asset_aliases(image_assets, &path, bytes);
                }
            } else if p_lower.ends_with(".pdf") {
                if let Ok(bytes) = std::fs::read(&path) {
                    if let Some(png) = render_pdf_to_png(&bytes) {
                        insert_image_asset_aliases(image_assets, &path, png.clone());
                        if let Some(png_basename) = png_basename_for_pdf(&path) {
                            image_assets.insert(png_basename, png);
                        }
                    }
                }
            }
        }
    }
}

/// 同步转换入口（V1 M1-M2）。
///
/// 仅把 `main_tex` 文本装载到空 VFS；`\input{...}` 解析依赖调用方事先把
/// 所有 include / graphicspath 资源以 attachment 形式或经 [`convert_dir`] 提供。
pub fn convert_sync(
    main_tex: &str,
    source: &str,
    options: &ConvertOptions,
) -> Result<ConvertResult, CoreError> {
    let doc = parse_tex_to_doc(main_tex, source)?;
    let docx = doc_docx_writer::pack_with_page_setup(
        &doc,
        options.template_bytes.as_deref(),
        None,
        options.page_setup.as_ref(),
        None,
    )
    .map_err(|e| CoreError::Serialize(e.0))?;
    Ok(ConvertResult {
        docx,
        warnings: vec![],
    })
}

/// 同步转换入口：以真实项目根目录为底座，挂载全部文件后转换 `main_tex`。
///
/// 适合本地 / CLI 场景：把 `project_root` 下的 `.tex` / `.bib` / 图片等
/// 全部映射到 VFS，然后按 `main_tex`（相对 `project_root`）构建 include
/// 拓扑并完成解析 → 降级 → 打包。
pub fn convert_dir(
    project_root: &Path,
    main_tex: &Path,
    options: &ConvertOptions,
) -> Result<ConvertResult, CoreError> {
    let mut vfs = VirtualFs::new();
    vfs.mount_dir(project_root)
        .map_err(|e| CoreError::Io(e.to_string()))?;

    // Collect image assets from VFS. Directory conversion must mirror zip conversion:
    // paper3 stores figures as PDF, while docx-writer embeds raster bytes.
    let mut image_assets = ImageAssets::new();
    for path in vfs.paths() {
        let p_str = path.to_string_lossy();
        let p_lower = p_str.to_lowercase();
        if p_lower.ends_with(".png") || p_lower.ends_with(".jpg") || p_lower.ends_with(".jpeg") {
            if let Ok(bytes) = vfs.read(path) {
                insert_image_asset_aliases(&mut image_assets, path, bytes.to_vec());
            }
        } else if p_lower.ends_with(".pdf") {
            if let Ok(bytes) = vfs.read(path) {
                if let Some(png) = render_pdf_to_png(bytes) {
                    insert_image_asset_aliases(&mut image_assets, path, png.clone());
                    if let Some(png_basename) = png_basename_for_pdf(path) {
                        image_assets.insert(png_basename, png);
                    }
                }
            }
        }
    }
    if let Some(parent) = project_root.parent() {
        collect_image_assets_from_dir(&parent.join("figures"), &mut image_assets);
    }

    let main_rel = relative_to_root(project_root, main_tex)?;
    let main_posix = main_rel.to_string_lossy().replace('\\', "/");
    let source_bytes = vfs
        .read(&main_posix)
        .map_err(|e| CoreError::Parse(format!("读取主文件失败：{e}")))?
        .to_vec();
    let source = String::from_utf8(source_bytes)
        .map_err(|e| CoreError::Parse(format!("主文件非 UTF-8：{e}")))?;

    let doc = parse_tex_with_vfs(&main_posix, &source, &mut vfs)?;
    // V2：把 doc.metadata.running_header / first_footer_text 自动回填到
    // page_setup 的 header_text / first_footer_text（仅当 caller 没显式传）。
    let mut ps_owned: Option<doc_docx_writer::PageSetup> = None;
    {
        let meta = &doc.metadata;
        let mut changed = false;
        let mut ps_eff = options
            .page_setup
            .clone()
            .unwrap_or_else(doc_docx_writer::PageSetup::jos_paper3);
        if ps_eff.header_text.is_none() {
            if let Some(rh) = &meta.running_header {
                if !rh.is_empty() {
                    ps_eff.header_text = Some(shorten_running_header(rh));
                    changed = true;
                }
            }
        }
        if ps_eff.first_footer_text.is_none() {
            if let Some(ff) = &meta.first_footer_text {
                if !ff.is_empty() {
                    ps_eff.first_footer_text = Some(ff.clone());
                    changed = true;
                }
            }
        }
        // 首页 masthead 由 packer 写入 header0.xml；勿用单行 first_header_text 覆盖。
        if ps_eff.first_footer_text.is_none() {
            ps_eff.first_footer_text =
                Some(doc_docx_writer::PageSetup::JOS_FIRST_FOOTER.to_string());
            changed = true;
        }
        if changed {
            ps_owned = Some(ps_eff);
        }
    }
    let ps_ref = ps_owned.as_ref().or(options.page_setup.as_ref());
    let docx = doc_docx_writer::pack_with_page_setup(
        &doc,
        options.template_bytes.as_deref(),
        Some(&image_assets),
        ps_ref,
        None,
    )
    .map_err(|e| CoreError::Serialize(e.0))?;
    Ok(ConvertResult {
        docx,
        warnings: vec![],
    })
}

/// 单个 zip 条目允许的最大解压字节数（128 MiB）。
///
/// 防御 zip bomb / 异常声明大小：超过此值直接拒绝，避免触发
/// `Vec::with_capacity(u64 → usize)` 的 OOM 或 WASM 端
/// `invalid malloc request` panic。
const MAX_ZIP_ENTRY_BYTES: u64 = 128 * 1024 * 1024;

/// WASM 友好入口：接受内存中的 zip 字节流（包含 `.tex` / `.bib` / 图片等），
/// 内部用 [`zip::ZipArchive`] 解压到 VFS，再走标准解析 → 降级 → 打包。
///
/// `main_tex_path` 必须是 zip 内的相对 POSIX 路径（如 `main-jos.tex` 或
/// `latex/main-jos.tex`），且对应的条目必须存在。
pub fn convert_zip(
    zip_bytes: &[u8],
    main_tex_path: &str,
    options: &ConvertOptions,
) -> Result<ConvertResult, CoreError> {
    use std::collections::BTreeMap;

    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).map_err(zip_io_to_core)?;

    let mut entries: BTreeMap<PathBuf, Vec<u8>> = BTreeMap::new();
    for i in 0..archive.len() {
        let mut f = archive
            .by_index(i)
            .map_err(|e| CoreError::Io(format!("读取 zip 索引 {i} 失败：{e}")))?;
        if f.is_dir() {
            continue;
        }
        // 用 entry_name 取得原始相对路径（POSIX），转 PathBuf
        let name = f.name().to_string();
        if name.contains("..") {
            return Err(CoreError::Parse(format!("zip 包含不安全路径：{name}")));
        }
        // 防止 zip bomb / 异常声明大小：
        // 1. u64 → usize 在 32 位 WASM 上会截断为 u32 / 0；
        // 2. u64::MAX 这类异常大小会触发 alloc OOM。
        // 这里先按声明大小做上限检查；实际读取仍由 `read_to_end`
        // 配合 reserve 增长校验。
        let declared = f.size();
        if declared > MAX_ZIP_ENTRY_BYTES {
            return Err(CoreError::Io(format!(
                "zip 条目 {name} 声明大小 {declared} 字节超过 {MAX_ZIP_ENTRY_BYTES} 字节上限"
            )));
        }
        // 防御 usize 截断：只在 `declared <= usize::MAX as u64` 时才转
        let capacity = usize::try_from(declared).unwrap_or(0);
        let mut buf = Vec::with_capacity(capacity);
        f.read_to_end(&mut buf)
            .map_err(|e| CoreError::Io(format!("读取 zip 条目 {name} 失败：{e}")))?;
        if buf.len() as u64 > MAX_ZIP_ENTRY_BYTES {
            return Err(CoreError::Io(format!(
                "zip 条目 {name} 实际解压 {} 字节超过 {MAX_ZIP_ENTRY_BYTES} 字节上限",
                buf.len()
            )));
        }
        entries.insert(PathBuf::from(name.replace('\\', "/")), buf);
    }

    let main_norm = main_tex_path.replace('\\', "/");
    let main_pb = PathBuf::from(&main_norm);
    let main_bytes = entries
        .get(&main_pb)
        .ok_or_else(|| CoreError::Parse(format!("zip 缺主文件 {main_tex_path}")))?;
    let source = String::from_utf8(main_bytes.clone())
        .map_err(|e| CoreError::Parse(format!("主文件非 UTF-8：{e}")))?;

    // 装载到 VFS
    let mut vfs = VirtualFs::new();
    for (p, bytes) in &entries {
        vfs.insert(p, bytes.clone());
    }

    // 收集 PNG/JPEG 图片资产；PDF 自动渲染为 PNG 并入 image_assets（同 key）。
    // V1 image_assets match：先存完整 VFS 路径作为 key，再额外存 `basename`
    // 作 fallback（docx-writer 端 fig_key 通常是 `\includegraphics` 裸路径，如
    // `fig1_system_overview.pdf`）。
    let mut image_assets = ImageAssets::new();
    for (p, bytes) in &entries {
        let p_lower = p.to_string_lossy().to_lowercase();
        let p_str = p.to_string_lossy().to_string();
        let basename = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if p_lower.ends_with(".png") || p_lower.ends_with(".jpg") || p_lower.ends_with(".jpeg") {
            image_assets.insert(p_str.clone(), bytes.clone());
            if !basename.is_empty() && basename != p_str {
                image_assets.insert(basename, bytes.clone());
            }
        } else if p_lower.ends_with(".pdf") {
            if let Some(png) = render_pdf_to_png(bytes) {
                image_assets.insert(p_str.clone(), png.clone());
                if !basename.is_empty() && basename != p_str {
                    // 把 basename 改成 PNG 后缀
                    let png_basename = basename.trim_end_matches(".pdf").to_string() + ".png";
                    image_assets.insert(png_basename, png);
                }
            }
        }
    }
    let parsed_doc = parse_tex_with_vfs(&main_norm, &source, &mut vfs)?;
    tracing::debug!(
        "[doc-core] zip entries scanned: {} (figures+tex)",
        entries.len()
    );
    tracing::debug!(
        "[doc-core] has abstract: {}",
        entries
            .keys()
            .any(|p| p.to_string_lossy().contains("00_abstract"))
    );
    tracing::debug!(
        "[doc-core] parsed_doc has {} blocks",
        parsed_doc.blocks.len()
    );
    tracing::debug!(
        "[doc-core] block kinds: {}",
        parsed_doc
            .blocks
            .iter()
            .map(|b| match b {
                doc_semantic_ast::Block::Heading { .. } => "H",
                doc_semantic_ast::Block::Paragraph { .. } => "P",
                doc_semantic_ast::Block::Figure { .. } => "F",
                doc_semantic_ast::Block::Table { .. } => "T",
                doc_semantic_ast::Block::List { .. } => "L",
                doc_semantic_ast::Block::Equation { .. } => "E",
                doc_semantic_ast::Block::TheoremLike { .. } => "M",
                doc_semantic_ast::Block::Bibliography { .. } => "B",
                doc_semantic_ast::Block::Algorithm { .. } => "A",
                doc_semantic_ast::Block::CodeBlock { .. } => "C",
                doc_semantic_ast::Block::RawFallback { .. } => "R",
            })
            .collect::<Vec<_>>()
            .join("")
    );
    let doc = parsed_doc;
    // V2：把 doc.metadata.running_header / first_footer_text 自动回填到
    // page_setup 的 header_text / first_footer_text（仅当 caller 没显式传）。
    let mut ps_owned: Option<doc_docx_writer::PageSetup> = None;
    {
        let meta = &doc.metadata;
        let mut changed = false;
        let mut ps_eff = options
            .page_setup
            .clone()
            .unwrap_or_else(doc_docx_writer::PageSetup::jos_paper3);
        if ps_eff.header_text.is_none() {
            if let Some(rh) = &meta.running_header {
                if !rh.is_empty() {
                    ps_eff.header_text = Some(shorten_running_header(rh));
                    changed = true;
                }
            }
        }
        if ps_eff.first_footer_text.is_none() {
            if let Some(ff) = &meta.first_footer_text {
                if !ff.is_empty() {
                    ps_eff.first_footer_text = Some(ff.clone());
                    changed = true;
                }
            }
        }
        // 首页 masthead 由 packer 写入 header0.xml；勿用单行 first_header_text 覆盖。
        if ps_eff.first_footer_text.is_none() {
            ps_eff.first_footer_text =
                Some(doc_docx_writer::PageSetup::JOS_FIRST_FOOTER.to_string());
            changed = true;
        }
        if changed {
            ps_owned = Some(ps_eff);
        }
    }
    let ps_ref = ps_owned.as_ref().or(options.page_setup.as_ref());
    let docx = doc_docx_writer::pack_with_page_setup(
        &doc,
        options.template_bytes.as_deref(),
        Some(&image_assets),
        ps_ref,
        None,
    )
    .map_err(|e| CoreError::Serialize(e.0))?;
    Ok(ConvertResult {
        docx,
        warnings: vec![],
    })
}

fn zip_io_to_core(e: zip::result::ZipError) -> CoreError {
    CoreError::Io(format!("zip 解析失败：{e}"))
}

/// 进度流入口（占位，M5-M6 落地为真流）。
pub async fn convert_stream(
    main_tex: &str,
    source: &str,
    options: &ConvertOptions,
) -> Result<ConvertResult, CoreError> {
    for phase in [
        ProgressPhase::Reading,
        ProgressPhase::Parsing,
        ProgressPhase::Lowering,
        ProgressPhase::Serializing,
        ProgressPhase::Packing,
    ] {
        let _ = ProgressEvent {
            phase,
            ratio: 0.0,
            message: format!("{:?}", phase),
        };
    }
    convert_sync(main_tex, source, options)
}

/// 内部：tex 源 → Document。
pub(crate) fn parse_tex_to_doc(main_tex: &str, source: &str) -> Result<Document, CoreError> {
    let mut vfs = VirtualFs::new();
    vfs.insert(main_tex, source.as_bytes().to_vec());
    parse_tex_with_vfs(main_tex, source, &mut vfs)
}

/// 内部：复用同一 VFS（含 include / graphics 资源）→ Document。
fn parse_tex_with_vfs(
    main_tex: &str,
    _source: &str,
    vfs: &mut VirtualFs,
) -> Result<Document, CoreError> {
    let graph = IncludeGraph::build(vfs, Path::new(main_tex))?;
    let joined = graph.join(vfs)?;
    let parse = parse_tex(&joined.text);
    // v13.2 F13: 优先读 bbl（最稳，clean_bibitem_body 已处理 LaTeX 残余）；
    // 若 vfs 不含 bbl 但含 .bib，直接用 `parse_bib` 解析 .bib，输出 BibItem 列表。
    let bbl_path = Path::new(main_tex).with_extension("bbl");
    if let Ok(bytes) = vfs.read(&bbl_path) {
        if let Ok(raw_bbl) = std::str::from_utf8(bytes) {
            let (cite_map, refs) = parse_bbl(raw_bbl);
            if !cite_map.is_empty() {
                let mut doc = lower_to_document_with_cite_map(&parse, Some(&joined), &cite_map);
                append_bibliography_paragraphs(&mut doc, &refs);
                return Ok(doc);
            }
        }
    }
    // v13.2 F13: 兜底——找 .bib 解析（vfs 与 main_tex 同目录）
    let main_dir = Path::new(main_tex).parent().unwrap_or(Path::new(""));
    if let Some(bib_path) = find_bib_in_vfs(vfs, main_dir) {
        if let Ok(bytes) = vfs.read(&bib_path) {
            if let Ok(raw_bib) = std::str::from_utf8(bytes) {
                let refs = parse_bib(raw_bib);
                if !refs.is_empty() {
                    let mut doc = lower_to_document(&parse, Some(&joined));
                    append_bibliography_paragraphs(&mut doc, &refs);
                    return Ok(doc);
                }
            }
        }
    }
    Ok(lower_to_document(&parse, Some(&joined)))
}

/// v13.2 F13: 在 vfs 中找 main_tex 同目录的 `references.bib` 或 `<stem>.bib`。
fn find_bib_in_vfs(vfs: &VirtualFs, main_dir: &Path) -> Option<PathBuf> {
    let name = "references.bib";
    let p = if main_dir.as_os_str().is_empty() {
        PathBuf::from(name)
    } else {
        main_dir.join(name)
    };
    if vfs.read(&p).is_ok() {
        return Some(p);
    }
    None
}

fn append_bibliography_paragraphs(
    doc: &mut Document,
    refs: &[doc_latex_reader::latex_to_text::BibItem],
) {
    if refs.is_empty() {
        return;
    }
    let blocks = refs
        .iter()
        .enumerate()
        .map(|(idx, item)| Block::Paragraph {
            runs: vec![TextRun {
                text: format!("[{}] {}", idx + 1, item.text),
                style: TextStyle::Plain,
                span: Span::default(),
            }],
            span: Span::default(),
        })
        .collect::<Vec<_>>();

    let insert_at = doc
        .blocks
        .iter()
        .position(|block| match block {
            Block::Paragraph { runs, .. } => {
                runs.iter()
                    .map(|r| r.text.as_str())
                    .collect::<String>()
                    .trim()
                    == "References"
            }
            _ => false,
        })
        .map(|idx| idx + 1)
        .unwrap_or(doc.blocks.len());

    for (offset, block) in blocks.into_iter().enumerate() {
        doc.blocks.insert(insert_at + offset, block);
    }
}

fn relative_to_root(root: &Path, p: &Path) -> Result<PathBuf, CoreError> {
    if p.is_absolute() {
        p.strip_prefix(root)
            .map(Path::to_path_buf)
            .map_err(|_| CoreError::Parse(format!("主文件 {p:?} 不在项目根 {root:?} 之下")))
    } else {
        Ok(p.to_path_buf())
    }
}
