//! LaTeX compatibility analysis for Tex2Doc.
//!
//! Scans TeX sources to detect unsupported packages, environments, and document
//! classes, producing a compatibility score and issue list.

#![forbid(unsafe_code)]

use doc_utils::VirtualFs;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet};
use std::path::Path;

// ---------------------------------------------------------------------------
// Report types
// ---------------------------------------------------------------------------

/// A compatibility analysis report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompatibilityReport {
    /// 0–100 compatibility score.
    pub score: u8,
    /// Number of TeX source files scanned.
    pub scanned_files: usize,
    /// Detected document classes.
    pub document_classes: Vec<String>,
    /// Detected packages.
    pub packages: Vec<String>,
    /// Custom macro definitions detected across all files.
    pub custom_macro_count: usize,
    /// Features that are not supported by the semantic DOCX path.
    pub unsupported: Vec<CompatibilityIssue>,
    /// Features that are partially supported or may need attention.
    pub warnings: Vec<CompatibilityIssue>,
}

impl CompatibilityReport {
    /// Returns true if the compatibility score meets the minimum threshold.
    pub fn is_acceptable(&self, min_score: u8) -> bool {
        self.score >= min_score
    }
}

/// A single compatibility issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityIssue {
    /// Short machine-readable code, e.g., "unsupported_package".
    pub code: String,
    /// The specific feature name, e.g., "minted".
    pub feature: String,
    /// Human-readable description and guidance.
    pub message: String,
}

// ---------------------------------------------------------------------------
// Analyzer
// ---------------------------------------------------------------------------

/// Configuration for compatibility analysis.
#[derive(Debug, Clone)]
pub struct CompatibilityRules {
    /// Minimum acceptable score (0–100). Defaults to 70.
    pub min_score: u8,
    /// Treat TikZ as a warning instead of an error.
    pub tikz_warning_only: bool,
    /// Treat minted as a warning instead of an error.
    pub minted_warning_only: bool,
}

impl Default for CompatibilityRules {
    fn default() -> Self {
        Self {
            min_score: 70,
            tikz_warning_only: false,
            minted_warning_only: false,
        }
    }
}

/// The main compatibility analyzer.
#[derive(Debug, Default, Clone)]
pub struct CompatibilityAnalyzer {
    rules: CompatibilityRules,
}

impl CompatibilityAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_rules(rules: CompatibilityRules) -> Self {
        Self { rules }
    }

    /// Analyze a virtual filesystem and produce a compatibility report.
    pub fn analyze(&self, vfs: &VirtualFs, profile: ProfileKind) -> CompatibilityReport {
        analyze_compatibility_impl(vfs, profile, &self.rules)
    }
}

/// Profile classification used during compatibility analysis.
#[derive(Debug, Clone, Copy, Default)]
pub enum ProfileKind {
    /// Generic / arXiv-like document.
    #[default]
    Generic,
    /// Chinese academic papers (CTeX-based templates).
    GenericArticle,
    /// Chinese academic papers (CTeX-based templates). Alias for GenericArticle.
    ChineseAcademic,
    /// Journal of Software / 软件学报 oriented profile.
    JosPaper,
    /// ACL/TACL conference paper.
    Tacl,
    /// CVPR/ICCV conference paper.
    Cvpr,
    /// Nature research article.
    Nature,
    /// Springer journal article.
    Springer,
    /// Medical journal manuscripts.
    MedicalJournal,
}

impl ProfileKind {
    /// Returns true if the given document class is supported by this profile.
    pub fn supports_document_class(&self, class: &str) -> bool {
        match self {
            Self::Generic | Self::GenericArticle => true,
            Self::ChineseAcademic => {
                ["article", "report", "book", "ctexart", "ctexbook", "ctexrep"]
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(class))
            }
            Self::JosPaper => {
                ["article", "rjthesis"].iter().any(|s| s.eq_ignore_ascii_case(class))
            }
            Self::Tacl => ["acl"].iter().any(|s| s.eq_ignore_ascii_case(class)),
            Self::Cvpr => ["IEEEtran"].iter().any(|s| s.eq_ignore_ascii_case(class)),
            Self::Nature => ["nature"].iter().any(|s| s.eq_ignore_ascii_case(class)),
            Self::Springer => {
                ["springer", "svjour3", "llncs", "sn-jnl"]
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(class))
            }
            Self::MedicalJournal => {
                ["article", "elsarticle", "wlscirep"]
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(class))
            }
        }
    }

    /// Returns the canonical name of this profile for display/reporting.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::GenericArticle => "generic-article",
            Self::ChineseAcademic => "chinese-academic",
            Self::JosPaper => "jos-paper",
            Self::Tacl => "tacl",
            Self::Cvpr => "cvpr",
            Self::Nature => "nature",
            Self::Springer => "springer",
            Self::MedicalJournal => "medical-journal",
        }
    }

    /// Try to resolve a string profile ID to a ProfileKind variant.
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "generic" => Some(Self::Generic),
            "generic-article" => Some(Self::GenericArticle),
            "chinese-academic" => Some(Self::ChineseAcademic),
            "jos-paper" | "jos-paper-toml" => Some(Self::JosPaper),
            "tacl" | "acl" | "acl-paper" => Some(Self::Tacl),
            "cvpr" | "iccv" | "cvpr-paper" => Some(Self::Cvpr),
            "nature" | "nature-research" => Some(Self::Nature),
            "springer" | "svjour3" | "llncs" | "sn-jnl" => Some(Self::Springer),
            "medical-journal" => Some(Self::MedicalJournal),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Profile-aware package compatibility
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackageCompat {
    Supported,
    Warning,
    Unsupported,
}

fn profile_package_compat(profile: ProfileKind, package: &str) -> Option<(PackageCompat, &'static str)> {
    use PackageCompat::*;

    let compat = match profile {
        // Generic / arXiv: no specific package warnings.
        ProfileKind::Generic | ProfileKind::GenericArticle | ProfileKind::MedicalJournal => return None,

        ProfileKind::JosPaper => {
            return Some(match package {
                "IEEEtran" => (Supported, "IEEEtran is the primary class for JOS papers"),
                "amsmath" => (Supported, "amsmath is fully supported for JOS papers"),
                "graphicx" => (Supported, "graphicx is fully supported"),
                "algorithm2e" => (Warning, "algorithm2e may lose fine styling in IEEE JOS format"),
                "tabularx" => (Warning, "tabularx advanced layout may be simplified"),
                _ => return None,
            });
        }

        ProfileKind::Tacl => {
            return Some(match package {
                "acl" => (Supported, "acl class is the primary class for TACL papers"),
                "natbib" => (Supported, "natbib is fully supported for ACL/TACL"),
                "biblatex" => (Warning, "biblatex is not recommended for TACL; use natbib instead"),
                "tikz" => (Warning, "TikZ may need rasterization fallback in TACL papers"),
                _ => return None,
            });
        }

        ProfileKind::Cvpr => {
            return Some(match package {
                "IEEEtran" => (Supported, "IEEEtran[conference] is the primary class for CVPR"),
                "amsmath" => (Supported, "amsmath is fully supported for CVPR papers"),
                "algorithmicx" => (Warning, "algorithmicx may lose styling in CVPR format"),
                "subcaption" => (Warning, "subcaption support is limited in CVPR papers"),
                _ => return None,
            });
        }

        ProfileKind::Nature => {
            return Some(match package {
                "nature" => (Supported, "nature class is fully supported"),
                "natbib" => (Supported, "natbib is fully supported for Nature articles"),
                "biblatex" => (Warning, "biblatex is not recommended for Nature; use natbib"),
                "pstricks" => (Unsupported, "PSTricks is not supported for Nature articles"),
                _ => return None,
            });
        }

        ProfileKind::Springer => {
            return Some(match package {
                "springer" => (Supported, "springer class is fully supported"),
                "svjour3" => (Supported, "svjour3 class is fully supported for Springer"),
                "llncs" => (Supported, "LLNCS class is supported for Springer"),
                "algorithm2e" => (Warning, "algorithm2e may conflict with Springer style"),
                "longtable" => (Warning, "longtable is not recommended in Springer articles"),
                "beamer" => (Unsupported, "beamer is not supported in Springer journal articles"),
                _ => return None,
            });
        }

        ProfileKind::ChineseAcademic => {
            return Some(match package {
                "ctex" => (Supported, "CTeX suite is fully supported for Chinese academic"),
                "xeCJK" => (Supported, "xeCJK is fully supported"),
                "fontspec" => (Supported, "fontspec is fully supported"),
                "gbt7714" => (Warning, "gbt7714 has partial compatibility; verify bibliography format"),
                "biblatex" => (Warning, "biblatex has limited support for Chinese academic papers"),
                "minted" => (Unsupported, "minted is not supported for Chinese academic papers"),
                _ => return None,
            });
        }
    };
}

fn apply_profile_package_checks(
    profile: ProfileKind,
    packages: Vec<String>,
    report: &mut CompatibilityReport,
    warning_seen: &mut HashSet<String>,
) {
    use PackageCompat::*;
    for pkg in packages {
        let Some((compat, msg)) = profile_package_compat(profile, &pkg) else {
            continue;
        };
        match compat {
            Supported => { /* no issue */ }
            Warning => {
                add_compatibility_issue(
                    &mut report.warnings,
                    warning_seen,
                    "profile_limited_package",
                    &pkg,
                    msg,
                );
            }
            Unsupported => {
                add_compatibility_issue(
                    &mut report.unsupported,
                    warning_seen,
                    "profile_unsupported_package",
                    &pkg,
                    msg,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Analysis implementation
// ---------------------------------------------------------------------------

fn analyze_compatibility_impl(
    vfs: &VirtualFs,
    profile: ProfileKind,
    rules: &CompatibilityRules,
) -> CompatibilityReport {
    let mut report = CompatibilityReport::default();
    let mut document_classes = HashSet::new();
    let mut packages = HashSet::new();
    let mut unsupported_seen: HashSet<String> = HashSet::new();
    let mut warning_seen: HashSet<String> = HashSet::new();

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
            if rules.tikz_warning_only {
                add_compatibility_issue(
                    &mut report.warnings,
                    &mut warning_seen,
                    "limited_environment",
                    "tikzpicture",
                    "TikZ graphics are partially supported via rasterization fallback",
                );
            } else {
                add_compatibility_issue(
                    &mut report.unsupported,
                    &mut warning_seen,
                    "unsupported_environment",
                    "tikzpicture",
                    "TikZ graphics need rasterization or a semantic drawing plugin before high-fidelity DOCX output",
                );
            }
        }
        if contains_tex_environment(&source, "minted") {
            add_compatibility_issue(
                &mut report.unsupported,
                &mut warning_seen,
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
    let sorted_packages = sorted_strings(packages);
    report.packages = sorted_packages.clone();

    // Check document classes.
    let classes_to_check: Vec<String> = report.document_classes.iter().cloned().collect();
    for class_str in classes_to_check {
        if matches!(class_str.as_str(), "beamer" | "standalone") {
            add_compatibility_issue(
                &mut report.unsupported,
                &mut warning_seen,
                "unsupported_document_class",
                &class_str,
                "presentation or standalone drawing classes are outside the paper-oriented semantic profile",
            );
        } else if !profile.supports_document_class(&class_str) {
            add_compatibility_issue(
                &mut report.warnings,
                &mut warning_seen,
                "profile_document_class_mismatch",
                &class_str,
                "document class is outside the active profile and may need profile-specific lowering rules",
            );
        }
    }

    // Check packages (generic compatibility).
    for package in sorted_packages.clone() {
        match package.as_str() {
            "tikz" | "pgf" | "pgfplots" | "circuitikz" => {
                if rules.tikz_warning_only {
                    add_compatibility_issue(
                        &mut report.warnings,
                        &mut warning_seen,
                        "limited_package",
                        package.as_str(),
                        "PGF/TikZ graphics are partially supported via rasterization fallback",
                    );
                } else {
                    add_compatibility_issue(
                        &mut report.unsupported,
                        &mut warning_seen,
                        "unsupported_package",
                        package.as_str(),
                        "PGF/TikZ graphics are not semantically converted to editable DOCX drawing objects yet",
                    );
                }
            }
            "pstricks" => add_compatibility_issue(
                &mut report.unsupported,
                &mut warning_seen,
                "unsupported_package",
                package.as_str(),
                "PSTricks output requires a PostScript rendering path that the semantic engine does not provide yet",
            ),
            "minted" => {
                if rules.minted_warning_only {
                    add_compatibility_issue(
                        &mut report.warnings,
                        &mut warning_seen,
                        "limited_package",
                        package.as_str(),
                        "minted content may be downgraded to plain code text",
                    );
                } else {
                    add_compatibility_issue(
                        &mut report.unsupported,
                        &mut warning_seen,
                        "unsupported_package",
                        package.as_str(),
                        "minted requires external Pygments processing and shell-escape semantics",
                    );
                }
            }
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

    // Profile-aware package checks.
    apply_profile_package_checks(profile, sorted_packages, &mut report, &mut warning_seen);

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

fn apply_compatibility_score(report: &mut CompatibilityReport) {
    let unsupported_penalty = report.unsupported.len() * 18;
    let warning_penalty = report.warnings.len() * 6;
    let macro_penalty = (report.custom_macro_count * 2).min(12);
    let penalty = (unsupported_penalty + warning_penalty + macro_penalty).min(100);
    report.score = 100usize.saturating_sub(penalty) as u8;
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

fn is_tex_source_path(path: &Path) -> bool {
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

fn count_custom_macro_definitions(source: &str) -> usize {
    let mut count = 0usize;
    let bytes = source.as_bytes();
    let mut i = 0usize;

    while i < bytes.len().saturating_sub(3) {
        if bytes[i] == b'\\' && bytes[i + 1] == b'd' && bytes[i + 2] == b'e' && bytes[i + 3] == b'f' {
            if !bytes[i + 4..].first().is_some_and(|b| b.is_ascii_alphabetic()) {
                count += 1;
                i += 4;
                continue;
            }
        }
        if bytes[i] == b'\\' && bytes[i + 1] == b'c' && bytes[i + 2] == b'o' && bytes[i + 3] == b'm' && bytes[i + 4] == b'm' && bytes[i + 5] == b'a' && bytes[i + 6] == b'n' && bytes[i + 7] == b'd' {
            if !bytes[i + 8..].first().is_some_and(|b| b.is_ascii_alphabetic()) {
                count += 1;
                i += 8;
                continue;
            }
        }
        if bytes[i] == b'\\' && bytes[i + 1] == b'n' && bytes[i + 2] == b'e' && bytes[i + 3] == b'w' && bytes[i + 4] == b'c' && bytes[i + 5] == b'o' && bytes[i + 6] == b'm' && bytes[i + 7] == b'm' && bytes[i + 8] == b'a' && bytes[i + 9] == b'n' && bytes[i + 10] == b'd' {
            if !bytes[i + 11..].first().is_some_and(|b| b.is_ascii_alphabetic()) {
                count += 1;
                i += 11;
                continue;
            }
        }
        i += 1;
    }
    count
}

fn contains_tex_environment(source: &str, environment: &str) -> bool {
    let begin = format!("\\begin{{{environment}}}");
    source.contains(&begin)
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

        if let Some(end) = find_matching_bracket(text, pos) {
            pos = end + 1;
        } else {
            return pos;
        }
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

fn split_tex_name_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn sorted_strings(set: HashSet<String>) -> Vec<String> {
    let mut v: Vec<String> = set.into_iter().collect();
    v.sort();
    v
}

#[derive(Debug, Clone)]
struct ScannedTexCommand {
    name: String,
    argument: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_score_calculation() {
        let mut report = CompatibilityReport::default();
        report.unsupported.push(CompatibilityIssue {
            code: "test".to_string(),
            feature: "test".to_string(),
            message: "test".to_string(),
        });
        apply_compatibility_score(&mut report);
        assert_eq!(report.score, 82); // 100 - 18
    }

    #[test]
    fn report_score_max_100() {
        let mut report = CompatibilityReport::default();
        for i in 0..10 {
            report.unsupported.push(CompatibilityIssue {
                code: format!("u{}", i),
                feature: format!("f{}", i),
                message: "test".to_string(),
            });
        }
        apply_compatibility_score(&mut report);
        assert_eq!(report.score, 0);
    }

    #[test]
    fn profile_supports_document_class() {
        let profile = ProfileKind::ChineseAcademic;
        assert!(profile.supports_document_class("ctexart"));
        assert!(profile.supports_document_class("article"));
        assert!(!profile.supports_document_class("beamer"));

        let generic = ProfileKind::GenericArticle;
        assert!(generic.supports_document_class("beamer")); // generic accepts everything
    }

    #[test]
    fn is_tex_source_path_works() {
        assert!(is_tex_source_path(Path::new("main.tex")));
        assert!(is_tex_source_path(Path::new("sty.sty")));
        assert!(is_tex_source_path(Path::new("cls.cls")));
        assert!(!is_tex_source_path(Path::new("main.pdf")));
        assert!(!is_tex_source_path(Path::new("figure.png")));
    }

    #[test]
    fn test_contains_tex_environment() {
        assert!(contains_tex_environment(r"\begin{document}", "document"));
        assert!(!contains_tex_environment(r"\begin{figure}", "table"));
    }

    #[test]
    fn test_count_custom_macro_definitions() {
        let source = r"\def\foo{bar} \command{x} \def\baz{qux}";
        let count = count_custom_macro_definitions(source);
        assert_eq!(count, 3); // 2×\def + 1×\command (but not \begin/\end)
    }

    #[test]
    fn test_split_tex_name_list() {
        let names = split_tex_name_list("geometry, hyperref, amsmath");
        assert_eq!(names.len(), 3);
        assert_eq!(names[0], "geometry");
        assert_eq!(names[2], "amsmath");
    }

    #[test]
    fn compatibility_report_is_acceptable() {
        let report = CompatibilityReport {
            score: 80,
            ..Default::default()
        };
        assert!(report.is_acceptable(70));
        assert!(!report.is_acceptable(85));
    }

    #[test]
    fn compatibility_analyzer_default_rules() {
        let analyzer = CompatibilityAnalyzer::new();
        let vfs = VirtualFs::new();
        let report = analyzer.analyze(&vfs, ProfileKind::GenericArticle);
        assert_eq!(report.scanned_files, 0);
    }

    #[test]
    fn compatibility_issue_serialization() {
        let issue = CompatibilityIssue {
            code: "test".to_string(),
            feature: "minted".to_string(),
            message: "unsupported".to_string(),
        };
        let json = serde_json::to_string(&issue).unwrap();
        let round_trip: CompatibilityIssue = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip.feature, "minted");
    }

    #[test]
    fn compatibility_report_serialization() {
        let mut report = CompatibilityReport::default();
        report.packages.push("amsmath".to_string());
        let json = serde_json::to_string(&report).unwrap();
        let round_trip: CompatibilityReport = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip.packages[0], "amsmath");
    }

    #[test]
    fn scan_tex_commands_finds_documentclass() {
        let source = r"\documentclass{article}";
        let results = scan_tex_commands(source, &["documentclass"]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "documentclass");
        assert_eq!(results[0].argument, "article");
    }

    #[test]
    fn scan_tex_commands_handles_options() {
        let source = r"\usepackage[utf8]{inputenc}";
        let results = scan_tex_commands(source, &["usepackage"]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].argument, "inputenc");
    }

    #[test]
    fn tikz_warning_only_rule() {
        let rules = CompatibilityRules {
            min_score: 70,
            tikz_warning_only: true,
            minted_warning_only: false,
        };
        let analyzer = CompatibilityAnalyzer::with_rules(rules);
        let mut vfs = VirtualFs::new();
        vfs.insert("main.tex", r"\usepackage{tikz}".as_bytes().to_vec());
        let report = analyzer.analyze(&vfs, ProfileKind::GenericArticle);
        // tikz should now be a warning, not unsupported
        assert!(report.warnings.iter().any(|i| i.feature == "tikz"));
        assert!(!report.unsupported.iter().any(|i| i.feature == "tikz"));
    }
}
