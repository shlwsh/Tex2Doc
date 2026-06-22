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
//!
//! ## M4-2: Node Tree Integration
//!
//! The `LayoutGraph` can also be populated from detailed node tree entries
//! collected by the LuaTeX hook. Use `node_entry_to_layout_node()` to convert
//! individual node entries into `LayoutNode`s for detailed layout analysis.

use doc_semantic_collector::LayoutGraph as CollectorLayoutGraph;
use doc_semantic_collector::LayoutNode as CollectorLayoutNode;

use std::collections::HashMap;

use crate::model::{FontDefExt, XdvCommand, XdvDocument};

/// A glyph resolved from a native font (XeTeX/OpenType).
#[derive(Debug, Clone)]
pub struct NativeGlyphInfo {
    /// Font ID from the XDV stream.
    pub font_id: u32,
    /// Font name (from FontDefExt).
    pub font_name: String,
    /// Glyph ID within the font.
    pub glyph_id: u32,
    /// Resolved Unicode code point (None if unmapped).
    pub unicode_cp: Option<u32>,
    /// Character width in DVI units.
    pub width: i32,
}

/// A native node (whitespace, boundary, etc.) from XeTeX.
#[derive(Debug, Clone)]
pub struct NativeNodeInfo {
    /// Node type discriminator.
    pub node_type: u8,
    /// Width in DVI units.
    pub width: i32,
    /// Human-readable type name.
    pub kind: &'static str,
}

impl NativeNodeInfo {
    /// Maps XeTeX native node types to descriptive names.
    /// 0 = discretionary (hyphen), 1 = ligature disc, 2 = math disc
    pub fn from_type(t: u8) -> Self {
        let kind = match t {
            0 => "discretionary",
            1 => "ligature_boundary",
            2 => "math_boundary",
            3 => "insertion",
            4 => "inserted",
            5 => "pos",
            _ => "unknown",
        };
        Self { node_type: t, width: 0, kind }
    }
}

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

/// Extended result of XDV → LayoutGraph conversion.
pub struct XdvLayoutResult {
    /// Layout nodes, one per page.
    pub nodes: Vec<XdvLayoutNode>,
    /// Font ID → FontDefExt map built from commands.
    pub font_map: HashMap<u32, FontDefExt>,
    /// All native glyphs encountered.
    pub native_glyphs: Vec<NativeGlyphInfo>,
    /// All native nodes encountered.
    pub native_nodes: Vec<NativeNodeInfo>,
}

/// Convert an `XdvDocument` into layout nodes + extended metadata.
///
/// This is the extended entry point that also builds a font map and resolves
/// native glyphs to Unicode. Use `xdv_to_layout_nodes()` when only page-level
/// kind detection is needed.
pub fn xdv_to_layout_full(xdv: &XdvDocument) -> XdvLayoutResult {
    let mut font_map: HashMap<u32, FontDefExt> = HashMap::new();
    let mut native_glyphs: Vec<NativeGlyphInfo> = Vec::new();
    let mut native_nodes: Vec<NativeNodeInfo> = Vec::new();

    // First, populate font_map from doc.ext_fonts (canonical source)
    for ext in &xdv.ext_fonts {
        font_map.insert(ext.id, ext.clone());
    }

    // Then scan commands to catch any FontDefExt that only appear in-page
    for page in &xdv.pages {
        for cmd in &page.commands {
            match cmd {
                XdvCommand::FontDefExt(ext) => {
                    font_map.entry(ext.id).or_insert_with(|| ext.clone());
                }
                XdvCommand::NativeGlyph { font_id, glyph_id, width, special: _ } => {
                    let font_name = font_map
                        .get(font_id)
                        .map(|f| f.name.clone())
                        .unwrap_or_else(|| format!("font{}", font_id));
                    let unicode_cp = resolve_native_glyph_to_unicode(*font_id, *glyph_id, &font_map);
                    native_glyphs.push(NativeGlyphInfo {
                        font_id: *font_id,
                        font_name,
                        glyph_id: *glyph_id,
                        unicode_cp,
                        width: *width,
                    });
                }
                XdvCommand::NativeNode { node_type, width, special: _ } => {
                    let mut info = NativeNodeInfo::from_type(*node_type);
                    info.width = *width;
                    native_nodes.push(info);
                }
                _ => {}
            }
        }
    }

    let nodes = xdv_to_layout_nodes(xdv);
    XdvLayoutResult { nodes, font_map, native_glyphs, native_nodes }
}

/// Try to resolve a native glyph to a Unicode code point.
///
/// For XeTeX native fonts, the glyph_id may be a Unicode code point directly,
/// or may need font-specific mapping. This function tries the most common cases:
/// 1. If the font's flags indicate "glyph_id = Unicode" (flag bit 19 set), use glyph_id directly
/// 2. For ASCII-range glyphs (< 128), use the glyph_id as the code point
fn resolve_native_glyph_to_unicode(
    font_id: u32,
    glyph_id: u32,
    font_map: &HashMap<u32, FontDefExt>,
) -> Option<u32> {
    // XeTeX flag bit 19: "Treat glyph IDs as Unicode code points"
    const UNICODE_GID_FLAG: u32 = 1 << 19;

    let font = font_map.get(&font_id)?;
    if font.flags & UNICODE_GID_FLAG != 0 {
        // glyph_id is already a Unicode code point
        return Some(glyph_id);
    }

    // For common ASCII ranges, glyph_id often maps directly
    if glyph_id < 128 {
        return Some(glyph_id);
    }

    // For other ranges, we can't reliably map without the font's cmap table
    None
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

    // ─── NativeFont extended tests ─────────────────────────────────────────

    fn make_ext_font(id: u32, name: &str, flags: u32) -> FontDefExt {
        FontDefExt {
            id,
            checksum: 0,
            scale: 1000,
            design_size: 10,
            area: String::new(),
            name: name.to_string(),
            flags,
            char_count: 256,
            native_data: vec![],
        }
    }

    #[test]
    fn xdv_to_layout_full_registers_ext_fonts() {
        let ext = make_ext_font(5, "Latin Modern Roman", 0);
        let mut doc = XdvDocument::default();
        doc.ext_fonts.push(ext.clone());
        doc.pages.push(XdvPage {
            number: 0,
            commands: vec![
                XdvCommand::FontDefExt(ext),
            ],
        });
        let result = xdv_to_layout_full(&doc);
        assert_eq!(result.font_map.len(), 1);
        assert_eq!(result.font_map.get(&5).unwrap().name, "Latin Modern Roman");
    }

    #[test]
    fn xdv_to_layout_full_captures_native_glyph() {
        let ext = make_ext_font(1, "TestFont", 0);
        let mut doc = XdvDocument::default();
        doc.ext_fonts.push(ext.clone());
        doc.pages.push(XdvPage {
            number: 0,
            commands: vec![
                XdvCommand::FontDefExt(ext),
                XdvCommand::NativeGlyph {
                    font_id: 1,
                    glyph_id: 65,
                    width: 10,
                    special: vec![],
                },
            ],
        });
        let result = xdv_to_layout_full(&doc);
        assert_eq!(result.native_glyphs.len(), 1);
        assert_eq!(result.native_glyphs[0].font_id, 1);
        assert_eq!(result.native_glyphs[0].glyph_id, 65);
        assert_eq!(result.native_glyphs[0].width, 10);
    }

    #[test]
    fn native_glyph_ascii_resolves_directly() {
        let ext = make_ext_font(1, "TestFont", 0);
        let doc = XdvDocument {
            ext_fonts: vec![ext],
            pages: vec![XdvPage {
                number: 0,
                commands: vec![
                    XdvCommand::NativeGlyph {
                        font_id: 1,
                        glyph_id: 65, // 'A'
                        width: 10,
                        special: vec![],
                    },
                ],
            }],
            ..Default::default()
        };
        let result = xdv_to_layout_full(&doc);
        assert_eq!(result.native_glyphs.len(), 1);
        assert_eq!(result.native_glyphs[0].unicode_cp, Some(65));
    }

    #[test]
    fn native_glyph_unicode_flag_uses_glyph_id() {
        // Flag bit 19 set means glyph_id IS the Unicode code point
        let ext = make_ext_font(2, "UnicodeFont", 1 << 19);
        let doc = XdvDocument {
            ext_fonts: vec![ext],
            pages: vec![XdvPage {
                number: 0,
                commands: vec![
                    XdvCommand::NativeGlyph {
                        font_id: 2,
                        glyph_id: 0x4E2D, // U+4E2D '中'
                        width: 20,
                        special: vec![],
                    },
                ],
            }],
            ..Default::default()
        };
        let result = xdv_to_layout_full(&doc);
        assert_eq!(result.native_glyphs.len(), 1);
        assert_eq!(result.native_glyphs[0].unicode_cp, Some(0x4E2D));
    }

    #[test]
    fn native_glyph_high_code_point_unmapped_without_flag() {
        let ext = make_ext_font(3, "LegacyFont", 0);
        let doc = XdvDocument {
            ext_fonts: vec![ext],
            pages: vec![XdvPage {
                number: 0,
                commands: vec![
                    XdvCommand::NativeGlyph {
                        font_id: 3,
                        glyph_id: 0xC000, // high glyph ID, no direct mapping
                        width: 15,
                        special: vec![],
                    },
                ],
            }],
            ..Default::default()
        };
        let result = xdv_to_layout_full(&doc);
        assert_eq!(result.native_glyphs.len(), 1);
        assert_eq!(result.native_glyphs[0].unicode_cp, None);
    }

    #[test]
    fn native_node_discriminates_types() {
        let doc = XdvDocument {
            pages: vec![XdvPage {
                number: 0,
                commands: vec![
                    XdvCommand::NativeNode { node_type: 0, width: 0, special: vec![] },
                    XdvCommand::NativeNode { node_type: 1, width: 0, special: vec![] },
                    XdvCommand::NativeNode { node_type: 2, width: 0, special: vec![] },
                    XdvCommand::NativeNode { node_type: 99, width: 0, special: vec![] },
                ],
            }],
            ..Default::default()
        };
        let result = xdv_to_layout_full(&doc);
        assert_eq!(result.native_nodes.len(), 4);
        assert_eq!(result.native_nodes[0].kind, "discretionary");
        assert_eq!(result.native_nodes[1].kind, "ligature_boundary");
        assert_eq!(result.native_nodes[2].kind, "math_boundary");
        assert_eq!(result.native_nodes[3].kind, "unknown");
    }

    #[test]
    fn xdv_to_layout_full_unknown_font_uses_fallback_name() {
        let doc = XdvDocument {
            pages: vec![XdvPage {
                number: 0,
                commands: vec![
                    XdvCommand::NativeGlyph {
                        font_id: 999,
                        glyph_id: 65,
                        width: 10,
                        special: vec![],
                    },
                ],
            }],
            ..Default::default()
        };
        let result = xdv_to_layout_full(&doc);
        assert_eq!(result.native_glyphs.len(), 1);
        assert_eq!(result.native_glyphs[0].font_name, "font999");
    }

    #[test]
    fn xdv_to_layout_full_returns_simple_nodes() {
        let doc = XdvDocument {
            pages: vec![
                make_page(vec![XdvCommand::Special {
                    data: b"pdf:figure".to_vec(),
                }]),
                make_page(vec![]),
            ],
            ..Default::default()
        };
        let result = xdv_to_layout_full(&doc);
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.nodes[0].kind, XdvPageKind::Figure);
        assert_eq!(result.nodes[1].kind, XdvPageKind::Text);
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
                // M4-2: Extended fields default to None for XDV-based layout
                x: None,
                y: None,
                font_id: None,
                font_name: None,
                char: None,
                width: None,
                height: None,
                depth: None,
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// M4-2: Node Tree to LayoutGraph Conversion
// ---------------------------------------------------------------------------

/// NodeEntry type for deserializing node tree JSONL entries from LuaTeX hook.
/// This is a simplified version that matches the compiler-engine's NodeEntry.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeEntry {
    Glyph {
        subtype: u32,
        x: i64,
        y: i64,
        char: u32,
        #[serde(default)]
        char_str: Option<String>,
        font_id: u32,
        #[serde(default)]
        font_name: Option<String>,
        width: i64,
        height: i64,
        depth: i64,
    },
    Hlist {
        subtype: u32,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        depth: i64,
        #[serde(default)]
        head_id: Option<u32>,
    },
    Vlist {
        subtype: u32,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        depth: i64,
        #[serde(default)]
        head_id: Option<u32>,
    },
    Glue {
        subtype: u32,
        x: i64,
        y: i64,
        width: i64,
        #[serde(default)]
        stretch: Option<i64>,
        #[serde(default)]
        shrink: Option<i64>,
        #[serde(default)]
        stretch_order: Option<u32>,
        #[serde(default)]
        shrink_order: Option<u32>,
    },
    Rule {
        subtype: u32,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        depth: i64,
    },
    Kern {
        subtype: u32,
        x: i64,
        y: i64,
        kern: i64,
    },
    Penalty {
        subtype: u32,
        x: i64,
        y: i64,
        penalty: i64,
    },
    LocalPar {
        subtype: u32,
        x: i64,
        y: i64,
    },
    Dir {
        subtype: u32,
        x: i64,
        y: i64,
        #[serde(default)]
        dir: Option<String>,
    },
    #[serde(rename = "node_tree")]
    NodeTree {
        hlist: u32,
        vlist: u32,
        glyph: u32,
        glue: u32,
        rule: u32,
        #[serde(default)]
        page: Option<u32>,
    },
}

/// Convert a single node entry into a CollectorLayoutNode.
pub fn node_entry_to_layout_node(entry: &NodeEntry) -> Option<CollectorLayoutNode> {
    let (id, kind, x, y, width, height, depth, font_id, font_name, char) = match entry {
        NodeEntry::Glyph {
            subtype: _,
            x,
            y,
            char,
            char_str: _,
            font_id,
            font_name,
            width,
            height,
            depth,
        } => (
            format!("glyph_{}_{}", x, y),
            "glyph".to_string(),
            Some(*x),
            Some(*y),
            Some(*width),
            Some(*height),
            Some(*depth),
            Some(*font_id),
            font_name.clone(),
            Some(*char),
        ),
        NodeEntry::Hlist {
            subtype: _,
            x,
            y,
            width,
            height,
            depth,
            head_id: _,
        } => (
            format!("hlist_{}_{}", x, y),
            "hlist".to_string(),
            Some(*x),
            Some(*y),
            Some(*width),
            Some(*height),
            Some(*depth),
            None,
            None,
            None,
        ),
        NodeEntry::Vlist {
            subtype: _,
            x,
            y,
            width,
            height,
            depth,
            head_id: _,
        } => (
            format!("vlist_{}_{}", x, y),
            "vlist".to_string(),
            Some(*x),
            Some(*y),
            Some(*width),
            Some(*height),
            Some(*depth),
            None,
            None,
            None,
        ),
        NodeEntry::Glue {
            subtype: _,
            x,
            y,
            width,
            stretch: _,
            shrink: _,
            stretch_order: _,
            shrink_order: _,
        } => (
            format!("glue_{}_{}", x, y),
            "glue".to_string(),
            Some(*x),
            Some(*y),
            Some(*width),
            None,
            None,
            None,
            None,
            None,
        ),
        NodeEntry::Rule {
            subtype: _,
            x,
            y,
            width,
            height,
            depth,
        } => (
            format!("rule_{}_{}", x, y),
            "rule".to_string(),
            Some(*x),
            Some(*y),
            Some(*width),
            Some(*height),
            Some(*depth),
            None,
            None,
            None,
        ),
        NodeEntry::Kern {
            subtype: _,
            x,
            y,
            kern,
        } => (
            format!("kern_{}_{}", x, y),
            "kern".to_string(),
            Some(*x),
            Some(*y),
            Some(*kern),
            None,
            None,
            None,
            None,
            None,
        ),
        NodeEntry::Penalty {
            subtype: _,
            x,
            y,
            penalty: _,
        } => (
            format!("penalty_{}_{}", x, y),
            "penalty".to_string(),
            Some(*x),
            Some(*y),
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        NodeEntry::LocalPar {
            subtype: _,
            x,
            y,
        } => (
            format!("local_par_{}_{}", x, y),
            "local_par".to_string(),
            Some(*x),
            Some(*y),
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        NodeEntry::Dir {
            subtype: _,
            x,
            y,
            dir: _,
        } => (
            format!("dir_{}_{}", x, y),
            "dir".to_string(),
            Some(*x),
            Some(*y),
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        NodeEntry::NodeTree { .. } => {
            return None;
        }
    };

    Some(CollectorLayoutNode {
        id,
        kind,
        page: None,
        x,
        y,
        font_id,
        font_name,
        char,
        width,
        height,
        depth,
    })
}

/// Convert a sequence of node entries into a CollectorLayoutGraph.
///
/// This function filters out summary entries and converts each detailed node
/// entry into a CollectorLayoutNode, preserving position, font, and dimension info.
pub fn node_entries_to_layout_graph(
    entries: &[NodeEntry],
    page: Option<u32>,
) -> CollectorLayoutGraph {
    let nodes: Vec<CollectorLayoutNode> = entries
        .iter()
        .filter_map(|entry| {
            let mut node = node_entry_to_layout_node(entry)?;
            node.page = page;
            Some(node)
        })
        .collect();

    CollectorLayoutGraph { nodes }
}

/// Parse node tree JSONL content and convert to a LayoutGraph.
///
/// This function takes the raw JSONL content from the LuaTeX hook's node tree
/// sidecar and converts it into a CollectorLayoutGraph for integration with
/// the rest of the pipeline.
pub fn parse_node_tree_jsonl(content: &str) -> Result<CollectorLayoutGraph, String> {
    let mut entries = Vec::new();
    let mut current_page = 1u32;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        match serde_json::from_str::<NodeEntry>(trimmed) {
            Ok(entry) => {
                // Track page from summary entries
                if let NodeEntry::NodeTree { page, .. } = &entry {
                    if let Some(p) = page {
                        current_page = *p;
                    }
                }
                entries.push(entry);
            }
            Err(e) => {
                return Err(format!("failed to parse node entry at line: {}", e));
            }
        }
    }

    Ok(node_entries_to_layout_graph(&entries, Some(current_page)))
}

#[cfg(test)]
mod node_tree_tests {
    use super::*;

    #[test]
    fn node_entry_glyph_deserializes() {
        let json = r#"{"type":"glyph","subtype":0,"x":100,"y":200,"char":65,"char_str":"A","font_id":1,"font_name":"Latin Modern Roman","width":10,"height":8,"depth":2}"#;
        let entry: NodeEntry = serde_json::from_str(json).unwrap();
        match entry {
            NodeEntry::Glyph {
                x,
                y,
                char,
                font_id,
                width,
                ..
            } => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
                assert_eq!(char, 65);
                assert_eq!(font_id, 1);
                assert_eq!(width, 10);
            }
            _ => panic!("expected Glyph variant"),
        }
    }

    #[test]
    fn node_entry_hlist_deserializes() {
        let json = r#"{"type":"hlist","subtype":0,"x":0,"y":0,"width":500,"height":12,"depth":3}"#;
        let entry: NodeEntry = serde_json::from_str(json).unwrap();
        match entry {
            NodeEntry::Hlist {
                x, y, width, height, depth, ..
            } => {
                assert_eq!(x, 0);
                assert_eq!(y, 0);
                assert_eq!(width, 500);
                assert_eq!(height, 12);
                assert_eq!(depth, 3);
            }
            _ => panic!("expected Hlist variant"),
        }
    }

    #[test]
    fn node_entry_glue_deserializes() {
        let json = r#"{"type":"glue","subtype":0,"x":100,"y":0,"width":200,"stretch":100,"shrink":50}"#;
        let entry: NodeEntry = serde_json::from_str(json).unwrap();
        match entry {
            NodeEntry::Glue {
                x, y, width, stretch, shrink, ..
            } => {
                assert_eq!(x, 100);
                assert_eq!(y, 0);
                assert_eq!(width, 200);
                assert_eq!(stretch, Some(100));
                assert_eq!(shrink, Some(50));
            }
            _ => panic!("expected Glue variant"),
        }
    }

    #[test]
    fn node_entry_rule_deserializes() {
        let json = r#"{"type":"rule","subtype":0,"x":0,"y":0,"width":500,"height":1,"depth":0}"#;
        let entry: NodeEntry = serde_json::from_str(json).unwrap();
        match entry {
            NodeEntry::Rule { x, y, width, height, depth, .. } => {
                assert_eq!(x, 0);
                assert_eq!(y, 0);
                assert_eq!(width, 500);
                assert_eq!(height, 1);
                assert_eq!(depth, 0);
            }
            _ => panic!("expected Rule variant"),
        }
    }

    #[test]
    fn node_entry_summary_deserializes() {
        let json = r#"{"type":"node_tree","hlist":2,"vlist":1,"glyph":42,"glue":15,"rule":3,"page":1}"#;
        let entry: NodeEntry = serde_json::from_str(json).unwrap();
        match entry {
            NodeEntry::NodeTree {
                hlist,
                vlist,
                glyph,
                glue,
                rule,
                page,
            } => {
                assert_eq!(hlist, 2);
                assert_eq!(vlist, 1);
                assert_eq!(glyph, 42);
                assert_eq!(glue, 15);
                assert_eq!(rule, 3);
                assert_eq!(page, Some(1));
            }
            _ => panic!("expected NodeTree variant"),
        }
    }

    #[test]
    fn node_entry_kern_deserializes() {
        let json = r#"{"type":"kern","subtype":0,"x":100,"y":0,"kern":50}"#;
        let entry: NodeEntry = serde_json::from_str(json).unwrap();
        match entry {
            NodeEntry::Kern { x, y, kern, .. } => {
                assert_eq!(x, 100);
                assert_eq!(y, 0);
                assert_eq!(kern, 50);
            }
            _ => panic!("expected Kern variant"),
        }
    }

    #[test]
    fn node_entry_penalty_deserializes() {
        let json = r#"{"type":"penalty","subtype":0,"x":200,"y":0,"penalty":100}"#;
        let entry: NodeEntry = serde_json::from_str(json).unwrap();
        match entry {
            NodeEntry::Penalty { x, y, penalty, .. } => {
                assert_eq!(x, 200);
                assert_eq!(y, 0);
                assert_eq!(penalty, 100);
            }
            _ => panic!("expected Penalty variant"),
        }
    }

    #[test]
    fn node_entry_dir_deserializes() {
        let json = r#"{"type":"dir","subtype":0,"x":0,"y":0,"dir":"TRT"}"#;
        let entry: NodeEntry = serde_json::from_str(json).unwrap();
        match entry {
            NodeEntry::Dir { x, y, dir, .. } => {
                assert_eq!(x, 0);
                assert_eq!(y, 0);
                assert_eq!(dir, Some("TRT".to_string()));
            }
            _ => panic!("expected Dir variant"),
        }
    }

    #[test]
    fn node_entry_local_par_deserializes() {
        let json = r#"{"type":"local_par","subtype":0,"x":0,"y":0}"#;
        let entry: NodeEntry = serde_json::from_str(json).unwrap();
        match entry {
            NodeEntry::LocalPar { x, y, .. } => {
                assert_eq!(x, 0);
                assert_eq!(y, 0);
            }
            _ => panic!("expected LocalPar variant"),
        }
    }

    #[test]
    fn glyph_to_layout_node() {
        let json = r#"{"type":"glyph","subtype":0,"x":100,"y":200,"char":65,"font_id":1,"font_name":"TestFont","width":10,"height":8,"depth":2}"#;
        let entry: NodeEntry = serde_json::from_str(json).unwrap();
        let node = node_entry_to_layout_node(&entry).unwrap();

        assert!(node.id.contains("glyph"));
        assert_eq!(node.kind, "glyph");
        assert_eq!(node.x, Some(100));
        assert_eq!(node.y, Some(200));
        assert_eq!(node.font_id, Some(1));
        assert_eq!(node.font_name, Some("TestFont".to_string()));
        assert_eq!(node.char, Some(65));
        assert_eq!(node.width, Some(10));
        assert_eq!(node.height, Some(8));
        assert_eq!(node.depth, Some(2));
    }

    #[test]
    fn hlist_to_layout_node() {
        let json = r#"{"type":"hlist","subtype":0,"x":0,"y":100,"width":500,"height":12,"depth":2}"#;
        let entry: NodeEntry = serde_json::from_str(json).unwrap();
        let node = node_entry_to_layout_node(&entry).unwrap();

        assert_eq!(node.kind, "hlist");
        assert_eq!(node.x, Some(0));
        assert_eq!(node.y, Some(100));
        assert_eq!(node.width, Some(500));
    }

    #[test]
    fn summary_entry_returns_none() {
        let json = r#"{"type":"node_tree","hlist":2,"vlist":1,"glyph":42,"glue":15,"rule":3}"#;
        let entry: NodeEntry = serde_json::from_str(json).unwrap();
        let node = node_entry_to_layout_node(&entry);
        assert!(node.is_none());
    }

    #[test]
    fn parse_node_tree_jsonl_single_page() {
        let jsonl = r#"{"type":"hlist","subtype":0,"x":0,"y":0,"width":500,"height":12,"depth":2}
{"type":"glyph","subtype":0,"x":0,"y":0,"char":72,"font_id":1,"width":10,"height":8,"depth":2}
{"type":"glyph","subtype":0,"x":10,"y":0,"char":105,"font_id":1,"width":8,"height":8,"depth":2}
{"type":"node_tree","hlist":1,"vlist":0,"glyph":2,"glue":0,"rule":0,"page":1}"#;

        let graph = parse_node_tree_jsonl(jsonl).unwrap();
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.nodes[0].kind, "hlist");
        assert_eq!(graph.nodes[1].kind, "glyph");
        assert_eq!(graph.nodes[2].kind, "glyph");
    }

    #[test]
    fn parse_node_tree_jsonl_with_page_info() {
        let jsonl = r#"{"type":"node_tree","hlist":2,"vlist":0,"glyph":5,"glue":3,"rule":1,"page":2}"#;
        let graph = parse_node_tree_jsonl(jsonl).unwrap();
        // Summary entries are filtered out
        assert!(graph.nodes.is_empty());
    }

    #[test]
    fn parse_node_tree_jsonl_skips_empty_and_comment_lines() {
        let jsonl = r#"

{"type":"hlist","subtype":0,"x":0,"y":0,"width":100,"height":10,"depth":0}
# This is a comment

{"type":"glyph","subtype":0,"x":0,"y":0,"char":65,"font_id":1,"width":8,"height":8,"depth":0}

"#;
        let graph = parse_node_tree_jsonl(jsonl).unwrap();
        assert_eq!(graph.nodes.len(), 2);
    }
}
