//! Semantic TeX compiler engine.
//!
//! This crate is the facade for the next-generation TeX -> DOCX pipeline. It
//! keeps the current rule-based LaTeX reader and DOCX writer behind explicit
//! compiler stages, so later LuaHook/XDV/OMML implementations can replace
//! individual stages without changing callers.

#![forbid(unsafe_code)]

use std::io::Read;
use std::path::{Path, PathBuf};

use doc_latex_reader::{
    lower_to_document, lower_to_document_with_cite_map, parse_bbl, parse_bib, parse_tex,
    IncludeGraph, JoinedStream, Parse,
};
use doc_semantic_ast::{
    Block, Document, SourceBundle, SourceFile, Span, StandardDocument, TextRun, TextStyle,
};
use doc_utils::{ImageAssets, VirtualFs};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Built-in conversion profiles.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineProfile {
    /// General article/report/book style documents.
    #[default]
    GenericArticle,
    /// Chinese academic papers, including CTeX-based templates.
    ChineseAcademic,
    /// Journal of Software / 软件学报 oriented profile.
    JosPaper,
    /// Medical journal manuscripts with strict title/abstract/table needs.
    MedicalJournal,
}

impl EngineProfile {
    pub fn id(self) -> &'static str {
        match self {
            Self::GenericArticle => "generic-article",
            Self::ChineseAcademic => "chinese-academic",
            Self::JosPaper => "jos-paper",
            Self::MedicalJournal => "medical-journal",
        }
    }
}

/// Options controlling semantic collection and DOCX rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileOptions {
    pub profile: EngineProfile,
    pub template_bytes: Option<Vec<u8>>,
    pub page_setup: Option<doc_docx_writer::PageSetup>,
    pub collect_standard_ast: bool,
    pub enable_bibliography: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            profile: EngineProfile::ChineseAcademic,
            template_bytes: None,
            page_setup: None,
            collect_standard_ast: true,
            enable_bibliography: true,
        }
    }
}

/// High-level compiler facade.
#[derive(Debug, Default, Clone)]
pub struct SemanticTexEngine;

impl SemanticTexEngine {
    pub fn new() -> Self {
        Self
    }

    /// Compile a single in-memory TeX source to DOCX.
    pub fn compile_source_to_docx(
        &self,
        main_tex: &str,
        source: &str,
        options: &CompileOptions,
    ) -> Result<CompileArtifact, EngineError> {
        let mut vfs = VirtualFs::new();
        vfs.insert(main_tex, source.as_bytes().to_vec());
        self.compile_vfs_to_docx(main_tex, &mut vfs, options)
    }

    /// Compile a real project directory to DOCX.
    pub fn compile_dir_to_docx(
        &self,
        project_root: &Path,
        main_tex: &Path,
        options: &CompileOptions,
    ) -> Result<CompileArtifact, EngineError> {
        let mut vfs = VirtualFs::new();
        vfs.mount_dir(project_root)
            .map_err(|e| EngineError::Io(e.to_string()))?;
        if let Some(parent) = project_root.parent() {
            let sibling_figures = parent.join("figures");
            if sibling_figures.is_dir() {
                vfs.mount_dir(&sibling_figures)
                    .map_err(|e| EngineError::Io(e.to_string()))?;
            }
        }
        let main_rel = relative_to_root(project_root, main_tex)?;
        let main_posix = path_to_posix(&main_rel);
        self.compile_vfs_to_docx(&main_posix, &mut vfs, options)
    }

    /// Compile a zip package containing TeX sources and assets to DOCX.
    pub fn compile_zip_to_docx(
        &self,
        zip_bytes: &[u8],
        main_tex_path: &str,
        options: &CompileOptions,
    ) -> Result<CompileArtifact, EngineError> {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes))
            .map_err(|e| EngineError::Zip(e.to_string()))?;
        let mut vfs = VirtualFs::new();

        for idx in 0..archive.len() {
            let mut file = archive
                .by_index(idx)
                .map_err(|e| EngineError::Zip(format!("读取 zip 索引 {idx} 失败：{e}")))?;
            if file.is_dir() {
                continue;
            }
            let name = file.name().replace('\\', "/");
            if name.contains("..") {
                return Err(EngineError::Parse(format!("zip 包含不安全路径：{name}")));
            }
            let mut bytes = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut bytes)
                .map_err(|e| EngineError::Io(e.to_string()))?;
            vfs.insert(name, bytes);
        }

        let main_norm = main_tex_path.replace('\\', "/");
        if !vfs.contains(&main_norm) {
            return Err(EngineError::Parse(format!("zip 缺主文件 {main_tex_path}")));
        }
        self.compile_vfs_to_docx(&main_norm, &mut vfs, options)
    }

    /// Compile a populated VFS to DOCX.
    pub fn compile_vfs_to_docx(
        &self,
        main_tex: &str,
        vfs: &mut VirtualFs,
        options: &CompileOptions,
    ) -> Result<CompileArtifact, EngineError> {
        let mut graph = self.compile_vfs_to_graph(main_tex, vfs, options)?;
        graph.report.push(
            CompileStage::DocxRender,
            StageStatus::Completed,
            "DOCX renderer packed document.xml, styles.xml, relationships and media",
        );

        let docx = doc_docx_writer::pack_with_page_setup(
            &graph.document,
            options.template_bytes.as_deref(),
            Some(&graph.image_assets),
            options.page_setup.as_ref(),
        )
        .map_err(|e| EngineError::Serialize(e.to_string()))?;
        graph.report.docx_bytes = docx.len();

        Ok(CompileArtifact {
            docx,
            document: graph.document,
            standard_document: graph.standard_document,
            report: graph.report,
        })
    }

    /// Compile TeX inputs to the semantic document graph without rendering DOCX.
    pub fn compile_vfs_to_graph(
        &self,
        main_tex: &str,
        vfs: &mut VirtualFs,
        options: &CompileOptions,
    ) -> Result<DocumentGraph, EngineError> {
        let mut report = CompileReport::new(options.profile);
        report.push(
            CompileStage::SourceMount,
            StageStatus::Completed,
            format!("mounted {} VFS entries", vfs.paths().count()),
        );

        let graph = IncludeGraph::build(vfs, Path::new(main_tex))?;
        let joined = graph.join(vfs)?;
        report.push(
            CompileStage::IncludeGraph,
            StageStatus::Completed,
            format!("joined source stream has {} bytes", joined.text.len()),
        );

        let parse = parse_tex(&joined.text);
        report.push(
            CompileStage::TexParse,
            StageStatus::Completed,
            "parsed TeX stream with the current Logos/Rowan reader",
        );

        let document = lower_semantic_document(main_tex, vfs, &parse, &joined, options)?;
        report.block_count = document.blocks.len();
        report.push(
            CompileStage::SemanticCollect,
            StageStatus::Completed,
            format!("collected {} semantic blocks", document.blocks.len()),
        );

        let image_assets = collect_image_assets_from_vfs(vfs);
        report.image_asset_count = image_assets.len();

        let source = source_bundle(main_tex, vfs);
        let standard_document = if options.collect_standard_ast {
            let standard =
                StandardDocument::from_legacy_document(&document, source, options.profile.id());
            report.push(
                CompileStage::DocumentGraph,
                StageStatus::Completed,
                format!(
                    "document graph contains {} block nodes",
                    standard.blocks.len()
                ),
            );
            Some(standard)
        } else {
            report.push(
                CompileStage::DocumentGraph,
                StageStatus::Skipped,
                "standard AST collection disabled",
            );
            None
        };

        Ok(DocumentGraph {
            document,
            standard_document,
            image_assets,
            report,
        })
    }
}

/// Rendered output and intermediate semantic models.
#[derive(Debug, Clone)]
pub struct CompileArtifact {
    pub docx: Vec<u8>,
    pub document: Document,
    pub standard_document: Option<StandardDocument>,
    pub report: CompileReport,
}

/// Unified document graph used between semantic collection and renderers.
#[derive(Debug, Clone)]
pub struct DocumentGraph {
    pub document: Document,
    pub standard_document: Option<StandardDocument>,
    pub image_assets: ImageAssets,
    pub report: CompileReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileReport {
    pub profile: EngineProfile,
    pub stages: Vec<StageReport>,
    pub diagnostics: Vec<EngineDiagnostic>,
    pub block_count: usize,
    pub image_asset_count: usize,
    pub docx_bytes: usize,
}

impl CompileReport {
    pub fn new(profile: EngineProfile) -> Self {
        Self {
            profile,
            stages: Vec::new(),
            diagnostics: Vec::new(),
            block_count: 0,
            image_asset_count: 0,
            docx_bytes: 0,
        }
    }

    pub fn push(&mut self, stage: CompileStage, status: StageStatus, message: impl Into<String>) {
        self.stages.push(StageReport {
            stage,
            status,
            message: message.into(),
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompileStage {
    SourceMount,
    IncludeGraph,
    TexParse,
    SemanticCollect,
    DocumentGraph,
    DocxRender,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageStatus {
    Completed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageReport {
    pub stage: CompileStage,
    pub status: StageStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum EngineError {
    #[error("IO 错误：{0}")]
    Io(String),
    #[error("解析错误：{0}")]
    Parse(String),
    #[error("序列化错误：{0}")]
    Serialize(String),
    #[error("zip 错误：{0}")]
    Zip(String),
    #[error("不支持的操作：{0}")]
    Unsupported(String),
}

impl From<doc_utils::DocError> for EngineError {
    fn from(err: doc_utils::DocError) -> Self {
        match err {
            doc_utils::DocError::Io(e) => Self::Io(e.to_string()),
            doc_utils::DocError::VfsMissing(path) => {
                Self::Parse(format!("VFS 缺失：{}", path.display()))
            }
            doc_utils::DocError::InvalidPath(message) => Self::Parse(message),
            doc_utils::DocError::ImageDecode(message) => Self::Serialize(message),
            doc_utils::DocError::Unsupported(message) => Self::Unsupported(message),
        }
    }
}

impl From<std::io::Error> for EngineError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

fn lower_semantic_document(
    main_tex: &str,
    vfs: &VirtualFs,
    parse: &Parse,
    joined: &JoinedStream,
    options: &CompileOptions,
) -> Result<Document, EngineError> {
    if !options.enable_bibliography {
        return Ok(lower_to_document(parse, Some(joined)));
    }

    let bbl_path = Path::new(main_tex).with_extension("bbl");
    if let Ok(bytes) = vfs.read(&bbl_path) {
        if let Ok(raw_bbl) = std::str::from_utf8(bytes) {
            let (cite_map, refs) = parse_bbl(raw_bbl);
            if !cite_map.is_empty() {
                let mut doc = lower_to_document_with_cite_map(parse, Some(joined), &cite_map);
                append_bibliography_paragraphs(&mut doc, &refs);
                return Ok(doc);
            }
        }
    }

    let main_dir = Path::new(main_tex).parent().unwrap_or(Path::new(""));
    if let Some(bib_path) = find_bib_in_vfs(vfs, main_tex, main_dir) {
        if let Ok(bytes) = vfs.read(&bib_path) {
            if let Ok(raw_bib) = std::str::from_utf8(bytes) {
                let refs = parse_bib(raw_bib);
                if !refs.is_empty() {
                    let mut doc = lower_to_document(parse, Some(joined));
                    append_bibliography_paragraphs(&mut doc, &refs);
                    return Ok(doc);
                }
            }
        }
    }

    Ok(lower_to_document(parse, Some(joined)))
}

fn find_bib_in_vfs(vfs: &VirtualFs, main_tex: &str, main_dir: &Path) -> Option<PathBuf> {
    let mut candidates = vec!["references.bib".to_string()];
    if let Some(stem) = Path::new(main_tex).file_stem().and_then(|s| s.to_str()) {
        candidates.push(format!("{stem}.bib"));
    }

    candidates.into_iter().find_map(|name| {
        let path = if main_dir.as_os_str().is_empty() {
            PathBuf::from(name)
        } else {
            main_dir.join(name)
        };
        vfs.contains(&path).then_some(path)
    })
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
                    .map(|run| run.text.as_str())
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

fn collect_image_assets_from_vfs(vfs: &VirtualFs) -> ImageAssets {
    let mut image_assets = ImageAssets::new();
    for path in vfs.paths() {
        let path_lower = path.to_string_lossy().to_lowercase();
        if path_lower.ends_with(".png")
            || path_lower.ends_with(".jpg")
            || path_lower.ends_with(".jpeg")
        {
            if let Ok(bytes) = vfs.read(path) {
                insert_image_asset_aliases(&mut image_assets, path, bytes.to_vec());
            }
        } else if path_lower.ends_with(".pdf") {
            if let Ok(bytes) = vfs.read(path) {
                if let Some(png) = render_pdf_to_png(bytes) {
                    insert_pdf_image_asset_aliases(&mut image_assets, path, png);
                }
            }
        }
    }
    image_assets
}

fn insert_image_asset_aliases(image_assets: &mut ImageAssets, path: &Path, bytes: Vec<u8>) {
    let path_key = path_to_posix(path);
    image_assets.insert(path_key.clone(), bytes.clone());
    if let Some(basename) = path.file_name().and_then(|name| name.to_str()) {
        if basename != path_key {
            image_assets.insert(basename.to_string(), bytes);
        }
    }
}

fn insert_pdf_image_asset_aliases(image_assets: &mut ImageAssets, path: &Path, png: Vec<u8>) {
    insert_image_asset_aliases(image_assets, path, png.clone());
    let png_path = path.with_extension("png");
    insert_image_asset_aliases(image_assets, &png_path, png);
}

fn render_pdf_to_png(pdf_bytes: &[u8]) -> Option<Vec<u8>> {
    use pdfium_render::prelude::{PdfRenderConfig, Pdfium};

    let bindings = Pdfium::bind_to_system_library().ok()?;
    let pdfium = Pdfium::new(bindings);
    let doc = pdfium.load_pdf_from_byte_slice(pdf_bytes, None).ok()?;
    let page = doc.pages().get(0).ok()?;
    let bitmap = page
        .render_with_config(&PdfRenderConfig::new().set_target_width(1600))
        .ok()?;
    let image = bitmap.as_image();
    let buf: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = image.into_rgba8();
    let dyn_img = image::DynamicImage::ImageRgba8(buf);
    let mut png_bytes = Vec::new();
    dyn_img
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .ok()?;
    Some(png_bytes)
}

fn source_bundle(main_tex: &str, vfs: &VirtualFs) -> SourceBundle {
    SourceBundle {
        main_path: main_tex.to_string(),
        files: vfs
            .paths()
            .map(|path| SourceFile {
                path: path_to_posix(path),
                hash: None,
            })
            .collect(),
    }
}

fn relative_to_root(root: &Path, path: &Path) -> Result<PathBuf, EngineError> {
    if path.is_absolute() {
        path.strip_prefix(root).map(Path::to_path_buf).map_err(|_| {
            EngineError::Parse(format!(
                "主文件 {} 不在项目根 {} 之下",
                path.display(),
                root.display()
            ))
        })
    } else {
        Ok(path.to_path_buf())
    }
}

fn path_to_posix(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
\title{Demo}
\section{Introduction}

A paragraph with \textbf{bold} text.

\begin{equation}
E = mc^2
\end{equation}
"#;

    #[test]
    fn compiles_single_source_to_docx() {
        let engine = SemanticTexEngine::new();
        let artifact = engine
            .compile_source_to_docx("main.tex", SAMPLE, &CompileOptions::default())
            .expect("compile source");

        assert_eq!(&artifact.docx[..4], b"PK\x03\x04");
        assert!(artifact.document.blocks.iter().any(|block| {
            matches!(block, Block::Heading { text, .. } if text == "Introduction")
        }));
        assert!(artifact.standard_document.is_some());
        assert_eq!(
            artifact.report.stages.last().map(|stage| stage.stage),
            Some(CompileStage::DocxRender)
        );
    }

    #[test]
    fn compiles_zip_to_docx() {
        let mut out = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut out);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("main.tex", opts).unwrap();
            use std::io::Write;
            zip.write_all(SAMPLE.as_bytes()).unwrap();
            zip.finish().unwrap();
        }

        let engine = SemanticTexEngine::new();
        let artifact = engine
            .compile_zip_to_docx(out.get_ref(), "main.tex", &CompileOptions::default())
            .expect("compile zip");

        assert_eq!(&artifact.docx[..4], b"PK\x03\x04");
        assert!(artifact.report.block_count >= 2);
    }
}
