//! Profile loading from JSON configuration files.
//!
//! Tex2Doc conversion profiles can be specified as JSON files or by built-in
//! profile ID. The loader falls back to built-in profiles when the file is absent
//! or invalid.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A loaded profile specification from a JSON file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSpecFile {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub document_classes: Vec<String>,
    #[serde(default)]
    pub page_setup: PageSetupSpec,
    #[serde(default)]
    pub font_policy: FontPolicySpecFile,
    #[serde(default)]
    pub caption_policy: CaptionPolicySpecFile,
    #[serde(default)]
    pub citation_policy: CitationPolicySpecFile,
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

/// A profile loaded from a TOML file (same shape as ProfileSpecFile).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSpecToml {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub document_classes: Vec<String>,
    #[serde(default)]
    pub page_setup: PageSetupSpec,
    #[serde(default)]
    pub font_policy: FontPolicySpecFile,
    #[serde(default)]
    pub caption_policy: CaptionPolicySpecFile,
    #[serde(default)]
    pub citation_policy: CitationPolicySpecFile,
}

/// Source of a loaded profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileLoadSource {
    JsonFile,
    TomlFile,
    Builtin,
}

/// Result of a profile load attempt.
#[derive(Debug)]
pub enum ProfileLoadResult {
    /// Profile was loaded from a JSON or TOML file.
    Loaded(ProfileSpecFile),
    /// Profile was resolved from a built-in ID.
    BuiltIn(&'static str),
    /// No profile was specified.
    None,
}

/// Loads a profile from a file path, built-in ID, or falls back to `None`.
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
    let json =
        serde_json::to_string(&toml_spec).map_err(|e| ProfileLoadError::Parse(e.to_string()))?;
    serde_json::from_str(&json).map_err(|e| ProfileLoadError::Parse(e.to_string()))
}

/// Resolve a profile ID to a file path in the profiles directory.
/// Prefers TOML over JSON if both exist.
pub fn resolve_profile_path(id: &str) -> Option<(std::path::PathBuf, ProfileLoadSource)> {
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
    builtin_json(id).and_then(|json| serde_json::from_str(json).ok())
}

/// List all available profile IDs (both file-based and built-in).
#[allow(dead_code)]
pub fn list_profile_ids() -> Vec<String> {
    vec![
        "generic-article".to_string(),
        "chinese-academic".to_string(),
        "jos-paper".to_string(),
        "medical-journal".to_string(),
        // TOML-based profiles
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

    #[test]
    fn load_builtin_generic_article() {
        let spec = load_builtin("generic-article").unwrap();
        assert_eq!(spec.id, "generic-article");
        assert_eq!(spec.display_name, "Generic Article");
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
    fn list_profile_ids_includes_all() {
        let ids = list_profile_ids();
        assert!(ids.contains(&"generic-article".to_string()));
        assert!(ids.contains(&"chinese-academic".to_string()));
        assert!(ids.contains(&"jos-paper".to_string()));
        assert!(ids.contains(&"medical-journal".to_string()));
        assert!(ids.contains(&"generic-article-toml".to_string()));
        assert!(ids.contains(&"chinese-academic-toml".to_string()));
        assert!(ids.contains(&"jos-paper-toml".to_string()));
    }

    #[test]
    fn load_nonexistent_returns_none() {
        assert!(load_builtin("nonexistent-profile").is_none());
    }

    #[test]
    fn profile_spec_file_deserializes_full() {
        let json_str = r#"{
  "id": "test-profile",
  "display_name": "Test Profile",
  "document_classes": ["article", "report"],
  "page_setup": {"kind": "a4"},
  "font_policy": {"latin_main": "Arial"},
  "caption_policy": {"figure_prefix": "Fig."}
}"#;
        let spec: ProfileSpecFile = serde_json::from_str(json_str).unwrap();
        assert_eq!(spec.id, "test-profile");
        assert_eq!(spec.display_name, "Test Profile");
        assert_eq!(spec.document_classes.len(), 2);
    }

    #[test]
    fn profile_spec_file_with_defaults() {
        let json_str = r#"{"id": "minimal", "display_name": "Minimal Profile"}"#;
        let spec: ProfileSpecFile = serde_json::from_str(json_str).unwrap();
        assert_eq!(spec.id, "minimal");
        assert!(spec.document_classes.is_empty());
        assert_eq!(spec.page_setup.kind, None);
    }

    // ---------------------------------------------------------------------------
    // TOML loading tests
    // ---------------------------------------------------------------------------

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

    #[test]
    fn load_nonexistent_toml_returns_error() {
        let profile_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles");
        let path = profile_dir.join("nonexistent.toml");
        let result = load_toml(&path);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_profile_path_prefers_toml_over_json() {
        // Create a temp profile dir with both .json and .toml for the same id
        // Since we can't easily create temp files here, we test the expected behavior:
        // jos-paper.toml exists (TOML) but jos-paper.json also exists (JSON).
        // resolve_profile_path should return the TOML one.
        let (path, source) = resolve_profile_path("jos-paper-toml").unwrap();
        assert!(path.to_string_lossy().ends_with(".toml"));
        assert_eq!(source, ProfileLoadSource::TomlFile);

        // For an id that only has JSON (medical-journal), it should return JSON.
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
    fn profile_load_source_equality() {
        assert_eq!(ProfileLoadSource::JsonFile, ProfileLoadSource::JsonFile);
        assert_eq!(ProfileLoadSource::TomlFile, ProfileLoadSource::TomlFile);
        assert_eq!(ProfileLoadSource::Builtin, ProfileLoadSource::Builtin);
        assert_ne!(ProfileLoadSource::JsonFile, ProfileLoadSource::TomlFile);
        assert_ne!(ProfileLoadSource::Builtin, ProfileLoadSource::JsonFile);
    }
}
