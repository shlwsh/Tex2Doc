//! Profile loading from JSON/TOML configuration files and registry management.
//!
//! Tex2Doc conversion profiles can be specified as JSON or TOML files, by built-in
//! profile ID, or auto-detected via `JournalDetector`. The loader falls back to
//! built-in profiles when the file is absent or invalid.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Extended schema types
// ---------------------------------------------------------------------------

/// Detection signal specification from a profile file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionSpec {
    /// Minimum confidence score (0.0–1.0) for auto-selection.
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f32,
    /// Profile to fall back to when detection confidence is too low.
    #[serde(default = "default_fallback_profile")]
    pub fallback_profile: String,
    /// Ordered list of signals used to detect this profile.
    #[serde(default)]
    pub signals: Vec<DetectionSignal>,
}

impl Default for DetectionSpec {
    fn default() -> Self {
        Self {
            min_confidence: 0.75,
            fallback_profile: "generic".to_string(),
            signals: Vec::new(),
        }
    }
}

fn default_min_confidence() -> f32 {
    0.75
}

fn default_fallback_profile() -> String {
    "generic".to_string()
}

/// A single signal used for journal/profile detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionSignal {
    /// Kind of signal: "documentclass" | "documentclass_option" | "macro" |
    /// "package" | "bibliographystyle" | "engine_feature".
    pub kind: String,
    /// Expected value of the signal.
    pub value: String,
    /// Weight contributed to the confidence score when matched.
    #[serde(default = "default_signal_weight")]
    pub weight: f32,
}

fn default_signal_weight() -> f32 {
    0.05
}

/// Backend selection policy for a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendSpec {
    /// Preferred semantic collection backend.
    /// One of: "luatex-node" | "xelatex-hook" | "rule-based".
    #[serde(default)]
    pub preferred: String,
    /// Fallback backend chain, tried in order.
    #[serde(default)]
    pub fallback: Vec<String>,
    /// If true, requires XeTeX engine.
    #[serde(default)]
    pub requires_xetex: bool,
    /// If true, prefers LuaTeX engine.
    #[serde(default)]
    pub prefers_luatex: bool,
}

impl Default for BackendSpec {
    fn default() -> Self {
        Self {
            preferred: String::new(),
            fallback: Vec::new(),
            requires_xetex: false,
            prefers_luatex: false,
        }
    }
}

/// Semantic policy controlling unknown-macro handling and event collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticPolicySpec {
    /// How to handle unknown macros: "rule-engine" | "preserve-raw".
    #[serde(default = "default_unknown_macro")]
    pub unknown_macro: String,
    /// Whether to preserve unrecognized blocks as raw LaTeX.
    #[serde(default = "default_true")]
    pub preserve_raw_fallback: bool,
    /// Whether to collect runtime semantic events.
    #[serde(default = "default_true")]
    pub collect_runtime_events: bool,
    /// Whether to collect layout graph from XDV/node tree.
    #[serde(default = "default_true")]
    pub collect_layout_graph: bool,
}

impl Default for SemanticPolicySpec {
    fn default() -> Self {
        Self {
            unknown_macro: "rule-engine".to_string(),
            preserve_raw_fallback: true,
            collect_runtime_events: true,
            collect_layout_graph: true,
        }
    }
}

fn default_unknown_macro() -> String {
    "rule-engine".to_string()
}

fn default_true() -> bool {
    true
}

/// A macro rule defined in a profile TOML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroRuleToml {
    /// LaTeX macro name (without backslash).
    pub name: String,
    /// Semantic category: "citation" | "metadata" | "author" | "affiliation" |
    /// "keyword" | "inline-text" | "ignore".
    #[serde(default)]
    pub semantic: String,
    /// Number of mandatory arguments.
    #[serde(default)]
    pub args: usize,
    /// Optional style hint (e.g., "textual" for citations).
    #[serde(default)]
    pub style: String,
}

/// Mapping from semantic role to DOCX style name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleMapSpec {
    /// Semantic role, e.g. "heading.level1", "paragraph.body".
    pub semantic: String,
    /// Target DOCX style name.
    pub docx_style: String,
}

/// Quality and acceptance thresholds for a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySpec {
    /// Minimum acceptable compatibility score (0–100).
    #[serde(default = "default_min_score")]
    pub min_compatibility_score: u8,
    /// Maximum number of raw-fallback blocks tolerated.
    #[serde(default)]
    pub max_raw_fallback_blocks: usize,
    /// Whether a reference graph is required.
    #[serde(default)]
    pub require_reference_graph: bool,
}

impl Default for QualitySpec {
    fn default() -> Self {
        Self {
            min_compatibility_score: 75,
            max_raw_fallback_blocks: 0,
            require_reference_graph: false,
        }
    }
}

fn default_min_score() -> u8 {
    75
}

// ---------------------------------------------------------------------------
// Profile specification (canonical shape used by both JSON and TOML loaders)
// ---------------------------------------------------------------------------

/// A loaded profile specification from a JSON or TOML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSpecFile {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub document_classes: Vec<String>,
    /// Alternative IDs that resolve to this profile.
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub page_setup: PageSetupSpec,
    #[serde(default)]
    pub font_policy: FontPolicySpecFile,
    #[serde(default)]
    pub caption_policy: CaptionPolicySpecFile,
    #[serde(default)]
    pub citation_policy: CitationPolicySpecFile,
    /// Detection rules for journal auto-detection.
    #[serde(default)]
    pub detection: DetectionSpec,
    /// Backend selection policy.
    #[serde(default)]
    pub backend: BackendSpec,
    /// Semantic processing policy.
    #[serde(default)]
    pub semantic_policy: SemanticPolicySpec,
    /// Profile-specific macro rules.
    #[serde(default)]
    pub macro_rules: Vec<MacroRuleToml>,
    /// Semantic → DOCX style mappings.
    #[serde(default)]
    pub style_map: Vec<StyleMapSpec>,
    /// Quality thresholds.
    #[serde(default)]
    pub quality: QualitySpec,
}

impl Default for ProfileSpecFile {
    fn default() -> Self {
        Self {
            id: String::new(),
            display_name: String::new(),
            document_classes: Vec::new(),
            aliases: Vec::new(),
            page_setup: PageSetupSpec::default(),
            font_policy: FontPolicySpecFile::default(),
            caption_policy: CaptionPolicySpecFile::default(),
            citation_policy: CitationPolicySpecFile::default(),
            detection: DetectionSpec::default(),
            backend: BackendSpec::default(),
            semantic_policy: SemanticPolicySpec::default(),
            macro_rules: Vec::new(),
            style_map: Vec::new(),
            quality: QualitySpec::default(),
        }
    }
}

/// Page setup section in a profile file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageSetupSpec {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub width_mm: Option<f64>,
    #[serde(default)]
    pub height_mm: Option<f64>,
    #[serde(default)]
    pub margin_top_mm: Option<f64>,
    #[serde(default)]
    pub margin_bottom_mm: Option<f64>,
    #[serde(default)]
    pub margin_left_mm: Option<f64>,
    #[serde(default)]
    pub margin_right_mm: Option<f64>,
}

/// Font policy section in a profile file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FontPolicySpecFile {
    #[serde(default)]
    pub latin_main: Option<String>,
    #[serde(default)]
    pub cjk_main: Option<String>,
    #[serde(default)]
    pub math: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Caption policy section in a profile file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CaptionPolicySpecFile {
    #[serde(default)]
    pub figure_prefix: Option<String>,
    #[serde(default)]
    pub table_prefix: Option<String>,
    #[serde(default)]
    pub equation_prefix: Option<String>,
    #[serde(default)]
    pub numbering: Option<String>,
}

/// Citation policy section in a profile file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CitationPolicySpecFile {
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub bibliography_style: Option<String>,
    #[serde(default)]
    pub reference_section_title: Option<String>,
}

// ---------------------------------------------------------------------------
// TOML shape (mirrors ProfileSpecFile but serde RenameAll to TOML convention)
// ---------------------------------------------------------------------------

/// A profile loaded from a TOML file (same shape as ProfileSpecFile).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSpecToml {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub document_classes: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub page_setup: PageSetupSpec,
    #[serde(default)]
    pub font_policy: FontPolicySpecFile,
    #[serde(default)]
    pub caption_policy: CaptionPolicySpecFile,
    #[serde(default)]
    pub citation_policy: CitationPolicySpecFile,
    #[serde(default)]
    pub detection: DetectionSpec,
    #[serde(default)]
    pub backend: BackendSpec,
    #[serde(default)]
    pub semantic_policy: SemanticPolicySpec,
    #[serde(default)]
    pub macro_rules: Vec<MacroRuleToml>,
    #[serde(default)]
    pub style_map: Vec<StyleMapSpec>,
    #[serde(default)]
    pub quality: QualitySpec,
}

impl From<ProfileSpecToml> for ProfileSpecFile {
    fn from(t: ProfileSpecToml) -> Self {
        Self {
            id: t.id,
            display_name: t.display_name,
            document_classes: t.document_classes,
            aliases: t.aliases,
            page_setup: t.page_setup,
            font_policy: t.font_policy,
            caption_policy: t.caption_policy,
            citation_policy: t.citation_policy,
            detection: t.detection,
            backend: t.backend,
            semantic_policy: t.semantic_policy,
            macro_rules: t.macro_rules,
            style_map: t.style_map,
            quality: t.quality,
        }
    }
}

// ---------------------------------------------------------------------------
// Profile registry
// ---------------------------------------------------------------------------

/// Global registry of all available profiles.
///
/// Provides canonical lookup by ID (including alias resolution), file-based
/// loading from the `profiles/` directory, and built-in JSON fallback.
#[derive(Debug, Clone, Default)]
pub struct ProfileRegistry {
    /// Canonical ID → loaded spec.
    profiles: HashMap<String, ProfileSpecFile>,
    /// Alias → canonical ID.
    aliases: HashMap<String, String>,
}

impl ProfileRegistry {
    /// Load all built-in JSON profiles and scan the profiles directory for
    /// TOML files, registering every valid entry.
    pub fn load_default() -> Result<Self, ProfileLoadError> {
        let mut registry = Self::default();

        // 1. Register built-in JSON profiles.
        let builtin_ids = [
            ("generic-article", "generic-article"),
            ("chinese-academic", "chinese-academic"),
            ("jos-paper", "jos-paper"),
            ("medical-journal", "medical-journal"),
        ];
        for (id, _) in builtin_ids {
            if let Some(spec) = load_builtin(id) {
                registry.register_with_aliases(&spec);
            }
            // Also register the short "generic" alias.
            if id == "generic-article" {
                registry
                    .aliases
                    .insert("generic".to_string(), "generic-article".to_string());
            }
        }

        // 2. Scan profiles/ directory for TOML files.
        let profile_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles");
        if profile_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&profile_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                        match load_toml(&path) {
                            Ok(spec) => registry.register_with_aliases(&spec),
                            Err(e) => {
                                eprintln!("warning: failed to load profile {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(registry)
    }

    /// Register a profile and any aliases it declares.
    fn register_with_aliases(&mut self, spec: &ProfileSpecFile) {
        let canonical = spec.id.clone();
        self.profiles.insert(canonical.clone(), spec.clone());
        for alias in &spec.aliases {
            self.aliases.insert(alias.clone(), canonical.clone());
        }
    }

    /// Register a profile (overwrites any existing entry with the same id).
    pub fn register(&mut self, spec: ProfileSpecFile) {
        self.profiles.insert(spec.id.clone(), spec);
    }

    /// Get a profile by ID, resolving aliases.
    pub fn get(&self, id: &str) -> Option<&ProfileSpecFile> {
        // Direct lookup.
        if let Some(spec) = self.profiles.get(id) {
            return Some(spec);
        }
        // Alias lookup.
        if let Some(canonical) = self.aliases.get(id) {
            return self.profiles.get(canonical);
        }
        // Try prefix-free: treat "generic" as "generic-article".
        if id == "generic" {
            return self.profiles.get("generic-article");
        }
        None
    }

    /// Resolve an alias to its canonical ID.
    pub fn resolve_alias(&self, id: &str) -> Option<&str> {
        self.aliases.get(id).map(|s| s.as_str())
    }

    /// List all available canonical profile IDs.
    pub fn all_ids(&self) -> Vec<&str> {
        self.profiles.keys().map(|s| s.as_str()).collect()
    }

    /// Register an alias that maps a secondary ID to a canonical ID.
    pub fn register_alias(&mut self, alias: impl Into<String>, canonical: impl Into<String>) {
        self.aliases.insert(alias.into(), canonical.into());
    }
}

// ---------------------------------------------------------------------------
// Loading helpers
// ---------------------------------------------------------------------------

/// Source of a loaded profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ProfileLoadSource {
    JsonFile,
    TomlFile,
    Builtin,
}

/// Result of a profile load attempt.
#[derive(Debug)]
#[allow(dead_code)]
pub enum ProfileLoadResult {
    /// Profile was loaded from a JSON or TOML file.
    Loaded(ProfileSpecFile),
    /// Profile was resolved from a built-in ID.
    BuiltIn(&'static str),
    /// No profile was specified.
    None,
}

/// Loads a profile from a file path, built-in ID, or falls back to `None`.
#[allow(dead_code)]
pub fn load_profile(
    profile_path: Option<&Path>,
    profile_id: Option<&str>,
) -> (ProfileLoadResult, ProfileLoadSource) {
    if let Some(path) = profile_path {
        let path_str = path.to_string_lossy();
        let result = if path_str.ends_with(".toml") {
            load_toml(path).map(ProfileLoadResult::Loaded)
        } else {
            load_from_file(path).map(ProfileLoadResult::Loaded)
        };
        return match result {
            Ok(spec) => (
                spec,
                if path_str.ends_with(".toml") {
                    ProfileLoadSource::TomlFile
                } else {
                    ProfileLoadSource::JsonFile
                },
            ),
            Err(e) => {
                eprintln!("warning: failed to load profile from {:?}: {}", path, e);
                (ProfileLoadResult::None, ProfileLoadSource::Builtin)
            }
        };
    }

    if let Some(id) = profile_id {
        if let Some(builtin) = builtin_json(id) {
            return (
                ProfileLoadResult::BuiltIn(builtin),
                ProfileLoadSource::Builtin,
            );
        }
        if let Some((path, source)) = resolve_profile_path(id) {
            let result = match source {
                ProfileLoadSource::TomlFile => load_toml(&path).map(ProfileLoadResult::Loaded),
                ProfileLoadSource::JsonFile => load_from_file(&path).map(ProfileLoadResult::Loaded),
                ProfileLoadSource::Builtin => unreachable!(),
            };
            return match result {
                Ok(spec) => (spec, source),
                Err(e) => {
                    eprintln!("warning: failed to load profile '{}': {}", id, e);
                    (ProfileLoadResult::None, source)
                }
            };
        }
        eprintln!("warning: unknown profile ID '{}'; using default", id);
        return (
            ProfileLoadResult::BuiltIn(builtin_json("generic-article").unwrap()),
            ProfileLoadSource::Builtin,
        );
    }

    (ProfileLoadResult::None, ProfileLoadSource::Builtin)
}

/// Load a profile from a file (JSON or TOML, auto-detected by extension).
pub fn load_from_file(path: impl AsRef<Path>) -> Result<ProfileSpecFile, ProfileLoadError> {
    let path_ref = path.as_ref();
    let bytes = fs::read(path_ref).map_err(|e| ProfileLoadError::Io(e.to_string()))?;
    let path_str = path_ref.to_string_lossy();
    if path_str.ends_with(".toml") {
        load_toml_from_bytes(&bytes)
    } else {
        serde_json::from_slice(&bytes).map_err(|e| ProfileLoadError::Parse(e.to_string()))
    }
}

/// Load a profile from a TOML file.
#[allow(dead_code)]
pub fn load_toml(path: impl AsRef<Path>) -> Result<ProfileSpecFile, ProfileLoadError> {
    let bytes = fs::read(path.as_ref()).map_err(|e| ProfileLoadError::Io(e.to_string()))?;
    load_toml_from_bytes(&bytes)
}

/// Parse TOML bytes into ProfileSpecFile.
fn load_toml_from_bytes(bytes: &[u8]) -> Result<ProfileSpecFile, ProfileLoadError> {
    let toml_str = std::str::from_utf8(bytes)
        .map_err(|e| ProfileLoadError::Parse(format!("invalid UTF-8: {}", e)))?;
    let toml_spec: ProfileSpecToml =
        toml::from_str(toml_str).map_err(|e| ProfileLoadError::Parse(e.to_string()))?;
    Ok(ProfileSpecFile::from(toml_spec))
}

/// Resolve a profile ID to a file path in the profiles directory.
/// Prefers TOML over JSON if both exist.
#[allow(dead_code)]
pub fn resolve_profile_path(id: &str) -> Option<(PathBuf, ProfileLoadSource)> {
    let profile_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles");
    let toml_path = profile_dir.join(format!("{}.toml", id));
    if toml_path.is_file() {
        return Some((toml_path, ProfileLoadSource::TomlFile));
    }
    let json_path = profile_dir.join(format!("{}.json", id));
    if json_path.is_file() {
        return Some((json_path, ProfileLoadSource::JsonFile));
    }
    None
}

/// Built-in profile JSON strings keyed by profile ID.
fn builtin_json(id: &str) -> Option<&'static str> {
    match id {
        "generic-article" => Some(include_str!("../profiles/generic-article.json")),
        "chinese-academic" => Some(include_str!("../profiles/chinese-academic.json")),
        "jos-paper" => Some(include_str!("../profiles/jos-paper.json")),
        "medical-journal" => Some(include_str!("../profiles/medical-journal.json")),
        _ => None,
    }
}

/// Load a built-in profile by ID.
#[allow(dead_code)]
pub fn load_builtin(id: &str) -> Option<ProfileSpecFile> {
    builtin_json(id).and_then(|json| serde_json::from_str::<ProfileSpecFile>(json).ok())
}

/// List all available profile IDs (both file-based and built-in).
#[allow(dead_code)]
pub fn list_profile_ids() -> Vec<String> {
    vec![
        "generic-article".to_string(),
        "chinese-academic".to_string(),
        "jos-paper".to_string(),
        "medical-journal".to_string(),
        "generic".to_string(),
        "tacl".to_string(),
        "cvpr".to_string(),
        "nature".to_string(),
        "springer".to_string(),
        "generic-article-toml".to_string(),
        "chinese-academic-toml".to_string(),
        "jos-paper-toml".to_string(),
    ]
}

/// Profile loading errors.
#[allow(dead_code)]
#[derive(Debug)]
pub enum ProfileLoadError {
    Io(String),
    Parse(String),
}

impl std::fmt::Display for ProfileLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(s) => write!(f, "I/O error: {}", s),
            Self::Parse(s) => write!(f, "parse error (JSON/TOML): {}", s),
        }
    }
}

impl std::error::Error for ProfileLoadError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // ProfileSpecFile default and extension
    // -------------------------------------------------------------------------

    #[test]
    fn profile_spec_file_deserializes_minimal() {
        let json_str = r#"{"id": "test", "display_name": "Test"}"#;
        let spec: ProfileSpecFile = serde_json::from_str(json_str).unwrap();
        assert_eq!(spec.id, "test");
        assert!(spec.detection.signals.is_empty());
        assert_eq!(spec.backend.preferred, "");
        assert_eq!(spec.quality.min_compatibility_score, 75);
    }

    #[test]
    fn profile_spec_file_deserializes_full() {
        let json_str = r#"{
          "id": "test-profile",
          "display_name": "Test Profile",
          "document_classes": ["article", "report"],
          "aliases": ["tp", "tprofile"],
          "detection": {
            "min_confidence": 0.8,
            "fallback_profile": "generic",
            "signals": [
              {"kind": "documentclass", "value": "article", "weight": 0.7}
            ]
          },
          "backend": {
            "preferred": "luatex-node",
            "fallback": ["rule-based"],
            "requires_xetex": false,
            "prefers_luatex": true
          },
          "semantic_policy": {
            "unknown_macro": "preserve-raw",
            "preserve_raw_fallback": true,
            "collect_runtime_events": true,
            "collect_layout_graph": true
          },
          "macro_rules": [
            {"name": "citet", "semantic": "citation", "args": 1, "style": "textual"}
          ],
          "style_map": [
            {"semantic": "heading.level1", "docx_style": "Heading1"}
          ],
          "quality": {
            "min_compatibility_score": 80,
            "max_raw_fallback_blocks": 5,
            "require_reference_graph": true
          }
        }"#;
        let spec: ProfileSpecFile = serde_json::from_str(json_str).unwrap();
        assert_eq!(spec.id, "test-profile");
        assert_eq!(spec.detection.min_confidence, 0.8);
        assert_eq!(spec.backend.preferred, "luatex-node");
        assert_eq!(spec.backend.fallback, &["rule-based"]);
        assert!(spec.backend.prefers_luatex);
        assert_eq!(spec.semantic_policy.unknown_macro, "preserve-raw");
        assert_eq!(spec.macro_rules.len(), 1);
        assert_eq!(spec.macro_rules[0].name, "citet");
        assert_eq!(spec.style_map[0].semantic, "heading.level1");
        assert_eq!(spec.quality.min_compatibility_score, 80);
        assert_eq!(spec.quality.max_raw_fallback_blocks, 5);
        assert!(spec.quality.require_reference_graph);
    }

    // -------------------------------------------------------------------------
    // TOML round-trip
    // -------------------------------------------------------------------------

    #[test]
    fn load_toml_profile_roundtrip() {
        let toml_str = r#"
id = "roundtrip-test"
display_name = "Roundtrip Test"
document_classes = ["article"]
aliases = ["rt"]

[detection]
min_confidence = 0.80
fallback_profile = "generic"

[[detection.signals]]
kind = "documentclass"
value = "article"
weight = 0.70

[backend]
preferred = "luatex-node"
fallback = ["rule-based"]
requires_xetex = false
prefers_luatex = true

[semantic_policy]
unknown_macro = "rule-engine"
preserve_raw_fallback = true
collect_runtime_events = true
collect_layout_graph = false

[[macro_rules]]
name = "citep"
semantic = "citation"
args = 1
style = "parenthetical"

[[style_map]]
semantic = "heading.level1"
docx_style = "Heading1"

[quality]
min_compatibility_score = 70
max_raw_fallback_blocks = 10
require_reference_graph = true
"#;
        let spec: ProfileSpecFile = load_toml_from_bytes(toml_str.as_bytes()).unwrap();
        assert_eq!(spec.id, "roundtrip-test");
        assert_eq!(spec.detection.min_confidence, 0.80);
        assert_eq!(spec.backend.preferred, "luatex-node");
        assert_eq!(spec.macro_rules[0].name, "citep");
        assert_eq!(spec.style_map[0].docx_style, "Heading1");
        assert_eq!(spec.quality.min_compatibility_score, 70);
    }

    #[test]
    fn load_toml_profile_jos_paper() {
        let profile_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles");
        let path = profile_dir.join("jos-paper-toml.toml");
        let spec = load_toml(&path).unwrap();
        assert_eq!(spec.id, "jos-paper-toml");
        assert_eq!(spec.display_name, "IEEE/JOS Paper");
        assert!(spec.document_classes.contains(&"IEEEtran".to_string()));
        assert_eq!(spec.page_setup.kind.as_deref(), Some("a4"));
        assert_eq!(
            spec.font_policy.latin_main.as_deref(),
            Some("Times New Roman")
        );
        assert_eq!(spec.caption_policy.figure_prefix.as_deref(), Some("Fig."));
        assert_eq!(spec.citation_policy.style.as_deref(), Some("ieee"));
    }

    #[test]
    fn load_toml_profile_chinese_academic() {
        let profile_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles");
        let path = profile_dir.join("chinese-academic-toml.toml");
        let spec = load_toml(&path).unwrap();
        assert_eq!(spec.id, "chinese-academic-toml");
        assert_eq!(spec.display_name, "Chinese Academic Paper");
        assert!(spec.document_classes.contains(&"ctexart".to_string()));
        assert_eq!(spec.font_policy.cjk_main.as_deref(), Some("宋体"));
        assert_eq!(spec.caption_policy.figure_prefix.as_deref(), Some("图"));
        assert_eq!(
            spec.citation_policy.reference_section_title.as_deref(),
            Some("参考文献")
        );
    }

    // -------------------------------------------------------------------------
    // Built-in profiles
    // -------------------------------------------------------------------------

    #[test]
    fn load_builtin_generic_article() {
        let spec = load_builtin("generic-article").unwrap();
        assert_eq!(spec.id, "generic-article");
        assert_eq!(spec.display_name, "Generic Article");
        // Extended fields should have defaults.
        assert_eq!(spec.detection.min_confidence, 0.75);
        assert_eq!(spec.quality.min_compatibility_score, 75);
    }

    #[test]
    fn load_builtin_chinese_academic() {
        let spec = load_builtin("chinese-academic").unwrap();
        assert_eq!(spec.id, "chinese-academic");
        assert!(spec.caption_policy.figure_prefix.is_some());
    }

    #[test]
    fn load_builtin_jos_paper() {
        let spec = load_builtin("jos-paper").unwrap();
        assert_eq!(spec.id, "jos-paper");
        assert!(spec.page_setup.kind.is_some());
    }

    #[test]
    fn load_builtin_medical_journal() {
        let spec = load_builtin("medical-journal").unwrap();
        assert_eq!(spec.id, "medical-journal");
    }

    #[test]
    fn load_nonexistent_returns_none() {
        assert!(load_builtin("nonexistent-profile").is_none());
    }

    // -------------------------------------------------------------------------
    // ProfileRegistry
    // -------------------------------------------------------------------------

    #[test]
    fn profile_registry_loads_builtins() {
        let registry = ProfileRegistry::load_default().unwrap();
        assert!(registry.get("generic-article").is_some());
        assert!(registry.get("chinese-academic").is_some());
        assert!(registry.get("jos-paper").is_some());
        assert!(registry.get("medical-journal").is_some());
    }

    #[test]
    fn profile_registry_resolves_generic_alias() {
        let registry = ProfileRegistry::load_default().unwrap();
        // "generic" is an alias for "generic-article" (defined in generic.toml).
        let spec = registry.get("generic");
        assert!(spec.is_some());
        assert_eq!(spec.unwrap().id, "generic-article");
    }

    #[test]
    fn profile_registry_all_ids_includes_builtins() {
        let registry = ProfileRegistry::load_default().unwrap();
        let ids = registry.all_ids();
        assert!(ids.contains(&"generic-article"));
        assert!(ids.contains(&"chinese-academic"));
        assert!(ids.contains(&"jos-paper"));
        assert!(ids.contains(&"medical-journal"));
    }

    #[test]
    fn profile_registry_loads_toml_profiles() {
        let registry = ProfileRegistry::load_default().unwrap();
        // TOML profiles should be loaded.
        assert!(registry.get("jos-paper-toml").is_some());
        assert!(registry.get("chinese-academic-toml").is_some());
        // New profile IDs added in this phase.
        assert!(registry.get("generic").is_some());
    }

    #[test]
    fn profile_registry_register_and_lookup() {
        let mut registry = ProfileRegistry::default();
        let spec = ProfileSpecFile {
            id: "test-registry".to_string(),
            display_name: "Test Registry".to_string(),
            ..Default::default()
        };
        registry.register(spec);
        assert!(registry.get("test-registry").is_some());
    }

    #[test]
    fn profile_registry_alias_registration() {
        let mut registry = ProfileRegistry::default();
        let spec = ProfileSpecFile {
            id: "canonical".to_string(),
            display_name: "Canonical".to_string(),
            ..Default::default()
        };
        registry.register(spec);
        registry.register_alias("alias1", "canonical");
        registry.register_alias("alias2", "canonical");
        assert_eq!(registry.resolve_alias("alias1"), Some("canonical"));
        assert_eq!(registry.resolve_alias("alias2"), Some("canonical"));
        assert!(registry.get("alias1").is_some());
        assert!(registry.get("alias2").is_some());
    }

    #[test]
    fn profile_registry_missing_id_returns_none() {
        let registry = ProfileRegistry::load_default().unwrap();
        assert!(registry.get("this-does-not-exist").is_none());
    }

    // -------------------------------------------------------------------------
    // List profile IDs
    // -------------------------------------------------------------------------

    #[test]
    fn list_profile_ids_includes_all() {
        let ids = list_profile_ids();
        assert!(ids.contains(&"generic-article".to_string()));
        assert!(ids.contains(&"chinese-academic".to_string()));
        assert!(ids.contains(&"jos-paper".to_string()));
        assert!(ids.contains(&"medical-journal".to_string()));
        assert!(ids.contains(&"generic".to_string()));
        assert!(ids.contains(&"tacl".to_string()));
        assert!(ids.contains(&"cvpr".to_string()));
        assert!(ids.contains(&"nature".to_string()));
        assert!(ids.contains(&"springer".to_string()));
        assert!(ids.contains(&"generic-article-toml".to_string()));
        assert!(ids.contains(&"chinese-academic-toml".to_string()));
        assert!(ids.contains(&"jos-paper-toml".to_string()));
    }

    // -------------------------------------------------------------------------
    // Path resolution
    // -------------------------------------------------------------------------

    #[test]
    fn resolve_profile_path_prefers_toml_over_json() {
        let (path, source) = resolve_profile_path("jos-paper-toml").unwrap();
        assert!(path.to_string_lossy().ends_with(".toml"));
        assert_eq!(source, ProfileLoadSource::TomlFile);

        let (path, source) = resolve_profile_path("medical-journal").unwrap();
        assert!(path.to_string_lossy().ends_with(".json"));
        assert_eq!(source, ProfileLoadSource::JsonFile);
    }

    #[test]
    fn load_profile_from_id_resolves_toml() {
        let (result, source) = load_profile(None, Some("jos-paper-toml"));
        match result {
            ProfileLoadResult::Loaded(spec) => {
                assert_eq!(spec.id, "jos-paper-toml");
                assert_eq!(source, ProfileLoadSource::TomlFile);
            }
            _ => panic!("expected Loaded, got {:?}", result),
        }
    }

    #[test]
    fn load_profile_from_explicit_toml_path() {
        let profile_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles");
        let path = profile_dir.join("chinese-academic-toml.toml");
        let (result, source) = load_profile(Some(path.as_path()), None);
        match result {
            ProfileLoadResult::Loaded(spec) => {
                assert_eq!(spec.id, "chinese-academic-toml");
                assert_eq!(source, ProfileLoadSource::TomlFile);
            }
            _ => panic!("expected Loaded, got {:?}", result),
        }
    }

    #[test]
    fn load_nonexistent_toml_returns_error() {
        let profile_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles");
        let path = profile_dir.join("nonexistent.toml");
        assert!(load_toml(&path).is_err());
    }

    #[test]
    fn profile_load_source_equality() {
        assert_eq!(ProfileLoadSource::JsonFile, ProfileLoadSource::JsonFile);
        assert_eq!(ProfileLoadSource::TomlFile, ProfileLoadSource::TomlFile);
        assert_eq!(ProfileLoadSource::Builtin, ProfileLoadSource::Builtin);
        assert_ne!(ProfileLoadSource::JsonFile, ProfileLoadSource::TomlFile);
        assert_ne!(ProfileLoadSource::Builtin, ProfileLoadSource::JsonFile);
    }

    // -------------------------------------------------------------------------
    // DetectionSpec defaults
    // -------------------------------------------------------------------------

    #[test]
    fn detection_spec_defaults() {
        let json_str = r#"{"id": "d", "display_name": "D"}"#;
        let spec: ProfileSpecFile = serde_json::from_str(json_str).unwrap();
        assert_eq!(spec.detection.min_confidence, 0.75);
        assert_eq!(spec.detection.fallback_profile, "generic");
        assert!(spec.detection.signals.is_empty());
    }

    #[test]
    fn backend_spec_defaults() {
        let json_str = r#"{"id": "b", "display_name": "B"}"#;
        let spec: ProfileSpecFile = serde_json::from_str(json_str).unwrap();
        assert_eq!(spec.backend.preferred, "");
        assert!(spec.backend.fallback.is_empty());
        assert!(!spec.backend.requires_xetex);
        assert!(!spec.backend.prefers_luatex);
    }

    #[test]
    fn semantic_policy_spec_defaults() {
        let json_str = r#"{"id": "s", "display_name": "S"}"#;
        let spec: ProfileSpecFile = serde_json::from_str(json_str).unwrap();
        assert_eq!(spec.semantic_policy.unknown_macro, "rule-engine");
        assert!(spec.semantic_policy.preserve_raw_fallback);
        assert!(spec.semantic_policy.collect_runtime_events);
        assert!(spec.semantic_policy.collect_layout_graph);
    }

    #[test]
    fn quality_spec_defaults() {
        let json_str = r#"{"id": "q", "display_name": "Q"}"#;
        let spec: ProfileSpecFile = serde_json::from_str(json_str).unwrap();
        assert_eq!(spec.quality.min_compatibility_score, 75);
        assert_eq!(spec.quality.max_raw_fallback_blocks, 0);
        assert!(!spec.quality.require_reference_graph);
    }
}
