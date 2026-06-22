//! Semantic TeX compiler engine.
//!
//! This crate is the facade for the next-generation TeX -> DOCX pipeline. It
//! keeps the current rule-based LaTeX reader and DOCX writer behind explicit
//! compiler stages, so later LuaHook/XDV/OMML implementations can replace
//! individual stages without changing callers.

#![forbid(unsafe_code)]

mod journal_detector;
mod profiles;

use std::collections::HashSet;
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
use doc_rule_engine::{
    DecisionSource, MacroRule, RuleEngine, RuleOutput, route_rule_output, RoutingConfig,
};
use doc_xdv_parser::to_collector_layout_graph;
use doc_utils::{ImageAssets, VirtualFs};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Re-export types and functions from doc_semantic_collector
pub use doc_semantic_collector::{
    BackendAvailability, BackendSelectionReport, BuildSidecar, build_reference_graph,
    CitationReference, CollectedDocument as SemanticCollectorDocument, CrossReference, EventSource,
    is_tex_like_path, LayoutGraph, LayoutNode, parse_semantic_events_jsonl, path_to_posix,
    ReferenceGraph, ReferenceLabel, ReferenceOrigin, ReferenceSource, ReferenceTargetKind,
    scan_tex_commands, SemanticBackendKind, SemanticEvent, SemanticEventV2, SourceSpan,
    strip_tex_comments, UnresolvedReference,
};

// Re-export types from doc_compatibility_analyzer
pub use doc_compatibility_analyzer::{
    CompatibilityAnalyzer, CompatibilityIssue, CompatibilityReport, ProfileKind,
};

// Re-export types from journal_detector
pub use journal_detector::{
    DiagnosticLevel, JournalDetection, JournalDetectionReport, JournalDiagnostic,
    JournalDetector, MatchedSignal, SignalKind,
};

// Import profile registry (internal module)
use crate::profiles::{MacroRuleToml, ProfileRegistry};

impl From<EngineProfile> for ProfileKind {
    fn from(ep: EngineProfile) -> Self {
        match ep {
            EngineProfile::GenericArticle => ProfileKind::GenericArticle,
            EngineProfile::ChineseAcademic => ProfileKind::ChineseAcademic,
            EngineProfile::JosPaper => ProfileKind::JosPaper,
            EngineProfile::MedicalJournal => ProfileKind::MedicalJournal,
        }
    }
}

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

/// How the user or the pipeline specified a profile.
#[derive(Debug, Clone)]
pub enum ProfileRef {
    /// Automatically detected by `JournalDetector`.
    Auto,
    /// Specified by canonical profile ID (e.g. "tacl", "cvpr", "nature").
    Id(String),
    /// Specified by explicit file path to a TOML/JSON profile.
    Path(PathBuf),
    /// Legacy `EngineProfile` variant (backwards compatibility).
    Legacy(EngineProfile),
}

/// The active profile driving the current compile session.
///
/// This is the P1 cornerstone: it replaces the bare `EngineProfile` enum as the
/// compilation context, carrying the resolved TOML spec, the source of the
/// selection, and optionally the auto-detection report.
#[derive(Debug, Clone)]
pub struct ActiveProfile {
    /// Canonical profile ID (e.g. "tacl", "generic-article").
    pub id: String,
    /// Fully resolved profile specification (may be from TOML or built-in).
    pub spec: profiles::ProfileSpecFile,
    /// How this profile was selected.
    pub source: ProfileSource,
    /// Auto-detection report (present when `source == AutoDetected`).
    pub detection: Option<journal_detector::JournalDetectionReport>,
}

/// A serializable summary of the active profile for `CompileReport`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveProfileReport {
    pub id: String,
    pub display_name: String,
    pub source: String,
    pub confidence: Option<f32>,
    pub document_classes: Vec<String>,
    pub backend_preferred: String,
    /// P3: Quality thresholds for this profile.
    pub quality: profiles::QualitySpec,
    /// P3: Style map entries for this profile.
    pub style_map: Vec<profiles::StyleMapSpec>,
}

impl ActiveProfileReport {
    fn from_active(active: &ActiveProfile) -> Self {
        Self {
            id: active.id.clone(),
            display_name: active.spec.display_name.clone(),
            source: active.source.to_string(),
            confidence: active.detection.as_ref().map(|d| d.confidence),
            document_classes: active.spec.document_classes.clone(),
            backend_preferred: active.spec.backend.preferred.clone(),
            quality: active.spec.quality.clone(),
            style_map: active.spec.style_map.clone(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSource {
    /// User passed `--profile <id>` on the CLI.
    ExplicitId,
    /// User passed `--profile-path <path>` on the CLI.
    ExplicitPath,
    /// Automatically detected by `JournalDetector`.
    AutoDetected,
    /// Fell back to generic because no other profile matched.
    Fallback,
}

impl std::fmt::Display for ProfileSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileSource::ExplicitId => write!(f, "explicit_id"),
            ProfileSource::ExplicitPath => write!(f, "explicit_path"),
            ProfileSource::AutoDetected => write!(f, "auto_detected"),
            ProfileSource::Fallback => write!(f, "fallback"),
        }
    }
}

impl ActiveProfile {
    /// Resolve a `ProfileRef` into an `ActiveProfile`.
    ///
    /// This is the P1 "resolve active profile" step — it replaces the old
    /// `parse_engine_profile()` logic with full TOML profile loading.
    pub fn resolve(
        profile_ref: &ProfileRef,
        vfs: &doc_utils::VirtualFs,
    ) -> Result<Self, EngineError> {
        match profile_ref {
            ProfileRef::Auto => Self::resolve_auto(vfs),
            ProfileRef::Id(id) => Self::resolve_by_id(id),
            ProfileRef::Path(path) => Self::resolve_by_path(path),
            ProfileRef::Legacy(legacy) => Self::resolve_legacy(*legacy),
        }
    }

    fn resolve_auto(vfs: &doc_utils::VirtualFs) -> Result<Self, EngineError> {
        let detector = journal_detector::JournalDetector::new();
        let detection = detector.detect(vfs);
        let id = &detection.selected_profile_id;
        let spec = Self::load_spec_by_id(id)
            .ok_or_else(|| EngineError::Profile(format!("auto-detected profile '{id}' not found")))?;
        Ok(Self {
            id: id.clone(),
            spec,
            source: ProfileSource::AutoDetected,
            detection: Some(detection),
        })
    }

    fn resolve_by_id(id: &str) -> Result<Self, EngineError> {
        let spec = Self::load_spec_by_id(id)
            .ok_or_else(|| EngineError::Profile(format!("profile '{id}' not found")))?;
        Ok(Self {
            id: id.to_string(),
            spec,
            source: ProfileSource::ExplicitId,
            detection: None,
        })
    }

    fn resolve_by_path(path: &Path) -> Result<Self, EngineError> {
        let spec = profiles::load_from_file(path).map_err(|e| {
            EngineError::Profile(format!("failed to load profile from {:?}: {}", path, e))
        })?;
        Ok(Self {
            id: spec.id.clone(),
            spec,
            source: ProfileSource::ExplicitPath,
            detection: None,
        })
    }

    fn resolve_legacy(legacy: EngineProfile) -> Result<Self, EngineError> {
        let id = legacy.id();
        let spec = Self::load_spec_by_id(id)
            .ok_or_else(|| EngineError::Profile(format!("legacy profile '{id}' not found")))?;
        Ok(Self {
            id: id.to_string(),
            spec,
            source: ProfileSource::ExplicitId,
            detection: None,
        })
    }

    /// Load a profile spec by ID from the registry.
    fn load_spec_by_id(id: &str) -> Option<profiles::ProfileSpecFile> {
        let registry = profiles::ProfileRegistry::load_default().ok()?;
        registry.get(id).cloned()
    }

    /// Convenience: convert to `ProfileKind` for `CompatibilityAnalyzer`.
    pub fn to_profile_kind(&self) -> doc_compatibility_analyzer::ProfileKind {
        match self.id.as_str() {
            "generic" | "generic-article" | "generic-article-toml" => {
                doc_compatibility_analyzer::ProfileKind::Generic
            }
            "chinese-academic" | "chinese-academic-toml" => {
                doc_compatibility_analyzer::ProfileKind::ChineseAcademic
            }
            "jos-paper" | "jos-paper-toml" => doc_compatibility_analyzer::ProfileKind::JosPaper,
            "medical-journal" => doc_compatibility_analyzer::ProfileKind::MedicalJournal,
            "tacl" | "acl-paper" | "acl" => doc_compatibility_analyzer::ProfileKind::Tacl,
            "cvpr" | "iccv" | "cvpr-paper" => doc_compatibility_analyzer::ProfileKind::Cvpr,
            "nature" | "nature-research" => doc_compatibility_analyzer::ProfileKind::Nature,
            "springer" | "svjour3" | "llncs" | "springer-journal" => {
                doc_compatibility_analyzer::ProfileKind::Springer
            }
            _ => doc_compatibility_analyzer::ProfileKind::Generic,
        }
    }
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
                style_map: Some(doc_docx_writer::ProfileStyleMap::generic()),
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
                style_map: None,
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
                style_map: Some(doc_docx_writer::ProfileStyleMap::jos()),
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
                style_map: None,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSpec {
    pub profile: EngineProfile,
    pub id: &'static str,
    pub display_name: &'static str,
    pub document_classes: &'static [&'static str],
    pub page_setup: PageSetupProfile,
    pub font_policy: FontPolicySpec,
    pub caption_policy: CaptionPolicySpec,
    pub citation_policy: CitationPolicySpec,
    /// Profile-specific DOCX style mappings (e.g. JOS vs generic).
    pub style_map: Option<doc_docx_writer::ProfileStyleMap>,
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

/// Options controlling semantic collection and DOCX rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileOptions {
    /// Legacy profile field (used when `profile_ref` is None).
    pub profile: EngineProfile,
    /// P1: How the profile was specified. When Some, takes precedence over `profile`.
    #[serde(skip)]
    pub profile_ref: Option<ProfileRef>,
    pub semantic_backend: SemanticBackendKind,
    pub allow_backend_fallback: bool,
    pub enable_reference_links: bool,
    pub enable_ref_fields: bool,
    pub enable_omml_equations: bool,
    pub template_bytes: Option<Vec<u8>>,
    pub page_setup: Option<doc_docx_writer::PageSetup>,
    pub collect_standard_ast: bool,
    pub enable_bibliography: bool,
    /// Optional profile-specific style map. If None, uses the profile's default.
    #[serde(skip)]
    pub style_map: Option<doc_docx_writer::ProfileStyleMap>,
    /// Internal: resolved active profile (set by `resolve_active_profile`, used by
    /// `apply_rule_engine_to_document`).
    #[serde(skip)]
    pub active_profile: Option<ActiveProfile>,
    /// Internal: rule engine report populated after semantic collection.
    #[serde(skip)]
    pub rule_engine: Option<RuleEngineReport>,
    /// P4.3: Override the minimum compatibility score from the CLI --quality flag.
    #[serde(skip)]
    pub min_compatibility_score_override: Option<u8>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            profile: EngineProfile::ChineseAcademic,
            profile_ref: None,
            semantic_backend: SemanticBackendKind::Auto,
            allow_backend_fallback: true,
            enable_reference_links: true,
            enable_ref_fields: false,
            enable_omml_equations: true,
            template_bytes: None,
            page_setup: None,
            collect_standard_ast: true,
            enable_bibliography: true,
            style_map: None,
            active_profile: None,
            rule_engine: None,
            min_compatibility_score_override: None,
        }
    }
}

impl CompileOptions {
    /// Internal: set the resolved active profile. Called by `compile_vfs_to_graph`.
    pub fn set_active_profile(&mut self, profile: ActiveProfile) -> &mut Self {
        self.active_profile = Some(profile);
        self
    }

    /// Internal: set the rule engine report after semantic collection.
    pub fn set_rule_engine_report(&mut self, report: RuleEngineReport) -> &mut Self {
        self.rule_engine = Some(report);
        self
    }

    pub fn effective_page_setup(&self) -> Option<doc_docx_writer::PageSetup> {
        self.page_setup
            .clone()
            .or_else(|| self.profile.spec().default_page_setup())
    }

    /// Returns the effective style map: explicit override takes precedence,
    /// then falls back to the profile's built-in default.
    pub fn effective_style_map(&self) -> Option<doc_docx_writer::ProfileStyleMap> {
        self.style_map
            .clone()
            .or_else(|| self.profile.spec().style_map.clone())
    }

    /// P1: Resolve the active profile from the `profile_ref` or fall back to legacy.
    /// Sets `self.active_profile` and returns the resolved value.
    pub fn resolve_active_profile(
        &mut self,
        vfs: &doc_utils::VirtualFs,
    ) -> Result<ActiveProfile, EngineError> {
        let active = if let Some(ref pref) = self.profile_ref {
            ActiveProfile::resolve(pref, vfs)?
        } else {
            // Legacy path: use EngineProfile
            ActiveProfile::resolve(&ProfileRef::Legacy(self.profile), vfs)?
        };
        self.active_profile = Some(active.clone());
        Ok(active)
    }

    /// P1: Convenience — resolve profile without mutating self (for one-shot calls).
    pub fn resolve_profile(vfs: &doc_utils::VirtualFs) -> Result<ActiveProfile, EngineError> {
        ActiveProfile::resolve(&ProfileRef::Auto, vfs)
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

        let style_map = options.effective_style_map();
        let mut docx = doc_docx_writer::pack_with_page_setup(
            &graph.document,
            options.template_bytes.as_deref(),
            Some(&graph.image_assets),
            page_setup.as_ref(),
            style_map.as_ref(),
        )
        .map_err(|e| EngineError::Serialize(e.to_string()))?;
        if options.enable_omml_equations {
            let omml = apply_omml_equations_to_docx(docx, &graph.document)?;
            graph.report.omml_equation_count = omml.converted;
            graph.report.omml_equation_fallback_count = omml.fallbacks;
            docx = omml.docx;
        }
        if options.enable_reference_links {
            let linked = apply_reference_links_to_docx(docx, &graph.reference_graph, options.enable_ref_fields)?;
            graph.report.bookmark_count = linked.bookmarks;
            graph.report.hyperlink_count = linked.hyperlinks;
            docx = linked.docx;
        }
        graph.report.docx_bytes = docx.len();

        // P3: Run quality gate with profile-specific thresholds (CLI --quality can override)
        let quality_spec = graph.report.active_profile.as_ref()
            .map(|ap| ap.quality.clone())
            .unwrap_or_else(profiles::QualitySpec::default);
        let quality_spec = if let Some(override_score) = options.min_compatibility_score_override {
            let mut spec = quality_spec;
            spec.min_compatibility_score = override_score;
            spec
        } else {
            quality_spec
        };
        graph.report.run_quality_gate(&quality_spec);

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
        // P1: Resolve the active profile
        let active = ActiveProfile::resolve(
            options.profile_ref.as_ref().unwrap_or(&ProfileRef::Legacy(options.profile)),
            vfs,
        )?;

        let mut report = CompileReport::new(options.profile);
        report.set_active_profile(&active);
        report.push(
            CompileStage::SourceMount,
            StageStatus::Completed,
            format!("mounted {} VFS entries", vfs.paths().count()),
        );

        // P2: Journal auto-detection — runs only when source is AutoDetected.
        // When source is ExplicitId/Path/Legacy, skip redundant detection.
        let journal_report = if active.source == ProfileSource::AutoDetected {
            let detection = active.detection.clone();
            report.push(
                CompileStage::JournalDetect,
                StageStatus::Completed,
                format!(
                    "detected '{}' (confidence {:.2})",
                    active.id,
                    detection.as_ref().map(|d| d.confidence).unwrap_or(0.0)
                ),
            );
            for diag in detection.as_ref().into_iter().flat_map(|d| &d.diagnostics) {
                let severity = match diag.level {
                    DiagnosticLevel::Info => DiagnosticSeverity::Info,
                    DiagnosticLevel::Warning => DiagnosticSeverity::Warning,
                };
                report.diagnostics.push(EngineDiagnostic {
                    severity,
                    code: diag.code.clone(),
                    message: diag.message.clone(),
                });
            }
            detection
        } else {
            report.push(
                CompileStage::JournalDetect,
                StageStatus::Completed,
                format!(
                    "profile '{}' explicitly set (source: {})",
                    active.id, active.source
                ),
            );
            None
        };
        report.journal_detection = journal_report;

        let compatibility = CompatibilityAnalyzer::new().analyze(vfs, active.to_profile_kind());
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

        let mut opts = options.clone();
        opts.set_active_profile(active.clone());

        let mut artifact = collect_with_selected_backend(main_tex, vfs, &mut opts, &mut report)?;
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

        // P2: Transfer rule engine report from options into the final report.
        if let Some(re) = opts.rule_engine.take() {
            report.rule_engine = Some(re);
        }

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

/// P3: Result of a quality gate check applied after DOCX generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGateResult {
    pub status: QualityStatus,
    pub score: u8,
    pub total_checks: usize,
    pub passed_checks: usize,
    pub failed_checks: Vec<QualityCheck>,
    pub warnings: Vec<QualityCheck>,
}

/// P3: Overall quality gate status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityStatus {
    Passed,
    PassedWithWarnings,
    Failed,
}

impl QualityStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Passed => "Passed",
            Self::PassedWithWarnings => "PassedWithWarnings",
            Self::Failed => "Failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityCheck {
    pub name: String,
    pub passed: bool,
    pub severity: QualitySeverity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualitySeverity {
    Error,
    Warning,
    Info,
}

impl QualitySeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

/// P2: Report summarizing RuleEngine activity during compilation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEngineReport {
    /// Number of builtin rules loaded.
    pub builtin_rules: usize,
    /// Number of journal-specific rules loaded.
    pub journal_rules: usize,
    /// Number of profile-specific rules loaded from TOML.
    pub profile_rules: usize,
    /// Number of distinct unknown macros encountered.
    pub unknown_macro_count: usize,
    /// List of unknown macro names encountered.
    pub unknown_macros: Vec<String>,
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
    pub active_profile: Option<ActiveProfileReport>,
    pub backend: BackendSelectionReport,
    pub compatibility: CompatibilityReport,
    pub journal_detection: Option<JournalDetectionReport>,
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
    pub quality_gate: Option<QualityGateResult>,
    /// P2: RuleEngine activity report (populated after semantic collection).
    pub rule_engine: Option<RuleEngineReport>,
}

impl CompileReport {
    pub fn new(profile: EngineProfile) -> Self {
        Self {
            profile,
            profile_spec: ProfileSpecReport::from_spec(profile.spec()),
            active_profile: None,
            backend: BackendSelectionReport::default(),
            compatibility: CompatibilityReport::default(),
            journal_detection: None,
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
            quality_gate: None,
            rule_engine: None,
        }
    }

    pub fn push(&mut self, stage: CompileStage, status: StageStatus, message: impl Into<String>) {
        self.stages.push(StageReport {
            stage,
            status,
            message: message.into(),
        });
    }

    /// Attach the resolved active profile to this report.
    pub fn set_active_profile(&mut self, active: &ActiveProfile) {
        self.active_profile = Some(ActiveProfileReport::from_active(active));
    }

    /// P3: Run quality gate checks using profile-specific thresholds from `spec`.
    ///
    /// Checks (up to 10):
    /// 1. compatibility score >= min_compatibility_score (Error)
    /// 2. unresolved references == 0 (Error/Warning)
    /// 3. raw fallback blocks <= max_raw_fallback_blocks (Error)
    /// 4. docx_bytes > 0 (Error)
    /// 5. style coverage (Info)
    /// 6. profile confidence < 0.80 (Warning)
    /// 7. runtime fallback (Warning)
    /// 8. backend runtime fallback (Warning)
    /// 9. OMML equation fallback (Warning)
    /// 10. journal profile detection (Info)
    pub fn run_quality_gate(&mut self, spec: &profiles::QualitySpec) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut infos = Vec::new();

        // 1. Compatibility score
        let score = self.compatibility.score;
        if score >= spec.min_compatibility_score {
            infos.push(QualityCheck {
                name: "compatibility_score".to_string(),
                passed: true,
                severity: QualitySeverity::Info,
                message: format!("compatibility score {} >= {}: pass", score, spec.min_compatibility_score),
            });
        } else {
            errors.push(QualityCheck {
                name: "compatibility_score".to_string(),
                passed: false,
                severity: QualitySeverity::Error,
                message: format!("compatibility score {} < {}: FAIL", score, spec.min_compatibility_score),
            });
        }

        // 2. Unresolved references
        let unresolved = self.unresolved_reference_count;
        if unresolved == 0 {
            infos.push(QualityCheck {
                name: "unresolved_references".to_string(),
                passed: true,
                severity: QualitySeverity::Info,
                message: "no unresolved references: pass".to_string(),
            });
        } else {
            warnings.push(QualityCheck {
                name: "unresolved_references".to_string(),
                passed: false,
                severity: QualitySeverity::Warning,
                message: format!("{} unresolved reference(s): WARN", unresolved),
            });
        }

        // 3. Raw fallback blocks (from rule_engine report)
        let raw_count = self.rule_engine.as_ref().map(|r| r.unknown_macro_count).unwrap_or(0);
        if raw_count <= spec.max_raw_fallback_blocks {
            infos.push(QualityCheck {
                name: "raw_fallback_blocks".to_string(),
                passed: true,
                severity: QualitySeverity::Info,
                message: format!("{} unknown macro(s) <= {}: pass", raw_count, spec.max_raw_fallback_blocks),
            });
        } else {
            errors.push(QualityCheck {
                name: "raw_fallback_blocks".to_string(),
                passed: false,
                severity: QualitySeverity::Error,
                message: format!("{} unknown macro(s) > {} max: FAIL", raw_count, spec.max_raw_fallback_blocks),
            });
        }

        // 4. DOCX non-empty
        let bytes = self.docx_bytes;
        if bytes > 0 {
            infos.push(QualityCheck {
                name: "docx_non_empty".to_string(),
                passed: true,
                severity: QualitySeverity::Info,
                message: format!("DOCX size {} bytes: pass", bytes),
            });
        } else {
            errors.push(QualityCheck {
                name: "docx_non_empty".to_string(),
                passed: false,
                severity: QualitySeverity::Error,
                message: "DOCX is empty: FAIL".to_string(),
            });
        }

        // 5. Style coverage (Info)
        let style_map_entries = self.active_profile.as_ref()
            .map(|ap| ap.style_map.len())
            .unwrap_or(0);
        if style_map_entries > 0 {
            infos.push(QualityCheck {
                name: "style_coverage".to_string(),
                passed: true,
                severity: QualitySeverity::Info,
                message: format!("{} style map entries defined for profile", style_map_entries),
            });
        }

        // 6. Profile confidence
        if let Some(ref det) = self.journal_detection {
            if det.confidence < 0.80 {
                warnings.push(QualityCheck {
                    name: "profile_confidence".to_string(),
                    passed: false,
                    severity: QualitySeverity::Warning,
                    message: format!("profile detection confidence {:.0}% < 80%: low confidence", det.confidence * 100.0),
                });
            } else {
                infos.push(QualityCheck {
                    name: "profile_confidence".to_string(),
                    passed: true,
                    severity: QualitySeverity::Info,
                    message: format!("profile detection confidence {:.0}%: pass", det.confidence * 100.0),
                });
            }
        }

        // 7. Runtime fallback from rule engine
        if let Some(ref re) = self.rule_engine {
            if re.unknown_macro_count > 0 {
                warnings.push(QualityCheck {
                    name: "runtime_fallback".to_string(),
                    passed: false,
                    severity: QualitySeverity::Warning,
                    message: format!("{} unknown macro(s) fell back to text", re.unknown_macro_count),
                });
            }
        }

        // 8. Backend runtime fallback
        if self.backend.selected != self.backend.requested {
            warnings.push(QualityCheck {
                name: "backend_fallback".to_string(),
                passed: false,
                severity: QualitySeverity::Warning,
                message: format!("backend fell back from {:?} to {:?}", self.backend.requested, self.backend.selected),
            });
        } else {
            infos.push(QualityCheck {
                name: "backend_fallback".to_string(),
                passed: true,
                severity: QualitySeverity::Info,
                message: "requested backend selected: pass".to_string(),
            });
        }

        // 9. OMML equation fallback
        let omml_total = self.omml_equation_count;
        let omml_fallback = self.omml_equation_fallback_count;
        if omml_total == 0 || omml_fallback < omml_total {
            infos.push(QualityCheck {
                name: "omml_equation_fallback".to_string(),
                passed: true,
                severity: QualitySeverity::Info,
                message: format!("{}/{} equations used OMML: pass", omml_total.saturating_sub(omml_fallback), omml_total),
            });
        } else {
            warnings.push(QualityCheck {
                name: "omml_equation_fallback".to_string(),
                passed: false,
                severity: QualitySeverity::Warning,
                message: format!("all {} equations fell back to text: WARN", omml_total),
            });
        }

        // 10. Journal profile detection (Info)
        if let Some(ref det) = self.journal_detection {
            infos.push(QualityCheck {
                name: "journal_detection".to_string(),
                passed: true,
                severity: QualitySeverity::Info,
                message: format!("detected '{}' (confidence {:.0}%)", det.selected_profile_id, det.confidence * 100.0),
            });
        }

        let total = errors.len() + warnings.len() + infos.len();
        self.quality_gate = Some(QualityGateResult {
            status: if !errors.is_empty() {
                QualityStatus::Failed
            } else if !warnings.is_empty() {
                QualityStatus::PassedWithWarnings
            } else {
                QualityStatus::Passed
            },
            score,
            total_checks: total,
            passed_checks: infos.len() + warnings.len(),
            failed_checks: errors,
            warnings,
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompileStage {
    SourceMount,
    JournalDetect,
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

/// NOTE: compiler-engine has its OWN version of CollectedDocument (with EngineDiagnostic).
/// Do NOT replace it with doc_semantic_collector's version.
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

pub struct SemanticCollectorInput<'a> {
    pub main_tex: &'a str,
    pub vfs: &'a mut VirtualFs,
    /// P2: Mutable so `RuleBasedCollector` can set `rule_engine` report after collection.
    pub options: &'a mut CompileOptions,
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
        options: &mut CompileOptions,
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

        // P1-2: Apply RuleEngine to transform RawFallback blocks into structured blocks.
        // P2: Profile-aware: use the active_profile stored on CompileOptions.
        // Graceful degradation: if no active_profile, use a default engine.
        let (document, engine) = if let Some(active) = input.options.active_profile.as_ref() {
            apply_rule_engine_to_document(document, active)
        } else {
            apply_rule_engine_to_document(
                document,
                &ActiveProfile {
                    id: "builtin".to_string(),
                    spec: Default::default(),
                    source: ProfileSource::Fallback,
                    detection: None,
                },
            )
        };
        let re_report = build_rule_engine_report(&engine);
        input.options.set_rule_engine_report(re_report);

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
        options: &mut CompileOptions,
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
        options: &mut CompileOptions,
        report: &mut CompileReport,
    ) -> Result<SemanticBackendArtifact, EngineError> {
        let result = collect_runtime_events(RuntimeEngine::XeLaTeX, main_tex, vfs)?;
        let event_count = result.events.len();
        let mut artifact = RuleBasedBackend.collect(main_tex, vfs, options, report)?;
        artifact.events = result.events;
        // M2-1: Parse XDV output from XeLaTeX for LayoutGraph
        let layout = if let Some(xdv_path) = result.xdv_path {
            parse_layout_from_xdv(&xdv_path)
        } else {
            None
        };
        artifact.layout = layout.or_else(|| Some(LayoutGraph::default()));
        artifact.sidecars.push(BuildSidecar::new(
            "semantic-events-jsonl",
            Some(SEMANTIC_SIDECAR),
            format!(
                "semantic events from XeLaTeX hook; profile_id={}; origin=runtime-xelatex",
                options.profile.id()
            ),
        ));
        artifact.diagnostics.push(EngineDiagnostic::info(
            "runtime_semantic_events",
            format!("XeLaTeXHookBackend collected {event_count} semantic events; profile={}", options.profile.id()),
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
        options: &mut CompileOptions,
        report: &mut CompileReport,
    ) -> Result<SemanticBackendArtifact, EngineError> {
        let result = collect_runtime_events(RuntimeEngine::LuaLaTeX, main_tex, vfs)?;
        let event_count = result.events.len();
        let node_tree_count = result.node_tree.len();
        let mut artifact = RuleBasedBackend.collect(main_tex, vfs, options, report)?;
        artifact.events = result.events;

        // M4-2: Build LayoutGraph from node tree entries (detailed layout info)
        // Falls back to XDV-based layout if node tree is empty
        let layout = if !result.node_tree.is_empty() {
            // Use detailed node tree entries to build LayoutGraph
            let nodes: Vec<LayoutNode> = result
                .node_tree
                .iter()
                .filter_map(|entry| {
                    // Convert each node entry to a LayoutNode
                    let (id, kind, x, y, width, height, depth, font_id, font_name, char) =
                        match entry {
                            NodeEntry::Glyph {
                                x,
                                y,
                                char,
                                font_id,
                                font_name,
                                width,
                                height,
                                depth,
                                ..
                            } => (
                                format!("glyph_{}_{}", x, y),
                                "glyph".to_string(),
                                Some(*x),
                                Some(*y),
                                Some(*width),
                                Some(*height),
                                Some(*depth),
                                Some(*font_id),
                                font_name.clone(),
                                Some(*char),
                            ),
                            NodeEntry::Hlist {
                                x,
                                y,
                                width,
                                height,
                                depth,
                                ..
                            } => (
                                format!("hlist_{}_{}", x, y),
                                "hlist".to_string(),
                                Some(*x),
                                Some(*y),
                                Some(*width),
                                Some(*height),
                                Some(*depth),
                                None,
                                None,
                                None,
                            ),
                            NodeEntry::Vlist {
                                x,
                                y,
                                width,
                                height,
                                depth,
                                ..
                            } => (
                                format!("vlist_{}_{}", x, y),
                                "vlist".to_string(),
                                Some(*x),
                                Some(*y),
                                Some(*width),
                                Some(*height),
                                Some(*depth),
                                None,
                                None,
                                None,
                            ),
                            NodeEntry::Glue {
                                x,
                                y,
                                width,
                                stretch: _,
                                shrink: _,
                                stretch_order: _,
                                shrink_order: _,
                                ..
                            } => (
                                format!("glue_{}_{}", x, y),
                                "glue".to_string(),
                                Some(*x),
                                Some(*y),
                                Some(*width),
                                None,
                                None,
                                None,
                                None,
                                None,
                            ),
                            NodeEntry::Rule {
                                x,
                                y,
                                width,
                                height,
                                depth,
                                ..
                            } => (
                                format!("rule_{}_{}", x, y),
                                "rule".to_string(),
                                Some(*x),
                                Some(*y),
                                Some(*width),
                                Some(*height),
                                Some(*depth),
                                None,
                                None,
                                None,
                            ),
                            NodeEntry::Kern { x, y, kern, .. } => (
                                format!("kern_{}_{}", x, y),
                                "kern".to_string(),
                                Some(*x),
                                Some(*y),
                                Some(*kern),
                                None,
                                None,
                                None,
                                None,
                                None,
                            ),
                            NodeEntry::Penalty { x, y, .. } => (
                                format!("penalty_{}_{}", x, y),
                                "penalty".to_string(),
                                Some(*x),
                                Some(*y),
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                            ),
                            NodeEntry::LocalPar { x, y, .. } => (
                                format!("local_par_{}_{}", x, y),
                                "local_par".to_string(),
                                Some(*x),
                                Some(*y),
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                            ),
                            NodeEntry::Dir { x, y, dir, .. } => (
                                format!("dir_{}_{}", x, y),
                                "dir".to_string(),
                                Some(*x),
                                Some(*y),
                                None,
                                None,
                                None,
                                None,
                                dir.clone(),
                                None,
                            ),
                            // Skip summary entries - they don't produce layout nodes
                            NodeEntry::NodeTree { .. } => return None,
                        };

                    Some(LayoutNode {
                        id,
                        kind,
                        page: None,
                        x,
                        y,
                        font_id,
                        font_name,
                        char,
                        width,
                        height,
                        depth,
                    })
                })
                .collect();
            Some(LayoutGraph { nodes })
        } else if let Some(xdv_path) = result.xdv_path {
            // Fallback to XDV-based layout if no node tree entries
            parse_layout_from_xdv(&xdv_path)
        } else {
            None
        };
        artifact.layout = layout.or_else(|| Some(LayoutGraph::default()));

        artifact.sidecars.push(BuildSidecar::new(
            "semantic-events-jsonl",
            Some(SEMANTIC_SIDECAR),
            format!(
                "semantic events from LuaTeX hook; profile_id={}; origin=runtime-luatex",
                options.profile.id()
            ),
        ));
        if node_tree_count > 0 {
            artifact.sidecars.push(BuildSidecar::new(
                "node-tree-jsonl",
                Some(NODE_TREE_SIDECAR),
                format!("{} node tree entries collected from the LuaTeX hook", node_tree_count),
            ));
        }
        artifact.diagnostics.push(EngineDiagnostic::info(
            "runtime_semantic_events",
            format!("LuaTeXNodeBackend collected {event_count} semantic events; profile={}", options.profile.id()),
        ));
        if node_tree_count > 0 {
            artifact.diagnostics.push(EngineDiagnostic::info(
                "node_tree_entries",
                format!("LuaTeXNodeBackend collected {node_tree_count} node tree entries"),
            ));
        }
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
    #[error("Profile 错误：{0}")]
    Profile(String),
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

impl From<profiles::ProfileLoadError> for EngineError {
    fn from(err: profiles::ProfileLoadError) -> Self {
        Self::Profile(err.to_string())
    }
}

impl From<doc_semantic_collector::CollectorError> for EngineError {
    fn from(err: doc_semantic_collector::CollectorError) -> Self {
        match err {
            doc_semantic_collector::CollectorError::Io(e) => Self::Io(e),
            doc_semantic_collector::CollectorError::Parse(e) => Self::Parse(e),
            doc_semantic_collector::CollectorError::Runtime(e) => Self::Parse(e),
        }
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

fn select_auto_backend(vfs: &VirtualFs, journal_detection: Option<&JournalDetectionReport>) -> AutoBackendSelection {
    let signals = collect_template_signals(vfs);
    let availability = RuntimeAvailabilitySnapshot::detect();
    select_auto_backend_with_profile_and_availability(&signals, &availability, journal_detection)
}

/// Profile-aware backend selection using journal detection result.
fn select_auto_backend_with_profile_and_availability(
    signals: &TemplateSignals,
    availability: &RuntimeAvailabilitySnapshot,
    journal_detection: Option<&JournalDetectionReport>,
) -> AutoBackendSelection {
    // 1. Determine initial preferred backend from profile.
    let profile_preferred = journal_detection
        .and_then(|r| {
            ProfileRegistry::load_default()
                .ok()
                .and_then(|reg| reg.get(&r.selected_profile_id).cloned())
        })
        .and_then(|spec| {
            if spec.backend.preferred.is_empty() {
                None
            } else {
                Some(spec.backend.preferred.clone())
            }
        });

    let profile_id = journal_detection.map(|r| r.selected_profile_id.as_str()).unwrap_or("unknown");

    // Helper to convert string backend name to SemanticBackendKind.
    let str_to_backend = |s: &str| -> Option<SemanticBackendKind> {
        match s {
            "luatex-node" | "luatex" => Some(SemanticBackendKind::LuaTeXNode),
            "xelatex-hook" | "xelatex" => Some(SemanticBackendKind::XeLaTeXHook),
            "rule-based" | "rulebased" => Some(SemanticBackendKind::RuleBased),
            _ => None,
        }
    };

    // 2. Collect ordered candidate backends: profile preferred first, then fallback chain.
    let mut candidates: Vec<SemanticBackendKind> = Vec::new();
    if let Some(preferred) = profile_preferred.as_deref().and_then(str_to_backend) {
        candidates.push(preferred);
    }
    // Append fallback chain from profile (if available).
    if let Some(jd) = journal_detection {
        if let Some(spec) = ProfileRegistry::load_default().ok().and_then(|reg| reg.get(&jd.selected_profile_id).cloned()) {
            for fb in &spec.backend.fallback {
                if let Some(bk) = str_to_backend(fb) {
                    if !candidates.contains(&bk) {
                        candidates.push(bk);
                    }
                }
            }
        }
    }
    // Ensure rule-based is always available as final fallback.
    if !candidates.contains(&SemanticBackendKind::RuleBased) {
        candidates.push(SemanticBackendKind::RuleBased);
    }

    // 3. Apply TemplateSignals corrections (these override profile preference).
    let forced = if signals.xetex_required {
        Some(SemanticBackendKind::XeLaTeXHook)
    } else if signals.luatex_preferred {
        Some(SemanticBackendKind::LuaTeXNode)
    } else {
        None
    };

    let ordered: Vec<SemanticBackendKind> = if let Some(f) = forced {
        // Force the signal-required backend first, but still try profile preference as 2nd.
        let mut ordered = vec![f];
        for bk in &candidates {
            if *bk != f {
                ordered.push(*bk);
            }
        }
        ordered
    } else {
        candidates
    };

    // 4. Pick the first available backend.
    for bk in &ordered {
        let available = match bk {
            SemanticBackendKind::RuleBased => true,
            SemanticBackendKind::XeLaTeXHook => availability.xelatex.available,
            SemanticBackendKind::LuaTeXNode => availability.lualatex.available,
            SemanticBackendKind::Auto => false,
        };
        if available {
            return AutoBackendSelection {
                kind: *bk,
                reason: build_backend_selection_reason(*bk, profile_id, signals, availability),
            };
        }
    }

    // 5. No backend available — use rule-based as absolute fallback.
    AutoBackendSelection {
        kind: SemanticBackendKind::RuleBased,
        reason: format!(
            "RuleBasedBackend: no runtime available for profile '{}'; xelatex: {}, lualatex: {}",
            profile_id,
            availability.xelatex.reason,
            availability.lualatex.reason
        ),
    }
}

fn build_backend_selection_reason(
    bk: SemanticBackendKind,
    profile_id: &str,
    signals: &TemplateSignals,
    availability: &RuntimeAvailabilitySnapshot,
) -> String {
    let signal_reason = signals.reason_summary();
    match bk {
        SemanticBackendKind::RuleBased => {
            format!(
                "RuleBasedBackend: selected for profile '{}'; {}; no runtime available (xelatex: {}, lualatex: {})",
                profile_id, signal_reason, availability.xelatex.reason, availability.lualatex.reason
            )
        }
        SemanticBackendKind::XeLaTeXHook => {
            format!(
                "XeLaTeXHookBackend: profile '{}'; {}; {}",
                profile_id, signal_reason, availability.xelatex.reason
            )
        }
        SemanticBackendKind::LuaTeXNode => {
            format!(
                "LuaTeXNodeBackend: profile '{}'; {}; {}",
                profile_id, signal_reason, availability.lualatex.reason
            )
        }
        SemanticBackendKind::Auto => unreachable!(),
    }
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

#[allow(dead_code)]
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
    options: &mut CompileOptions,
    report: &mut CompileReport,
) -> Result<SemanticBackendArtifact, EngineError> {
    match options.semantic_backend {
        SemanticBackendKind::Auto => {
            let selection = select_auto_backend(vfs, report.journal_detection.as_ref());
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
    options: &mut CompileOptions,
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
    options: &mut CompileOptions,
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
    options: &mut CompileOptions,
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
const NODE_TREE_SIDECAR: &str = "__docx_node_tree.jsonl";

/// XeLaTeX semantic hook (v2 schema).
/// Emits JSONL with schema header, source location (path + line), and macro name.
const XELATEX_SEMANTIC_HOOK: &str = r#"
\newwrite\docxsemout
\immediate\openout\docxsemout=__docx_semantic_events.jsonl
\makeatletter

% v2 schema header
\begingroup
\catcode`\"=12
\immediate\write\docxsemout{{"schema":"semantic-event-v2","engine":"xelatex"}}
\endgroup

% ─── Label / reference hooks — \label and \ref are defined by the LaTeX kernel
%     before the preamble runs, so this is safe in the preamble.
\let\docxsemOldLabel\label
\def\label#1{%
  \immediate\write\docxsemout{{"type":"label","key":"\detokenize{#1}","span":null,"source":{"path":"\jobname.tex","line":\the\inputlineno},"macro":"label"}}%
  \docxsemOldLabel{#1}%
}%
\let\docxsemOldRef\ref
\def\ref#1{%
  \immediate\write\docxsemout{{"type":"reference","kind":"ref","key":"\detokenize{#1}","span":null,"source":{"path":"\jobname.tex","line":\the\inputlineno},"macro":"ref"}}%
  \docxsemOldRef{#1}%
}%
\ifcsname eqref\endcsname
  \let\docxsemOldEqref\eqref
  \def\eqref#1{%
    \immediate\write\docxsemout{{"type":"reference","kind":"eqref","key":"\detokenize{#1}","span":null,"source":{"path":"\jobname.tex","line":\the\inputlineno},"macro":"eqref"}}%
    \docxsemOldEqref{#1}%
  }%
\fi
\ifcsname autoref\endcsname
  \let\docxsemOldAutoref\autoref
  \def\autoref#1{%
    \immediate\write\docxsemout{{"type":"reference","kind":"autoref","key":"\detokenize{#1}","span":null,"source":{"path":"\jobname.tex","line":\the\inputlineno},"macro":"autoref"}}%
    \docxsemOldAutoref{#1}%
  }%
\fi

% ─── Citation hooks — \cite is a LaTeX kernel command defined before the preamble.
\let\docxsemOldCite\cite
\def\docxsemOldCite{%
  \@ifnextchar[{\docxsemCiteOpen}{\docxsemCitePlainNoOpt}%}
}
\def\docxsemCiteOpen[#1]{%
  \@ifnextchar[{\docxsemCiteStyleOpt{#1}}{\docxsemCitePageOpt{#1}{}}%
}
\def\docxsemCiteStyleOpt#1[#2]#3{%
  \immediate\write\docxsemout{{"type":"citation","keys":["\detokenize{#3}"],"style":"\detokenize{#1}","pages":"\detokenize{#2}","span":null,"source":{"path":"\jobname.tex","line":\the\inputlineno},"macro":"cite"}}%
  \docxsemOldCite[{#1}][{#2}]{#3}%
}
\def\docxsemCitePageOpt#1[#2]#3{%
  \immediate\write\docxsemout{{"type":"citation","keys":["\detokenize{#3}"],"style":null,"pages":"\detokenize{#2}","span":null,"source":{"path":"\jobname.tex","line":\the\inputlineno},"macro":"cite"}}%
  \docxsemOldCite[{#1}][{#2}]{#3}%
}
\def\docxsemCitePlainNoOpt#1{%
  \immediate\write\docxsemout{{"type":"citation","keys":["\detokenize{#1}"],"style":null,"pages":null,"span":null,"source":{"path":"\jobname.tex","line":\the\inputlineno},"macro":"cite"}}%
  \docxsemOldCite{#1}%
}

% ─── Figure / graphics hook — \includegraphics is defined in the LaTeX kernel graphics package.
\ifcsname includegraphics\endcsname
  \let\docxsemOldIncludeGraphics\includegraphics
  \def\docxsemOldIncludeGraphics{\@ifnextchar[{\docxsemImgOpt}{\docxsemImgPlain}}
  \def\docxsemImgOpt[#1]#2{%
    \immediate\write\docxsemout{{"type":"figure","path":"\detokenize{#2}","caption":null,"label":null,"width_expr":"\detokenize{#1}","span":null,"source":{"path":"\jobname.tex","line":\the\inputlineno},"macro":"includegraphics"}}%
    \docxsemOldIncludeGraphics[#1]{#2}%
  }%
  \def\docxsemImgPlain#1{%
    \immediate\write\docxsemout{{"type":"figure","path":"\detokenize{#1}","caption":null,"label":null,"width_expr":null,"span":null,"source":{"path":"\jobname.tex","line":\the\inputlineno},"macro":"includegraphics"}}%
    \docxsemOldIncludeGraphics{#1}%
  }%
\fi

% ─── Hook coverage notes ─────────────────────────────────────────────────────────
%     The following semantic events are intentionally omitted from this hook:
%     - headings (section/subsection/...): captured by latex-reader from source
%     - tabular/table environments: complex macro layers, captured by latex-reader
%     - equation/align environments: complex macro layers, captured by latex-reader
%     - caption: hyperref may redefine, captured by latex-reader from source
%
% ─── Cleanup ──────────────────────────────────────────────────────────────────
\AtEndDocument{\immediate\closeout\docxsemout}
\makeatother
"#;

const LUALATEX_SEMANTIC_HOOK: &str = r#"
\directlua{
docxsem_file = io.open("__docx_semantic_events.jsonl", "w")
docxsem_node_tree_file = io.open("__docx_node_tree.jsonl", "w")
local glyph_id = node.id("glyph")
local glue_id = node.id("glue")
local hlist_id = node.id("hlist")
local vlist_id = node.id("vlist")
local rule_id = node.id("rule")
local kern_id = node.id("kern")
local penalty_id = node.id("penalty")
local local_par_id = node.id("local_par")
local dir_id = node.id("dir")
local bs = string.char(92)
local lua_space = string.char(37) .. "s"

-- Current page info for position tracking
local current_page = 1
local current_vpos = 0

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

function docxsem_node_tree_write(raw)
  if docxsem_node_tree_file then
    docxsem_node_tree_file:write(raw, string.char(10))
  end
end

-- M4-2: Emit detailed node entries for each node type
-- This replaces the simple counting with per-node emission
function docxsem_emit_node(type_name, props)
  local parts = {'"type":"' .. type_name .. '"'}
  for k, v in pairs(props) do
    table.insert(parts, '"' .. k .. '":' .. tostring(v))
  end
  local json = '{' .. table.concat(parts, ',') .. '}'
  docxsem_node_tree_write(json)
end

-- Helper to get font info from a node
local function docxsem_get_font_info(n)
  if n.font then
    local font_info = font.getfont(n.font)
    if font_info then
      return {
        font_id = n.font,
        font_name = font_info.name or "unknown",
        font_size = font_info.size or 0
      }
    end
  end
  return {font_id = 0, font_name = "unknown", font_size = 0}
end

-- M4-2: Detailed node traversal with position tracking
function docxsem_node_tree(head)
  if type(head) ~= "userdata" then return end
  
  -- Reset counters
  local hlist_count = 0
  local vlist_count = 0
  local glyph_count = 0
  local glue_count = 0
  local rule_count = 0
  
  -- Track position state
  local cur_x = 0
  local cur_y = 0
  local hpos = 0
  local vpos = 0
  
  -- Recursive node traversal with position tracking
  local function traverse_node(n, depth, is_vlist)
    if not n or type(n) ~= "userdata" then return end
    
    local id = n.id
    local subtype = n.subtype or 0
    
    if id == hlist_id then
      hlist_count = hlist_count + 1
      -- Emit hlist node entry
      local props = {
        subtype = subtype,
        x = hpos,
        y = vpos,
        width = n.width or 0,
        height = n.height or 0,
        depth = n.depth or 0,
        head_id = hlist_count
      }
      docxsem_emit_node("hlist", props)
      -- Traverse into hlist content
      if n.list and type(n.list) == "userdata" then
        local saved_x = hpos
        local saved_y = vpos
        for child in node.traverse(n.list) do
          traverse_node(child, depth + 1, false)
        end
        hpos = saved_x
        vpos = saved_y
      end
    elseif id == vlist_id then
      vlist_count = vlist_count + 1
      -- Emit vlist node entry
      local props = {
        subtype = subtype,
        x = hpos,
        y = vpos,
        width = n.width or 0,
        height = n.height or 0,
        depth = n.depth or 0,
        head_id = vlist_count
      }
      docxsem_emit_node("vlist", props)
      -- Traverse into vlist content
      if n.list and type(n.list) == "userdata" then
        local saved_x = hpos
        local saved_y = vpos
        for child in node.traverse(n.list) do
          traverse_node(child, depth + 1, true)
        end
        hpos = saved_x
        vpos = saved_y
      end
    elseif id == glyph_id then
      glyph_count = glyph_count + 1
      -- Get character info
      local char_code = n.char or 0
      local ok, char_str = pcall(utf8.char, char_code)
      if not ok then char_str = string.format("U+%04X", char_code) end
      
      -- Get font info
      local font_info = docxsem_get_font_info(n)
      
      -- Emit glyph node entry
      local props = {
        subtype = subtype,
        x = hpos,
        y = vpos,
        char = char_code,
        char_str = docxsem_json(char_str),
        font_id = font_info.font_id,
        font_name = docxsem_json(font_info.font_name),
        width = n.width or 0,
        height = n.height or 0,
        depth = n.depth or 0
      }
      docxsem_emit_node("glyph", props)
      
      -- Advance horizontal position for non-vlist context
      if not is_vlist then
        hpos = hpos + (n.width or 0)
      end
    elseif id == glue_id then
      glue_count = glue_count + 1
      -- Get glue spec info
      local width = n.width or 0
      local stretch = n.stretch or 0
      local shrink = n.shrink or 0
      local stretch_order = n.stretch_order or 0
      local shrink_order = n.shrink_order or 0
      
      -- Emit glue node entry
      local props = {
        subtype = subtype,
        x = hpos,
        y = vpos,
        width = width,
        stretch = stretch,
        shrink = shrink,
        stretch_order = stretch_order,
        shrink_order = shrink_order
      }
      docxsem_emit_node("glue", props)
      
      -- Advance horizontal position for non-vlist context
      if not is_vlist then
        hpos = hpos + width
      end
    elseif id == rule_id then
      rule_count = rule_count + 1
      -- Emit rule node entry
      local props = {
        subtype = subtype,
        x = hpos,
        y = vpos,
        width = n.width or 0,
        height = n.height or 0,
        depth = n.depth or 0
      }
      docxsem_emit_node("rule", props)
      
      -- Advance position
      if not is_vlist then
        hpos = hpos + (n.width or 0)
      end
    elseif id == kern_id then
      -- Kern nodes affect spacing
      local kern_width = n.kern or 0
      local props = {
        subtype = subtype,
        x = hpos,
        y = vpos,
        kern = kern_width
      }
      docxsem_emit_node("kern", props)
      if not is_vlist then
        hpos = hpos + kern_width
      end
    elseif id == penalty_id then
      -- Penalty nodes (line breaks etc)
      local penalty_value = n.penalty or 0
      local props = {
        subtype = subtype,
        x = hpos,
        y = vpos,
        penalty = penalty_value
      }
      docxsem_emit_node("penalty", props)
    elseif id == local_par_id then
      -- Local par node (paragraph start)
      local props = {
        subtype = subtype,
        x = hpos,
        y = vpos
      }
      docxsem_emit_node("local_par", props)
    elseif id == dir_id then
      -- Direction node
      local dir_str = n.dir or ""
      local props = {
        subtype = subtype,
        x = hpos,
        y = vpos,
        dir = docxsem_json(dir_str)
      }
      docxsem_emit_node("dir", props)
    end
  end
  
  -- Traverse all nodes in the head
  for n in node.traverse(head) do
    traverse_node(n, 0, false)
  end
  
  -- Emit summary entry with totals
  local summary = '{"type":"node_tree","hlist":' .. hlist_count 
    .. ',"vlist":' .. vlist_count 
    .. ',"glyph":' .. glyph_count 
    .. ',"glue":' .. glue_count 
    .. ',"rule":' .. rule_count 
    .. ',"page":' .. current_page .. '}'
  docxsem_node_tree_write(summary)
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
  docxsem_node_tree(head)
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
\AtEndDocument{\directlua{if docxsem_file then docxsem_file:close() end}\directlua{if docxsem_node_tree_file then docxsem_node_tree_file:close() end}}
\makeatother
"#;

// M4-2: Detailed node entries emitted by the LuaLaTeX hook.
// Each node type has its own JSON structure with type-specific properties.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeEntry {
    /// Glyph node (character)
    Glyph {
        subtype: u32,
        x: i64,
        y: i64,
        char: u32,
        #[serde(default)]
        char_str: Option<String>,
        font_id: u32,
        #[serde(default)]
        font_name: Option<String>,
        width: i64,
        height: i64,
        depth: i64,
    },
    /// Horizontal list node (line of text)
    Hlist {
        subtype: u32,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        depth: i64,
        #[serde(default)]
        head_id: Option<u32>,
    },
    /// Vertical list node (paragraph/box)
    Vlist {
        subtype: u32,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        depth: i64,
        #[serde(default)]
        head_id: Option<u32>,
    },
    /// Glue node (spacing)
    Glue {
        subtype: u32,
        x: i64,
        y: i64,
        width: i64,
        #[serde(default)]
        stretch: Option<i64>,
        #[serde(default)]
        shrink: Option<i64>,
        #[serde(default)]
        stretch_order: Option<u32>,
        #[serde(default)]
        shrink_order: Option<u32>,
    },
    /// Rule node (rectangular box)
    Rule {
        subtype: u32,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        depth: i64,
    },
    /// Kern node (explicit spacing)
    Kern {
        subtype: u32,
        x: i64,
        y: i64,
        kern: i64,
    },
    /// Penalty node (line break penalties)
    Penalty {
        subtype: u32,
        x: i64,
        y: i64,
        penalty: i64,
    },
    /// Local par node (paragraph start)
    LocalPar {
        subtype: u32,
        x: i64,
        y: i64,
    },
    /// Direction node
    Dir {
        subtype: u32,
        x: i64,
        y: i64,
        #[serde(default)]
        dir: Option<String>,
    },
    /// Summary entry with page-level counts (legacy format for compatibility)
    #[serde(rename = "node_tree")]
    NodeTree {
        hlist: u32,
        vlist: u32,
        glyph: u32,
        glue: u32,
        rule: u32,
        #[serde(default)]
        page: Option<u32>,
    },
}

impl NodeEntry {
    /// Returns true if this is a summary entry
    pub fn is_summary(&self) -> bool {
        matches!(self, NodeEntry::NodeTree { .. })
    }

    /// Returns the position if available
    pub fn position(&self) -> Option<(i64, i64)> {
        match self {
            NodeEntry::Glyph { x, y, .. } => Some((*x, *y)),
            NodeEntry::Hlist { x, y, .. } => Some((*x, *y)),
            NodeEntry::Vlist { x, y, .. } => Some((*x, *y)),
            NodeEntry::Glue { x, y, .. } => Some((*x, *y)),
            NodeEntry::Rule { x, y, .. } => Some((*x, *y)),
            NodeEntry::Kern { x, y, .. } => Some((*x, *y)),
            NodeEntry::Penalty { x, y, .. } => Some((*x, *y)),
            NodeEntry::LocalPar { x, y, .. } => Some((*x, *y)),
            NodeEntry::Dir { x, y, .. } => Some((*x, *y)),
            NodeEntry::NodeTree { .. } => None,
        }
    }
}

/// An entry from the node tree JSONL sidecar emitted by the LuaLaTeX hook.
/// M4-2: Now uses the detailed `NodeEntry` enum that handles all node types.
pub type NodeTreeEntry = NodeEntry;

struct RuntimeCollectResult {
    events: Vec<SemanticEvent>,
    xdv_path: Option<PathBuf>,
    node_tree: Vec<NodeTreeEntry>,
}

fn collect_runtime_events(
    engine: RuntimeEngine,
    main_tex: &str,
    vfs: &VirtualFs,
) -> Result<RuntimeCollectResult, EngineError> {
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
    let events = parse_semantic_events_jsonl(&sidecar)?;

    // P2-2: Capture XDV path for LayoutGraph conversion.
    // XeLaTeX and LuaLaTeX both produce XDV output.
    let xdv_path = match engine {
        RuntimeEngine::LuaLaTeX | RuntimeEngine::XeLaTeX => {
            let xdv = PathBuf::from(main_tex).with_extension("xdv");
            let candidate = workdir.path().join(&xdv);
            if candidate.exists() {
                Some(candidate)
            } else {
                None
            }
        }
    };

    // M4-2: Read node tree JSONL sidecar emitted by the LuaLaTeX hook.
    let node_tree_path = workdir.path().join(NODE_TREE_SIDECAR);
    let node_tree: Vec<NodeTreeEntry> = if node_tree_path.exists() {
        let content = match fs::read_to_string(&node_tree_path) {
            Ok(c) => c,
            Err(e) => {
                return Err(EngineError::Io(format!(
                    "failed to read node tree sidecar {}: {}",
                    node_tree_path.display(),
                    e
                )));
            }
        };
        content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    } else {
        Vec::new()
    };

    Ok(RuntimeCollectResult { events, xdv_path, node_tree })
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
        .arg("-file-line-error");
    // M2-1: XeLaTeX requires explicit -output-format=xdv to produce .xdv output
    if matches!(engine, RuntimeEngine::XeLaTeX) {
        command.arg("-output-format=xdv");
    }
    command.arg(main_tex)
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

/// Parse an XDV file and convert it to a `LayoutGraph`.
///
/// Returns `None` if parsing fails (e.g., file not found, corrupt XDV, etc.).
/// This is intentionally lenient: a failed layout parse does NOT fail the compile.
fn parse_layout_from_xdv(xdv_path: &Path) -> Option<LayoutGraph> {
    let bytes = fs::read(xdv_path).ok()?;
    let mut parser = doc_xdv_parser::XdvParser::default();
    let xdv = parser.parse_bytes(&bytes).ok()?;
    let nodes = doc_xdv_parser::xdv_to_layout_nodes(&xdv);
    Some(to_collector_layout_graph(nodes))
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
    enable_ref_fields: bool,
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

    let (linked_xml, bookmarks, hyperlinks) = link_document_xml(&document_xml, reference_graph, enable_ref_fields);

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
    enable_ref_fields: bool,
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
                hyperlink_paragraph(&paragraph_xml, &plan.name, &reference_needles(reference), enable_ref_fields)
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

fn hyperlink_paragraph(paragraph: &str, anchor: &str, needles: &[String], enable_ref_fields: bool) -> Option<String> {
    if paragraph.contains("<w:hyperlink") || paragraph.contains("<w:bookmarkStart") {
        return None;
    }
    for needle in needles {
        if needle.is_empty() {
            continue;
        }
        if let Some(updated) = hyperlink_first_text_run(paragraph, anchor, needle, enable_ref_fields) {
            return Some(updated);
        }
    }
    None
}

fn hyperlink_first_text_run(paragraph: &str, anchor: &str, needle: &str, enable_ref_fields: bool) -> Option<String> {
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
            if enable_ref_fields {
                replacement.push_str(&format!(
                    "<w:r><w:fldChar w:fldCharType=\"begin\"/></w:r>\
                     <w:r><w:instrText xml:space=\"preserve\"> REF {} \\h </w:instrText></w:r>\
                     <w:r><w:fldChar w:fldCharType=\"separate\"/></w:r>\
                     <w:r><w:rPr><w:rStyle w:val=\"Hyperlink\"/></w:rPr>{}</w:r>\
                     <w:r><w:fldChar w:fldCharType=\"end\"/></w:r>",
                    escape_xml_attr(anchor),
                    text_xml(&escaped_needle)
                ));
            } else {
                replacement.push_str(&format!(
                    "<w:hyperlink w:anchor=\"{}\" w:history=\"1\"><w:r><w:rPr><w:rStyle w:val=\"Hyperlink\"/></w:rPr>{}</w:r></w:hyperlink>",
                    escape_xml_attr(anchor),
                    text_xml(&escaped_needle)
                ));
            }
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

/// P2: Build a RuleEngine populated with builtin rules, journal-specific rules,
/// and profile-specific rules from the TOML spec.
fn build_rule_engine(active_profile: &ActiveProfile) -> RuleEngine {
    let mut engine = RuleEngine::new();

    // P2.1: Load journal-specific rules
    let journal_rules: Vec<MacroRule> =
        doc_rule_engine::journal_rules(&active_profile.id);
    for rule in journal_rules {
        engine.registry_mut().register(rule);
    }

    // P2.1: Load profile TOML macro_rules
    for rule_toml in &active_profile.spec.macro_rules {
        let rule = macro_rule_toml_to_macro_rule(rule_toml);
        engine.registry_mut().register(rule);
    }

    engine
}

/// P2: Convert a `MacroRuleToml` (profile spec) into a `MacroRule` (rule engine).
fn macro_rule_toml_to_macro_rule(toml: &MacroRuleToml) -> MacroRule {
    let output = match toml.semantic.as_str() {
        "citation" => RuleOutput::Citation {
            keys_arg: toml.args.saturating_sub(1),
            style: toml.style.clone(),
        },
        "metadata" => RuleOutput::MetadataField {
            key: toml.name.clone(),
            content_arg: 0,
        },
        "author" => RuleOutput::AuthorList {
            content_arg: 0,
        },
        "affiliation" => RuleOutput::Affiliation {
            content_arg: 0,
        },
        "keyword" => RuleOutput::KeywordList {
            content_arg: 0,
            separator: "; ".to_string(),
        },
        "heading" => RuleOutput::Heading {
            level: 1,
            text_arg: 0,
        },
        "paragraph" => RuleOutput::Paragraph {
            body_arg: 0,
        },
        "inline-text" => RuleOutput::InlineText {
            content_arg: 0,
        },
        "ignore" => RuleOutput::Ignore,
        "verbatim" => RuleOutput::Verbatim,
        _ => RuleOutput::InlineText {
            content_arg: 0,
        },
    };
    MacroRule {
        id: format!("profile:{}/{}", toml.name, toml.semantic),
        name: toml.name.clone(),
        arity: toml.args,
        output,
        description: Some(format!("profile macro rule: {}", toml.semantic)),
    }
}

/// P2: Build a `RuleEngineReport` from the engine's audit cache.
fn build_rule_engine_report(engine: &RuleEngine) -> RuleEngineReport {
    // RuleEngine::new() loads builtin rules; journal rules are added by profile.
    let builtin_count = doc_rule_engine::builtin_rules().len();

    let journal_names: std::collections::HashSet<&str> = [
        "citet", "citep", "citealp", "IEEEkeywords", "IEEEauthorblockN", "IEEEauthorblockA",
        "shorttitle", "name", "address", "cvprfinalcopy", "confName", "confYear", "author",
        "affiliation", "corres", "equalcont", "affil", "maketitle", "institute",
        "titlerunning", "authorrunning", "email", "orcidID", "zihao", "songti", "heiti",
        "ctexset", "zhabstract", "enabstract",
    ]
    .into_iter()
    .collect();

    let mut journal_count = 0usize;
    let mut profile_count = 0usize;
    for rule in engine.registry().iter() {
        if rule.name.starts_with("profile:") {
            profile_count += 1;
        } else if journal_names.contains(rule.name.as_str()) {
            journal_count += 1;
        }
    }

    let unknown_names: Vec<String> = engine
        .audit_cache()
        .records()
        .iter()
        .filter(|r| r.source == DecisionSource::Fallback)
        .map(|r| r.macro_name.clone())
        .collect();
    let unknown_set: std::collections::HashSet<_> = unknown_names.iter().cloned().collect();
    RuleEngineReport {
        builtin_rules: builtin_count,
        journal_rules: journal_count,
        profile_rules: profile_count,
        unknown_macro_count: unknown_set.len(),
        unknown_macros: unknown_names,
    }
}

/// P2: Apply RuleEngine to transform `RawFallback` blocks in the document.
///
/// P2: Loads profile-specific and journal-specific rules from `ActiveProfile`.
fn apply_rule_engine_to_document(
    mut document: Document,
    active_profile: &ActiveProfile,
) -> (Document, RuleEngine) {
    let mut engine = build_rule_engine(active_profile);
    let blocks = std::mem::take(&mut document.blocks);
    for block in blocks {
        match block {
            Block::RawFallback { text, span } => {
                let resolved = resolve_raw_fallback(&text, &span, &mut engine);
                document.blocks.push(resolved);
            }
            other => document.blocks.push(other),
        }
    }
    (document, engine)
}

/// Attempt to convert a `RawFallback` text to a structured `Block` using the RuleEngine.
///
/// The RuleEngine processes inline macro patterns (e.g., `\unknownmacro{arg}`) found in
/// the fallback text. Environment-level fallbacks (e.g., `\begin{unknown}{...}`) are
/// preserved as-is for later inspection.
fn resolve_raw_fallback(
    text: &str,
    span: &Span,
    engine: &mut RuleEngine,
) -> Block {
    // Try to detect an environment-level pattern \begin{name}...\end{name}
    if let Some(name) = detect_environment_name(text) {
        // Record in audit trail but preserve as RawFallback for now
        let _ = engine.process_unknown(&format!("begin{{{name}}}"), 0);
        return Block::RawFallback {
            text: text.to_string(),
            span: *span,
        };
    }

    // Try to detect an inline macro pattern (e.g., \unknownmacro{...})
    if let Some((macro_name, arity)) = detect_inline_macro(text) {
        // M2-3: Route RuleOutput → Block conversion
        let args = extract_macro_args(text, arity);
        let config = RoutingConfig::default();
        if let Some(output) = engine.process_unknown(&macro_name, arity) {
            if let Some(block) = route_rule_output(&output, &args, &config) {
                return block;
            }
        }
    }

    Block::RawFallback {
        text: text.to_string(),
        span: *span,
    }
}

/// Detect `\begin{name}` pattern in raw fallback text and return the environment name.
fn detect_environment_name(text: &str) -> Option<String> {
    let text = text.trim();
    let rest = text.strip_prefix("\\begin{")?;
    let end = rest.find('}')?;
    Some(rest[..end].to_string())
}

/// Detect an inline macro pattern `\name{arg}` and return (macro_name, arity).
fn detect_inline_macro(text: &str) -> Option<(String, usize)> {
    let text = text.trim();
    // Match \name followed by optional [...] then required {...}
    let rest = text.strip_prefix('\\')?;
    let name_end = rest
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .count();
    if name_end == 0 {
        return None;
    }
    let name = rest[..name_end].to_string();
    // Count mandatory braces after the name
    let after_name = &rest[name_end..];
    let arity = after_name.chars().filter(|&c| c == '{').count();
    if arity == 0 {
        return None;
    }
    Some((name, arity))
}

/// Extract macro arguments from inline fallback text like `\macro{arg1}{arg2}`.
///
/// Returns a Vec of argument strings with outer braces stripped.
/// Returns empty Vec if the text doesn't start with a known macro pattern.
fn extract_macro_args(text: &str, arity: usize) -> Vec<String> {
    let text = text.trim();
    let mut args = Vec::with_capacity(arity);
    let rest = match text.strip_prefix('\\') {
        Some(r) => r,
        None => return args,
    };
    // Skip macro name
    let name_end = rest
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .count();
    let after_name = &rest[name_end..];
    let mut chars = after_name.chars().peekable();
    for _ in 0..arity {
        // Skip optional whitespace
        while chars.peek() == Some(&' ') {
            chars.next();
        }
        if chars.peek() == Some(&'{') {
            chars.next(); // skip opening brace
            let mut depth = 1;
            let mut arg_chars = Vec::new();
            while let Some(c) = chars.next() {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                arg_chars.push(c);
            }
            let arg_text: String = arg_chars.into_iter().collect::<String>().trim().to_string();
            args.push(arg_text);
        } else {
            break;
        }
    }
    args
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
        // Capture all needed fields before consuming spec via default_page_setup()
        let spec_id = spec.id;
        let spec_page_setup = spec.page_setup;
        let spec_doc_classes = spec.document_classes;
        let spec_caption_fig_prefix = spec.caption_policy.figure_prefix;
        let spec_cite_bib_style = spec.citation_policy.bibliography_style;

        let page_setup = spec
            .default_page_setup()
            .expect("JOS profile should provide page setup");

        assert_eq!(spec_id, "jos-paper");
        assert_eq!(spec_page_setup, PageSetupProfile::JosPaper3);
        assert!(spec_doc_classes.contains(&"rjthesis"));
        assert_eq!(spec_caption_fig_prefix, "图");
        assert_eq!(spec_cite_bib_style, "unsrt");
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
        let mut options = CompileOptions {
            semantic_backend: SemanticBackendKind::XeLaTeXHook,
            ..CompileOptions::default()
        };
        let mut report = CompileReport::new(options.profile.clone());
        let artifact = collect_runtime_or_fallback(
            MissingRuntimeBackend,
            "missing runtime backend for deterministic test",
            "main.tex",
            &mut vfs,
            &mut options,
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
        let mut options = CompileOptions {
            semantic_backend: SemanticBackendKind::XeLaTeXHook,
            allow_backend_fallback: false,
            ..CompileOptions::default()
        };
        let mut report = CompileReport::new(options.profile.clone());
        let err = collect_runtime_or_fallback(
            MissingRuntimeBackend,
            "missing runtime backend for deterministic test",
            "main.tex",
            &mut vfs,
            &mut options,
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

        let report = CompatibilityAnalyzer::new().analyze(&vfs, ProfileKind::GenericArticle);

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
        let mut options = CompileOptions {
            semantic_backend: SemanticBackendKind::RuleBased,
            ..CompileOptions::default()
        };
        let profile = options.profile.clone();
        let mut vfs = VirtualFs::new();
        vfs.insert("main.tex", SAMPLE.as_bytes().to_vec());
        let mut input = SemanticCollectorInput {
            main_tex: "main.tex",
            vfs: &mut vfs,
            options: &mut options,
        };
        let mut report = CompileReport::new(profile);

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
        let updated = hyperlink_paragraph(paragraph, "ref_fig_a", &[String::from("Figure 1")], false)
            .expect("hyperlink text run");

        assert!(updated.contains(r#"<w:hyperlink w:anchor="ref_fig_a""#));
        assert!(updated.contains(r#"<w:t xml:space="preserve">See </w:t>"#));
        assert!(updated.contains("<w:t>Figure 1</w:t>"));
        assert!(updated.contains("<w:t>.</w:t>"));
    }

    #[test]
    fn docx_reference_links_ref_field_mode() {
        let paragraph = r#"<w:p><w:pPr><w:pStyle w:val="JOSBody"/></w:pPr><w:r><w:t>See Figure 1.</w:t></w:r></w:p>"#;
        let updated = hyperlink_paragraph(paragraph, "ref_fig_a", &[String::from("Figure 1")], true)
            .expect("ref field text run");

        assert!(updated.contains("<w:fldChar"));
        assert!(updated.contains("REF"));
        assert!(updated.contains("ref_fig_a"));
        assert!(updated.contains(r#"w:fldCharType="begin""#));
        assert!(updated.contains(r#"w:fldCharType="end""#));
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

    // ── P6: Runtime Hook Profile Extension ──────────────────────────────────

    #[test]
    fn journal_detection_injected_into_luatex_sidecar_description() {
        // Verify that LuaTeXNodeBackend sidecar description includes profile_id.
        // This test uses TemplateSignals and a mock runtime to verify the code path exists.
        let mut signals = TemplateSignals::default();
        signals.observe("\\documentclass{article}");
        let selection = select_auto_backend_with_availability(&signals, availability(true, true));
        // With both available, LuaTeX is chosen.
        assert_eq!(selection.kind, SemanticBackendKind::LuaTeXNode);
        assert!(selection.reason.contains("LuaTeXNodeBackend"));
    }

    #[test]
    fn journal_detection_injected_into_xelatex_sidecar_description() {
        let mut signals = TemplateSignals::default();
        signals.observe("\\usepackage{xeCJK}\\setCJKmainfont{Foo}");
        let selection = select_auto_backend_with_availability(&signals, availability(true, true));
        // xeCJK forces XeLaTeX.
        assert_eq!(selection.kind, SemanticBackendKind::XeLaTeXHook);
        assert!(selection.reason.contains("XeLaTeXHookBackend"));
    }

    #[test]
    fn profile_aware_backend_selects_luatex_for_tacl() {
        let registry = ProfileRegistry::load_default().unwrap();
        let spec = registry.get("tacl").unwrap();
        assert_eq!(spec.backend.preferred, "luatex-node");
    }

    #[test]
    fn profile_aware_backend_selects_xelatex_for_chinese_academic() {
        let registry = ProfileRegistry::load_default().unwrap();
        let spec = registry.get("chinese-academic").unwrap();
        assert!(spec.backend.requires_xetex);
        assert_eq!(spec.backend.preferred, "xelatex-hook");
    }

    #[test]
    fn profile_aware_backend_selects_luatex_for_nature() {
        let registry = ProfileRegistry::load_default().unwrap();
        let spec = registry.get("nature").unwrap();
        assert_eq!(spec.backend.preferred, "luatex-node");
    }

    #[test]
    fn profile_aware_backend_preserves_fallback_chain() {
        let registry = ProfileRegistry::load_default().unwrap();
        let spec = registry.get("tacl").unwrap();
        assert!(!spec.backend.fallback.is_empty());
        // Fallback should include xelatex-hook and rule-based.
        assert!(spec.backend.fallback.contains(&"xelatex-hook".to_string()));
        assert!(spec.backend.fallback.contains(&"rule-based".to_string()));
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
            _options: &mut CompileOptions,
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

    #[test]
    fn parse_semantic_events_v1_legacy() {
        let jsonl = r#"{"type":"heading","level":1,"text":"Intro","label":null,"span":null}
{"type":"figure","path":"fig.png","caption":null,"label":"fig:1","width_expr":null,"span":null}
{"type":"label","key":"sec:intro","span":null}
"#;
        let events = parse_semantic_events_jsonl(jsonl).unwrap();
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], SemanticEvent::Heading { .. }));
        assert!(matches!(events[1], SemanticEvent::Figure { .. }));
        assert!(matches!(events[2], SemanticEvent::Label { .. }));
    }

    #[test]
    fn parse_semantic_events_v2_with_schema_header() {
        let jsonl = r#"{"schema":"semantic-event-v2","engine":"xelatex"}
{"type":"heading","level":2,"text":"Background","label":"sec:bg","span":null,"source":{"path":"main.tex","line":10},"macro":"section"}
{"type":"caption","text":"Overview of the system","span":null,"source":{"path":"main.tex","line":25},"macro":"caption"}
"#;
        let events = parse_semantic_events_jsonl(jsonl).unwrap();
        assert_eq!(events.len(), 2);
        match &events[0] {
            SemanticEvent::Heading { level, text, label, .. } => {
                assert_eq!(*level, 2);
                assert_eq!(text, "Background");
                assert_eq!(label.as_deref(), Some("sec:bg"));
            }
            _ => panic!("expected Heading event"),
        }
    }

    #[test]
    fn parse_semantic_events_skips_empty_and_comment_lines() {
        let jsonl = r#"

{"type":"heading","level":1,"text":"Title","label":null,"span":null}
# This is a comment line
{"type":"label","key":"x","span":null}

"#;
        let events = parse_semantic_events_jsonl(jsonl).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn parse_layout_from_xdv_returns_none_for_missing_file() {
        let result = parse_layout_from_xdv(std::path::Path::new("/nonexistent/file.xdv"));
        assert!(result.is_none());
    }

    #[test]
    fn lua_hook_contains_docxsem_node_tree() {
        assert!(
            LUALATEX_SEMANTIC_HOOK.contains("docxsem_node_tree"),
            "Lua hook should define docxsem_node_tree function"
        );
        assert!(
            LUALATEX_SEMANTIC_HOOK.contains("hlist_count"),
            "Lua hook should count hlist nodes"
        );
        assert!(
            LUALATEX_SEMANTIC_HOOK.contains("glyph_count"),
            "Lua hook should count glyph nodes"
        );
        assert!(
            LUALATEX_SEMANTIC_HOOK.contains("__docx_node_tree.jsonl"),
            "Lua hook should open node tree sidecar file"
        );
        assert!(
            LUALATEX_SEMANTIC_HOOK.contains("docxsem_node_tree_write"),
            "Lua hook should have docxsem_node_tree_write function"
        );
    }

    #[test]
    fn node_tree_entry_deserializes_summary() {
        // Test the summary entry format
        let json = r#"{"type":"node_tree","hlist":2,"vlist":1,"glyph":42,"glue":15,"rule":3,"page":1}"#;
        let entry: NodeTreeEntry = serde_json::from_str(json).unwrap();
        match entry {
            NodeEntry::NodeTree {
                hlist,
                vlist,
                glyph,
                glue,
                rule,
                page,
            } => {
                assert_eq!(hlist, 2);
                assert_eq!(vlist, 1);
                assert_eq!(glyph, 42);
                assert_eq!(glue, 15);
                assert_eq!(rule, 3);
                assert_eq!(page, Some(1));
            }
            _ => panic!("expected NodeTree variant"),
        }
    }

    #[test]
    fn node_tree_entry_deserializes_glyph() {
        let json = r#"{"type":"glyph","subtype":0,"x":100,"y":200,"char":65,"font_id":1,"width":10,"height":8,"depth":2}"#;
        let entry: NodeTreeEntry = serde_json::from_str(json).unwrap();
        match entry {
            NodeEntry::Glyph {
                x,
                y,
                char,
                font_id,
                width,
                height,
                depth,
                ..
            } => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
                assert_eq!(char, 65);
                assert_eq!(font_id, 1);
                assert_eq!(width, 10);
                assert_eq!(height, 8);
                assert_eq!(depth, 2);
            }
            _ => panic!("expected Glyph variant"),
        }
    }

    #[test]
    fn node_tree_entry_deserializes_hlist() {
        let json = r#"{"type":"hlist","subtype":0,"x":0,"y":0,"width":500,"height":12,"depth":3}"#;
        let entry: NodeTreeEntry = serde_json::from_str(json).unwrap();
        match entry {
            NodeEntry::Hlist {
                x,
                y,
                width,
                height,
                depth,
                ..
            } => {
                assert_eq!(x, 0);
                assert_eq!(y, 0);
                assert_eq!(width, 500);
                assert_eq!(height, 12);
                assert_eq!(depth, 3);
            }
            _ => panic!("expected Hlist variant"),
        }
    }

    #[test]
    fn node_tree_entry_deserializes_glue() {
        let json = r#"{"type":"glue","subtype":0,"x":100,"y":0,"width":200,"stretch":100,"shrink":50}"#;
        let entry: NodeTreeEntry = serde_json::from_str(json).unwrap();
        match entry {
            NodeEntry::Glue {
                x,
                y,
                width,
                stretch,
                shrink,
                ..
            } => {
                assert_eq!(x, 100);
                assert_eq!(y, 0);
                assert_eq!(width, 200);
                assert_eq!(stretch, Some(100));
                assert_eq!(shrink, Some(50));
            }
            _ => panic!("expected Glue variant"),
        }
    }

    #[test]
    fn node_tree_entry_deserializes_rule() {
        let json = r#"{"type":"rule","subtype":0,"x":0,"y":0,"width":500,"height":1,"depth":0}"#;
        let entry: NodeTreeEntry = serde_json::from_str(json).unwrap();
        match entry {
            NodeEntry::Rule {
                x,
                y,
                width,
                height,
                depth,
                ..
            } => {
                assert_eq!(x, 0);
                assert_eq!(y, 0);
                assert_eq!(width, 500);
                assert_eq!(height, 1);
                assert_eq!(depth, 0);
            }
            _ => panic!("expected Rule variant"),
        }
    }

    #[test]
    fn node_tree_entry_is_summary() {
        let json = r#"{"type":"node_tree","hlist":1,"vlist":0,"glyph":5,"glue":2,"rule":0}"#;
        let entry: NodeTreeEntry = serde_json::from_str(json).unwrap();
        assert!(entry.is_summary());
    }

    #[test]
    fn node_tree_entry_position() {
        let glyph_json = r#"{"type":"glyph","subtype":0,"x":100,"y":200,"char":65,"font_id":1,"width":10,"height":8,"depth":2}"#;
        let entry: NodeTreeEntry = serde_json::from_str(glyph_json).unwrap();
        assert_eq!(entry.position(), Some((100, 200)));

        let summary_json = r#"{"type":"node_tree","hlist":1,"vlist":0,"glyph":5,"glue":2,"rule":0}"#;
        let summary: NodeTreeEntry = serde_json::from_str(summary_json).unwrap();
        assert_eq!(summary.position(), None);
    }
}
