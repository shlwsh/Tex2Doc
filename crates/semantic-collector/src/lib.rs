//! Semantic document collector types and traits for Tex2Doc.
//!
//! This crate defines the core data types and traits for semantic document
//! collection. Concrete implementations (rule-based, XeLaTeX, LuaTeX backends)
//! live in `doc-compiler-engine` which has access to the full compilation pipeline.

#![forbid(unsafe_code)]

mod reference_graph;
mod tex_utils;

use doc_semantic_ast::{Document, StandardDocument};
use doc_utils::ImageAssets;
use serde::{Deserialize, Serialize};

pub use reference_graph::{
    build_reference_graph, CitationReference, CrossReference, ReferenceGraph, ReferenceLabel,
    ReferenceOrigin, ReferenceSource, ReferenceTargetKind, UnresolvedReference,
};
pub use tex_utils::{
    is_tex_like_path, parse_semantic_events_jsonl, path_to_posix, scan_tex_commands,
    split_citation_keys, split_tex_name_list, strip_tex_comments, Command,
};

use thiserror::Error;

// ---------------------------------------------------------------------------
// Semantic event types
// ---------------------------------------------------------------------------

/// A semantic event emitted by a TeX compiler hook or rule-based scanner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
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
    #[serde(rename = "caption")]
    Caption {
        text: String,
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

/// Source location for a semantic event (v2 schema).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventSource {
    pub path: String,
    pub line: u32,
    pub column: Option<u32>,
}

/// Metadata for a v2 semantic event, including source location and macro name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticEventV2 {
    #[serde(rename = "schema", alias = "v")]
    pub schema: String,
    #[serde(flatten)]
    pub event: SemanticEvent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<EventSource>,
    #[serde(default, rename = "macro", skip_serializing_if = "Option::is_none")]
    pub macro_name: Option<String>,
}

impl SemanticEventV2 {
    pub fn into_event(self) -> SemanticEvent {
        self.event
    }
}

/// A byte range in the source file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceSpan {
    pub path: Option<String>,
    pub start: Option<u32>,
    pub end: Option<u32>,
}

// ---------------------------------------------------------------------------
// Layout types
// ---------------------------------------------------------------------------

// M4-2: Detailed layout types for node-based layout information
// These are stored in LayoutGraph.nodes as detailed node entries

/// A layout node representing a physical element in the rendered document.
/// M4-2: Extended with node-based layout information from LuaTeX node traversal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutNode {
    pub id: String,
    pub kind: String,
    pub page: Option<u32>,
    /// M4-2: Position information from node tree
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<i64>,
    /// M4-2: Font information for glyph nodes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_name: Option<String>,
    /// M4-2: Character information for glyph nodes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub char: Option<u32>,
    /// M4-2: Dimensions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<i64>,
}

impl LayoutNode {
    /// Create a basic layout node
    pub fn new(id: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            page: None,
            x: None,
            y: None,
            font_id: None,
            font_name: None,
            char: None,
            width: None,
            height: None,
            depth: None,
        }
    }

    /// Set position
    pub fn with_position(mut self, x: i64, y: i64) -> Self {
        self.x = Some(x);
        self.y = Some(y);
        self
    }

    /// Set font info
    pub fn with_font(mut self, font_id: u32, font_name: impl Into<String>) -> Self {
        self.font_id = Some(font_id);
        self.font_name = Some(font_name.into());
        self
    }

    /// Set character
    pub fn with_char(mut self, char: u32) -> Self {
        self.char = Some(char);
        self
    }

    /// Set dimensions
    pub fn with_dimensions(mut self, width: i64, height: i64, depth: i64) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self.depth = Some(depth);
        self
    }
}

/// A layout graph mapping semantic events to physical pages.
/// M4-2: Extended to include detailed node-based layout information.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LayoutGraph {
    pub nodes: Vec<LayoutNode>,
}

// ---------------------------------------------------------------------------
// Backend types
// ---------------------------------------------------------------------------

/// The available semantic collection backends.
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

/// Report describing how the backend was selected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendSelectionReport {
    pub requested: SemanticBackendKind,
    pub selected: SemanticBackendKind,
    pub fallback_from: Option<SemanticBackendKind>,
    pub reason: String,
}

impl Default for BackendSelectionReport {
    fn default() -> Self {
        Self {
            requested: SemanticBackendKind::default(),
            selected: SemanticBackendKind::default(),
            fallback_from: None,
            reason: String::new(),
        }
    }
}

impl BackendSelectionReport {
    pub fn new(
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

    pub fn fallback(
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

/// Whether a backend is available on the current system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendAvailability {
    pub available: bool,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Collector types
// ---------------------------------------------------------------------------

/// A sidecar artifact produced during collection.
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

/// A diagnostic message emitted during collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorDiagnostic {
    pub severity: CollectorDiagnosticSeverity,
    pub code: String,
    pub message: String,
}

impl CollectorDiagnostic {
    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: CollectorDiagnosticSeverity::Info,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: CollectorDiagnosticSeverity::Warning,
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CollectorDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// The output artifact of semantic collection.
#[derive(Debug, Clone)]
pub struct CollectedDocument {
    pub document: Document,
    pub standard_document: Option<StandardDocument>,
    pub image_assets: ImageAssets,
    pub events: Vec<SemanticEvent>,
    pub layout: Option<LayoutGraph>,
    pub diagnostics: Vec<CollectorDiagnostic>,
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

/// Alias for the collected document artifact.
pub type SemanticBackendArtifact = CollectedDocument;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error types for semantic collection.
#[derive(Debug, Error)]
pub enum CollectorError {
    #[error("I/O error: {0}")]
    Io(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("TeX runtime error: {0}")]
    Runtime(String),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_event_deserializes_from_json() {
        let json = r#"{"type":"heading","level":1,"text":"Test","label":null,"span":null}"#;
        let event: SemanticEvent = serde_json::from_str(json).unwrap();
        match event {
            SemanticEvent::Heading { level, text, .. } => {
                assert_eq!(level, 1);
                assert_eq!(text, "Test");
            }
            _ => panic!("expected Heading variant"),
        }
    }

    #[test]
    fn semantic_event_v2_deserializes() {
        let json = r#"{"schema":"semantic-event-v2","type":"heading","level":2,"text":"V2","label":null,"span":null,"source":{"path":"main.tex","line":5},"macro":"section"}"#;
        let v2: SemanticEventV2 = serde_json::from_str(json).unwrap();
        assert_eq!(v2.schema, "semantic-event-v2");
        assert!(v2.source.is_some());
        assert_eq!(v2.macro_name.as_deref(), Some("section"));
    }

    #[test]
    fn layout_graph_default_is_empty() {
        let graph = LayoutGraph::default();
        assert!(graph.nodes.is_empty());
    }

    #[test]
    fn backend_kind_ids() {
        assert_eq!(SemanticBackendKind::Auto.id(), "auto");
        assert_eq!(SemanticBackendKind::RuleBased.id(), "rule-based");
        assert_eq!(SemanticBackendKind::XeLaTeXHook.id(), "xelatex-hook");
        assert_eq!(SemanticBackendKind::LuaTeXNode.id(), "luatex-node");
    }

    #[test]
    fn build_sidecar_new() {
        let sidecar = BuildSidecar::new("test-kind", Some("/path/to/file"), "a description");
        assert_eq!(sidecar.kind, "test-kind");
        assert_eq!(sidecar.path.as_deref(), Some("/path/to/file"));
        assert_eq!(sidecar.description, "a description");
    }

    #[test]
    fn collector_diagnostic_helpers() {
        let info = CollectorDiagnostic::info("code", "message");
        assert!(matches!(info.severity, CollectorDiagnosticSeverity::Info));

        let warn = CollectorDiagnostic::warning("code", "warning");
        assert!(matches!(warn.severity, CollectorDiagnosticSeverity::Warning));
    }

    #[test]
    fn split_citation_keys_handles_braces() {
        let keys = split_citation_keys("smith2024, jones2023, {brown2022}");
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0], "smith2024");
        assert_eq!(keys[1], "jones2023");
        assert_eq!(keys[2], "{brown2022}"); // braces are NOT stripped by this function
    }

    #[test]
    fn strip_tex_comments_works() {
        let input = "hello % this is a comment\nworld % another\n";
        let output = strip_tex_comments(input);
        assert!(output.contains("hello"));
        assert!(output.contains("world"));
        assert!(!output.contains("this is a comment"));
        assert!(!output.contains("another"));
    }

    #[test]
    fn is_tex_like_path_works() {
        use std::path::Path;
        assert!(is_tex_like_path(Path::new("main.tex")));
        assert!(is_tex_like_path(Path::new("chapter1.tex")));
        assert!(!is_tex_like_path(Path::new("main.pdf")));
        assert!(!is_tex_like_path(Path::new("figure.png")));
    }

    #[test]
    fn reference_graph_default_is_empty() {
        let graph = ReferenceGraph::default();
        assert!(graph.labels.is_empty());
        assert!(graph.references.is_empty());
        assert!(graph.citations.is_empty());
        assert!(graph.unresolved_references.is_empty());
    }

    #[test]
    fn source_span_serialize_round_trip() {
        let span = SourceSpan {
            path: Some("main.tex".to_string()),
            start: Some(1),
            end: Some(10),
        };
        let json = serde_json::to_string(&span).unwrap();
        let round_trip: SourceSpan = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip.path, span.path);
        assert_eq!(round_trip.start, span.start);
        assert_eq!(round_trip.end, span.end);
    }

    #[test]
    fn event_source_default() {
        let src = EventSource {
            path: "test.tex".to_string(),
            line: 42,
            column: Some(10),
        };
        assert_eq!(src.path, "test.tex");
        assert_eq!(src.line, 42);
        assert!(src.column.is_some());
    }
}
