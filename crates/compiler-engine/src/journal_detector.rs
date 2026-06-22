//! Journal/profile auto-detection for the semantic TeX engine.
//!
//! Scans TeX sources in a virtual filesystem, extracts document class, options,
//! packages, macros and bibliography style signals, then scores each registered
//! profile's detection rules to determine the most likely journal template.

use doc_utils::VirtualFs;
use serde::{Deserialize, Serialize};

use crate::profiles::{ProfileRegistry, ProfileSpecFile};

/// Kind of detection signal extracted from TeX source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalKind {
    DocumentClass,
    DocumentClassOption,
    Package,
    Macro,
    BibliographyStyle,
    EngineFeature,
}

impl SignalKind {
    /// Match this kind against a detection-signal `kind` string from a profile.
    pub fn matches_profile_kind(&self, profile_kind: &str) -> bool {
        match self {
            Self::DocumentClass => profile_kind.eq_ignore_ascii_case("documentclass"),
            Self::DocumentClassOption => {
                profile_kind.eq_ignore_ascii_case("documentclass_option")
                    || profile_kind.eq_ignore_ascii_case("documentclassoption")
            }
            Self::Package => {
                profile_kind.eq_ignore_ascii_case("package")
                    || profile_kind.eq_ignore_ascii_case("usepackage")
            }
            Self::Macro => profile_kind.eq_ignore_ascii_case("macro"),
            Self::BibliographyStyle => {
                profile_kind.eq_ignore_ascii_case("bibliographystyle")
                    || profile_kind.eq_ignore_ascii_case("bibstyle")
            }
            Self::EngineFeature => {
                profile_kind.eq_ignore_ascii_case("engine_feature")
                    || profile_kind.eq_ignore_ascii_case("enginefeature")
            }
        }
    }
}

impl std::fmt::Display for SignalKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DocumentClass => write!(f, "documentclass"),
            Self::DocumentClassOption => write!(f, "documentclass_option"),
            Self::Package => write!(f, "package"),
            Self::Macro => write!(f, "macro"),
            Self::BibliographyStyle => write!(f, "bibliographystyle"),
            Self::EngineFeature => write!(f, "engine_feature"),
        }
    }
}

/// A signal matched during journal detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedSignal {
    /// Kind of signal that matched.
    pub kind: SignalKind,
    /// The actual value that was found in the source.
    pub value: String,
    /// Weight contributed to the confidence score.
    pub weight: f32,
    /// File where the signal was found.
    pub source_path: String,
    /// Line number in the source file (1-indexed), if available.
    pub line: Option<usize>,
}

/// Result of scoring a single profile candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalDetection {
    /// Profile ID that was scored.
    pub profile_id: String,
    /// Accumulated confidence score (0.0–1.0).
    pub confidence: f32,
    /// All signals matched for this profile, with weights.
    pub matched_signals: Vec<MatchedSignal>,
    /// True if this was reached via fallback (confidence below threshold).
    pub fallback: bool,
}

/// Full detection report with ranked candidates and the selected profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalDetectionReport {
    /// The profile ID that should be used for compilation.
    pub selected_profile_id: String,
    /// Confidence score of the selected profile.
    pub confidence: f32,
    /// All scored candidates, sorted by confidence descending.
    pub candidates: Vec<JournalDetection>,
    /// Human-readable diagnostics.
    pub diagnostics: Vec<JournalDiagnostic>,
}

/// A diagnostic message produced during detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalDiagnostic {
    /// Diagnostic severity.
    pub level: DiagnosticLevel,
    /// Short code, e.g. "low_confidence", "no_signals".
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticLevel {
    Info,
    Warning,
}

/// JournalDetector: scans TeX sources and scores them against registered profiles.
pub struct JournalDetector {
    registry: ProfileRegistry,
}

impl JournalDetector {
    /// Create a new detector backed by the default profile registry.
    pub fn new() -> Self {
        Self {
            registry: ProfileRegistry::load_default().expect("profile registry should always load"),
        }
    }

    /// Create a detector with a custom registry (for testing).
    pub fn with_registry(registry: ProfileRegistry) -> Self {
        Self { registry }
    }

    /// Scan the virtual filesystem and return a detection report.
    pub fn detect(&self, vfs: &VirtualFs) -> JournalDetectionReport {
        let signals = self.extract_signals(vfs);
        let candidates = self.score_profiles(&signals);
        let mut sorted = candidates.clone();
        sorted.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Pick the best candidate that meets its min_confidence, or fall back to generic.
        let selected = self.select_best(&sorted);

        let mut diagnostics = Vec::new();

        // Warn if auto-detection confidence is low.
        if selected.confidence < 0.75 {
            diagnostics.push(JournalDiagnostic {
                level: DiagnosticLevel::Warning,
                code: "low_confidence".to_string(),
                message: format!(
                    "profile '{}' detected with confidence {:.2} (below 0.75); recommend explicit --profile-id",
                    selected.profile_id, selected.confidence
                ),
            });
        }

        // Warn if multiple candidates are very close.
        if let [first, second, ..] = sorted.as_slice() {
            if (first.confidence - second.confidence).abs() < 0.05 && first.confidence > 0.3 {
                diagnostics.push(JournalDiagnostic {
                    level: DiagnosticLevel::Warning,
                    code: "ambiguous_profile".to_string(),
                    message: format!(
                        "profiles '{}' ({:.2}) and '{}' ({:.2}) are within 0.05 confidence; chose '{}'",
                        first.profile_id, first.confidence,
                        second.profile_id, second.confidence,
                        selected.profile_id
                    ),
                });
            }
        }

        JournalDetectionReport {
            selected_profile_id: selected.profile_id.clone(),
            confidence: selected.confidence,
            candidates: sorted,
            diagnostics,
        }
    }

    /// Extract all signals from every TeX-like file in the VFS.
    fn extract_signals(&self, vfs: &VirtualFs) -> Vec<ExtractedSignal> {
        let mut out = Vec::new();
        for path in vfs.paths() {
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if !ext.eq_ignore_ascii_case("tex")
                && !ext.eq_ignore_ascii_case("sty")
                && !ext.eq_ignore_ascii_case("cls")
                && !ext.eq_ignore_ascii_case("ltx")
            {
                continue;
            }

            let Ok(bytes) = vfs.read(path) else { continue };
            let Ok(raw) = std::str::from_utf8(&bytes) else {
                continue;
            };

            let source = strip_comments(raw);
            let path_str = path.to_string_lossy().to_string();
            let file_signals = extract_signals_from_source(&source, &path_str);
            out.extend(file_signals);
        }
        out
    }

    /// Score every registered profile against the extracted signals.
    fn score_profiles(&self, signals: &[ExtractedSignal]) -> Vec<JournalDetection> {
        let mut detections = Vec::new();

        for id in self.registry.all_ids() {
            let Some(spec) = self.registry.get(id) else {
                continue;
            };
            let detection = score_profile(spec, signals);
            detections.push(detection);
        }

        detections
    }

    /// Choose the best profile, applying fallback if needed.
    fn select_best<'a>(&self, sorted: &'a [JournalDetection]) -> &'a JournalDetection {
        for candidate in sorted.iter() {
            let Some(spec) = self.registry.get(&candidate.profile_id) else {
                continue;
            };
            if candidate.confidence >= spec.detection.min_confidence {
                return candidate;
            }
        }

        // Fall back to generic.
        sorted
            .iter()
            .find(|d| d.profile_id == "generic-article" || d.profile_id == "generic")
            .unwrap_or(&sorted[0])
    }
}
impl Default for JournalDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Signal extraction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ExtractedSignal {
    kind: SignalKind,
    value: String,
    path: String,
    line: Option<usize>,
}

/// Strip TeX comments (% to end of line, not inside braces) from source.
fn strip_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && !is_escaped(bytes, i) {
            // Skip to end of line.
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
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

/// Extract all signals from a single source file's comment-stripped content.
fn extract_signals_from_source(source: &str, path: &str) -> Vec<ExtractedSignal> {
    let mut signals = Vec::new();

    // 1. \documentclass[options]{class}
    for m in scan_command_with_optarg(source, "documentclass") {
        let line = line_number(source, m.start);
        signals.push(ExtractedSignal {
            kind: SignalKind::DocumentClass,
            value: m.class_name.clone(),
            path: path.to_string(),
            line,
        });
        if let Some(opts) = &m.optarg {
            for opt in opts.split(',') {
                let opt = opt.trim().to_string();
                if !opt.is_empty() {
                    signals.push(ExtractedSignal {
                        kind: SignalKind::DocumentClassOption,
                        value: opt,
                        path: path.to_string(),
                        line,
                    });
                }
            }
        }
    }

    // 2. \usepackage / \RequirePackage
    for m in scan_command(source, &["usepackage", "RequirePackage"]) {
        let line = line_number(source, m.start);
        for pkg in m.args.split(',') {
            let pkg = pkg.trim().to_string();
            if !pkg.is_empty() {
                signals.push(ExtractedSignal {
                    kind: SignalKind::Package,
                    value: pkg,
                    path: path.to_string(),
                    line,
                });
            }
        }
    }

    // 3. \bibliographystyle
    for m in scan_command(source, &["bibliographystyle"]) {
        let line = line_number(source, m.start);
        signals.push(ExtractedSignal {
            kind: SignalKind::BibliographyStyle,
            value: m.args.clone(),
            path: path.to_string(),
            line,
        });
    }

    // 4. Template-specific macros (author blocks, conference markers, etc.)
    let macro_signals = [
        // IEEE
        ("IEEEauthorblockN", "IEEEauthorblockN"),
        ("IEEEauthorblockA", "IEEEauthorblockA"),
        ("IEEEkeywords", "IEEEkeywords"),
        ("IEEEpeerreviewmaketitle", "IEEEpeerreviewmaketitle"),
        ("cvprfinalcopy", "cvprfinalcopy"),
        ("iccvfinalcopy", "iccvfinalcopy"),
        ("cvprPaperID", "cvprPaperID"),
        // ACL
        ("aclfinalcopy", "aclfinalcopy"),
        ("aclpaperid", "aclpaperid"),
        ("shorttitle", "shorttitle"),
        // Nature
        ("corres", "corres"),
        ("equalcont", "equalcont"),
        ("affil", "affil"),
        // Springer
        ("institute", "institute"),
        ("titlerunning", "titlerunning"),
        ("authorrunning", "authorrunning"),
        ("orcidID", "orcidID"),
        // Chinese
        ("setCJKmainfont", "setCJKmainfont"),
        ("zhabstract", "zhabstract"),
        ("enabstract", "enabstract"),
        ("ctexset", "ctexset"),
    ];

    let source_compact: String = source
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();

    for (macro_name, signal_value) in macro_signals {
        if source_compact.contains(&format!("\\{}", macro_name)) {
            let line = source
                .lines()
                .position(|l| l.contains(&format!("\\{}", macro_name)));
            signals.push(ExtractedSignal {
                kind: SignalKind::Macro,
                value: signal_value.to_string(),
                path: path.to_string(),
                line,
            });
        }
    }

    signals
}

struct CmdMatch {
    start: usize,
    class_name: String,
    optarg: Option<String>,
    args: String,
}

fn scan_command_with_optarg(source: &str, cmd: &str) -> Vec<CmdMatch> {
    let mut matches = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;

    while i < bytes.len().saturating_sub(2) {
        if bytes[i] != b'\\' {
            i += 1;
            continue;
        }
        let rest = &source[i + 1..];
        if !rest.starts_with(cmd) {
            i += 1;
            continue;
        }
        let after = &rest[cmd.len()..];
        if after
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '@')
        {
            i += 1;
            continue;
        }

        // Skip whitespace.
        let mut pos = 0;
        while pos < after.len()
            && after[pos..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_whitespace())
        {
            pos += after[pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
        }

        // Optional [options]
        let optarg = if after[pos..].starts_with('[') {
            find_matching_bracket(after, pos).map(|(end, _opt)| {
                let result = after[pos + 1..end].to_string();
                (end + 1, result)
            })
        } else {
            None
        };
        let (opt_end, opt_val) = optarg.unwrap_or((pos, String::new()));
        let args_start = opt_end;

        // Mandatory {args}
        if args_start < after.len() && after[args_start..].starts_with('{') {
            if let Some(end) = find_matching_brace(after, args_start) {
                let class_name = after[args_start + 1..end].trim().to_string();
                matches.push(CmdMatch {
                    start: i,
                    class_name,
                    optarg: if opt_val.is_empty() {
                        None
                    } else {
                        Some(opt_val)
                    },
                    args: String::new(),
                });
                i = i + 1 + args_start + end - args_start + 1;
                continue;
            }
        }
        i += 1;
    }
    matches
}

fn scan_command(source: &str, commands: &[&str]) -> Vec<CmdMatch> {
    let mut matches = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;

    while i < bytes.len().saturating_sub(2) {
        if bytes[i] != b'\\' {
            i += 1;
            continue;
        }
        let rest = &source[i + 1..];
        let mut found_cmd = None;
        for &cmd in commands {
            if rest.starts_with(cmd) {
                let after = &rest[cmd.len()..];
                if after
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '@')
                {
                    continue;
                }
                found_cmd = Some(cmd);
                break;
            }
        }
        if found_cmd.is_none() {
            i += 1;
            continue;
        }
        let cmd = found_cmd.unwrap();
        let after = &rest[cmd.len()..];

        // Skip whitespace.
        let mut pos = 0;
        while pos < after.len()
            && after[pos..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_whitespace())
        {
            pos += after[pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
        }

        if pos < after.len() && after[pos..].starts_with('[') {
            if let Some((end, _opt)) = find_matching_bracket(after, pos) {
                let args = after[pos + 1..end].trim().to_string();
                matches.push(CmdMatch {
                    start: i,
                    class_name: String::new(),
                    optarg: None,
                    args,
                });
                i = i + 1 + end + 1;
                continue;
            }
        }

        if pos < after.len() && after[pos..].starts_with('{') {
            if let Some(end) = find_matching_brace(after, pos) {
                let args = after[pos + 1..end].trim().to_string();
                matches.push(CmdMatch {
                    start: i,
                    class_name: String::new(),
                    optarg: None,
                    args,
                });
                i = i + 1 + end + 1;
                continue;
            }
        }
        i += 1;
    }
    matches
}

fn find_matching_bracket(s: &str, open: usize) -> Option<(usize, String)> {
    if s[open..].starts_with('[') {
        let mut depth = 0i32;
        let mut i = open;
        while i < s.len() {
            let ch = s[i..].chars().next()?;
            let ch_bytes = ch.len_utf8();
            if s[i..].starts_with('[') && (i == open || !s[i..].starts_with("\\[")) {
                depth += 1;
            } else if s[i..].starts_with(']') && (i == open || !s[i..].starts_with("\\]")) {
                depth -= 1;
                if depth == 0 {
                    return Some((i, s[open + 1..i].to_string()));
                }
            }
            i += ch_bytes;
        }
    }
    None
}

fn find_matching_brace(s: &str, open: usize) -> Option<usize> {
    if !s[open..].starts_with('{') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = open;
    while i < s.len() {
        let ch = s[i..].chars().next()?;
        let ch_bytes = ch.len_utf8();
        if s[i..].starts_with('{') {
            depth += 1;
        } else if s[i..].starts_with('}') {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += ch_bytes;
    }
    None
}

fn line_number(source: &str, byte_offset: usize) -> Option<usize> {
    source[..byte_offset.min(source.len())]
        .chars()
        .filter(|&c| c == '\n')
        .count()
        .checked_add(1)
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

fn score_profile(spec: &ProfileSpecFile, signals: &[ExtractedSignal]) -> JournalDetection {
    let mut total_score = 0.0_f32;
    let mut matched_signals = Vec::new();
    let mut seen_signals: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();

    for ds in &spec.detection.signals {
        let signal_kind = match ds.kind.as_str() {
            "documentclass" => SignalKind::DocumentClass,
            "documentclass_option" | "documentclassoption" => SignalKind::DocumentClassOption,
            "package" | "usepackage" => SignalKind::Package,
            "macro" => SignalKind::Macro,
            "bibliographystyle" | "bibstyle" => SignalKind::BibliographyStyle,
            "engine_feature" | "enginefeature" => SignalKind::EngineFeature,
            _ => continue,
        };

        for extracted in signals {
            if extracted.kind == signal_kind
                && extracted.value.eq_ignore_ascii_case(&ds.value)
                && seen_signals.insert((ds.kind.clone(), extracted.value.clone()))
            {
                total_score += ds.weight;
                matched_signals.push(MatchedSignal {
                    kind: signal_kind,
                    value: extracted.value.clone(),
                    weight: ds.weight,
                    source_path: extracted.path.clone(),
                    line: extracted.line,
                });
            }
        }
    }

    JournalDetection {
        profile_id: spec.id.clone(),
        confidence: total_score.min(1.0),
        matched_signals,
        fallback: false,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use doc_utils::VirtualFs;

    fn vfs_with_source(name: &str, source: &str) -> VirtualFs {
        let mut vfs = VirtualFs::new();
        vfs.insert(name, source.as_bytes().to_vec());
        vfs
    }

    #[test]
    fn detect_ieee_journal() {
        let vfs = vfs_with_source(
            "main.tex",
            r#"\documentclass[journal]{IEEEtran}
\usepackage{graphicx}
\begin{document}\end{document}"#,
        );
        let detector = JournalDetector::new();
        let report = detector.detect(&vfs);
        assert_eq!(report.selected_profile_id, "jos-paper-toml");
        assert!(report.confidence >= 0.80);
    }

    #[test]
    fn detect_acl_aclang() {
        let vfs = vfs_with_source(
            "main.tex",
            r#"\documentclass[aclang]{acl}
\usepackage{natbib}
\begin{document}\end{document}"#,
        );
        let detector = JournalDetector::new();
        let report = detector.detect(&vfs);
        assert_eq!(report.selected_profile_id, "tacl");
        assert!(report.confidence >= 0.80);
    }

    #[test]
    fn detect_cvpr_conference() {
        let vfs = vfs_with_source(
            "main.tex",
            r#"\documentclass[conference]{IEEEtran}
\usepackage{amsmath}
\cvprfinalcopy
\begin{document}\end{document}"#,
        );
        let detector = JournalDetector::new();
        let report = detector.detect(&vfs);
        assert_eq!(report.selected_profile_id, "cvpr");
        assert!(report.confidence >= 0.85);
    }

    #[test]
    fn detect_nature() {
        let vfs = vfs_with_source(
            "main.tex",
            r#"\documentclass{nature}
\bibliographystyle{naturemag}
\begin{document}\end{document}"#,
        );
        let detector = JournalDetector::new();
        let report = detector.detect(&vfs);
        assert_eq!(report.selected_profile_id, "nature");
        assert!(report.confidence >= 0.75);
    }

    #[test]
    fn detect_springer() {
        let vfs = vfs_with_source(
            "main.tex",
            r#"\documentclass{springer}
\usepackage{graphicx}
\institute{Test University}
\begin{document}\end{document}"#,
        );
        let detector = JournalDetector::new();
        let report = detector.detect(&vfs);
        assert_eq!(report.selected_profile_id, "springer");
        assert!(report.confidence >= 0.75);
    }

    #[test]
    fn detect_chinese_academic() {
        let vfs = vfs_with_source(
            "main.tex",
            r#"\documentclass{ctexart}
\usepackage{ctex}
\setCJKmainfont{SimSun}
\begin{document}\end{document}"#,
        );
        let detector = JournalDetector::new();
        let report = detector.detect(&vfs);
        assert_eq!(report.selected_profile_id, "chinese-academic");
        assert!(report.confidence >= 0.75);
    }

    #[test]
    fn detect_generic_article() {
        let vfs = vfs_with_source(
            "main.tex",
            r#"\documentclass{article}
\usepackage{amsmath}
\begin{document}\end{document}"#,
        );
        let detector = JournalDetector::new();
        let report = detector.detect(&vfs);
        // Falls back to generic-article since no strong signals match.
        assert!(
            report.selected_profile_id == "generic-article"
                || report.selected_profile_id == "generic"
        );
        assert!(!report.candidates.is_empty());
    }

    #[test]
    fn no_tex_source_falls_back_to_generic() {
        let vfs = vfs_with_source("readme.txt", "This is not a TeX file.");
        let detector = JournalDetector::new();
        let report = detector.detect(&vfs);
        assert!(
            report.selected_profile_id == "generic-article"
                || report.selected_profile_id == "generic"
        );
    }

    #[test]
    fn multiple_signals_accumulate() {
        let vfs = vfs_with_source(
            "main.tex",
            r#"\documentclass[conference]{IEEEtran}
\usepackage{amsmath}
\cvprfinalcopy
\confName{CVPR}
\confYear{2024}
\begin{document}\end{document}"#,
        );
        let detector = JournalDetector::new();
        let report = detector.detect(&vfs);
        // CVPR has documentclass + option + cvprfinalcopy = 0.70 + 0.20 + 0.10 = 1.00.
        assert_eq!(report.selected_profile_id, "cvpr");
        assert!(report.confidence >= 0.95);
    }

    #[test]
    fn comments_are_stripped_before_detection() {
        // The commented-out class should NOT be detected.
        let vfs = vfs_with_source(
            "main.tex",
            r#"\documentclass{article}
%\documentclass{journal}{IEEEtran}
\begin{document}\end{document}"#,
        );
        let detector = JournalDetector::new();
        let report = detector.detect(&vfs);
        // Only article class should be detected -> generic.
        assert!(
            report.selected_profile_id == "generic-article"
                || report.selected_profile_id == "generic"
        );
    }

    #[test]
    fn strip_comments_removes_percent() {
        let src = "line one\n%comment\nline two";
        let stripped = strip_comments(src);
        assert!(!stripped.contains("%comment"));
        assert!(stripped.contains("line one"));
        assert!(stripped.contains("line two"));
    }

    #[test]
    fn signal_kind_display() {
        assert_eq!(SignalKind::DocumentClass.to_string(), "documentclass");
        assert_eq!(SignalKind::Package.to_string(), "package");
    }

    #[test]
    fn diagnostic_level_serialization() {
        let d = JournalDiagnostic {
            level: DiagnosticLevel::Warning,
            code: "test".to_string(),
            message: "test message".to_string(),
        };
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains("Warning"));
    }

    #[test]
    fn journal_detection_report_serialization() {
        let vfs = vfs_with_source("main.tex", r"\documentclass{article}");
        let detector = JournalDetector::new();
        let report = detector.detect(&vfs);
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("selected_profile_id"));
        assert!(json.contains("confidence"));
        assert!(json.contains("candidates"));
    }

    #[test]
    fn journal_detection_includes_candidates() {
        let vfs = vfs_with_source(
            "main.tex",
            r"\documentclass{article}\begin{document}\end{document}",
        );
        let detector = JournalDetector::new();
        let report = detector.detect(&vfs);
        assert!(!report.candidates.is_empty());
        // Candidates should be sorted by confidence descending.
        for window in report.candidates.windows(2) {
            assert!(window[0].confidence >= window[1].confidence);
        }
    }
}
