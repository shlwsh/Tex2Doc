//! XDV → LayoutGraph conversion layer.
//!
//! Transforms parsed XDV bytecode into a layout graph that maps semantic elements
//! to their physical page positions, enabling cross-reference page number resolution
//! and precise figure/table placement in the DOCX output.
//!
//! ## Design
//!
//! Each `LayoutNode` in the output corresponds to one page in the XDV document.
//! The `kind` field captures the dominant content type on that page:
//!   - `"text"` — primarily text/paragraph content
//!   - `"figure"` — contains at least one figure element (via specials)
//!   - `"table"` — contains at least one table element (via specials)
//!   - `"equation"` — contains a display equation (via specials)
//!   - `"mixed"` — multiple content types
//!
//! Future extensions may add per-element granularity by correlating XDV specials
//! (e.g., `pdf:annot`, `xdv:float`) with page positions.

use doc_semantic_collector::LayoutGraph as CollectorLayoutGraph;
use doc_semantic_collector::LayoutNode as CollectorLayoutNode;

use crate::model::{XdvCommand, XdvDocument};

/// A layout node extracted from XDV bytecode analysis.
#[derive(Debug, Clone)]
pub struct XdvLayoutNode {
    pub id: String,
    pub kind: XdvPageKind,
    pub page: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XdvPageKind {
    Text,
    Figure,
    Table,
    Equation,
    Mixed,
}

impl XdvPageKind {
    fn merge(&mut self, other: XdvPageKind) {
        if *self == other {
            return;
        }
        if *self == XdvPageKind::Text {
            *self = other;
        } else if other != XdvPageKind::Text {
            *self = XdvPageKind::Mixed;
        }
    }
}

/// Convert an `XdvDocument` into a sequence of `XdvLayoutNode`, one per page.
///
/// This is the public entry point for the XDV → LayoutGraph pipeline.
/// Each page becomes one layout node with an auto-detected `kind`.
pub fn xdv_to_layout_nodes(xdv: &XdvDocument) -> Vec<XdvLayoutNode> {
    xdv.pages
        .iter()
        .enumerate()
        .map(|(idx, page)| {
            let kind = detect_page_kind(page);
            XdvLayoutNode {
                id: format!("page_{}", idx),
                kind,
                page: idx as u32,
            }
        })
        .collect()
}

/// Detect the dominant content kind of a page based on its commands.
fn detect_page_kind(page: &crate::model::XdvPage) -> XdvPageKind {
    let mut kind = XdvPageKind::Text;
    for cmd in &page.commands {
        match cmd {
            XdvCommand::Special { data } => {
                let data_str = String::from_utf8_lossy(data);
                if data_str.contains("xdv:float")
                    || data_str.contains("pdf:figure")
                    || data_str.contains("xdv:figure")
                {
                    kind.merge(XdvPageKind::Figure);
                } else if data_str.contains("pdf:table")
                    || data_str.contains("xdv:table")
                    || data_str.contains("tab:*")
                {
                    kind.merge(XdvPageKind::Table);
                } else if data_str.contains("pdf:equation")
                    || data_str.contains("xdv:equation")
                    || data_str.contains("begin_math")
                {
                    kind.merge(XdvPageKind::Equation);
                }
            }
            XdvCommand::SetRule { .. } | XdvCommand::PutRule { .. } => {
                let h = match cmd {
                    XdvCommand::SetRule { height, .. } => *height,
                    XdvCommand::PutRule { height, .. } => *height,
                    _ => 0,
                };
                let w = match cmd {
                    XdvCommand::SetRule { width, .. } => *width,
                    XdvCommand::PutRule { width, .. } => *width,
                    _ => 0,
                };
                // Short-thick rules are likely table borders; tall-narrow are likely separators
                if h > w * 2 {
                    kind.merge(XdvPageKind::Table);
                }
            }
            _ => {}
        }
    }
    kind
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::XdvPage;

    fn make_page(commands: Vec<XdvCommand>) -> XdvPage {
        XdvPage { number: 0, commands }
    }

    #[test]
    fn empty_page_is_text() {
        let page = make_page(vec![]);
        assert_eq!(detect_page_kind(&page), XdvPageKind::Text);
    }

    #[test]
    fn figure_special_detected() {
        let page = make_page(vec![XdvCommand::Special {
            data: b"pdf:figure file=fig1.png".to_vec(),
        }]);
        assert_eq!(detect_page_kind(&page), XdvPageKind::Figure);
    }

    #[test]
    fn table_special_detected() {
        let page = make_page(vec![XdvCommand::Special {
            data: b"pdf:table label=tab:1".to_vec(),
        }]);
        assert_eq!(detect_page_kind(&page), XdvPageKind::Table);
    }

    #[test]
    fn mixed_page() {
        let page = make_page(vec![
            XdvCommand::Special { data: b"pdf:figure".to_vec() },
            XdvCommand::Special { data: b"pdf:table".to_vec() },
        ]);
        assert_eq!(detect_page_kind(&page), XdvPageKind::Mixed);
    }

    #[test]
    fn set_rule_short_thick_is_table() {
        let page = make_page(vec![XdvCommand::SetRule { height: 100, width: 10 }]);
        assert_eq!(detect_page_kind(&page), XdvPageKind::Table);
    }

    #[test]
    fn xdv_to_layout_nodes_produces_one_per_page() {
        let xdv = XdvDocument {
            pages: vec![
                make_page(vec![]),
                make_page(vec![XdvCommand::Special {
                    data: b"pdf:figure".to_vec(),
                }]),
                make_page(vec![]),
            ],
            ..Default::default()
        };
        let nodes = xdv_to_layout_nodes(&xdv);
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0].kind, XdvPageKind::Text);
        assert_eq!(nodes[1].kind, XdvPageKind::Figure);
        assert_eq!(nodes[2].kind, XdvPageKind::Text);
    }
}

/// Convert a sequence of `XdvLayoutNode` into a `doc_semantic_collector::LayoutGraph`.
///
/// This bridges the XDV layer to the collector layer, enabling LayoutGraph to be
/// passed through the `CollectedDocument.layout` field into the compile pipeline.
pub fn to_collector_layout_graph(nodes: Vec<XdvLayoutNode>) -> CollectorLayoutGraph {
    CollectorLayoutGraph {
        nodes: nodes
            .into_iter()
            .map(|n| CollectorLayoutNode {
                id: n.id,
                kind: match n.kind {
                    XdvPageKind::Text => "text".to_string(),
                    XdvPageKind::Figure => "figure".to_string(),
                    XdvPageKind::Table => "table".to_string(),
                    XdvPageKind::Equation => "equation".to_string(),
                    XdvPageKind::Mixed => "mixed".to_string(),
                },
                page: Some(n.page),
            })
            .collect(),
    }
}
