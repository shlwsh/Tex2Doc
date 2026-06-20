//! Semantic TeX compiler engine.
//!
//! This crate is the facade for the next-generation TeX -> DOCX pipeline. It
//! keeps the current rule-based LaTeX reader and DOCX writer behind explicit
//! compiler stages, so later LuaHook/XDV/OMML implementations can replace
//! individual stages without changing callers.

#![forbid(unsafe_code)]

use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

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

/// Semantic collection backend selection.
///
/// `Auto` currently resolves to [`SemanticBackendKind::RuleBased`] unless a
/// future runtime backend is explicitly enabled and available.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticBackendKind {
    #[default]
    Auto,
    RuleBased,
    XeLaTeXHook,
    LuaTeXNode,
}

impl SemanticBackendKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::RuleBased => "rule-based",
            Self::XeLaTeXHook => "xelatex-hook",
            Self::LuaTeXNode => "luatex-node",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendSelectionReport {
    pub requested: SemanticBackendKind,
    pub selected: SemanticBackendKind,
    pub fallback_from: Option<SemanticBackendKind>,
    pub reason: String,
}

impl BackendSelectionReport {
    fn new(
        requested: SemanticBackendKind,
        selected: SemanticBackendKind,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            requested,
            selected,
            fallback_from: None,
            reason: reason.into(),
        }
    }

    fn fallback(
        requested: SemanticBackendKind,
        fallback_from: SemanticBackendKind,
        selected: SemanticBackendKind,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            requested,
            selected,
            fallback_from: Some(fallback_from),
            reason: reason.into(),
        }
    }
}

impl Default for BackendSelectionReport {
    fn default() -> Self {
        Self::new(
            SemanticBackendKind::Auto,
            SemanticBackendKind::RuleBased,
            "default rule-based backend",
        )
    }
}

/// Options controlling semantic collection and DOCX rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileOptions {
    pub profile: EngineProfile,
    pub semantic_backend: SemanticBackendKind,
    pub allow_backend_fallback: bool,
    pub template_bytes: Option<Vec<u8>>,
    pub page_setup: Option<doc_docx_writer::PageSetup>,
    pub collect_standard_ast: bool,
    pub enable_bibliography: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            profile: EngineProfile::ChineseAcademic,
            semantic_backend: SemanticBackendKind::Auto,
            allow_backend_fallback: true,
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

        let mut artifact = collect_with_selected_backend(main_tex, vfs, options, &mut report)?;
        report.semantic_event_count = artifact.events.len();
        report.layout_node_count = artifact
            .layout
            .as_ref()
            .map_or(0, |layout| layout.nodes.len());
        report.diagnostics.append(&mut artifact.diagnostics);

        Ok(DocumentGraph {
            document: artifact.document,
            standard_document: artifact.standard_document,
            image_assets: artifact.image_assets,
            semantic_events: artifact.events,
            layout: artifact.layout,
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
    pub semantic_events: Vec<SemanticEvent>,
    pub layout: Option<LayoutGraph>,
    pub report: CompileReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileReport {
    pub profile: EngineProfile,
    pub backend: BackendSelectionReport,
    pub stages: Vec<StageReport>,
    pub diagnostics: Vec<EngineDiagnostic>,
    pub block_count: usize,
    pub image_asset_count: usize,
    pub semantic_event_count: usize,
    pub layout_node_count: usize,
    pub docx_bytes: usize,
}

impl CompileReport {
    pub fn new(profile: EngineProfile) -> Self {
        Self {
            profile,
            backend: BackendSelectionReport::default(),
            stages: Vec::new(),
            diagnostics: Vec::new(),
            block_count: 0,
            image_asset_count: 0,
            semantic_event_count: 0,
            layout_node_count: 0,
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

impl EngineDiagnostic {
    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Info,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum SemanticEvent {
    #[serde(rename = "heading")]
    Heading {
        level: u8,
        text: String,
        label: Option<String>,
        span: Option<SourceSpan>,
    },
    #[serde(rename = "paragraph")]
    Paragraph {
        text: String,
        span: Option<SourceSpan>,
    },
    #[serde(rename = "figure")]
    Figure {
        path: String,
        caption: Option<String>,
        label: Option<String>,
        width_expr: Option<String>,
        span: Option<SourceSpan>,
    },
    #[serde(rename = "table")]
    Table {
        caption: Option<String>,
        label: Option<String>,
        span: Option<SourceSpan>,
    },
    #[serde(rename = "equation")]
    Equation {
        latex: String,
        label: Option<String>,
        display: bool,
        span: Option<SourceSpan>,
    },
    #[serde(rename = "citation")]
    Citation {
        keys: Vec<String>,
        span: Option<SourceSpan>,
    },
    #[serde(rename = "label")]
    Label {
        key: String,
        span: Option<SourceSpan>,
    },
    #[serde(rename = "reference")]
    Reference {
        kind: String,
        key: String,
        span: Option<SourceSpan>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceSpan {
    pub path: Option<String>,
    pub start: Option<u32>,
    pub end: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayoutGraph {
    pub nodes: Vec<LayoutNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayoutNode {
    pub id: String,
    pub kind: String,
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendAvailability {
    pub available: bool,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct SemanticBackendArtifact {
    pub document: Document,
    pub standard_document: Option<StandardDocument>,
    pub image_assets: ImageAssets,
    pub events: Vec<SemanticEvent>,
    pub layout: Option<LayoutGraph>,
    pub diagnostics: Vec<EngineDiagnostic>,
}

pub trait SemanticBackend {
    fn kind(&self) -> SemanticBackendKind;

    fn is_available(&self) -> BackendAvailability;

    fn collect(
        &self,
        main_tex: &str,
        vfs: &mut VirtualFs,
        options: &CompileOptions,
        report: &mut CompileReport,
    ) -> Result<SemanticBackendArtifact, EngineError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RuleBasedBackend;

impl SemanticBackend for RuleBasedBackend {
    fn kind(&self) -> SemanticBackendKind {
        SemanticBackendKind::RuleBased
    }

    fn is_available(&self) -> BackendAvailability {
        BackendAvailability {
            available: true,
            reason: "rule-based collector is built in".to_string(),
        }
    }

    fn collect(
        &self,
        main_tex: &str,
        vfs: &mut VirtualFs,
        options: &CompileOptions,
        report: &mut CompileReport,
    ) -> Result<SemanticBackendArtifact, EngineError> {
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

        Ok(SemanticBackendArtifact {
            document,
            standard_document,
            image_assets,
            events: Vec::new(),
            layout: None,
            diagnostics: Vec::new(),
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct XeLaTeXHookBackend;

impl SemanticBackend for XeLaTeXHookBackend {
    fn kind(&self) -> SemanticBackendKind {
        SemanticBackendKind::XeLaTeXHook
    }

    fn is_available(&self) -> BackendAvailability {
        command_available("xelatex")
    }

    fn collect(
        &self,
        main_tex: &str,
        vfs: &mut VirtualFs,
        options: &CompileOptions,
        report: &mut CompileReport,
    ) -> Result<SemanticBackendArtifact, EngineError> {
        let events = collect_runtime_events(RuntimeEngine::XeLaTeX, main_tex, vfs)?;
        let event_count = events.len();
        let mut artifact = RuleBasedBackend.collect(main_tex, vfs, options, report)?;
        artifact.events = events;
        artifact.diagnostics.push(EngineDiagnostic::info(
            "runtime_semantic_events",
            format!("XeLaTeXHookBackend collected {event_count} semantic events"),
        ));
        Ok(artifact)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LuaTeXNodeBackend;

impl SemanticBackend for LuaTeXNodeBackend {
    fn kind(&self) -> SemanticBackendKind {
        SemanticBackendKind::LuaTeXNode
    }

    fn is_available(&self) -> BackendAvailability {
        let lualatex = command_available("lualatex");
        if lualatex.available {
            lualatex
        } else {
            command_available("luatex")
        }
    }

    fn collect(
        &self,
        main_tex: &str,
        vfs: &mut VirtualFs,
        options: &CompileOptions,
        report: &mut CompileReport,
    ) -> Result<SemanticBackendArtifact, EngineError> {
        let events = collect_runtime_events(RuntimeEngine::LuaLaTeX, main_tex, vfs)?;
        let event_count = events.len();
        let mut artifact = RuleBasedBackend.collect(main_tex, vfs, options, report)?;
        artifact.events = events;
        artifact.layout = Some(LayoutGraph::default());
        artifact.diagnostics.push(EngineDiagnostic::info(
            "runtime_semantic_events",
            format!("LuaTeXNodeBackend collected {event_count} semantic events"),
        ));
        Ok(artifact)
    }
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TemplateSignals {
    has_tex_source: bool,
    xetex_required: bool,
    luatex_preferred: bool,
    reasons: Vec<&'static str>,
}

impl TemplateSignals {
    fn observe(&mut self, text: &str) {
        self.has_tex_source = true;
        let lower = text.to_ascii_lowercase();
        let compact = lower
            .chars()
            .filter(|ch| !ch.is_ascii_whitespace())
            .collect::<String>();

        if compact.contains("\\documentclass{ctex")
            || (compact.contains("\\documentclass") && compact.contains("{ctexart}"))
            || (compact.contains("\\documentclass") && compact.contains("{ctexrep}"))
            || (compact.contains("\\documentclass") && compact.contains("{ctexbook}"))
            || compact.contains("\\usepackage{ctex")
        {
            self.mark_xetex("ctex class/package");
        }

        if compact.contains("\\usepackage{xecjk") || compact.contains("\\requirepackage{xecjk") {
            self.mark_xetex("xeCJK package");
        }

        if compact.contains("\\usepackage{fontspec")
            || compact.contains("\\setmainfont")
            || compact.contains("\\setcjkmainfont")
            || compact.contains("\\xetex")
        {
            self.mark_xetex("fontspec/XeTeX font command");
        }

        if compact.contains("\\directlua")
            || compact.contains("\\usepackage{luatexja")
            || compact.contains("\\luatex")
        {
            self.luatex_preferred = true;
            self.push_reason("LuaTeX feature");
        }
    }

    fn mark_xetex(&mut self, reason: &'static str) {
        self.xetex_required = true;
        self.push_reason(reason);
    }

    fn push_reason(&mut self, reason: &'static str) {
        if !self.reasons.contains(&reason) {
            self.reasons.push(reason);
        }
    }

    fn reason_summary(&self) -> String {
        if self.reasons.is_empty() {
            "no XeTeX-only or LuaTeX-specific template feature detected".to_string()
        } else {
            format!("detected {}", self.reasons.join(", "))
        }
    }
}

#[derive(Debug, Clone)]
struct RuntimeAvailabilitySnapshot {
    xelatex: BackendAvailability,
    lualatex: BackendAvailability,
}

impl RuntimeAvailabilitySnapshot {
    fn detect() -> Self {
        Self {
            xelatex: command_available("xelatex"),
            lualatex: command_available("lualatex"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AutoBackendSelection {
    kind: SemanticBackendKind,
    reason: String,
}

fn select_auto_backend(vfs: &VirtualFs) -> AutoBackendSelection {
    let signals = collect_template_signals(vfs);
    select_auto_backend_with_availability(&signals, RuntimeAvailabilitySnapshot::detect())
}

fn collect_template_signals(vfs: &VirtualFs) -> TemplateSignals {
    let mut signals = TemplateSignals::default();
    for path in vfs.paths() {
        if !is_tex_like_path(path) {
            continue;
        }
        let Ok(bytes) = vfs.read(path) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(bytes) else {
            continue;
        };
        signals.observe(text);
    }
    signals
}

fn is_tex_like_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "tex" | "sty" | "cls" | "ltx"
            )
        })
        .unwrap_or(false)
}

fn select_auto_backend_with_availability(
    signals: &TemplateSignals,
    availability: RuntimeAvailabilitySnapshot,
) -> AutoBackendSelection {
    let signal_reason = signals.reason_summary();

    if !signals.has_tex_source {
        return AutoBackendSelection {
            kind: SemanticBackendKind::RuleBased,
            reason: "Auto selected RuleBasedBackend: no TeX-like source was available for runtime feature detection".to_string(),
        };
    }

    if signals.xetex_required {
        if availability.xelatex.available {
            return AutoBackendSelection {
                kind: SemanticBackendKind::XeLaTeXHook,
                reason: format!(
                    "Auto selected XeLaTeXHookBackend: {}; {}",
                    signal_reason, availability.xelatex.reason
                ),
            };
        }

        return AutoBackendSelection {
            kind: SemanticBackendKind::RuleBased,
            reason: format!(
                "Auto selected RuleBasedBackend: {}; xelatex unavailable: {}",
                signal_reason, availability.xelatex.reason
            ),
        };
    }

    if signals.luatex_preferred {
        if availability.lualatex.available {
            return AutoBackendSelection {
                kind: SemanticBackendKind::LuaTeXNode,
                reason: format!(
                    "Auto selected LuaTeXNodeBackend: {}; {}",
                    signal_reason, availability.lualatex.reason
                ),
            };
        }

        return AutoBackendSelection {
            kind: SemanticBackendKind::RuleBased,
            reason: format!(
                "Auto selected RuleBasedBackend: {}; lualatex unavailable: {}",
                signal_reason, availability.lualatex.reason
            ),
        };
    }

    if availability.lualatex.available {
        return AutoBackendSelection {
            kind: SemanticBackendKind::LuaTeXNode,
            reason: format!(
                "Auto selected LuaTeXNodeBackend: generic LaTeX document; {}",
                availability.lualatex.reason
            ),
        };
    }

    if availability.xelatex.available {
        return AutoBackendSelection {
            kind: SemanticBackendKind::XeLaTeXHook,
            reason: format!(
                "Auto selected XeLaTeXHookBackend: LuaLaTeX unavailable for generic document; {}",
                availability.xelatex.reason
            ),
        };
    }

    AutoBackendSelection {
        kind: SemanticBackendKind::RuleBased,
        reason: format!(
            "Auto selected RuleBasedBackend: no runtime backend available; xelatex: {}; lualatex: {}",
            availability.xelatex.reason, availability.lualatex.reason
        ),
    }
}

fn collect_with_selected_backend(
    main_tex: &str,
    vfs: &mut VirtualFs,
    options: &CompileOptions,
    report: &mut CompileReport,
) -> Result<SemanticBackendArtifact, EngineError> {
    match options.semantic_backend {
        SemanticBackendKind::Auto => {
            let selection = select_auto_backend(vfs);
            match selection.kind {
                SemanticBackendKind::RuleBased => {
                    report.backend = BackendSelectionReport::new(
                        SemanticBackendKind::Auto,
                        SemanticBackendKind::RuleBased,
                        selection.reason,
                    );
                    RuleBasedBackend.collect(main_tex, vfs, options, report)
                }
                SemanticBackendKind::XeLaTeXHook => collect_runtime_or_fallback_requested(
                    SemanticBackendKind::Auto,
                    XeLaTeXHookBackend,
                    selection.reason,
                    main_tex,
                    vfs,
                    options,
                    report,
                ),
                SemanticBackendKind::LuaTeXNode => collect_runtime_or_fallback_requested(
                    SemanticBackendKind::Auto,
                    LuaTeXNodeBackend,
                    selection.reason,
                    main_tex,
                    vfs,
                    options,
                    report,
                ),
                SemanticBackendKind::Auto => unreachable!("auto selector must resolve a backend"),
            }
        }
        SemanticBackendKind::RuleBased => {
            report.backend = BackendSelectionReport::new(
                SemanticBackendKind::RuleBased,
                SemanticBackendKind::RuleBased,
                "RuleBasedBackend explicitly requested",
            );
            RuleBasedBackend.collect(main_tex, vfs, options, report)
        }
        SemanticBackendKind::XeLaTeXHook => collect_runtime_or_fallback(
            XeLaTeXHookBackend,
            "XeLaTeXHookBackend explicitly requested",
            main_tex,
            vfs,
            options,
            report,
        ),
        SemanticBackendKind::LuaTeXNode => collect_runtime_or_fallback(
            LuaTeXNodeBackend,
            "LuaTeXNodeBackend explicitly requested",
            main_tex,
            vfs,
            options,
            report,
        ),
    }
}

fn collect_runtime_or_fallback<B: SemanticBackend>(
    backend: B,
    reason: impl Into<String>,
    main_tex: &str,
    vfs: &mut VirtualFs,
    options: &CompileOptions,
    report: &mut CompileReport,
) -> Result<SemanticBackendArtifact, EngineError> {
    collect_runtime_or_fallback_requested(
        backend.kind(),
        backend,
        reason,
        main_tex,
        vfs,
        options,
        report,
    )
}

fn collect_runtime_or_fallback_requested<B: SemanticBackend>(
    requested: SemanticBackendKind,
    backend: B,
    reason: impl Into<String>,
    main_tex: &str,
    vfs: &mut VirtualFs,
    options: &CompileOptions,
    report: &mut CompileReport,
) -> Result<SemanticBackendArtifact, EngineError> {
    let reason = reason.into();
    let attempted = backend.kind();
    let availability = backend.is_available();
    if availability.available {
        report.backend = BackendSelectionReport::new(
            requested,
            attempted,
            format!(
                "{}; {} available: {}",
                reason,
                attempted.id(),
                availability.reason
            ),
        );
        match backend.collect(main_tex, vfs, options, report) {
            Ok(artifact) => return Ok(artifact),
            Err(err) if options.allow_backend_fallback => {
                return collect_rule_based_fallback(
                    requested,
                    attempted,
                    format!("{} failed: {}; {}", attempted.id(), err, reason),
                    main_tex,
                    vfs,
                    options,
                    report,
                );
            }
            Err(err) => return Err(err),
        }
    }

    if options.allow_backend_fallback {
        return collect_rule_based_fallback(
            requested,
            attempted,
            format!(
                "{} unavailable: {}; {}",
                attempted.id(),
                availability.reason,
                reason
            ),
            main_tex,
            vfs,
            options,
            report,
        );
    }

    report.backend = BackendSelectionReport::new(
        requested,
        attempted,
        format!("{} unavailable: {}", attempted.id(), availability.reason),
    );
    Err(EngineError::Unsupported(format!(
        "{} unavailable: {}",
        attempted.id(),
        availability.reason
    )))
}

fn collect_rule_based_fallback(
    requested: SemanticBackendKind,
    fallback_from: SemanticBackendKind,
    reason: String,
    main_tex: &str,
    vfs: &mut VirtualFs,
    options: &CompileOptions,
    report: &mut CompileReport,
) -> Result<SemanticBackendArtifact, EngineError> {
    report.backend = BackendSelectionReport::fallback(
        requested,
        fallback_from,
        SemanticBackendKind::RuleBased,
        &reason,
    );
    let mut artifact = RuleBasedBackend.collect(main_tex, vfs, options, report)?;
    artifact.diagnostics.push(EngineDiagnostic::warning(
        "backend_fallback",
        format!("{} -> {}", fallback_from.id(), reason),
    ));
    Ok(artifact)
}

fn command_available(name: &str) -> BackendAvailability {
    let Some(paths) = std::env::var_os("PATH") else {
        return BackendAvailability {
            available: false,
            reason: "PATH is not set".to_string(),
        };
    };

    let candidates = command_candidates(name);
    for dir in std::env::split_paths(&paths) {
        for candidate in &candidates {
            let path = dir.join(candidate);
            if path.is_file() {
                return BackendAvailability {
                    available: true,
                    reason: format!("found {}", path.display()),
                };
            }
        }
    }

    BackendAvailability {
        available: false,
        reason: format!("{name} not found on PATH"),
    }
}

fn command_candidates(name: &str) -> Vec<String> {
    if cfg!(windows) && !name.ends_with(".exe") {
        vec![name.to_string(), format!("{name}.exe")]
    } else {
        vec![name.to_string()]
    }
}

#[derive(Debug, Clone, Copy)]
enum RuntimeEngine {
    XeLaTeX,
    LuaLaTeX,
}

impl RuntimeEngine {
    fn command(self) -> &'static str {
        match self {
            Self::XeLaTeX => "xelatex",
            Self::LuaLaTeX => "lualatex",
        }
    }

    fn hook_name(self) -> &'static str {
        match self {
            Self::XeLaTeX => "__docx_semantic_xelatex_hook.tex",
            Self::LuaLaTeX => "__docx_semantic_lualatex_hook.tex",
        }
    }

    fn hook_source(self) -> &'static str {
        match self {
            Self::XeLaTeX => XELATEX_SEMANTIC_HOOK,
            Self::LuaLaTeX => LUALATEX_SEMANTIC_HOOK,
        }
    }
}

const SEMANTIC_SIDECAR: &str = "__docx_semantic_events.jsonl";

const XELATEX_SEMANTIC_HOOK: &str = r#"
\newwrite\docxsemout
\immediate\openout\docxsemout=__docx_semantic_events.jsonl
\makeatletter
\newcommand{\docxsemwriteheading}[2]{%
  \immediate\write\docxsemout{{"type":"heading","level":#1,"text":"\detokenize{#2}","label":null,"span":null}}%
}
\let\docxsemoldsection\section
\renewcommand{\section}[1]{\docxsemwriteheading{1}{#1}\docxsemoldsection{#1}}
\let\docxsemoldsubsection\subsection
\renewcommand{\subsection}[1]{\docxsemwriteheading{2}{#1}\docxsemoldsubsection{#1}}
\let\docxsemoldsubsubsection\subsubsection
\renewcommand{\subsubsection}[1]{\docxsemwriteheading{3}{#1}\docxsemoldsubsubsection{#1}}
\AtEndDocument{\immediate\closeout\docxsemout}
\makeatother
"#;

const LUALATEX_SEMANTIC_HOOK: &str = r#"
\directlua{
docxsem_file = io.open("__docx_semantic_events.jsonl", "w")
local glyph_id = node.id("glyph")
local glue_id = node.id("glue")
local bs = string.char(92)
local lua_space = string.char(37) .. "s"

function docxsem_json(value)
  value = tostring(value or "")
  value = value:gsub(bs, bs .. bs)
  value = value:gsub('"', bs .. '"')
  value = value:gsub(string.char(13), bs .. "r")
  value = value:gsub(string.char(10), bs .. "n")
  return '"' .. value .. '"'
end

function docxsem_write(raw)
  if docxsem_file then
    docxsem_file:write(raw, string.char(10))
  end
end

function docxsem_heading(level, text)
  docxsem_write('{"type":"heading","level":' .. level .. ',"text":' .. docxsem_json(text) .. ',"label":null,"span":null}')
end

function docxsem_label(key)
  docxsem_write('{"type":"label","key":' .. docxsem_json(key) .. ',"span":null}')
end

function docxsem_reference(kind, key)
  docxsem_write('{"type":"reference","kind":' .. docxsem_json(kind) .. ',"key":' .. docxsem_json(key) .. ',"span":null}')
end

function docxsem_citation(raw_keys)
  local parts = {}
  for key in tostring(raw_keys or ""):gmatch("[^," .. lua_space .. "]+") do
    table.insert(parts, docxsem_json(key))
  end
  docxsem_write('{"type":"citation","keys":[' .. table.concat(parts, ",") .. '],"span":null}')
end

function docxsem_graphic(path, options)
  docxsem_write('{"type":"figure","path":' .. docxsem_json(path) .. ',"caption":null,"label":null,"width_expr":' .. docxsem_json(options) .. ',"span":null}')
end

function docxsem_equation()
  docxsem_write('{"type":"equation","latex":"","label":null,"display":true,"span":null}')
end

local function docxsem_collect_text(head, out)
  if type(head) ~= "userdata" then
    return
  end
  for n in node.traverse(head) do
    if n.id == glyph_id then
      local ok, char = pcall(utf8.char, n.char)
      if ok then
        table.insert(out, char)
      end
    elseif n.id == glue_id then
      table.insert(out, " ")
    elseif type(n.list) == "userdata" then
      docxsem_collect_text(n.list, out)
    end
  end
end

local function docxsem_post_linebreak(head)
  local ok, err = pcall(function()
    local out = {}
    docxsem_collect_text(head, out)
    local text = table.concat(out)
    text = text:gsub("^" .. lua_space .. "+", "")
    text = text:gsub(lua_space .. "+$", "")
    if text ~= "" then
      docxsem_write('{"type":"paragraph","text":' .. docxsem_json(text) .. ',"span":null}')
    end
  end)
  if not ok then
    docxsem_write('# luatex-node-error: ' .. tostring(err))
  end
  return head
end

if luatexbase and luatexbase.add_to_callback then
  luatexbase.add_to_callback("post_linebreak_filter", docxsem_post_linebreak, "docxsem_post_linebreak")
else
  callback.register("post_linebreak_filter", docxsem_post_linebreak)
end
}
\makeatletter
\newcommand{\docxsemluaheading}[2]{\directlua{docxsem_heading(#1, "\luaescapestring{\detokenize{#2}}")}}
\newcommand{\docxsemlualabel}[1]{\directlua{docxsem_label("\luaescapestring{\detokenize{#1}}")}}
\newcommand{\docxsemluareference}[2]{\directlua{docxsem_reference("\luaescapestring{\detokenize{#1}}", "\luaescapestring{\detokenize{#2}}")}}
\newcommand{\docxsemluacitation}[1]{\directlua{docxsem_citation("\luaescapestring{\detokenize{#1}}")}}
\newcommand{\docxsemluagraphic}[2]{\directlua{docxsem_graphic("\luaescapestring{\detokenize{#2}}", "\luaescapestring{\detokenize{#1}}")}}
\let\docxsemoldsection\section
\renewcommand{\section}{\@ifstar{\docxsemsectionstar}{\docxsemsectionnostar}}
\newcommand{\docxsemsectionstar}[1]{\docxsemluaheading{1}{#1}\docxsemoldsection*{#1}}
\newcommand{\docxsemsectionnostar}{\@ifnextchar[{\docxsemsectionopt}{\docxsemsectionplain}}
\def\docxsemsectionopt[#1]#2{\docxsemluaheading{1}{#2}\docxsemoldsection[#1]{#2}}
\newcommand{\docxsemsectionplain}[1]{\docxsemluaheading{1}{#1}\docxsemoldsection{#1}}
\let\docxsemoldsubsection\subsection
\renewcommand{\subsection}{\@ifstar{\docxsemsubsectionstar}{\docxsemsubsectionnostar}}
\newcommand{\docxsemsubsectionstar}[1]{\docxsemluaheading{2}{#1}\docxsemoldsubsection*{#1}}
\newcommand{\docxsemsubsectionnostar}{\@ifnextchar[{\docxsemsubsectionopt}{\docxsemsubsectionplain}}
\def\docxsemsubsectionopt[#1]#2{\docxsemluaheading{2}{#2}\docxsemoldsubsection[#1]{#2}}
\newcommand{\docxsemsubsectionplain}[1]{\docxsemluaheading{2}{#1}\docxsemoldsubsection{#1}}
\let\docxsemoldsubsubsection\subsubsection
\renewcommand{\subsubsection}{\@ifstar{\docxsemsubsubsectionstar}{\docxsemsubsubsectionnostar}}
\newcommand{\docxsemsubsubsectionstar}[1]{\docxsemluaheading{3}{#1}\docxsemoldsubsubsection*{#1}}
\newcommand{\docxsemsubsubsectionnostar}{\@ifnextchar[{\docxsemsubsubsectionopt}{\docxsemsubsubsectionplain}}
\def\docxsemsubsubsectionopt[#1]#2{\docxsemluaheading{3}{#2}\docxsemoldsubsubsection[#1]{#2}}
\newcommand{\docxsemsubsubsectionplain}[1]{\docxsemluaheading{3}{#1}\docxsemoldsubsubsection{#1}}
\let\docxsemoldlabel\label
\renewcommand{\label}[1]{\docxsemlualabel{#1}\docxsemoldlabel{#1}}
\let\docxsemoldref\ref
\renewcommand{\ref}[1]{\docxsemluareference{ref}{#1}\docxsemoldref{#1}}
\ifcsname eqref\endcsname
  \let\docxsemoldeqref\eqref
  \renewcommand{\eqref}[1]{\docxsemluareference{eqref}{#1}\docxsemoldeqref{#1}}
\fi
\ifcsname autoref\endcsname
  \let\docxsemoldautoref\autoref
  \renewcommand{\autoref}[1]{\docxsemluareference{autoref}{#1}\docxsemoldautoref{#1}}
\fi
\ifcsname cite\endcsname
  \let\docxsemoldcite\cite
  \renewcommand{\cite}{\@ifnextchar[{\docxsemciteopt}{\docxsemciteplain}}
  \def\docxsemciteopt[#1]#2{\docxsemluacitation{#2}\docxsemoldcite[#1]{#2}}
  \newcommand{\docxsemciteplain}[1]{\docxsemluacitation{#1}\docxsemoldcite{#1}}
\fi
\ifcsname includegraphics\endcsname
  \let\docxsemoldincludegraphics\includegraphics
  \renewcommand{\includegraphics}{\@ifnextchar[{\docxsemincludegraphicsopt}{\docxsemincludegraphicsplain}}
  \def\docxsemincludegraphicsopt[#1]#2{\docxsemluagraphic{#1}{#2}\docxsemoldincludegraphics[#1]{#2}}
  \newcommand{\docxsemincludegraphicsplain}[1]{\docxsemluagraphic{}{#1}\docxsemoldincludegraphics{#1}}
\fi
\ifcsname equation\endcsname
  \let\docxsemoldequation\equation
  \let\docxsemoldendequation\endequation
  \renewenvironment{equation}{\directlua{docxsem_equation()}\docxsemoldequation}{\docxsemoldendequation}
\fi
\AtEndDocument{\directlua{if docxsem_file then docxsem_file:close() end}}
\makeatother
"#;

fn collect_runtime_events(
    engine: RuntimeEngine,
    main_tex: &str,
    vfs: &VirtualFs,
) -> Result<Vec<SemanticEvent>, EngineError> {
    let workdir = tempfile::tempdir().map_err(|e| EngineError::Io(e.to_string()))?;
    materialize_vfs(vfs, workdir.path())?;

    let hook_name = engine.hook_name();
    let main_path = Path::new(main_tex);
    let hook_rel = main_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.join(hook_name))
        .unwrap_or_else(|| PathBuf::from(hook_name));
    let hook_path = workdir.path().join(&hook_rel);
    if let Some(parent) = hook_path.parent() {
        fs::create_dir_all(parent).map_err(|e| EngineError::Io(e.to_string()))?;
    }
    fs::write(&hook_path, engine.hook_source()).map_err(|e| EngineError::Io(e.to_string()))?;

    let main_disk = workdir.path().join(main_path);
    let main_bytes = fs::read(&main_disk).map_err(|e| {
        EngineError::Io(format!(
            "failed to read materialized main tex {}: {}",
            main_disk.display(),
            e
        ))
    })?;
    let main_text = String::from_utf8(main_bytes).map_err(|e| {
        EngineError::Parse(format!(
            "{main_tex} is not valid UTF-8 for semantic hook injection: {e}"
        ))
    })?;
    let hook_input = path_to_posix(&hook_rel);
    fs::write(
        &main_disk,
        inject_hook_input(&main_text, &format!("\\input{{{hook_input}}}\n")),
    )
    .map_err(|e| EngineError::Io(e.to_string()))?;

    run_latex_runtime(engine, workdir.path(), main_tex)?;

    let sidecar_path = workdir.path().join(SEMANTIC_SIDECAR);
    let sidecar = fs::read_to_string(&sidecar_path).map_err(|e| {
        EngineError::Parse(format!(
            "{} did not emit semantic sidecar {}: {}",
            engine.command(),
            sidecar_path.display(),
            e
        ))
    })?;
    parse_semantic_events_jsonl(&sidecar)
}

fn materialize_vfs(vfs: &VirtualFs, root: &Path) -> Result<(), EngineError> {
    for path in vfs.paths() {
        ensure_relative_vfs_path(path)?;
        let out_path = root.join(path);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| EngineError::Io(e.to_string()))?;
        }
        let bytes = vfs.read(path)?;
        fs::write(out_path, bytes).map_err(|e| EngineError::Io(e.to_string()))?;
    }
    Ok(())
}

fn ensure_relative_vfs_path(path: &Path) -> Result<(), EngineError> {
    if path.components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir
        )
    }) {
        return Err(EngineError::Parse(format!(
            "unsafe VFS path for runtime backend: {}",
            path.display()
        )));
    }
    Ok(())
}

fn inject_hook_input(source: &str, hook_input: &str) -> String {
    if source.contains(hook_input) {
        return source.to_string();
    }
    if let Some(pos) = source.find("\\begin{document}") {
        let mut out = String::with_capacity(source.len() + hook_input.len());
        out.push_str(&source[..pos]);
        out.push_str(hook_input);
        out.push_str(&source[pos..]);
        out
    } else {
        let mut out = source.to_string();
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(hook_input);
        out
    }
}

fn run_latex_runtime(
    engine: RuntimeEngine,
    workdir: &Path,
    main_tex: &str,
) -> Result<(), EngineError> {
    let tex_cache = workdir.join(".texlive-cache");
    fs::create_dir_all(&tex_cache).map_err(|e| EngineError::Io(e.to_string()))?;

    let mut command = Command::new(engine.command());
    command
        .arg("-interaction=nonstopmode")
        .arg("-halt-on-error")
        .arg("-file-line-error")
        .arg(main_tex)
        .env("TEXMFVAR", &tex_cache)
        .env("TEXMFCACHE", &tex_cache)
        .current_dir(workdir);

    let output = command
        .output()
        .map_err(|e| EngineError::Io(format!("failed to start {}: {}", engine.command(), e)))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(EngineError::Unsupported(format!(
        "{} exited with status {}; stdout tail: {}; stderr tail: {}",
        engine.command(),
        output.status,
        tail_for_diagnostic(&stdout),
        tail_for_diagnostic(&stderr)
    )))
}

fn tail_for_diagnostic(input: &str) -> String {
    const MAX: usize = 1200;
    let mut chars = input.chars().rev().take(MAX).collect::<Vec<_>>();
    chars.reverse();
    chars.into_iter().collect::<String>()
}

/// Parse semantic sidecar JSONL emitted by future XeLaTeX/LuaTeX collectors.
///
/// Empty lines and comment lines beginning with `#` are ignored.
pub fn parse_semantic_events_jsonl(input: &str) -> Result<Vec<SemanticEvent>, EngineError> {
    let mut events = Vec::new();
    for (idx, line) in input.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let event = serde_json::from_str::<SemanticEvent>(trimmed).map_err(|err| {
            EngineError::Parse(format!(
                "semantic sidecar line {} is invalid JSON: {}",
                idx + 1,
                err
            ))
        })?;
        events.push(event);
    }
    Ok(events)
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
        let options = CompileOptions {
            semantic_backend: SemanticBackendKind::RuleBased,
            ..CompileOptions::default()
        };
        let artifact = engine
            .compile_source_to_docx("main.tex", SAMPLE, &options)
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
        assert_eq!(
            artifact.report.backend.selected,
            SemanticBackendKind::RuleBased
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
        let options = CompileOptions {
            semantic_backend: SemanticBackendKind::RuleBased,
            ..CompileOptions::default()
        };
        let artifact = engine
            .compile_zip_to_docx(out.get_ref(), "main.tex", &options)
            .expect("compile zip");

        assert_eq!(&artifact.docx[..4], b"PK\x03\x04");
        assert!(artifact.report.block_count >= 2);
    }

    #[test]
    fn explicit_runtime_backend_falls_back_to_rule_based() {
        let mut vfs = VirtualFs::new();
        vfs.insert("main.tex", SAMPLE.as_bytes().to_vec());
        let options = CompileOptions {
            semantic_backend: SemanticBackendKind::XeLaTeXHook,
            ..CompileOptions::default()
        };
        let mut report = CompileReport::new(options.profile);
        let artifact = collect_runtime_or_fallback(
            MissingRuntimeBackend,
            "missing runtime backend for deterministic test",
            "main.tex",
            &mut vfs,
            &options,
            &mut report,
        )
        .expect("fallback to rule-based backend");

        assert_eq!(report.backend.requested, SemanticBackendKind::XeLaTeXHook);
        assert_eq!(report.backend.selected, SemanticBackendKind::RuleBased);
        assert_eq!(
            report.backend.fallback_from,
            Some(SemanticBackendKind::XeLaTeXHook)
        );
        assert!(artifact
            .diagnostics
            .iter()
            .any(|diag| diag.code == "backend_fallback"));
    }

    #[test]
    fn runtime_backend_without_fallback_returns_error() {
        let mut vfs = VirtualFs::new();
        vfs.insert("main.tex", SAMPLE.as_bytes().to_vec());
        let options = CompileOptions {
            semantic_backend: SemanticBackendKind::XeLaTeXHook,
            allow_backend_fallback: false,
            ..CompileOptions::default()
        };
        let mut report = CompileReport::new(options.profile);
        let err = collect_runtime_or_fallback(
            MissingRuntimeBackend,
            "missing runtime backend for deterministic test",
            "main.tex",
            &mut vfs,
            &options,
            &mut report,
        )
        .expect_err("strict missing runtime should error");

        assert!(matches!(err, EngineError::Unsupported(_)));
    }

    #[test]
    fn parses_semantic_event_jsonl() {
        let raw = r#"
# comment
{"type":"heading","level":1,"text":"Intro","label":"sec:intro","span":null}
{"type":"citation","keys":["a","b"],"span":null}
{"type":"label","key":"sec:intro","span":null}
{"type":"reference","kind":"ref","key":"sec:intro","span":null}
"#;
        let events = parse_semantic_events_jsonl(raw).expect("parse events");
        assert_eq!(events.len(), 4);
        assert!(matches!(
            &events[0],
            SemanticEvent::Heading { level: 1, text, .. } if text == "Intro"
        ));
        assert!(matches!(
            &events[1],
            SemanticEvent::Citation { keys, .. } if keys == &vec!["a".to_string(), "b".to_string()]
        ));
        assert!(matches!(
            &events[2],
            SemanticEvent::Label { key, .. } if key == "sec:intro"
        ));
        assert!(matches!(
            &events[3],
            SemanticEvent::Reference { kind, key, .. } if kind == "ref" && key == "sec:intro"
        ));
    }

    #[test]
    fn injects_hook_before_begin_document() {
        let source = "\\documentclass{article}\n\\begin{document}\nBody";
        let injected = inject_hook_input(source, "\\input{hook.tex}\n");
        assert!(injected.contains("\\input{hook.tex}\n\\begin{document}"));
    }

    #[test]
    fn rejects_unsafe_runtime_vfs_path() {
        let err = ensure_relative_vfs_path(Path::new("../main.tex"))
            .expect_err("parent paths must be rejected");
        assert!(matches!(err, EngineError::Parse(_)));
    }

    #[test]
    fn auto_selector_prefers_xelatex_for_xecjk_templates() {
        let mut signals = TemplateSignals::default();
        signals.observe(
            r#"
\documentclass{ctexart}
\usepackage{xeCJK}
\setCJKmainfont{SimSun}
"#,
        );
        let selection = select_auto_backend_with_availability(&signals, availability(true, true));

        assert_eq!(selection.kind, SemanticBackendKind::XeLaTeXHook);
        assert!(selection.reason.contains("XeLaTeXHookBackend"));
    }

    #[test]
    fn auto_selector_prefers_luatex_for_generic_templates() {
        let mut signals = TemplateSignals::default();
        signals.observe("\\documentclass{article}\n\\begin{document}Hi\\end{document}");
        let selection = select_auto_backend_with_availability(&signals, availability(true, true));

        assert_eq!(selection.kind, SemanticBackendKind::LuaTeXNode);
        assert!(selection.reason.contains("generic LaTeX"));
    }

    #[test]
    fn auto_selector_keeps_xecjk_templates_off_luatex_without_xelatex() {
        let mut signals = TemplateSignals::default();
        signals.observe("\\usepackage{xeCJK}");
        let selection = select_auto_backend_with_availability(&signals, availability(false, true));

        assert_eq!(selection.kind, SemanticBackendKind::RuleBased);
        assert!(selection.reason.contains("xelatex unavailable"));
    }

    #[test]
    #[ignore = "requires lualatex on PATH and exercises the external runtime backend"]
    fn luatex_runtime_collects_semantic_events() {
        if !command_available("lualatex").available {
            return;
        }

        let engine = SemanticTexEngine::new();
        let options = CompileOptions {
            semantic_backend: SemanticBackendKind::LuaTeXNode,
            allow_backend_fallback: false,
            ..CompileOptions::default()
        };
        let source = r#"
\documentclass{article}
\usepackage{graphicx}
\begin{document}
\section{Intro}\label{sec:intro}
A paragraph cites \cite{demo} and references Section~\ref{sec:intro}.
\begin{equation}
a+b=c
\end{equation}
\end{document}
"#;

        let mut vfs = VirtualFs::new();
        vfs.insert("main.tex", source.as_bytes().to_vec());
        let graph = engine
            .compile_vfs_to_graph("main.tex", &mut vfs, &options)
            .expect("compile through LuaTeX runtime backend");

        assert_eq!(
            graph.report.backend.selected,
            SemanticBackendKind::LuaTeXNode
        );
        assert!(graph
            .report
            .diagnostics
            .iter()
            .all(|diag| diag.code != "backend_fallback"));
        assert!(graph.semantic_events.iter().any(|event| {
            matches!(event, SemanticEvent::Heading { level: 1, text, .. } if text == "Intro")
        }));
        assert!(graph
            .semantic_events
            .iter()
            .any(|event| matches!(event, SemanticEvent::Label { key, .. } if key == "sec:intro")));
        assert!(graph.semantic_events.iter().any(|event| {
            matches!(event, SemanticEvent::Reference { kind, key, .. } if kind == "ref" && key == "sec:intro")
        }));
        assert!(graph.semantic_events.iter().any(|event| {
            matches!(event, SemanticEvent::Citation { keys, .. } if keys == &vec!["demo".to_string()])
        }));
        assert!(graph
            .semantic_events
            .iter()
            .any(|event| { matches!(event, SemanticEvent::Equation { display: true, .. }) }));
        assert!(graph.semantic_events.iter().any(|event| {
            matches!(event, SemanticEvent::Paragraph { text, .. } if text.contains("paragraph"))
        }));
    }

    #[derive(Debug, Clone, Copy)]
    struct MissingRuntimeBackend;

    impl SemanticBackend for MissingRuntimeBackend {
        fn kind(&self) -> SemanticBackendKind {
            SemanticBackendKind::XeLaTeXHook
        }

        fn is_available(&self) -> BackendAvailability {
            BackendAvailability {
                available: false,
                reason: "missing in test".to_string(),
            }
        }

        fn collect(
            &self,
            _main_tex: &str,
            _vfs: &mut VirtualFs,
            _options: &CompileOptions,
            _report: &mut CompileReport,
        ) -> Result<SemanticBackendArtifact, EngineError> {
            unreachable!("missing backend should not be collected")
        }
    }

    fn availability(xelatex: bool, lualatex: bool) -> RuntimeAvailabilitySnapshot {
        RuntimeAvailabilitySnapshot {
            xelatex: BackendAvailability {
                available: xelatex,
                reason: if xelatex {
                    "xelatex test binary".to_string()
                } else {
                    "xelatex missing in test".to_string()
                },
            },
            lualatex: BackendAvailability {
                available: lualatex,
                reason: if lualatex {
                    "lualatex test binary".to_string()
                } else {
                    "lualatex missing in test".to_string()
                },
            },
        }
    }
}
