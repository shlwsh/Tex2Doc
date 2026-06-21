//! Profile-specific DOCX style mappings.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A map from logical style roles to concrete DOCX style IDs.
///
/// Used to override the hardcoded JOS defaults when compiling
/// with a different profile (e.g. generic article, IEEE, RA-L).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileStyleMap {
    /// Map from role name (e.g. "body", "heading1") to style ID (e.g. "BodyText").
    #[serde(flatten)]
    pub by_role: BTreeMap<String, String>,
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
        Self { by_role: m }
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
        Self { by_role: m }
    }

    /// Look up the style ID for a given role.
    ///
    /// Returns `None` when the role is not mapped (caller should fall back
    /// to the hardcoded profile default).
    pub fn get(&self, role: &str) -> Option<&str> {
        self.by_role.get(role).map(|s| s.as_str())
    }
}
