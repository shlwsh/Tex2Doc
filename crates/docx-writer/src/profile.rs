//! Profile-specific DOCX style mappings.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// CJK-specific typography options.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CjkOptions {
    /// Chinese punctuation style: "chinese" (full-width) or "western" (half-width)
    #[serde(default = "default_punctuation_style")]
    pub punctuation_style: String,
    /// Ratio of half-width characters (0.0-1.0)
    #[serde(default)]
    pub half_width_ratio: f32,
    /// Font fallback chain for CJK text
    #[serde(default)]
    pub font_fallback_chain: Vec<String>,
    /// Line spacing ratio (1.0 = single, 1.5 = 1.5x, etc.)
    #[serde(default = "default_line_spacing")]
    pub line_spacing: f32,
    /// First line indent in points
    #[serde(default = "default_first_line_indent")]
    pub first_line_indent: i32,
    /// Whether to use Chinese number format
    #[serde(default)]
    pub use_chinese_numbers: bool,
}

impl PartialEq for CjkOptions {
    fn eq(&self, other: &Self) -> bool {
        self.punctuation_style == other.punctuation_style
            && (self.half_width_ratio - other.half_width_ratio).abs() < f32::EPSILON
            && self.font_fallback_chain == other.font_fallback_chain
            && (self.line_spacing - other.line_spacing).abs() < f32::EPSILON
            && self.first_line_indent == other.first_line_indent
            && self.use_chinese_numbers == other.use_chinese_numbers
    }
}

impl Eq for CjkOptions {}

fn default_punctuation_style() -> String {
    "chinese".to_string()
}

fn default_line_spacing() -> f32 {
    1.5
}

fn default_first_line_indent() -> i32 {
    24
}

/// A map from logical style roles to concrete DOCX style IDs.
///
/// Used to override the hardcoded JOS defaults when compiling
/// with a different profile (e.g. generic article, IEEE, RA-L).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileStyleMap {
    /// Map from role name (e.g. "body", "heading1") to style ID (e.g. "BodyText").
    #[serde(flatten)]
    pub by_role: BTreeMap<String, String>,
    /// CJK-specific typography options (applied when profile includes Chinese text)
    #[serde(default)]
    pub cjk_options: Option<CjkOptions>,
}

impl ProfileStyleMap {
    /// Journal of Software (软件学报) paper style map.
    ///
    /// Covers all 25 roles used in the JOS article template:
    /// abstract, author, body, caption, citation, code, comment,
    /// heading, image, institute, keywords, list, masthead, normal,
    /// reference, table, title.
    pub fn jos() -> Self {
        let mut m = BTreeMap::new();
        m.insert("abstract_zh".into(), "JOSAbstractZh".into());
        m.insert("abstract_en".into(), "JOSAbstractEn".into());
        m.insert("author_zh".into(), "JOSAuthorZh".into());
        m.insert("body".into(), "JOSBody".into());
        m.insert("body_no_indent".into(), "JOSBodyNoIndent".into());
        m.insert("caption".into(), "JOSCaption".into());
        m.insert("citation".into(), "JOSCitation".into());
        m.insert("code".into(), "JOSCode".into());
        m.insert("comment".into(), "Comment".into());
        m.insert("heading1".into(), "JOSHeading1".into());
        m.insert("heading2".into(), "JOSHeading2".into());
        m.insert("heading3".into(), "JOSHeading3".into());
        m.insert("english_title".into(), "JOSEnglishTitle".into());
        m.insert("image".into(), "JOSImage".into());
        m.insert("institute_zh".into(), "JOSInstituteZh".into());
        m.insert("keywords".into(), "JOSKeywords".into());
        m.insert("list_bullet".into(), "ListBullet".into());
        m.insert("list_number".into(), "ListNumber".into());
        m.insert("masthead".into(), "JOSMasthead".into());
        m.insert("normal".into(), "Normal".into());
        m.insert("reference".into(), "JOSReference".into());
        m.insert("reference_heading".into(), "JOSReferenceHeading".into());
        m.insert("table_header".into(), "TableHeader".into());
        m.insert("table_text".into(), "JOSTableText".into());
        m.insert("title_zh".into(), "JOSTitleZh".into());

        // CJK options for Chinese academic paper
        let cjk_options = CjkOptions {
            punctuation_style: "chinese".to_string(),
            half_width_ratio: 0.0,
            font_fallback_chain: vec![
                "宋体".to_string(),
                "SimSun".to_string(),
                "Noto Serif CJK SC".to_string(),
            ],
            line_spacing: 1.5,
            first_line_indent: 24,
            use_chinese_numbers: false,
        };

        Self {
            by_role: m,
            cjk_options: Some(cjk_options),
        }
    }

    /// Generic article style map (Word built-in styles).
    ///
    /// Covers only the essential roles needed for a generic document.
    pub fn generic() -> Self {
        let mut m = BTreeMap::new();
        m.insert("body".into(), "BodyText".into());
        m.insert("heading1".into(), "Heading1".into());
        m.insert("heading2".into(), "Heading2".into());
        m.insert("heading3".into(), "Heading3".into());
        Self {
            by_role: m,
            cjk_options: None,
        }
    }

    /// Look up the style ID for a given role.
    ///
    /// Returns `None` when the role is not mapped (caller should fall back
    /// to the hardcoded profile default).
    pub fn get(&self, role: &str) -> Option<&str> {
        self.by_role.get(role).map(|s| s.as_str())
    }

    /// Generate coverage report showing mapped and unmapped roles.
    pub fn coverage_report(&self, required_roles: &[&str]) -> StyleCoverageReport {
        let mut mapped = Vec::new();
        let mut unmapped = Vec::new();
        for role in required_roles {
            if let Some(style_id) = self.get(role) {
                mapped.push((role.to_string(), style_id.to_string()));
            } else {
                unmapped.push(role.to_string());
            }
        }
        StyleCoverageReport {
            total_required: required_roles.len(),
            mapped_count: mapped.len(),
            unmapped_count: unmapped.len(),
            coverage_rate: if required_roles.is_empty() {
                1.0
            } else {
                mapped.len() as f32 / required_roles.len() as f32
            },
            mapped_roles: mapped,
            unmapped_roles: unmapped,
        }
    }
}

/// Style coverage report for profile validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleCoverageReport {
    pub total_required: usize,
    pub mapped_count: usize,
    pub unmapped_count: usize,
    pub coverage_rate: f32,
    pub mapped_roles: Vec<(String, String)>,
    pub unmapped_roles: Vec<String>,
}
