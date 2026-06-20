//! Semantic TeX compiler engine.
//!
//! This crate is the facade for the next-generation TeX -> DOCX pipeline. It
//! keeps the current rule-based LaTeX reader and DOCX writer behind explicit
//! compiler stages, so later LuaHook/XDV/OMML implementations can replace
//! individual stages without changing callers.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
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
        self.spec().id
    }

    pub fn spec(self) -> ProfileSpec {
        match self {
            Self::GenericArticle => ProfileSpec {
                profile: self,
                id: "generic-article",
                display_name: "Generic Article",
                document_classes: &["article", "report", "book"],
                page_setup: PageSetupProfile::Default,
                font_policy: FontPolicySpec {
                    latin_main: "Times New Roman",
                    cjk_main: "",
                    math: "Cambria Math",
                    notes: "Default Word-compatible article font policy",
                },
                caption_policy: CaptionPolicySpec {
                    figure_prefix: "Figure",
                    table_prefix: "Table",
                    equation_prefix: "Equation",
                    numbering: "arabic",
                },
                citation_policy: CitationPolicySpec {
                    style: "numeric",
                    bibliography_style: "plain",
                    reference_section_title: "References",
                },
            },
            Self::ChineseAcademic => ProfileSpec {
                profile: self,
                id: "chinese-academic",
                display_name: "Chinese Academic Paper",
                document_classes: &["ctexart", "ctexrep", "ctexbook", "article"],
                page_setup: PageSetupProfile::A4,
                font_policy: FontPolicySpec {
                    latin_main: "Times New Roman",
                    cjk_main: "SimSun",
                    math: "Cambria Math",
                    notes: "CTeX-oriented Chinese academic font policy",
                },
                caption_policy: CaptionPolicySpec {
                    figure_prefix: "图",
                    table_prefix: "表",
                    equation_prefix: "式",
                    numbering: "chapter-or-section",
                },
                citation_policy: CitationPolicySpec {
                    style: "numeric-compressed",
                    bibliography_style: "gbt7714-like",
                    reference_section_title: "参考文献",
                },
            },
            Self::JosPaper => ProfileSpec {
                profile: self,
                id: "jos-paper",
                display_name: "Journal of Software Paper",
                document_classes: &["rjthesis", "ctexart"],
                page_setup: PageSetupProfile::JosPaper3,
                font_policy: FontPolicySpec {
                    latin_main: "Times New Roman",
                    cjk_main: "SimSun/FangSong/KaiTi",
                    math: "Cambria Math",
                    notes: "JOS paper3 profile; CTeX/XeCJK templates prefer XeLaTeX",
                },
                caption_policy: CaptionPolicySpec {
                    figure_prefix: "图",
                    table_prefix: "表",
                    equation_prefix: "式",
                    numbering: "section-scoped",
                },
                citation_policy: CitationPolicySpec {
                    style: "numeric-super-compressed",
                    bibliography_style: "unsrt",
                    reference_section_title: "References",
                },
            },
            Self::MedicalJournal => ProfileSpec {
                profile: self,
                id: "medical-journal",
                display_name: "Medical Journal Manuscript",
                document_classes: &["article", "elsarticle", "wlscirep"],
                page_setup: PageSetupProfile::A4,
                font_policy: FontPolicySpec {
                    latin_main: "Times New Roman",
                    cjk_main: "SimSun",
                    math: "Cambria Math",
                    notes: "Medical manuscript profile with restrained Word defaults",
                },
                caption_policy: CaptionPolicySpec {
                    figure_prefix: "Fig.",
                    table_prefix: "Table",
                    equation_prefix: "Equation",
                    numbering: "arabic",
                },
                citation_policy: CitationPolicySpec {
                    style: "numeric",
                    bibliography_style: "vancouver-like",
                    reference_section_title: "References",
                },
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileSpec {
    pub profile: EngineProfile,
    pub id: &'static str,
    pub display_name: &'static str,
    pub document_classes: &'static [&'static str],
    pub page_setup: PageSetupProfile,
    pub font_policy: FontPolicySpec,
    pub caption_policy: CaptionPolicySpec,
    pub citation_policy: CitationPolicySpec,
}

impl ProfileSpec {
    pub fn default_page_setup(self) -> Option<doc_docx_writer::PageSetup> {
        self.page_setup.to_page_setup()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageSetupProfile {
    Default,
    A4,
    JosPaper3,
}

impl PageSetupProfile {
    pub fn id(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::A4 => "a4",
            Self::JosPaper3 => "jos-paper3",
        }
    }

    pub fn to_page_setup(self) -> Option<doc_docx_writer::PageSetup> {
        match self {
            Self::Default => None,
            Self::A4 => Some(doc_docx_writer::PageSetup {
                width_twips: 11906,
                height_twips: 16838,
                margin_top: Some(1440),
                margin_right: Some(1440),
                margin_bottom: Some(1440),
                margin_left: Some(1440),
                margin_header: Some(720),
                margin_footer: Some(720),
                cols_space: Some(720),
                cols_num: Some(1),
                header_text: None,
                footer_text: None,
                first_header_text: None,
                first_footer_text: None,
                even_header_text: None,
                first_footer_indent_twips: None,
            }),
            Self::JosPaper3 => Some(doc_docx_writer::PageSetup::jos_paper3()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontPolicySpec {
    pub latin_main: &'static str,
    pub cjk_main: &'static str,
    pub math: &'static str,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptionPolicySpec {
    pub figure_prefix: &'static str,
    pub table_prefix: &'static str,
    pub equation_prefix: &'static str,
    pub numbering: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CitationPolicySpec {
    pub style: &'static str,
    pub bibliography_style: &'static str,
    pub reference_section_title: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSpecReport {
    pub id: String,
    pub display_name: String,
    pub document_classes: Vec<String>,
    pub default_page_setup: String,
    pub latin_main_font: String,
    pub cjk_main_font: String,
    pub math_font: String,
    pub figure_caption_prefix: String,
    pub table_caption_prefix: String,
    pub equation_caption_prefix: String,
    pub citation_style: String,
    pub bibliography_style: String,
    pub reference_section_title: String,
}

impl ProfileSpecReport {
    fn from_spec(spec: ProfileSpec) -> Self {
        Self {
            id: spec.id.to_string(),
            display_name: spec.display_name.to_string(),
            document_classes: spec
                .document_classes
                .iter()
                .map(|class| (*class).to_string())
                .collect(),
            default_page_setup: spec.page_setup.id().to_string(),
            latin_main_font: spec.font_policy.latin_main.to_string(),
            cjk_main_font: spec.font_policy.cjk_main.to_string(),
            math_font: spec.font_policy.math.to_string(),
            figure_caption_prefix: spec.caption_policy.figure_prefix.to_string(),
            table_caption_prefix: spec.caption_policy.table_prefix.to_string(),
            equation_caption_prefix: spec.caption_policy.equation_prefix.to_string(),
            citation_style: spec.citation_policy.style.to_string(),
            bibliography_style: spec.citation_policy.bibliography_style.to_string(),
            reference_section_title: spec.citation_policy.reference_section_title.to_string(),
        }
    }
}

/// Semantic collection backend selection.
///
/// `Auto` scans template features and available TeX runtimes before selecting
/// a concrete semantic backend.
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
    pub enable_reference_links: bool,
    pub enable_omml_equations: bool,
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
            enable_reference_links: true,
            enable_omml_equations: true,
            template_bytes: None,
            page_setup: None,
            collect_standard_ast: true,
            enable_bibliography: true,
        }
    }
}

impl CompileOptions {
    pub fn effective_page_setup(&self) -> Option<doc_docx_writer::PageSetup> {
        self.page_setup
            .clone()
            .or_else(|| self.profile.spec().default_page_setup())
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
        let page_setup = options.effective_page_setup();
        let page_setup_label = if options.page_setup.is_some() {
            "explicit page setup override".to_string()
        } else {
            format!(
                "profile default page setup: {}",
                options.profile.spec().page_setup.id()
            )
        };
        graph.report.push(
            CompileStage::DocxRender,
            StageStatus::Completed,
            format!(
                "DOCX renderer packed document.xml, styles.xml, relationships and media ({page_setup_label})"
            ),
        );

        let mut docx = doc_docx_writer::pack_with_page_setup(
            &graph.document,
            options.template_bytes.as_deref(),
            Some(&graph.image_assets),
            page_setup.as_ref(),
        )
        .map_err(|e| EngineError::Serialize(e.to_string()))?;
        if options.enable_omml_equations {
            let omml = apply_omml_equations_to_docx(docx, &graph.document)?;
            graph.report.omml_equation_count = omml.converted;
            graph.report.omml_equation_fallback_count = omml.fallbacks;
            docx = omml.docx;
        }
        if options.enable_reference_links {
            let linked = apply_reference_links_to_docx(docx, &graph.reference_graph)?;
            graph.report.bookmark_count = linked.bookmarks;
            graph.report.hyperlink_count = linked.hyperlinks;
            docx = linked.docx;
        }
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

        let compatibility = analyze_compatibility(vfs, options.profile);
        report.push(
            CompileStage::CompatibilityAnalyze,
            StageStatus::Completed,
            format!(
                "compatibility score {}, unsupported {}, warnings {}",
                compatibility.score,
                compatibility.unsupported.len(),
                compatibility.warnings.len()
            ),
        );
        for issue in &compatibility.unsupported {
            report.diagnostics.push(EngineDiagnostic::warning(
                "compatibility_unsupported",
                format!("{}: {}", issue.feature, issue.message),
            ));
        }
        for issue in &compatibility.warnings {
            report.diagnostics.push(EngineDiagnostic::warning(
                "compatibility_warning",
                format!("{}: {}", issue.feature, issue.message),
            ));
        }
        report.compatibility = compatibility;

        let mut artifact = collect_with_selected_backend(main_tex, vfs, options, &mut report)?;
        let reference_graph = build_reference_graph(vfs, &artifact.events);
        report.semantic_event_count = artifact.events.len();
        report.layout_node_count = artifact
            .layout
            .as_ref()
            .map_or(0, |layout| layout.nodes.len());
        report.sidecar_count = artifact.sidecars.len();
        report.reference_label_count = reference_graph.labels.len();
        report.reference_edge_count = reference_graph.references.len();
        report.citation_count = reference_graph.citations.len();
        report.unresolved_reference_count = reference_graph.unresolved_references.len();
        for unresolved in &reference_graph.unresolved_references {
            report.diagnostics.push(EngineDiagnostic::warning(
                "unresolved_reference",
                format!(
                    "{}{{{}}} has no matching label",
                    unresolved.command, unresolved.key
                ),
            ));
        }
        report.diagnostics.append(&mut artifact.diagnostics);

        Ok(DocumentGraph {
            document: artifact.document,
            standard_document: artifact.standard_document,
            image_assets: artifact.image_assets,
            semantic_events: artifact.events,
            reference_graph,
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
    pub reference_graph: ReferenceGraph,
    pub layout: Option<LayoutGraph>,
    pub report: CompileReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileReport {
    pub profile: EngineProfile,
    pub profile_spec: ProfileSpecReport,
    pub backend: BackendSelectionReport,
    pub compatibility: CompatibilityReport,
    pub stages: Vec<StageReport>,
    pub diagnostics: Vec<EngineDiagnostic>,
    pub block_count: usize,
    pub image_asset_count: usize,
    pub semantic_event_count: usize,
    pub layout_node_count: usize,
    pub sidecar_count: usize,
    pub reference_label_count: usize,
    pub reference_edge_count: usize,
    pub citation_count: usize,
    pub unresolved_reference_count: usize,
    pub bookmark_count: usize,
    pub hyperlink_count: usize,
    pub omml_equation_count: usize,
    pub omml_equation_fallback_count: usize,
    pub docx_bytes: usize,
}

impl CompileReport {
    pub fn new(profile: EngineProfile) -> Self {
        Self {
            profile,
            profile_spec: ProfileSpecReport::from_spec(profile.spec()),
            backend: BackendSelectionReport::default(),
            compatibility: CompatibilityReport::default(),
            stages: Vec::new(),
            diagnostics: Vec::new(),
            block_count: 0,
            image_asset_count: 0,
            semantic_event_count: 0,
            layout_node_count: 0,
            sidecar_count: 0,
            reference_label_count: 0,
            reference_edge_count: 0,
            citation_count: 0,
            unresolved_reference_count: 0,
            bookmark_count: 0,
            hyperlink_count: 0,
            omml_equation_count: 0,
            omml_equation_fallback_count: 0,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompatibilityReport {
    pub score: u8,
    pub scanned_files: usize,
    pub document_classes: Vec<String>,
    pub packages: Vec<String>,
    pub custom_macro_count: usize,
    pub unsupported: Vec<CompatibilityIssue>,
    pub warnings: Vec<CompatibilityIssue>,
}

impl Default for CompatibilityReport {
    fn default() -> Self {
        Self {
            score: 100,
            scanned_files: 0,
            document_classes: Vec::new(),
            packages: Vec::new(),
            custom_macro_count: 0,
            unsupported: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompatibilityIssue {
    pub code: String,
    pub feature: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompileStage {
    SourceMount,
    CompatibilityAnalyze,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReferenceGraph {
    pub labels: Vec<ReferenceLabel>,
    pub references: Vec<CrossReference>,
    pub citations: Vec<CitationReference>,
    pub unresolved_references: Vec<UnresolvedReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReferenceLabel {
    pub key: String,
    pub kind: ReferenceTargetKind,
    pub number: Option<String>,
    pub source: ReferenceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossReference {
    pub command: String,
    pub key: String,
    pub resolved: bool,
    pub target_kind: Option<ReferenceTargetKind>,
    pub rendered: Option<String>,
    pub source: ReferenceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CitationReference {
    pub command: String,
    pub keys: Vec<String>,
    pub source: ReferenceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnresolvedReference {
    pub command: String,
    pub key: String,
    pub source: ReferenceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReferenceSource {
    pub path: Option<String>,
    pub origin: ReferenceOrigin,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ReferenceOrigin {
    SourceScan,
    SemanticEvent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ReferenceTargetKind {
    Heading,
    Figure,
    Table,
    Equation,
    Algorithm,
    Theorem,
    Proposition,
    Unknown,
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
pub struct CollectedDocument {
    pub document: Document,
    pub standard_document: Option<StandardDocument>,
    pub image_assets: ImageAssets,
    pub events: Vec<SemanticEvent>,
    pub layout: Option<LayoutGraph>,
    pub diagnostics: Vec<EngineDiagnostic>,
    pub sidecars: Vec<BuildSidecar>,
}

impl CollectedDocument {
    pub fn new(
        document: Document,
        standard_document: Option<StandardDocument>,
        image_assets: ImageAssets,
    ) -> Self {
        Self {
            document,
            standard_document,
            image_assets,
            events: Vec::new(),
            layout: None,
            diagnostics: Vec::new(),
            sidecars: Vec::new(),
        }
    }
}

pub type SemanticBackendArtifact = CollectedDocument;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildSidecar {
    pub kind: String,
    pub path: Option<String>,
    pub description: String,
}

impl BuildSidecar {
    pub fn new(
        kind: impl Into<String>,
        path: Option<impl Into<String>>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            path: path.map(Into::into),
            description: description.into(),
        }
    }
}

pub struct SemanticCollectorInput<'a> {
    pub main_tex: &'a str,
    pub vfs: &'a mut VirtualFs,
    pub options: &'a CompileOptions,
}

pub trait SemanticCollector {
    fn name(&self) -> &'static str;

    fn collect(
        &self,
        input: &mut SemanticCollectorInput<'_>,
        report: &mut CompileReport,
    ) -> Result<CollectedDocument, EngineError>;
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
pub struct RuleBasedCollector;

impl SemanticCollector for RuleBasedCollector {
    fn name(&self) -> &'static str {
        "rule-based"
    }

    fn collect(
        &self,
        input: &mut SemanticCollectorInput<'_>,
        report: &mut CompileReport,
    ) -> Result<CollectedDocument, EngineError> {
        let graph = IncludeGraph::build(input.vfs, Path::new(input.main_tex))?;
        let joined = graph.join(input.vfs)?;
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

        let document =
            lower_semantic_document(input.main_tex, input.vfs, &parse, &joined, input.options)?;
        report.block_count = document.blocks.len();
        report.push(
            CompileStage::SemanticCollect,
            StageStatus::Completed,
            format!("collected {} semantic blocks", document.blocks.len()),
        );

        let image_assets = collect_image_assets_from_vfs(input.vfs);
        report.image_asset_count = image_assets.len();

        let source = source_bundle(input.main_tex, input.vfs);
        let standard_document = if input.options.collect_standard_ast {
            let standard = StandardDocument::from_legacy_document(
                &document,
                source,
                input.options.profile.id(),
            );
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

        Ok(CollectedDocument::new(
            document,
            standard_document,
            image_assets,
        ))
    }
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
        let mut input = SemanticCollectorInput {
            main_tex,
            vfs,
            options,
        };
        RuleBasedCollector.collect(&mut input, report)
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
        artifact.sidecars.push(BuildSidecar::new(
            "semantic-events-jsonl",
            Some(SEMANTIC_SIDECAR),
            "semantic events collected from the XeLaTeX hook sidecar",
        ));
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
        artifact.sidecars.push(BuildSidecar::new(
            "semantic-events-jsonl",
            Some(SEMANTIC_SIDECAR),
            "semantic events collected from the LuaTeX node sidecar",
        ));
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
\ifcsname includegraphics\endcsname
  \let\docxsemoldincludegraphics\includegraphics
  \renewcommand{\includegraphics}{\@ifnextchar[{\docxsemincludegraphicsopt}{\docxsemincludegraphicsplain}}
  \def\docxsemincludegraphicsopt[#1]#2{%
    \immediate\write\docxsemout{{"type":"figure","path":"\detokenize{#2}","caption":null,"label":null,"width_expr":"\detokenize{#1}","span":null}}%
    \docxsemoldincludegraphics[#1]{#2}%
  }
  \newcommand{\docxsemincludegraphicsplain}[1]{%
    \immediate\write\docxsemout{{"type":"figure","path":"\detokenize{#1}","caption":null,"label":null,"width_expr":null,"span":null}}%
    \docxsemoldincludegraphics{#1}%
  }
\fi
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

fn analyze_compatibility(vfs: &VirtualFs, profile: EngineProfile) -> CompatibilityReport {
    let mut report = CompatibilityReport::default();
    let mut document_classes = HashSet::new();
    let mut packages = HashSet::new();
    let mut unsupported_seen = HashSet::new();
    let mut warning_seen = HashSet::new();

    for path in vfs.paths() {
        if !is_tex_source_path(path) {
            continue;
        }

        let Ok(bytes) = vfs.read(path) else {
            continue;
        };
        let Ok(raw) = std::str::from_utf8(bytes) else {
            continue;
        };
        let source = strip_tex_comments(raw);
        report.scanned_files += 1;

        for command in scan_tex_commands(&source, &["documentclass"]) {
            for class in split_tex_name_list(&command.argument) {
                document_classes.insert(class);
            }
        }

        for command in scan_tex_commands(&source, &["usepackage", "RequirePackage"]) {
            for package in split_tex_name_list(&command.argument) {
                packages.insert(package);
            }
        }

        report.custom_macro_count += count_custom_macro_definitions(&source);

        if contains_tex_environment(&source, "tikzpicture") {
            add_compatibility_issue(
                &mut report.unsupported,
                &mut unsupported_seen,
                "unsupported_environment",
                "tikzpicture",
                "TikZ graphics need rasterization or a semantic drawing plugin before high-fidelity DOCX output",
            );
        }
        if contains_tex_environment(&source, "minted") {
            add_compatibility_issue(
                &mut report.unsupported,
                &mut unsupported_seen,
                "unsupported_environment",
                "minted",
                "minted depends on external syntax highlighting and is not preserved by the semantic DOCX path yet",
            );
        }
        if contains_tex_environment(&source, "lstlisting") {
            add_compatibility_issue(
                &mut report.warnings,
                &mut warning_seen,
                "limited_environment",
                "lstlisting",
                "listings environments are downgraded to code-like text in the current semantic DOCX path",
            );
        }
    }

    report.document_classes = sorted_strings(document_classes);
    report.packages = sorted_strings(packages);

    for class in report.document_classes.clone() {
        let class = class.as_str();
        if matches!(class, "beamer" | "standalone") {
            add_compatibility_issue(
                &mut report.unsupported,
                &mut unsupported_seen,
                "unsupported_document_class",
                class,
                "presentation or standalone drawing classes are outside the paper-oriented semantic profile",
            );
        } else if !profile_supports_document_class(profile, class) {
            add_compatibility_issue(
                &mut report.warnings,
                &mut warning_seen,
                "profile_document_class_mismatch",
                class,
                "document class is outside the active profile and may need profile-specific lowering rules",
            );
        }
    }

    for package in report.packages.clone() {
        match package.as_str() {
            "tikz" | "pgf" | "pgfplots" | "circuitikz" => add_compatibility_issue(
                &mut report.unsupported,
                &mut unsupported_seen,
                "unsupported_package",
                package.as_str(),
                "PGF/TikZ graphics are not semantically converted to editable DOCX drawing objects yet",
            ),
            "pstricks" => add_compatibility_issue(
                &mut report.unsupported,
                &mut unsupported_seen,
                "unsupported_package",
                package.as_str(),
                "PSTricks output requires a PostScript rendering path that the semantic engine does not provide yet",
            ),
            "minted" => add_compatibility_issue(
                &mut report.unsupported,
                &mut unsupported_seen,
                "unsupported_package",
                package.as_str(),
                "minted requires external Pygments processing and shell-escape semantics",
            ),
            "listings" => add_compatibility_issue(
                &mut report.warnings,
                &mut warning_seen,
                "limited_package",
                package.as_str(),
                "listings content is treated as code text; advanced styling is not preserved yet",
            ),
            "biblatex" => add_compatibility_issue(
                &mut report.warnings,
                &mut warning_seen,
                "limited_package",
                package.as_str(),
                "biblatex is detected; current bibliography support is strongest for BibTeX/bbl-style flows",
            ),
            "longtable" | "tabularx" | "tabulary" => add_compatibility_issue(
                &mut report.warnings,
                &mut warning_seen,
                "limited_package",
                package.as_str(),
                "advanced table layout is lowered semantically but may need renderer-specific refinement",
            ),
            "algorithm2e" | "algorithmicx" | "algpseudocode" => add_compatibility_issue(
                &mut report.warnings,
                &mut warning_seen,
                "limited_package",
                package.as_str(),
                "algorithm packages currently rely on generic block lowering and may lose fine-grained styling",
            ),
            _ => {}
        }
    }

    if report.custom_macro_count > 0 {
        add_compatibility_issue(
            &mut report.warnings,
            &mut warning_seen,
            "custom_macros",
            "custom macros",
            format!(
                "{} custom macro definitions detected; unknown macro semantics may require explicit rules",
                report.custom_macro_count
            ),
        );
    }

    apply_compatibility_score(&mut report);
    report
}

fn is_tex_source_path(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "tex" | "sty" | "cls" | "ltx"
    )
}

fn split_tex_name_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(normalize_tex_name)
        .filter(|name| !name.is_empty())
        .collect()
}

fn normalize_tex_name(raw: &str) -> String {
    raw.trim()
        .trim_matches('{')
        .trim_matches('}')
        .trim()
        .trim_start_matches('\\')
        .to_ascii_lowercase()
}

fn count_custom_macro_definitions(source: &str) -> usize {
    scan_tex_commands(
        source,
        &[
            "newcommand",
            "renewcommand",
            "providecommand",
            "DeclareRobustCommand",
            "NewDocumentCommand",
            "RenewDocumentCommand",
        ],
    )
    .len()
        + source.matches("\\def\\").count()
}

fn contains_tex_environment(source: &str, environment: &str) -> bool {
    source.contains(&format!("\\begin{{{environment}}}"))
}

fn sorted_strings(set: HashSet<String>) -> Vec<String> {
    let mut values = set.into_iter().collect::<Vec<_>>();
    values.sort();
    values
}

fn profile_supports_document_class(profile: EngineProfile, class: &str) -> bool {
    profile
        .spec()
        .document_classes
        .iter()
        .any(|supported| supported.eq_ignore_ascii_case(class))
}

fn add_compatibility_issue(
    issues: &mut Vec<CompatibilityIssue>,
    seen: &mut HashSet<String>,
    code: impl Into<String>,
    feature: impl Into<String>,
    message: impl Into<String>,
) {
    let code = code.into();
    let feature = feature.into();
    let key = format!("{code}:{feature}");
    if seen.insert(key) {
        issues.push(CompatibilityIssue {
            code,
            feature,
            message: message.into(),
        });
    }
}

fn apply_compatibility_score(report: &mut CompatibilityReport) {
    let unsupported_penalty = report.unsupported.len() * 18;
    let warning_penalty = report.warnings.len() * 6;
    let macro_penalty = (report.custom_macro_count * 2).min(12);
    let penalty = (unsupported_penalty + warning_penalty + macro_penalty).min(100);
    report.score = 100usize.saturating_sub(penalty) as u8;
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

fn build_reference_graph(vfs: &VirtualFs, events: &[SemanticEvent]) -> ReferenceGraph {
    let mut graph = ReferenceGraph::default();
    let mut label_keys = HashSet::<String>::new();
    let mut numbering = HashMap::<ReferenceTargetKind, usize>::new();

    collect_source_references(vfs, &mut graph, &mut label_keys, &mut numbering);
    collect_event_references(events, &mut graph, &mut label_keys);
    resolve_reference_graph(graph)
}

fn collect_source_references(
    vfs: &VirtualFs,
    graph: &mut ReferenceGraph,
    label_keys: &mut HashSet<String>,
    numbering: &mut HashMap<ReferenceTargetKind, usize>,
) {
    let mut paths = vfs
        .paths()
        .filter(|path| is_tex_like_path(path))
        .map(|path| path.to_path_buf())
        .collect::<Vec<_>>();
    paths.sort();

    for path in paths {
        let Ok(bytes) = vfs.read(&path) else {
            continue;
        };
        let Ok(raw) = std::str::from_utf8(bytes) else {
            continue;
        };
        let text = strip_tex_comments(raw);
        let source = ReferenceSource {
            path: Some(path_to_posix(&path)),
            origin: ReferenceOrigin::SourceScan,
        };

        for command in scan_tex_commands(&text, &["label"]) {
            let key = command.argument.trim();
            if key.is_empty() || !label_keys.insert(key.to_string()) {
                continue;
            }
            let kind = reference_kind_from_key(key);
            let number = next_reference_number(kind, numbering);
            graph.labels.push(ReferenceLabel {
                key: key.to_string(),
                kind,
                number,
                source: source.clone(),
            });
        }

        for command in scan_tex_commands(&text, &["autoref", "eqref", "ref"]) {
            let key = command.argument.trim();
            if key.is_empty() {
                continue;
            }
            graph.references.push(CrossReference {
                command: command.name,
                key: key.to_string(),
                resolved: false,
                target_kind: None,
                rendered: None,
                source: source.clone(),
            });
        }

        for command in scan_tex_commands(
            &text,
            &[
                "citeauthor",
                "citeyear",
                "citealp",
                "citealt",
                "citep",
                "citet",
                "cite",
            ],
        ) {
            let keys = split_citation_keys(&command.argument);
            if keys.is_empty() {
                continue;
            }
            graph.citations.push(CitationReference {
                command: command.name,
                keys,
                source: source.clone(),
            });
        }
    }
}

fn collect_event_references(
    events: &[SemanticEvent],
    graph: &mut ReferenceGraph,
    label_keys: &mut HashSet<String>,
) {
    let source = ReferenceSource {
        path: None,
        origin: ReferenceOrigin::SemanticEvent,
    };

    for event in events {
        match event {
            SemanticEvent::Heading {
                label: Some(label), ..
            } => add_event_label(
                graph,
                label_keys,
                label,
                ReferenceTargetKind::Heading,
                source.clone(),
            ),
            SemanticEvent::Figure {
                label: Some(label), ..
            } => add_event_label(
                graph,
                label_keys,
                label,
                ReferenceTargetKind::Figure,
                source.clone(),
            ),
            SemanticEvent::Table {
                label: Some(label), ..
            } => add_event_label(
                graph,
                label_keys,
                label,
                ReferenceTargetKind::Table,
                source.clone(),
            ),
            SemanticEvent::Equation {
                label: Some(label), ..
            } => add_event_label(
                graph,
                label_keys,
                label,
                ReferenceTargetKind::Equation,
                source.clone(),
            ),
            SemanticEvent::Label { key, .. } => {
                add_event_label(
                    graph,
                    label_keys,
                    key,
                    reference_kind_from_key(key),
                    source.clone(),
                );
            }
            SemanticEvent::Reference { kind, key, .. } => graph.references.push(CrossReference {
                command: kind.clone(),
                key: key.clone(),
                resolved: false,
                target_kind: None,
                rendered: None,
                source: source.clone(),
            }),
            SemanticEvent::Citation { keys, .. } => {
                if !keys.is_empty() {
                    graph.citations.push(CitationReference {
                        command: "cite".to_string(),
                        keys: keys.clone(),
                        source: source.clone(),
                    });
                }
            }
            _ => {}
        }
    }
}

fn add_event_label(
    graph: &mut ReferenceGraph,
    label_keys: &mut HashSet<String>,
    key: &str,
    kind: ReferenceTargetKind,
    source: ReferenceSource,
) {
    let key = key.trim();
    if key.is_empty() || !label_keys.insert(key.to_string()) {
        return;
    }
    graph.labels.push(ReferenceLabel {
        key: key.to_string(),
        kind,
        number: None,
        source,
    });
}

fn resolve_reference_graph(mut graph: ReferenceGraph) -> ReferenceGraph {
    let label_map = graph
        .labels
        .iter()
        .map(|label| {
            (
                label.key.clone(),
                (
                    label.kind,
                    label.number.clone().unwrap_or_else(|| label.key.clone()),
                ),
            )
        })
        .collect::<HashMap<_, _>>();

    let mut unresolved = Vec::new();
    for reference in &mut graph.references {
        if let Some((kind, rendered)) = label_map.get(&reference.key) {
            reference.resolved = true;
            reference.target_kind = Some(*kind);
            reference.rendered = Some(rendered.clone());
        } else {
            unresolved.push(UnresolvedReference {
                command: reference.command.clone(),
                key: reference.key.clone(),
                source: reference.source.clone(),
            });
        }
    }
    graph.unresolved_references = unresolved;
    graph
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScannedTexCommand {
    name: String,
    argument: String,
}

fn scan_tex_commands(text: &str, commands: &[&str]) -> Vec<ScannedTexCommand> {
    let mut found = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            i += 1;
            continue;
        }

        let command_start = i + 1;
        let Some(command) = commands
            .iter()
            .find(|command| tex_command_matches(text, command_start, command))
        else {
            i += 1;
            continue;
        };

        let mut arg_pos = command_start + command.len();
        arg_pos = skip_tex_space_and_options(text, arg_pos);
        if text.as_bytes().get(arg_pos) != Some(&b'{') {
            i += command.len() + 1;
            continue;
        }

        if let Some(end) = doc_latex_reader::normalize::find_matching_brace(text, arg_pos) {
            found.push(ScannedTexCommand {
                name: (*command).to_string(),
                argument: text[arg_pos + 1..end].trim().to_string(),
            });
            i = end + 1;
        } else {
            i += command.len() + 1;
        }
    }
    found
}

fn tex_command_matches(text: &str, command_start: usize, command: &str) -> bool {
    let end = command_start + command.len();
    if text.get(command_start..end) != Some(command) {
        return false;
    }

    let next = text.as_bytes().get(end).copied();
    !matches!(next, Some(b'a'..=b'z' | b'A'..=b'Z' | b'@'))
}

fn skip_tex_space_and_options(text: &str, mut pos: usize) -> usize {
    loop {
        while pos < text.len() && text.as_bytes()[pos].is_ascii_whitespace() {
            pos += 1;
        }

        if text.as_bytes().get(pos) != Some(&b'[') {
            return pos;
        }

        let Some(end) = find_matching_bracket(text, pos) else {
            return pos;
        };
        pos = end + 1;
    }
}

fn find_matching_bracket(text: &str, open_index: usize) -> Option<usize> {
    if text.as_bytes().get(open_index) != Some(&b'[') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = open_index;
    while i < text.len() {
        let b = text.as_bytes()[i];
        let escaped = is_escaped(text.as_bytes(), i);
        if b == b'[' && !escaped {
            depth += 1;
        } else if b == b']' && !escaped {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn strip_tex_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for line in input.split_inclusive('\n') {
        let bytes = line.as_bytes();
        let mut end = line.len();
        for (idx, b) in bytes.iter().enumerate() {
            if *b == b'%' && !is_escaped(bytes, idx) {
                end = idx;
                break;
            }
        }
        out.push_str(&line[..end]);
        if line.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

fn is_escaped(bytes: &[u8], idx: usize) -> bool {
    let mut count = 0usize;
    let mut cursor = idx;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        count += 1;
        cursor -= 1;
    }
    count % 2 == 1
}

fn split_citation_keys(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn reference_kind_from_key(key: &str) -> ReferenceTargetKind {
    let lower = key.to_ascii_lowercase();
    if lower.starts_with("fig:") || lower.starts_with("figure:") {
        ReferenceTargetKind::Figure
    } else if lower.starts_with("tab:") || lower.starts_with("table:") {
        ReferenceTargetKind::Table
    } else if lower.starts_with("eq:") || lower.starts_with("equation:") {
        ReferenceTargetKind::Equation
    } else if lower.starts_with("alg:") || lower.starts_with("algorithm:") {
        ReferenceTargetKind::Algorithm
    } else if lower.starts_with("thm:") || lower.starts_with("theorem:") {
        ReferenceTargetKind::Theorem
    } else if lower.starts_with("prop:") || lower.starts_with("proposition:") {
        ReferenceTargetKind::Proposition
    } else if lower.starts_with("sec:") || lower.starts_with("section:") {
        ReferenceTargetKind::Heading
    } else {
        ReferenceTargetKind::Unknown
    }
}

fn next_reference_number(
    kind: ReferenceTargetKind,
    numbering: &mut HashMap<ReferenceTargetKind, usize>,
) -> Option<String> {
    if kind == ReferenceTargetKind::Unknown {
        return None;
    }
    let next = numbering.entry(kind).or_insert(0);
    *next += 1;
    if kind == ReferenceTargetKind::Equation {
        Some(format!("({next})"))
    } else {
        Some(next.to_string())
    }
}

#[derive(Debug, Clone)]
struct OmmlEquationDocx {
    docx: Vec<u8>,
    converted: usize,
    fallbacks: usize,
}

#[derive(Debug, Clone)]
struct OmmlEquationPlan {
    latex: String,
    number: String,
}

fn apply_omml_equations_to_docx(
    docx: Vec<u8>,
    document: &Document,
) -> Result<OmmlEquationDocx, EngineError> {
    let plans = build_omml_equation_plans(document);
    if plans.is_empty() {
        return Ok(OmmlEquationDocx {
            docx,
            converted: 0,
            fallbacks: 0,
        });
    }

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(docx))
        .map_err(|err| EngineError::Zip(err.to_string()))?;
    let mut entries = Vec::with_capacity(archive.len());
    let mut document_xml = None;

    for idx in 0..archive.len() {
        let mut file = archive
            .by_index(idx)
            .map_err(|err| EngineError::Zip(err.to_string()))?;
        let name = file.name().to_string();
        if file.is_dir() {
            entries.push((name, None));
            continue;
        }
        let mut bytes = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut bytes)
            .map_err(|err| EngineError::Io(err.to_string()))?;
        if name == "word/document.xml" {
            document_xml = Some(String::from_utf8(bytes).map_err(|err| {
                EngineError::Parse(format!("word/document.xml is not valid UTF-8: {err}"))
            })?);
        } else {
            entries.push((name, Some(bytes)));
        }
    }

    let Some(document_xml) = document_xml else {
        return Ok(OmmlEquationDocx {
            docx: archive.into_inner().into_inner(),
            converted: 0,
            fallbacks: plans.len(),
        });
    };

    let (linked_xml, converted, fallbacks) = link_omml_equations_xml(&document_xml, &plans);

    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        if let Some(bytes) = bytes {
            writer
                .start_file(name, opts)
                .map_err(|err| EngineError::Zip(err.to_string()))?;
            std::io::Write::write_all(&mut writer, &bytes)
                .map_err(|err| EngineError::Io(err.to_string()))?;
        } else {
            writer
                .add_directory(name, opts)
                .map_err(|err| EngineError::Zip(err.to_string()))?;
        }
    }
    writer
        .start_file("word/document.xml", opts)
        .map_err(|err| EngineError::Zip(err.to_string()))?;
    std::io::Write::write_all(&mut writer, linked_xml.as_bytes())
        .map_err(|err| EngineError::Io(err.to_string()))?;
    let cursor = writer
        .finish()
        .map_err(|err| EngineError::Zip(err.to_string()))?;

    Ok(OmmlEquationDocx {
        docx: cursor.into_inner(),
        converted,
        fallbacks,
    })
}

fn build_omml_equation_plans(document: &Document) -> Vec<OmmlEquationPlan> {
    let mut plans = Vec::new();
    let mut number = 0usize;
    collect_omml_equation_plans(&document.blocks, &mut number, &mut plans);
    plans
}

fn collect_omml_equation_plans(
    blocks: &[Block],
    number: &mut usize,
    plans: &mut Vec<OmmlEquationPlan>,
) {
    for block in blocks {
        match block {
            Block::Equation {
                latex, is_block, ..
            } if *is_block => {
                *number += 1;
                plans.push(OmmlEquationPlan {
                    latex: latex.clone(),
                    number: format!("({number})"),
                });
            }
            Block::List { items, .. } => {
                for item in items {
                    collect_omml_equation_plans(item, number, plans);
                }
            }
            _ => {}
        }
    }
}

fn link_omml_equations_xml(
    document_xml: &str,
    plans: &[OmmlEquationPlan],
) -> (String, usize, usize) {
    let mut out = String::with_capacity(document_xml.len() + plans.len() * 160);
    let mut pos = 0usize;
    let mut plan_idx = 0usize;
    let mut converted = 0usize;
    let mut fallbacks = 0usize;

    while let Some(start) = find_next_paragraph(document_xml, pos) {
        out.push_str(&document_xml[pos..start]);
        let Some(end_rel) = document_xml[start..].find("</w:p>") else {
            out.push_str(&document_xml[start..]);
            fallbacks += plans.len().saturating_sub(plan_idx);
            return (out, converted, fallbacks);
        };
        let end = start + end_rel + "</w:p>".len();
        let paragraph = &document_xml[start..end];

        if plan_idx < plans.len() && paragraph_is_equation_text(paragraph) {
            let plan = &plans[plan_idx];
            let math_latex = normalize_omml_latex(&plan.latex);
            if math_latex.trim().is_empty() {
                out.push_str(paragraph);
                fallbacks += 1;
            } else {
                let expr = doc_mathml::parse_latex_math(&math_latex);
                let omml = String::from_utf8_lossy(&doc_mathml::to_omml(&expr)).to_string();
                let omml = strip_xml_decl(&omml);
                out.push_str(&omml_equation_paragraph(paragraph, omml, &plan.number));
                converted += 1;
            }
            plan_idx += 1;
        } else {
            out.push_str(paragraph);
        }
        pos = end;
    }
    out.push_str(&document_xml[pos..]);

    fallbacks += plans.len().saturating_sub(plan_idx);
    (out, converted, fallbacks)
}

fn paragraph_is_equation_text(paragraph: &str) -> bool {
    paragraph.contains(r#"<w:pStyle w:val="JOSCode""#)
}

fn normalize_omml_latex(latex: &str) -> String {
    let mut out = String::new();
    let mut i = 0usize;
    while i < latex.len() {
        if latex[i..].starts_with("\\label") {
            let arg_start = i + "\\label".len();
            let arg_start = skip_latex_space(latex, arg_start);
            if latex.as_bytes().get(arg_start) == Some(&b'{') {
                if let Some(end) =
                    doc_latex_reader::normalize::find_matching_brace(latex, arg_start)
                {
                    i = end + 1;
                    continue;
                }
            }
        }
        let ch = latex[i..].chars().next().unwrap_or_default();
        out.push(ch);
        i += ch.len_utf8();
    }
    out.replace("\\\\", " ").replace('\n', " ")
}

fn skip_latex_space(input: &str, mut pos: usize) -> usize {
    while pos < input.len() && input.as_bytes()[pos].is_ascii_whitespace() {
        pos += 1;
    }
    pos
}

fn strip_xml_decl(xml: &str) -> &str {
    let trimmed = xml.trim_start();
    if trimmed.starts_with("<?xml") {
        if let Some(end) = trimmed.find("?>") {
            return trimmed[end + "?>".len()..].trim_start();
        }
    }
    trimmed
}

fn omml_equation_paragraph(paragraph: &str, omml: &str, number: &str) -> String {
    let Some(open_end) = paragraph.find('>') else {
        return paragraph.to_string();
    };
    let Some(close_start) = paragraph.rfind("</w:p>") else {
        return paragraph.to_string();
    };
    let inner = &paragraph[open_end + 1..close_start];
    let ppr = if inner.starts_with("<w:pPr>") {
        inner
            .find("</w:pPr>")
            .map(|end| &inner[..end + "</w:pPr>".len()])
            .unwrap_or("")
    } else {
        ""
    };
    let escaped_number = escape_xml_text(&format!("    {number}"));
    format!(
        "{}{}{}<w:r>{}</w:r>{}",
        &paragraph[..open_end + 1],
        ppr,
        omml,
        text_xml(&escaped_number),
        &paragraph[close_start..]
    )
}

#[derive(Debug, Clone)]
struct ReferenceLinkedDocx {
    docx: Vec<u8>,
    bookmarks: usize,
    hyperlinks: usize,
}

fn apply_reference_links_to_docx(
    docx: Vec<u8>,
    reference_graph: &ReferenceGraph,
) -> Result<ReferenceLinkedDocx, EngineError> {
    if reference_graph.labels.is_empty() || reference_graph.references.is_empty() {
        return Ok(ReferenceLinkedDocx {
            docx,
            bookmarks: 0,
            hyperlinks: 0,
        });
    }

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(docx))
        .map_err(|err| EngineError::Zip(err.to_string()))?;
    let mut entries = Vec::with_capacity(archive.len());
    let mut document_xml = None;

    for idx in 0..archive.len() {
        let mut file = archive
            .by_index(idx)
            .map_err(|err| EngineError::Zip(err.to_string()))?;
        let name = file.name().to_string();
        if file.is_dir() {
            entries.push((name, None));
            continue;
        }
        let mut bytes = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut bytes)
            .map_err(|err| EngineError::Io(err.to_string()))?;
        if name == "word/document.xml" {
            document_xml = Some(String::from_utf8(bytes).map_err(|err| {
                EngineError::Parse(format!("word/document.xml is not valid UTF-8: {err}"))
            })?);
        } else {
            entries.push((name, Some(bytes)));
        }
    }

    let Some(document_xml) = document_xml else {
        return Ok(ReferenceLinkedDocx {
            docx: archive.into_inner().into_inner(),
            bookmarks: 0,
            hyperlinks: 0,
        });
    };

    let (linked_xml, bookmarks, hyperlinks) = link_document_xml(&document_xml, reference_graph);

    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        if let Some(bytes) = bytes {
            writer
                .start_file(name, opts)
                .map_err(|err| EngineError::Zip(err.to_string()))?;
            std::io::Write::write_all(&mut writer, &bytes)
                .map_err(|err| EngineError::Io(err.to_string()))?;
        } else {
            writer
                .add_directory(name, opts)
                .map_err(|err| EngineError::Zip(err.to_string()))?;
        }
    }
    writer
        .start_file("word/document.xml", opts)
        .map_err(|err| EngineError::Zip(err.to_string()))?;
    std::io::Write::write_all(&mut writer, linked_xml.as_bytes())
        .map_err(|err| EngineError::Io(err.to_string()))?;
    let cursor = writer
        .finish()
        .map_err(|err| EngineError::Zip(err.to_string()))?;

    Ok(ReferenceLinkedDocx {
        docx: cursor.into_inner(),
        bookmarks,
        hyperlinks,
    })
}

fn link_document_xml(
    document_xml: &str,
    reference_graph: &ReferenceGraph,
) -> (String, usize, usize) {
    let bookmark_plan = build_bookmark_plan(reference_graph);
    if bookmark_plan.is_empty() {
        return (document_xml.to_string(), 0, 0);
    }

    let mut bookmarked_xml = String::with_capacity(document_xml.len() + bookmark_plan.len() * 96);
    let mut pos = 0usize;
    let mut bookmark_hits = HashSet::<String>::new();
    let mut bookmark_id = 100usize;

    while let Some(start) = find_next_paragraph(document_xml, pos) {
        bookmarked_xml.push_str(&document_xml[pos..start]);
        let Some(end_rel) = document_xml[start..].find("</w:p>") else {
            bookmarked_xml.push_str(&document_xml[start..]);
            return (bookmarked_xml, bookmark_hits.len(), 0);
        };
        let end = start + end_rel + "</w:p>".len();
        let paragraph = &document_xml[start..end];
        let paragraph_text = paragraph_text(paragraph);

        let mut paragraph_xml = paragraph.to_string();
        for plan in &bookmark_plan {
            if !bookmark_hits.contains(&plan.key)
                && plan
                    .target_needles
                    .iter()
                    .any(|needle| !needle.is_empty() && paragraph_text.contains(needle))
                && paragraph_matches_reference_target(paragraph, plan)
            {
                let id = bookmark_id;
                bookmark_id += 1;
                paragraph_xml = insert_bookmark_in_paragraph(&paragraph_xml, id, &plan.name);
                bookmark_hits.insert(plan.key.clone());
                break;
            }
        }

        bookmarked_xml.push_str(&paragraph_xml);
        pos = end;
    }
    bookmarked_xml.push_str(&document_xml[pos..]);

    if bookmark_hits.is_empty() {
        return (bookmarked_xml, 0, 0);
    }

    let mut linked_xml = String::with_capacity(bookmarked_xml.len() + bookmark_hits.len() * 128);
    let mut pos = 0usize;
    let mut hyperlink_hits = 0usize;
    let mut reference_hits = HashSet::<usize>::new();

    while let Some(start) = find_next_paragraph(&bookmarked_xml, pos) {
        linked_xml.push_str(&bookmarked_xml[pos..start]);
        let Some(end_rel) = bookmarked_xml[start..].find("</w:p>") else {
            linked_xml.push_str(&bookmarked_xml[start..]);
            return (linked_xml, bookmark_hits.len(), hyperlink_hits);
        };
        let end = start + end_rel + "</w:p>".len();
        let mut paragraph_xml = bookmarked_xml[start..end].to_string();

        for (reference_idx, reference) in reference_graph
            .references
            .iter()
            .enumerate()
            .filter(|(_, reference)| reference.resolved)
        {
            if reference_hits.contains(&reference_idx) {
                continue;
            }
            let Some(plan) = bookmark_plan.iter().find(|plan| plan.key == reference.key) else {
                continue;
            };
            if !bookmark_hits.contains(&plan.key) {
                continue;
            }
            if let Some(updated) =
                hyperlink_paragraph(&paragraph_xml, &plan.name, &reference_needles(reference))
            {
                paragraph_xml = updated;
                reference_hits.insert(reference_idx);
                hyperlink_hits += 1;
            }
        }

        linked_xml.push_str(&paragraph_xml);
        pos = end;
    }
    linked_xml.push_str(&bookmarked_xml[pos..]);

    (linked_xml, bookmark_hits.len(), hyperlink_hits)
}

fn find_next_paragraph(xml: &str, from: usize) -> Option<usize> {
    let mut search_from = from;
    while let Some(rel) = xml[search_from..].find("<w:p") {
        let start = search_from + rel;
        match xml.as_bytes().get(start + "<w:p".len()).copied() {
            Some(b'>') | Some(b' ' | b'\t' | b'\r' | b'\n') => return Some(start),
            _ => search_from = start + "<w:p".len(),
        }
    }
    None
}

#[derive(Debug, Clone)]
struct BookmarkPlan {
    key: String,
    name: String,
    kind: ReferenceTargetKind,
    target_needles: Vec<String>,
}

fn build_bookmark_plan(reference_graph: &ReferenceGraph) -> Vec<BookmarkPlan> {
    reference_graph
        .labels
        .iter()
        .filter_map(|label| {
            let name = bookmark_name_for_label(&label.key);
            let target_needles = label
                .number
                .as_deref()
                .map(|number| target_needles(label.kind, number))
                .unwrap_or_default();
            (!target_needles.is_empty()).then_some(BookmarkPlan {
                key: label.key.clone(),
                name,
                kind: label.kind,
                target_needles,
            })
        })
        .collect()
}

fn bookmark_name_for_label(key: &str) -> String {
    let mut out = String::from("ref_");
    for ch in key.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
        if out.len() >= 40 {
            break;
        }
    }
    if out == "ref_" {
        out.push_str("target");
    }
    out
}

fn target_needles(kind: ReferenceTargetKind, number: &str) -> Vec<String> {
    match kind {
        ReferenceTargetKind::Figure => vec![
            format!("图 {number} "),
            format!("图 {number}:"),
            format!("Figure {number} "),
            format!("Figure {number}:"),
            format!("Fig. {number} "),
            format!("Fig. {number}:"),
        ],
        ReferenceTargetKind::Table => vec![
            format!("表 {number} "),
            format!("表 {number}:"),
            format!("Table {number} "),
            format!("Table {number}:"),
        ],
        ReferenceTargetKind::Equation => vec![number.to_string(), format!("式 {number}")],
        ReferenceTargetKind::Algorithm => {
            vec![format!("算法 {number} "), format!("Algorithm {number} ")]
        }
        ReferenceTargetKind::Theorem => {
            vec![format!("定理 {number} "), format!("Theorem {number} ")]
        }
        ReferenceTargetKind::Proposition => {
            vec![format!("命题 {number} "), format!("Proposition {number} ")]
        }
        ReferenceTargetKind::Heading => vec![number.to_string()],
        ReferenceTargetKind::Unknown => Vec::new(),
    }
}

fn paragraph_matches_reference_target(paragraph: &str, plan: &BookmarkPlan) -> bool {
    match plan.kind {
        ReferenceTargetKind::Figure
        | ReferenceTargetKind::Table
        | ReferenceTargetKind::Algorithm
        | ReferenceTargetKind::Theorem
        | ReferenceTargetKind::Proposition => {
            paragraph.contains(r#"<w:pStyle w:val="JOSCaption""#)
                || paragraph.contains(r#"<w:pStyle w:val="AlgorithmCaption""#)
        }
        ReferenceTargetKind::Equation => {
            paragraph.contains(r#"<w:pStyle w:val="JOSEquation""#)
                || paragraph.contains(r#"<w:pStyle w:val="Equation""#)
                || paragraph.contains(r#"<w:pStyle w:val="JOSCode""#)
        }
        ReferenceTargetKind::Heading => paragraph.contains(r#"<w:pStyle w:val="Heading"#),
        ReferenceTargetKind::Unknown => false,
    }
}

fn reference_needles(reference: &CrossReference) -> Vec<String> {
    let Some(rendered) = reference.rendered.as_deref() else {
        return Vec::new();
    };
    match reference
        .target_kind
        .unwrap_or(ReferenceTargetKind::Unknown)
    {
        ReferenceTargetKind::Figure => vec![
            format!("图 {rendered}"),
            format!("图{rendered}"),
            format!("Figure {rendered}"),
            format!("Fig. {rendered}"),
        ],
        ReferenceTargetKind::Table => vec![
            format!("表 {rendered}"),
            format!("表{rendered}"),
            format!("Table {rendered}"),
            format!("Tab. {rendered}"),
        ],
        ReferenceTargetKind::Equation => vec![format!("式 {rendered}"), rendered.to_string()],
        ReferenceTargetKind::Algorithm => {
            vec![
                format!("算法 {rendered}"),
                format!("算法{rendered}"),
                format!("Algorithm {rendered}"),
            ]
        }
        ReferenceTargetKind::Theorem => {
            vec![
                format!("定理 {rendered}"),
                format!("定理{rendered}"),
                format!("Theorem {rendered}"),
            ]
        }
        ReferenceTargetKind::Proposition => vec![
            format!("命题 {rendered}"),
            format!("命题{rendered}"),
            format!("Proposition {rendered}"),
        ],
        ReferenceTargetKind::Heading => vec![
            format!("第{rendered}节"),
            format!("Section {rendered}"),
            format!("Sec. {rendered}"),
        ],
        ReferenceTargetKind::Unknown => Vec::new(),
    }
}

fn insert_bookmark_in_paragraph(paragraph: &str, id: usize, name: &str) -> String {
    let Some(open_end) = paragraph.find('>') else {
        return paragraph.to_string();
    };
    let Some(close_start) = paragraph.rfind("</w:p>") else {
        return paragraph.to_string();
    };
    format!(
        "{}<w:bookmarkStart w:id=\"{}\" w:name=\"{}\"/>{}<w:bookmarkEnd w:id=\"{}\"/>{}",
        &paragraph[..open_end + 1],
        id,
        escape_xml_attr(name),
        &paragraph[open_end + 1..close_start],
        id,
        &paragraph[close_start..]
    )
}

fn hyperlink_paragraph(paragraph: &str, anchor: &str, needles: &[String]) -> Option<String> {
    if paragraph.contains("<w:hyperlink") || paragraph.contains("<w:bookmarkStart") {
        return None;
    }
    for needle in needles {
        if needle.is_empty() {
            continue;
        }
        if let Some(updated) = hyperlink_first_text_run(paragraph, anchor, needle) {
            return Some(updated);
        }
    }
    None
}

fn hyperlink_first_text_run(paragraph: &str, anchor: &str, needle: &str) -> Option<String> {
    let escaped_needle = escape_xml_text(needle);
    let mut search_from = 0usize;
    while let Some(rel) = paragraph[search_from..].find("<w:t") {
        let tag_start = search_from + rel;
        let tag_end = paragraph[tag_start..]
            .find('>')
            .map(|idx| tag_start + idx)?;
        let close_start = paragraph[tag_end + 1..]
            .find("</w:t>")
            .map(|idx| tag_end + 1 + idx)?;
        let text = &paragraph[tag_end + 1..close_start];
        if let Some(match_start) = text.find(&escaped_needle) {
            let match_end = match_start + escaped_needle.len();
            let before = &text[..match_start];
            let after = &text[match_end..];
            let mut replacement = String::new();
            if !before.is_empty() {
                replacement.push_str(&text_run_xml(before));
            }
            replacement.push_str(&format!(
                "<w:hyperlink w:anchor=\"{}\" w:history=\"1\"><w:r><w:rPr><w:rStyle w:val=\"Hyperlink\"/></w:rPr>{}</w:r></w:hyperlink>",
                escape_xml_attr(anchor),
                text_xml(&escaped_needle)
            ));
            if !after.is_empty() {
                replacement.push_str(&text_run_xml(after));
            }
            let run_start = find_enclosing_run_start(paragraph, tag_start).unwrap_or(tag_start);
            let run_end = paragraph[close_start + "</w:t>".len()..]
                .find("</w:r>")
                .map(|idx| close_start + "</w:t>".len() + idx + "</w:r>".len())
                .unwrap_or(close_start + "</w:t>".len());
            return Some(format!(
                "{}{}{}",
                &paragraph[..run_start],
                replacement,
                &paragraph[run_end..]
            ));
        }
        search_from = close_start + "</w:t>".len();
    }
    None
}

fn find_enclosing_run_start(xml: &str, before: usize) -> Option<usize> {
    let mut search_from = 0usize;
    let mut last = None;
    while search_from < before {
        let Some(rel) = xml[search_from..before].find("<w:r") else {
            break;
        };
        let start = search_from + rel;
        match xml.as_bytes().get(start + "<w:r".len()).copied() {
            Some(b'>') | Some(b' ' | b'\t' | b'\r' | b'\n') => last = Some(start),
            _ => {}
        }
        search_from = start + "<w:r".len();
    }
    last
}

fn paragraph_text(paragraph: &str) -> String {
    let mut out = String::new();
    let mut search_from = 0usize;
    while let Some(rel) = paragraph[search_from..].find("<w:t") {
        let tag_start = search_from + rel;
        let Some(tag_end) = paragraph[tag_start..].find('>').map(|idx| tag_start + idx) else {
            break;
        };
        let Some(close_start) = paragraph[tag_end + 1..]
            .find("</w:t>")
            .map(|idx| tag_end + 1 + idx)
        else {
            break;
        };
        out.push_str(&unescape_xml_text(&paragraph[tag_end + 1..close_start]));
        search_from = close_start + "</w:t>".len();
    }
    out
}

fn text_run_xml(escaped_text: &str) -> String {
    format!("<w:r>{}</w:r>", text_xml(escaped_text))
}

fn text_xml(escaped_text: &str) -> String {
    if escaped_text.starts_with(' ') || escaped_text.ends_with(' ') {
        format!("<w:t xml:space=\"preserve\">{escaped_text}</w:t>")
    } else {
        format!("<w:t>{escaped_text}</w:t>")
    }
}

fn escape_xml_text(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_xml_attr(input: &str) -> String {
    escape_xml_text(input).replace('"', "&quot;")
}

fn unescape_xml_text(input: &str) -> String {
    input
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
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
    fn profile_spec_exposes_jos_rules() {
        let spec = EngineProfile::JosPaper.spec();
        let page_setup = spec
            .default_page_setup()
            .expect("JOS profile should provide page setup");

        assert_eq!(spec.id, "jos-paper");
        assert_eq!(spec.page_setup, PageSetupProfile::JosPaper3);
        assert!(spec.document_classes.contains(&"rjthesis"));
        assert_eq!(spec.caption_policy.figure_prefix, "图");
        assert_eq!(spec.citation_policy.bibliography_style, "unsrt");
        assert_eq!(page_setup.width_twips, 10433);
        assert_eq!(page_setup.height_twips, 14742);
    }

    #[test]
    fn profile_default_page_setup_is_used_for_docx_render() {
        let engine = SemanticTexEngine::new();
        let options = CompileOptions {
            profile: EngineProfile::JosPaper,
            semantic_backend: SemanticBackendKind::RuleBased,
            page_setup: None,
            ..CompileOptions::default()
        };
        let artifact = engine
            .compile_source_to_docx("main.tex", SAMPLE, &options)
            .expect("compile source with profile setup");
        let document_xml = docx_document_xml(&artifact.docx);

        assert_eq!(artifact.report.profile_spec.id, "jos-paper");
        assert_eq!(
            artifact.report.profile_spec.default_page_setup,
            "jos-paper3"
        );
        assert!(document_xml.contains(r#"w:w="10433""#));
        assert!(document_xml.contains(r#"w:h="14742""#));
        assert!(artifact.report.stages.iter().any(|stage| stage
            .message
            .contains("profile default page setup: jos-paper3")));
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
    fn builds_reference_graph_from_source_scan() {
        let source = r#"
Text cites \cite{smith2020,jones2019}, see Figure~\ref{fig:a}, Eq.~\eqref{eq:e}, and \ref{missing}.
% \label{fig:commented}
\begin{figure}
\caption{A}\label{fig:a}
\end{figure}
\begin{equation}
x=1\label{eq:e}
\end{equation}
"#;
        let mut vfs = VirtualFs::new();
        vfs.insert("main.tex", source.as_bytes().to_vec());
        let graph = build_reference_graph(&vfs, &[]);

        assert_eq!(graph.labels.len(), 2);
        assert!(graph.labels.iter().any(|label| {
            label.key == "fig:a"
                && label.kind == ReferenceTargetKind::Figure
                && label.number.as_deref() == Some("1")
        }));
        assert!(graph.labels.iter().any(|label| {
            label.key == "eq:e"
                && label.kind == ReferenceTargetKind::Equation
                && label.number.as_deref() == Some("(1)")
        }));
        assert!(!graph
            .labels
            .iter()
            .any(|label| label.key == "fig:commented"));
        assert_eq!(graph.references.len(), 3);
        assert_eq!(graph.citations.len(), 1);
        assert_eq!(
            graph.citations[0].keys,
            vec!["smith2020".to_string(), "jones2019".to_string()]
        );
        assert!(graph
            .references
            .iter()
            .any(|reference| reference.key == "fig:a"
                && reference.resolved
                && reference.target_kind == Some(ReferenceTargetKind::Figure)
                && reference.rendered.as_deref() == Some("1")));
        assert!(graph
            .references
            .iter()
            .any(|reference| reference.key == "eq:e"
                && reference.resolved
                && reference.target_kind == Some(ReferenceTargetKind::Equation)
                && reference.rendered.as_deref() == Some("(1)")));
        assert_eq!(graph.unresolved_references.len(), 1);
        assert_eq!(graph.unresolved_references[0].key, "missing");
    }

    #[test]
    fn compatibility_analyzer_flags_unsupported_features() {
        let source = r#"
\documentclass{article}
\usepackage{tikz,minted,listings}
\newcommand{\mysection}[1]{\section{#1}}
\begin{tikzpicture}
\end{tikzpicture}
\begin{minted}{rust}
fn main() {}
\end{minted}
\begin{lstlisting}
let x = 1;
\end{lstlisting}
"#;
        let mut vfs = VirtualFs::new();
        vfs.insert("main.tex", source.as_bytes().to_vec());

        let report = analyze_compatibility(&vfs, EngineProfile::GenericArticle);

        assert_eq!(report.scanned_files, 1);
        assert!(report.document_classes.contains(&"article".to_string()));
        assert!(report.packages.contains(&"tikz".to_string()));
        assert!(report.packages.contains(&"minted".to_string()));
        assert_eq!(report.custom_macro_count, 1);
        assert!(report.score < 100);
        assert!(report
            .unsupported
            .iter()
            .any(|issue| { issue.code == "unsupported_package" && issue.feature == "tikz" }));
        assert!(report
            .unsupported
            .iter()
            .any(|issue| { issue.code == "unsupported_package" && issue.feature == "minted" }));
        assert!(report.unsupported.iter().any(|issue| {
            issue.code == "unsupported_environment" && issue.feature == "tikzpicture"
        }));
        assert!(report
            .warnings
            .iter()
            .any(|issue| { issue.code == "limited_package" && issue.feature == "listings" }));
        assert!(report
            .warnings
            .iter()
            .any(|issue| issue.code == "custom_macros"));
    }

    #[test]
    fn compile_report_includes_compatibility_analysis() {
        let source = r#"
\documentclass{article}
\usepackage{minted}
\begin{document}
Body text.
\end{document}
"#;
        let engine = SemanticTexEngine::new();
        let options = CompileOptions {
            semantic_backend: SemanticBackendKind::RuleBased,
            ..CompileOptions::default()
        };
        let mut vfs = VirtualFs::new();
        vfs.insert("main.tex", source.as_bytes().to_vec());
        let graph = engine
            .compile_vfs_to_graph("main.tex", &mut vfs, &options)
            .expect("compile graph");

        assert!(graph.report.stages.iter().any(|stage| {
            stage.stage == CompileStage::CompatibilityAnalyze
                && stage.status == StageStatus::Completed
        }));
        assert!(graph.report.compatibility.score < 100);
        assert!(graph
            .report
            .compatibility
            .unsupported
            .iter()
            .any(|issue| issue.feature == "minted"));
        assert!(graph.report.diagnostics.iter().any(|diag| {
            diag.code == "compatibility_unsupported" && diag.message.contains("minted")
        }));
    }

    #[test]
    fn rule_based_collector_outputs_collected_document() {
        let options = CompileOptions {
            semantic_backend: SemanticBackendKind::RuleBased,
            ..CompileOptions::default()
        };
        let mut vfs = VirtualFs::new();
        vfs.insert("main.tex", SAMPLE.as_bytes().to_vec());
        let mut input = SemanticCollectorInput {
            main_tex: "main.tex",
            vfs: &mut vfs,
            options: &options,
        };
        let mut report = CompileReport::new(options.profile);

        let collected = RuleBasedCollector
            .collect(&mut input, &mut report)
            .expect("collect rule-based document");

        assert_eq!(RuleBasedCollector.name(), "rule-based");
        assert!(collected.document.blocks.iter().any(|block| {
            matches!(block, Block::Heading { text, .. } if text == "Introduction")
        }));
        assert!(collected.standard_document.is_some());
        assert!(collected.events.is_empty());
        assert!(collected.sidecars.is_empty());
        assert_eq!(report.block_count, collected.document.blocks.len());
        assert!(report
            .stages
            .iter()
            .any(|stage| stage.stage == CompileStage::SemanticCollect));
    }

    #[test]
    fn build_sidecar_preserves_metadata() {
        let sidecar = BuildSidecar::new(
            "semantic-events-jsonl",
            Some(SEMANTIC_SIDECAR),
            "runtime semantic event stream",
        );

        assert_eq!(sidecar.kind, "semantic-events-jsonl");
        assert_eq!(sidecar.path.as_deref(), Some(SEMANTIC_SIDECAR));
        assert_eq!(sidecar.description, "runtime semantic event stream");
    }

    #[test]
    fn compile_report_includes_reference_graph_counts() {
        let source = r#"
See Figure~\ref{fig:a} and \ref{missing}.
\begin{figure}
\caption{A}\label{fig:a}
\end{figure}
"#;
        let engine = SemanticTexEngine::new();
        let options = CompileOptions {
            semantic_backend: SemanticBackendKind::RuleBased,
            ..CompileOptions::default()
        };
        let mut vfs = VirtualFs::new();
        vfs.insert("main.tex", source.as_bytes().to_vec());
        let graph = engine
            .compile_vfs_to_graph("main.tex", &mut vfs, &options)
            .expect("compile graph");

        assert_eq!(graph.report.reference_label_count, 1);
        assert_eq!(graph.report.reference_edge_count, 2);
        assert_eq!(graph.report.unresolved_reference_count, 1);
        assert!(graph
            .report
            .diagnostics
            .iter()
            .any(|diag| diag.code == "unresolved_reference" && diag.message.contains("missing")));
        assert_eq!(
            graph.reference_graph.unresolved_references[0].key,
            "missing"
        );
    }

    #[test]
    fn docx_reference_links_add_bookmark_and_hyperlink() {
        let source = r#"
See Figure~\ref{fig:a}.
\begin{figure}
\caption{Demo}\label{fig:a}
\end{figure}
"#;
        let engine = SemanticTexEngine::new();
        let options = CompileOptions {
            semantic_backend: SemanticBackendKind::RuleBased,
            ..CompileOptions::default()
        };
        let mut vfs = VirtualFs::new();
        vfs.insert("main.tex", source.as_bytes().to_vec());
        let graph = engine
            .compile_vfs_to_graph("main.tex", &mut vfs, &options)
            .expect("compile reference graph");
        assert!(graph.reference_graph.references.iter().any(|reference| {
            reference.key == "fig:a"
                && reference.resolved
                && reference.target_kind == Some(ReferenceTargetKind::Figure)
                && reference.rendered.as_deref() == Some("1")
        }));

        let artifact = engine
            .compile_source_to_docx("main.tex", source, &options)
            .expect("compile linked docx");
        let document_xml = docx_document_xml(&artifact.docx);

        assert_eq!(artifact.report.reference_label_count, 1);
        assert_eq!(artifact.report.reference_edge_count, 1);
        assert_eq!(artifact.report.unresolved_reference_count, 0);
        assert!(
            artifact.report.bookmark_count >= 1,
            "expected bookmark count, xml: {document_xml}"
        );
        assert!(
            artifact.report.hyperlink_count >= 1,
            "expected hyperlink count, xml: {document_xml}"
        );
        assert!(document_xml.contains("<w:bookmarkStart"));
        assert!(document_xml.contains(r#"w:name="ref_fig_a""#));
        assert!(document_xml.contains("<w:hyperlink"));
        assert!(document_xml.contains(r#"w:anchor="ref_fig_a""#));
        assert!(
            document_xml.find("<w:hyperlink").unwrap()
                < document_xml.find("<w:bookmarkStart").unwrap()
        );
    }

    #[test]
    fn docx_reference_links_hyperlink_plain_text_run() {
        let paragraph = r#"<w:p><w:pPr><w:pStyle w:val="JOSBody"/></w:pPr><w:r><w:t>See Figure 1.</w:t></w:r></w:p>"#;
        let updated = hyperlink_paragraph(paragraph, "ref_fig_a", &[String::from("Figure 1")])
            .expect("hyperlink text run");

        assert!(updated.contains(r#"<w:hyperlink w:anchor="ref_fig_a""#));
        assert!(updated.contains(r#"<w:t xml:space="preserve">See </w:t>"#));
        assert!(updated.contains("<w:t>Figure 1</w:t>"));
        assert!(updated.contains("<w:t>.</w:t>"));
    }

    #[test]
    fn docx_omml_equation_renders_fraction_and_bookmark() {
        let source = r#"
See Eq.~\eqref{eq:f}.
\begin{equation}
\frac{a+b}{c+d}\label{eq:f}
\end{equation}
"#;
        let engine = SemanticTexEngine::new();
        let options = CompileOptions {
            semantic_backend: SemanticBackendKind::RuleBased,
            ..CompileOptions::default()
        };
        let artifact = engine
            .compile_source_to_docx("main.tex", source, &options)
            .expect("compile omml docx");
        let document_xml = docx_document_xml(&artifact.docx);

        assert_eq!(artifact.report.reference_label_count, 1);
        assert_eq!(artifact.report.reference_edge_count, 1);
        assert_eq!(artifact.report.unresolved_reference_count, 0);
        assert_eq!(artifact.report.omml_equation_count, 1);
        assert_eq!(artifact.report.omml_equation_fallback_count, 0);
        assert!(document_xml.contains("<m:oMath"));
        assert!(document_xml.contains("<m:f>"));
        assert!(document_xml.contains("<m:num>"));
        assert!(document_xml.contains("<m:den>"));
        assert!(document_xml.contains(r#"<w:t xml:space="preserve">    (1)</w:t>"#));
        assert!(document_xml.contains(r#"w:name="ref_eq_f""#));
        assert_eq!(document_xml.matches("<?xml").count(), 1);
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

    fn docx_document_xml(docx: &[u8]) -> String {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(docx)).expect("docx zip");
        let mut file = archive
            .by_name("word/document.xml")
            .expect("document.xml part");
        let mut xml = String::new();
        file.read_to_string(&mut xml).expect("read document.xml");
        xml
    }
}
