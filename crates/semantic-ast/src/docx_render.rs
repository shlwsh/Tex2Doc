use std::{collections::BTreeSet, path::Path};

use serde::{Deserialize, Serialize};

use crate::{BlockKind, Diagnostic, StandardDocument};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MappingRegistry {
    pub profile_id: String,
    pub block_mappings: Vec<MappingRule>,
    pub inline_mappings: Vec<MappingRule>,
    pub resource_mappings: Vec<MappingRule>,
}

impl MappingRegistry {
    pub fn for_profile(profile_id: impl Into<String>) -> Self {
        Self {
            profile_id: profile_id.into(),
            block_mappings: vec![
                MappingRule::block("map.heading.docx", "heading", "w:p", "heading_by_level"),
                MappingRule::block("map.paragraph.docx", "paragraph", "w:p", "body"),
                MappingRule::block("map.figure.docx", "figure", "w:drawing", "figure"),
                MappingRule::block("map.table.docx", "table", "w:tbl", "table"),
                MappingRule::block("map.algorithm.docx", "algorithm", "w:tbl", "algorithm"),
                MappingRule::block("map.equation.docx", "equation", "m:oMathPara", "equation"),
                MappingRule::block("map.theorem_like.docx", "theorem_like", "w:p", "body"),
                MappingRule::block(
                    "map.bibliography.docx",
                    "bibliography",
                    "w:p[]",
                    "reference",
                ),
                MappingRule::block("map.list.docx", "list", "w:p[]", "body"),
                MappingRule::block("map.raw_fallback.docx", "raw_fallback", "w:p", "code"),
            ],
            inline_mappings: vec![
                MappingRule::inline("map.text.docx", "Text", "w:r"),
                MappingRule::inline("map.citation.docx", "Citation", "w:r"),
                MappingRule::inline("map.crossref.docx", "CrossRef", "w:r"),
                MappingRule::inline("map.math_inline.docx", "MathInline", "m:oMath"),
                MappingRule::inline("map.link.docx", "Link", "w:hyperlink"),
            ],
            resource_mappings: vec![MappingRule::resource(
                "opc.relationship.image",
                "image",
                "Relationship",
            )],
        }
    }

    pub fn block_rule(&self, source_kind: &str) -> Option<&MappingRule> {
        self.block_mappings
            .iter()
            .find(|rule| rule.source_kind == source_kind)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MappingRule {
    pub id: String,
    pub source_kind: String,
    pub target_kind: String,
    pub style_from_profile: Option<String>,
    pub rule_type: String,
}

impl MappingRule {
    fn block(
        id: impl Into<String>,
        source_kind: impl Into<String>,
        target_kind: impl Into<String>,
        style_from_profile: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            source_kind: source_kind.into(),
            target_kind: target_kind.into(),
            style_from_profile: Some(style_from_profile.into()),
            rule_type: "block".to_string(),
        }
    }

    fn inline(
        id: impl Into<String>,
        source_kind: impl Into<String>,
        target_kind: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            source_kind: source_kind.into(),
            target_kind: target_kind.into(),
            style_from_profile: None,
            rule_type: "inline".to_string(),
        }
    }

    fn resource(
        id: impl Into<String>,
        source_kind: impl Into<String>,
        target_kind: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            source_kind: source_kind.into(),
            target_kind: target_kind.into(),
            style_from_profile: None,
            rule_type: "resource".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DocxRenderTree {
    pub schema_version: String,
    pub profile_id: String,
    pub package: DocxPackagePlan,
    pub styles: Vec<DocxStylePlan>,
    pub document: Vec<DocxRenderNode>,
    pub relationships: Vec<DocxRelationship>,
    pub media: Vec<DocxMediaPlan>,
    pub diagnostics: Vec<Diagnostic>,
}

impl DocxRenderTree {
    pub fn from_standard(doc: &StandardDocument, registry: &MappingRegistry) -> Self {
        let mut diagnostics = Vec::new();
        let document: Vec<DocxRenderNode> = doc
            .blocks
            .iter()
            .map(|block| {
                let source_kind = block.kind_name();
                let rule = registry.block_rule(source_kind);
                if rule.is_none() {
                    diagnostics.push(Diagnostic {
                        severity: "warning".to_string(),
                        code: "missing_mapping_rule".to_string(),
                        message: format!("未找到 block 映射规则：{source_kind}"),
                        node_id: Some(block.id.clone()),
                    });
                }
                DocxRenderNode {
                    id: format!("R{}", block.id.trim_start_matches('B')),
                    source_node_id: block.id.clone(),
                    source_kind: source_kind.to_string(),
                    mapping_rule_id: rule
                        .map(|rule| rule.id.clone())
                        .unwrap_or_else(|| "map.missing".to_string()),
                    target_kind: rule
                        .map(|rule| rule.target_kind.clone())
                        .unwrap_or_else(|| "w:p".to_string()),
                    style: block.style_intent.profile_style.clone(),
                    text: render_text(&block.kind),
                    rule_ids: block.metadata.rule_ids.clone(),
                }
            })
            .collect();
        let relationships = doc
            .resources
            .items
            .iter()
            .enumerate()
            .map(|(idx, resource)| DocxRelationship {
                id: format!("rId{}", idx + 1),
                source_node_id: resource.node_id.clone(),
                relationship_type:
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                        .to_string(),
                target: format!("media/{}", file_name(&resource.path)),
                mapping_rule_id: "opc.relationship.image".to_string(),
            })
            .collect();
        let media = doc
            .resources
            .items
            .iter()
            .map(|resource| DocxMediaPlan {
                source_node_id: resource.node_id.clone(),
                source_path: resource.path.clone(),
                package_target: format!("word/media/{}", file_name(&resource.path)),
            })
            .collect();
        let styles = collect_styles(&document);
        Self {
            schema_version: "0.1".to_string(),
            profile_id: registry.profile_id.clone(),
            package: DocxPackagePlan::default(),
            styles,
            document,
            relationships,
            media,
            diagnostics,
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# DOCX Render Dump: {}\n\n", self.profile_id));
        out.push_str("## Package Parts\n\n");
        out.push_str("| Part | Content Type |\n|---|---|\n");
        for part in &self.package.parts {
            out.push_str(&format!("| `{}` | `{}` |\n", part.path, part.content_type));
        }
        out.push('\n');

        out.push_str("## Styles\n\n");
        out.push_str("| Style ID | Source |\n|---|---|\n");
        for style in &self.styles {
            out.push_str(&format!("| `{}` | {} |\n", style.style_id, style.source));
        }
        if self.styles.is_empty() {
            out.push_str("| - | - |\n");
        }
        out.push('\n');

        out.push_str("## Relationships\n\n");
        out.push_str("| ID | Source Node | Target | Mapping Rule |\n|---|---|---|---|\n");
        for rel in &self.relationships {
            out.push_str(&format!(
                "| {} | {} | `{}` | `{}` |\n",
                rel.id, rel.source_node_id, rel.target, rel.mapping_rule_id
            ));
        }
        if self.relationships.is_empty() {
            out.push_str("| - | - | - | - |\n");
        }
        out.push('\n');

        out.push_str("## Media\n\n");
        out.push_str("| Source Node | Source Path | Package Target |\n|---|---|---|\n");
        for media in &self.media {
            out.push_str(&format!(
                "| {} | `{}` | `{}` |\n",
                media.source_node_id, media.source_path, media.package_target
            ));
        }
        if self.media.is_empty() {
            out.push_str("| - | - | - |\n");
        }
        out.push('\n');

        out.push_str("## Render Nodes\n\n");
        out.push_str(
            "| ID | Source | Source Kind | Target | Style | Mapping Rule | Rule IDs | Text |\n",
        );
        out.push_str("|---|---|---|---|---|---|---|---|\n");
        for node in &self.document {
            out.push_str(&format!(
                "| {} | {} | {} | `{}` | {} | `{}` | `{}` | {} |\n",
                node.id,
                node.source_node_id,
                node.source_kind,
                node.target_kind,
                node.style.as_deref().unwrap_or("-"),
                node.mapping_rule_id,
                node.rule_ids.join(", "),
                escape_md(node.text.as_deref().unwrap_or("-"))
            ));
        }
        if self.document.is_empty() {
            out.push_str("| - | - | - | - | - | - | - | - |\n");
        }
        out.push('\n');

        out.push_str("## Diagnostics\n\n");
        out.push_str("| Severity | Code | Message | Node |\n|---|---|---|---|\n");
        for diag in &self.diagnostics {
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                diag.severity,
                diag.code,
                escape_md(&diag.message),
                diag.node_id.as_deref().unwrap_or("-")
            ));
        }
        if self.diagnostics.is_empty() {
            out.push_str("| - | - | none | - |\n");
        }
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocxPackagePlan {
    pub parts: Vec<DocxPartPlan>,
}

impl Default for DocxPackagePlan {
    fn default() -> Self {
        Self {
            parts: vec![
                DocxPartPlan::new("[Content_Types].xml", "application/xml"),
                DocxPartPlan::new(
                    "_rels/.rels",
                    "application/vnd.openxmlformats-package.relationships+xml",
                ),
                DocxPartPlan::new(
                    "word/document.xml",
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
                ),
                DocxPartPlan::new(
                    "word/styles.xml",
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml",
                ),
                DocxPartPlan::new(
                    "word/_rels/document.xml.rels",
                    "application/vnd.openxmlformats-package.relationships+xml",
                ),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocxPartPlan {
    pub path: String,
    pub content_type: String,
}

impl DocxPartPlan {
    fn new(path: impl Into<String>, content_type: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content_type: content_type.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocxRenderNode {
    pub id: String,
    pub source_node_id: String,
    pub source_kind: String,
    pub mapping_rule_id: String,
    pub target_kind: String,
    pub style: Option<String>,
    pub text: Option<String>,
    pub rule_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocxStylePlan {
    pub style_id: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocxMediaPlan {
    pub source_node_id: String,
    pub source_path: String,
    pub package_target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocxRelationship {
    pub id: String,
    pub source_node_id: String,
    pub relationship_type: String,
    pub target: String,
    pub mapping_rule_id: String,
}

fn collect_styles(nodes: &[DocxRenderNode]) -> Vec<DocxStylePlan> {
    let mut style_ids = BTreeSet::new();
    for node in nodes {
        if let Some(style) = &node.style {
            style_ids.insert(style.clone());
        }
    }
    style_ids
        .into_iter()
        .map(|style_id| DocxStylePlan {
            style_id,
            source: "profile".to_string(),
        })
        .collect()
}

fn render_text(kind: &BlockKind) -> Option<String> {
    match kind {
        BlockKind::Heading { runs, .. } | BlockKind::Paragraph { runs } => {
            Some(runs.iter().map(inline_text).collect::<Vec<_>>().join(""))
        }
        BlockKind::Figure(fig) => fig.caption.clone(),
        BlockKind::Table(table) => table.caption.clone(),
        BlockKind::Algorithm(alg) => alg.caption.clone(),
        BlockKind::Equation(eq) => Some(eq.latex.clone()),
        BlockKind::TheoremLike(thm) => {
            let title = thm
                .title
                .as_ref()
                .map(|title| format!("（{title}）"))
                .unwrap_or_default();
            Some(format!("{}{} {}", thm.kind.display_name(), title, thm.body))
        }
        BlockKind::Bibliography(bib) => Some(format!("{} bibliography entries", bib.entries.len())),
        BlockKind::List(list) => Some(format!("{} list items", list.item_count)),
        BlockKind::CodeBlock(node) => Some(format!("[{} code block]", node.language.as_deref().unwrap_or("text"))),
        BlockKind::RawFallback { raw, .. } => Some(raw.clone()),
        BlockKind::FrontMatter(_) | BlockKind::AuthorBio(_) => None,
    }
}

fn inline_text(inline: &crate::InlineNode) -> String {
    match inline {
        crate::InlineNode::Text { text, .. } => text.clone(),
        crate::InlineNode::Citation { rendered, .. } => rendered.clone(),
        crate::InlineNode::CrossRef { rendered, .. } => rendered.clone(),
        crate::InlineNode::MathInline { normalized, .. } => normalized.clone(),
        crate::InlineNode::Link { text, .. } => text.clone(),
    }
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn escape_md(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Block, Document, SourceBundle, SourceId, Span, TextRun};

    #[test]
    fn render_tree_maps_standard_blocks() {
        let doc = Document {
            blocks: vec![Block::Heading {
                level: 1,
                text: "Intro".to_string(),
                number: Some("1".to_string()),
                span: Span::new(0, 5, SourceId(0)),
            }],
            ..Default::default()
        };
        let standard = StandardDocument::from_legacy_document(
            &doc,
            SourceBundle {
                main_path: "main.tex".to_string(),
                files: vec![],
            },
            "jos-2025",
        );
        let registry = MappingRegistry::for_profile("jos-2025");
        let render = DocxRenderTree::from_standard(&standard, &registry);
        assert_eq!(render.document[0].mapping_rule_id, "map.heading.docx");
        assert_eq!(render.document[0].target_kind, "w:p");
        assert!(render.to_markdown().contains("map.heading.docx"));
    }

    #[test]
    fn render_tree_json_roundtrip() {
        let doc = Document {
            blocks: vec![Block::Paragraph {
                runs: vec![TextRun::plain("hello", Span::new(0, 5, SourceId(0)))],
                span: Span::new(0, 5, SourceId(0)),
            }],
            ..Default::default()
        };
        let standard = StandardDocument::from_legacy_document(
            &doc,
            SourceBundle {
                main_path: "main.tex".to_string(),
                files: vec![],
            },
            "jos-2025",
        );
        let registry = MappingRegistry::for_profile("jos-2025");
        let render = DocxRenderTree::from_standard(&standard, &registry);
        let json = serde_json::to_string(&render).unwrap();
        let restored: DocxRenderTree = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, render);
    }
}
