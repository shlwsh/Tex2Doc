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

use doc_latex_reader::{lower_to_document, parse_tex, IncludeGraph};
use doc_semantic_ast::Document;
use doc_utils::{ImageAssets, VirtualFs};

use crate::error::CoreError;
use crate::options::ConvertOptions;
use crate::result::{ConvertResult, ProgressEvent, ProgressPhase};

/// 把 PDF 第一页渲染为 PNG（200 dpi），用 pdfium-render。
///
/// V1 设计：zip 内 `\includegraphics{*.pdf}` 时，docx-writer 端只接受 PNG/JPG，
/// 所以这里把 PDF 翻译成 PNG byte 注入 image_assets（同 key）。
fn render_pdf_to_png(pdf_bytes: &[u8]) -> Option<Vec<u8>> {
    use pdfium_render::prelude::Pdfium;
    let pdfium = Pdfium::default();
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
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .ok()?;
    Some(png_bytes)
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

    // Collect PNG/JPEG image assets from VFS
    let mut image_assets = ImageAssets::new();
    for path in vfs.paths() {
        let p_str = path.to_string_lossy();
        let p_lower = p_str.to_lowercase();
        if p_lower.ends_with(".png") || p_lower.ends_with(".jpg") || p_lower.ends_with(".jpeg") {
            if let Ok(bytes) = vfs.read(path) {
                image_assets.insert(p_str.to_string(), bytes.to_vec());
            }
        }
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
    let docx = doc_docx_writer::pack_with_page_setup(
        &doc,
        options.template_bytes.as_deref(),
        Some(&image_assets),
        options.page_setup.as_ref(),
    )
    .map_err(|e| CoreError::Serialize(e.0))?;
    Ok(ConvertResult {
        docx,
        warnings: vec![],
    })
}

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
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf)
            .map_err(|e| CoreError::Io(format!("读取 zip 条目 {name} 失败：{e}")))?;
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
        entries.keys().any(|p| p.to_string_lossy().contains("00_abstract"))
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
                doc_semantic_ast::Block::Bibliography { .. } => "B",
                doc_semantic_ast::Block::RawFallback { .. } => "R",
            })
            .collect::<Vec<_>>()
            .join("")
    );
    let docx = doc_docx_writer::pack_with_page_setup(
        &parsed_doc,
        options.template_bytes.as_deref(),
        Some(&image_assets),
        options.page_setup.as_ref(),
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
    Ok(lower_to_document(&parse, Some(&joined)))
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
