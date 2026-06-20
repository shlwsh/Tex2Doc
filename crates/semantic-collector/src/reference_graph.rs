//! Reference graph data types and construction logic.

use crate::SemanticEvent;
use doc_utils::VirtualFs;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A graph of labels, cross-references, and citations extracted from a TeX source.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ReferenceGraph {
    pub labels: Vec<ReferenceLabel>,
    pub references: Vec<CrossReference>,
    pub citations: Vec<CitationReference>,
    pub unresolved_references: Vec<UnresolvedReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReferenceLabel {
    pub key: String,
    pub kind: ReferenceTargetKind,
    pub number: Option<String>,
    pub source: ReferenceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrossReference {
    pub command: String,
    pub key: String,
    pub resolved: bool,
    pub target_kind: Option<ReferenceTargetKind>,
    pub rendered: Option<String>,
    pub source: ReferenceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CitationReference {
    pub command: String,
    pub keys: Vec<String>,
    pub source: ReferenceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnresolvedReference {
    pub command: String,
    pub key: String,
    pub source: ReferenceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReferenceSource {
    pub path: Option<String>,
    pub origin: ReferenceOrigin,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Hash)]
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

// ---------------------------------------------------------------------------
// Graph construction
// ---------------------------------------------------------------------------

/// Build a reference graph by scanning TeX sources and semantic events.
pub fn build_reference_graph(vfs: &VirtualFs, events: &[SemanticEvent]) -> ReferenceGraph {
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
        .filter(|path| super::is_tex_like_path(path))
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
        let text = super::strip_tex_comments(raw);
        let source = ReferenceSource {
            path: Some(path_to_posix(&path)),
            origin: ReferenceOrigin::SourceScan,
        };

        for command in super::scan_tex_commands(&text, &["label"]) {
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

        for command in super::scan_tex_commands(&text, &["autoref", "eqref", "ref"]) {
            let key = command.argument.trim();
            if key.is_empty() {
                continue;
            }
            graph.references.push(CrossReference {
                command: command.name.clone(),
                key: key.to_string(),
                resolved: false,
                target_kind: None,
                rendered: None,
                source: source.clone(),
            });
        }

        for command in super::scan_tex_commands(
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
            let keys = super::split_citation_keys(&command.argument);
            if keys.is_empty() {
                continue;
            }
            graph.citations.push(CitationReference {
                command: command.name.clone(),
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
            SemanticEvent::Heading { label: Some(label), .. } => {
                add_event_label(graph, label_keys, label, ReferenceTargetKind::Heading, source.clone())
            }
            SemanticEvent::Figure { label: Some(label), .. } => {
                add_event_label(graph, label_keys, label, ReferenceTargetKind::Figure, source.clone())
            }
            SemanticEvent::Table { label: Some(label), .. } => {
                add_event_label(graph, label_keys, label, ReferenceTargetKind::Table, source.clone())
            }
            SemanticEvent::Equation { label: Some(label), .. } => {
                add_event_label(graph, label_keys, label, ReferenceTargetKind::Equation, source.clone())
            }
            SemanticEvent::Label { key, .. } => {
                add_event_label(
                    graph,
                    label_keys,
                    key,
                    reference_kind_from_key(key),
                    source.clone(),
                );
            }
            SemanticEvent::Reference { kind, key, .. } => {
                graph.references.push(CrossReference {
                    command: kind.clone(),
                    key: key.clone(),
                    resolved: false,
                    target_kind: None,
                    rendered: None,
                    source: source.clone(),
                });
            }
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

fn path_to_posix(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_graph_default_is_empty() {
        let graph = ReferenceGraph::default();
        assert!(graph.labels.is_empty());
        assert!(graph.references.is_empty());
        assert!(graph.citations.is_empty());
        assert!(graph.unresolved_references.is_empty());
    }

    #[test]
    fn reference_target_kind_serialization() {
        let kind = ReferenceTargetKind::Figure;
        let json = serde_json::to_string(&kind).unwrap();
        let round_trip: ReferenceTargetKind = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip, kind);
    }

    #[test]
    fn reference_origin_default() {
        let origin = ReferenceOrigin::SourceScan;
        assert!(matches!(origin, ReferenceOrigin::SourceScan));
    }

    #[test]
    fn cross_reference_serialization() {
        let cr = CrossReference {
            command: "ref".to_string(),
            key: "fig:1".to_string(),
            resolved: true,
            target_kind: Some(ReferenceTargetKind::Figure),
            rendered: Some("1".to_string()),
            source: ReferenceSource {
                path: Some("main.tex".to_string()),
                origin: ReferenceOrigin::SourceScan,
            },
        };
        let json = serde_json::to_string(&cr).unwrap();
        let round_trip: CrossReference = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip.command, "ref");
        assert!(round_trip.resolved);
    }

    #[test]
    fn unresolved_reference_serialization() {
        let ur = UnresolvedReference {
            command: "ref".to_string(),
            key: "missing".to_string(),
            source: ReferenceSource {
                path: Some("main.tex".to_string()),
                origin: ReferenceOrigin::SourceScan,
            },
        };
        let json = serde_json::to_string(&ur).unwrap();
        let round_trip: UnresolvedReference = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip.key, "missing");
    }

    #[test]
    fn reference_kind_from_key_prefixes() {
        assert!(matches!(reference_kind_from_key("fig:1"), ReferenceTargetKind::Figure));
        assert!(matches!(reference_kind_from_key("tab:1"), ReferenceTargetKind::Table));
        assert!(matches!(reference_kind_from_key("eq:1"), ReferenceTargetKind::Equation));
        assert!(matches!(reference_kind_from_key("alg:demo"), ReferenceTargetKind::Algorithm));
        assert!(matches!(reference_kind_from_key("unknown"), ReferenceTargetKind::Unknown));
    }

    #[test]
    fn next_reference_number_equations_have_parens() {
        let mut numbering = HashMap::new();
        let num = next_reference_number(ReferenceTargetKind::Equation, &mut numbering);
        assert_eq!(num, Some("(1)".to_string()));
    }

    #[test]
    fn next_reference_number_unknown_returns_none() {
        let mut numbering = HashMap::new();
        let num = next_reference_number(ReferenceTargetKind::Unknown, &mut numbering);
        assert!(num.is_none());
    }
}
