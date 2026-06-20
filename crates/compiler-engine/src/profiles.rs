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

/// Result of a profile load attempt.
#[derive(Debug)]
pub enum ProfileLoadResult {
    /// Profile was loaded from a JSON file.
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
) -> ProfileLoadResult {
    if let Some(path) = profile_path {
        return match load_from_file(path) {
            Ok(spec) => ProfileLoadResult::Loaded(spec),
            Err(e) => {
                eprintln!("warning: failed to load profile from {:?}: {}", path, e);
                ProfileLoadResult::None
            }
        };
    }

    if let Some(id) = profile_id {
        if let Some(builtin) = builtin_json(id) {
            return ProfileLoadResult::BuiltIn(builtin);
        }
        eprintln!("warning: unknown profile ID '{}'; using default", id);
        return ProfileLoadResult::BuiltIn(builtin_json("generic-article").unwrap());
    }

    ProfileLoadResult::None
}

/// Load a profile JSON file.
pub fn load_from_file(path: impl AsRef<Path>) -> Result<ProfileSpecFile, ProfileLoadError> {
    let bytes = fs::read(path.as_ref()).map_err(|e| ProfileLoadError::Io(e.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|e| ProfileLoadError::Parse(e.to_string()))
}

/// Resolve a profile ID to a JSON file path in the profiles directory.
pub fn resolve_profile_path(id: &str) -> Option<std::path::PathBuf> {
    let profile_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("profiles");
    let path = profile_dir.join(format!("{}.json", id));
    if path.is_file() {
        Some(path)
    } else {
        None
    }
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
            Self::Parse(s) => write!(f, "JSON parse error: {}", s),
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
}
