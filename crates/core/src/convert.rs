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
    let docx = doc_docx_writer::pack_with_template(&doc, options.template_bytes.as_deref())
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
    let docx = doc_docx_writer::pack_with_assets(
        &doc,
        options.template_bytes.as_deref(),
        Some(&image_assets),
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

    // 收集 PNG/JPEG 图片资产
    let mut image_assets = ImageAssets::new();
    for (p, bytes) in &entries {
        let p_lower = p.to_string_lossy().to_lowercase();
        if p_lower.ends_with(".png") || p_lower.ends_with(".jpg") || p_lower.ends_with(".jpeg") {
            image_assets.insert(p.to_string_lossy().to_string(), bytes.clone());
        }
    }

    let doc = parse_tex_with_vfs(&main_norm, &source, &mut vfs)?;
    let docx = doc_docx_writer::pack_with_assets(
        &doc,
        options.template_bytes.as_deref(),
        Some(&image_assets),
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
