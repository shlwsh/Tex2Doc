use serde::{Deserialize, Serialize};

use crate::{
    AlgLine, BibEntry, Block, Document, FigureSizing, Span, TableRow, TextDirection, TextRun,
    TheoremLikeKind,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StandardDocument {
    pub schema_version: String,
    pub source: SourceBundle,
    pub artifacts: BuildArtifacts,
    pub profile: FormatProfileRef,
    pub metadata: DocumentMetadata,
    pub numbering: NumberingState,
    pub bibliography: BibliographyState,
    pub resources: ResourceIndex,
    pub blocks: Vec<BlockNode>,
    pub diagnostics: Vec<Diagnostic>,
}

impl StandardDocument {
    pub fn from_legacy_document(
        doc: &Document,
        source: SourceBundle,
        profile_id: impl Into<String>,
    ) -> Self {
        let mut standard = Self {
            schema_version: "0.1".to_string(),
            source,
            profile: FormatProfileRef {
                id: profile_id.into(),
                version: None,
            },
            metadata: DocumentMetadata::from_legacy(doc),
            ..Self::default()
        };
        standard.blocks = doc
            .blocks
            .iter()
            .filter(|block| !matches!(block, Block::RawFallback { text, .. } if text.is_empty()))
            .enumerate()
            .map(|(idx, block)| BlockNode::from_legacy(idx + 1, block))
            .collect();
        standard.numbering = NumberingState::from_blocks(&standard.blocks);
        standard.bibliography = BibliographyState::from_legacy(doc);
        standard.resources = ResourceIndex::from_blocks(&standard.blocks);
        standard
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# AST Dump: {}\n\n", self.source.main_path));
        out.push_str("## Document Metadata\n\n");
        out.push_str("| Field | Value |\n|---|---|\n");
        out.push_str(&metadata_row("title_zh", self.metadata.title_zh.as_deref()));
        out.push_str(&metadata_row("title_en", self.metadata.title_en.as_deref()));
        out.push_str(&metadata_row(
            "running_header",
            self.metadata.running_header.as_deref(),
        ));
        out.push_str(&metadata_row("profile", Some(&self.profile.id)));
        out.push('\n');

        out.push_str("## Source Files\n\n");
        out.push_str("| Path | Hash |\n|---|---|\n");
        for source in &self.source.files {
            out.push_str(&format!(
                "| `{}` | `{}` |\n",
                source.path,
                source.hash.as_deref().unwrap_or("-")
            ));
        }
        if self.source.files.is_empty() {
            out.push_str("| - | - |\n");
        }
        out.push('\n');

        out.push_str("## Build Artifacts\n\n");
        out.push_str("| Artifact | Path | Status |\n|---|---|---|\n");
        for artifact in &self.artifacts.items {
            out.push_str(&format!(
                "| {} | `{}` | {} |\n",
                artifact.kind, artifact.path, artifact.status
            ));
        }
        if self.artifacts.items.is_empty() {
            out.push_str("| - | - | none |\n");
        }
        out.push('\n');

        out.push_str("## Numbering\n\n");
        out.push_str("| Kind | Label | Number | Node |\n|---|---|---:|---|\n");
        for item in &self.numbering.items {
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                item.kind,
                item.label.as_deref().unwrap_or("-"),
                item.number,
                item.node_id
            ));
        }
        if self.numbering.items.is_empty() {
            out.push_str("| - | - | - | - |\n");
        }
        out.push('\n');

        out.push_str("## Resources\n\n");
        out.push_str("| ID | Kind | Path | Node |\n|---|---|---|---|\n");
        for resource in &self.resources.items {
            out.push_str(&format!(
                "| {} | {} | `{}` | {} |\n",
                resource.id, resource.kind, resource.path, resource.node_id
            ));
        }
        if self.resources.items.is_empty() {
            out.push_str("| - | - | - | - |\n");
        }
        out.push('\n');

        out.push_str("## Blocks\n\n");
        for block in &self.blocks {
            write_block_markdown(&mut out, block);
        }

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SourceBundle {
    pub main_path: String,
    pub files: Vec<SourceFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceFile {
    pub path: String,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BuildArtifacts {
    pub items: Vec<ArtifactRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArtifactRef {
    pub kind: String,
    pub path: String,
    pub status: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FormatProfileRef {
    pub id: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DocumentMetadata {
    pub title_zh: Option<String>,
    pub title_en: Option<String>,
    pub authors_zh: Vec<String>,
    pub authors_en: Vec<String>,
    pub abstract_zh: Option<String>,
    pub abstract_en: Option<String>,
    pub keywords_zh: Vec<String>,
    pub keywords_en: Vec<String>,
    pub running_header: Option<String>,
    pub first_footer_text: Option<String>,
}

impl DocumentMetadata {
    fn from_legacy(doc: &Document) -> Self {
        Self {
            title_zh: doc.metadata.title.clone(),
            title_en: doc.metadata.title_en.clone(),
            authors_zh: doc.metadata.authors.clone(),
            authors_en: doc.metadata.authors_en.clone(),
            abstract_zh: doc.metadata.abstract_text.clone(),
            abstract_en: doc.metadata.abstract_en.clone(),
            keywords_zh: doc.metadata.keywords.clone(),
            keywords_en: doc.metadata.keywords_en.clone(),
            running_header: doc.metadata.running_header.clone(),
            first_footer_text: doc.metadata.first_footer_text.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NumberingState {
    pub items: Vec<NumberingItem>,
}

impl NumberingState {
    fn from_blocks(blocks: &[BlockNode]) -> Self {
        let items = blocks
            .iter()
            .filter_map(|block| {
                block.number.as_ref().map(|number| NumberingItem {
                    kind: block.kind_name().to_string(),
                    label: block.label.clone(),
                    number: number.value.clone(),
                    node_id: block.id.clone(),
                })
            })
            .collect();
        Self { items }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NumberingItem {
    pub kind: String,
    pub label: Option<String>,
    pub number: String,
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NumberingValue {
    pub value: String,
    pub rule_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BibliographyState {
    pub entries: Vec<BibEntry>,
}

impl BibliographyState {
    fn from_legacy(doc: &Document) -> Self {
        let mut entries = Vec::new();
        for block in &doc.blocks {
            if let Block::Bibliography {
                entries: block_entries,
            } = block
            {
                entries.extend(block_entries.clone());
            }
        }
        Self { entries }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ResourceIndex {
    pub items: Vec<ResourceRef>,
}

impl ResourceIndex {
    fn from_blocks(blocks: &[BlockNode]) -> Self {
        let mut items = Vec::new();
        for block in blocks {
            if let BlockKind::Figure(fig) = &block.kind {
                items.push(ResourceRef {
                    id: format!("res-{}", block.id),
                    kind: "image".to_string(),
                    path: fig.path.clone(),
                    node_id: block.id.clone(),
                });
            }
        }
        Self { items }
    }
}

fn figure_width_hint(scale: f32, sizing: Option<&FigureSizing>) -> Option<String> {
    if let Some(ratio) = sizing.and_then(|s| s.normalized_width_ratio) {
        return Some(format!("{ratio:.4}\\textwidth"));
    }
    if scale.is_finite() && (scale - 1.0).abs() > f32::EPSILON {
        return Some(format!("{scale:.4}\\textwidth"));
    }
    sizing.and_then(|s| s.width_expr.clone())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceRef {
    pub id: String,
    pub kind: String,
    pub path: String,
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlockNode {
    pub id: String,
    pub kind: BlockKind,
    pub source_span: Option<Span>,
    pub label: Option<String>,
    pub number: Option<NumberingValue>,
    pub style_intent: StyleIntent,
    pub layout: LayoutHints,
    pub metadata: NodeMetadata,
    pub children: Vec<BlockNode>,
}

impl BlockNode {
    fn from_legacy(index: usize, block: &Block) -> Self {
        let id = format!("B{index:04}");
        match block {
            Block::Heading {
                level,
                text,
                number,
                span,
            } => Self {
                id,
                kind: BlockKind::Heading {
                    level: *level,
                    runs: vec![InlineNode::text(text.clone())],
                },
                source_span: Some(*span),
                label: None,
                number: number.clone().map(|value| NumberingValue {
                    value,
                    rule_id: Some("latex2e.section".to_string()),
                }),
                style_intent: StyleIntent::profile(format!("JOSHeading{level}")),
                layout: LayoutHints::default(),
                metadata: NodeMetadata::from_rule("latex2e.section"),
                children: vec![],
            },
            Block::Paragraph { runs, span } => Self {
                id,
                kind: BlockKind::Paragraph {
                    runs: runs
                        .iter()
                        .cloned()
                        .map(InlineNode::from_text_run)
                        .collect(),
                },
                source_span: Some(*span),
                label: None,
                number: None,
                style_intent: StyleIntent::profile("JOSBody"),
                layout: LayoutHints::default(),
                metadata: NodeMetadata::from_rule("latex2e.paragraph"),
                children: vec![],
            },
            Block::List {
                is_ordered,
                items,
                span,
            } => Self {
                id,
                kind: BlockKind::List(ListNode {
                    ordered: *is_ordered,
                    item_count: items.len(),
                }),
                source_span: Some(*span),
                label: None,
                number: None,
                style_intent: StyleIntent::profile("JOSBody"),
                layout: LayoutHints::default(),
                metadata: NodeMetadata::from_rule("latex2e.list"),
                children: vec![],
            },
            Block::Table {
                rows,
                caption,
                number,
                span,
            } => Self {
                id,
                kind: BlockKind::Table(TableNode {
                    caption: caption.clone(),
                    rows: rows.clone(),
                }),
                source_span: Some(*span),
                label: None,
                number: number.clone().map(|value| NumberingValue {
                    value,
                    rule_id: Some("latex2e.caption".to_string()),
                }),
                style_intent: StyleIntent::profile("JOSTableText"),
                layout: LayoutHints {
                    keep_next: true,
                    keep_lines: true,
                    allow_split: false,
                    width: None,
                    alignment: Some("center".to_string()),
                    spacing: None,
                },
                metadata: NodeMetadata::from_rules(["latex2e.tabular", "booktabs.table"]),
                children: vec![],
            },
            Block::Figure {
                path,
                caption,
                scale,
                sizing,
                number,
                label,
                text_direction,
                span,
            } => Self {
                id,
                kind: BlockKind::Figure(FigureNode {
                    path: path.clone(),
                    caption: caption.clone(),
                    scale: *scale,
                    sizing: sizing.clone(),
                    label: label.clone(),
                    text_direction: *text_direction,
                }),
                source_span: Some(*span),
                label: None,
                number: number.clone().map(|value| NumberingValue {
                    value,
                    rule_id: Some("latex2e.caption".to_string()),
                }),
                style_intent: StyleIntent::profile("JOSImage"),
                layout: LayoutHints {
                    keep_next: true,
                    keep_lines: true,
                    allow_split: false,
                    width: figure_width_hint(*scale, sizing.as_ref()),
                    alignment: Some("center".to_string()),
                    spacing: None,
                },
                metadata: NodeMetadata::from_rules(["latex2e.figure", "graphicx.includegraphics"]),
                children: vec![],
            },
            Block::Equation {
                latex,
                is_block,
                span,
            } => Self {
                id,
                kind: BlockKind::Equation(EquationNode {
                    latex: latex.clone(),
                    is_block: *is_block,
                    normalized: None,
                }),
                source_span: Some(*span),
                label: None,
                number: None,
                style_intent: StyleIntent::profile("JOSCode"),
                layout: LayoutHints::default(),
                metadata: NodeMetadata::from_rule("latex2e.math"),
                children: vec![],
            },
            Block::TheoremLike {
                kind,
                title,
                body,
                span,
            } => Self {
                id,
                kind: BlockKind::TheoremLike(TheoremLikeNode {
                    kind: kind.clone(),
                    title: title.clone(),
                    body: body.clone(),
                }),
                source_span: Some(*span),
                label: None,
                number: None,
                style_intent: StyleIntent::profile("JOSBody"),
                layout: LayoutHints {
                    keep_next: true,
                    keep_lines: true,
                    allow_split: true,
                    width: None,
                    alignment: None,
                    spacing: None,
                },
                metadata: NodeMetadata::from_rule(match kind {
                    TheoremLikeKind::Proof => "latex2e.proof",
                    TheoremLikeKind::Theorem => "latex2e.theorem",
                    TheoremLikeKind::Proposition => "latex2e.proposition",
                    TheoremLikeKind::Lemma => "latex2e.lemma",
                    TheoremLikeKind::Corollary => "latex2e.corollary",
                    TheoremLikeKind::Definition => "latex2e.definition",
                    TheoremLikeKind::Remark => "latex2e.remark",
                    TheoremLikeKind::Example => "latex2e.example",
                }),
                children: vec![],
            },
            Block::Bibliography { entries } => Self {
                id,
                kind: BlockKind::Bibliography(BibliographyNode {
                    entries: entries.clone(),
                }),
                source_span: None,
                label: None,
                number: None,
                style_intent: StyleIntent::profile("JOSReference"),
                layout: LayoutHints::default(),
                metadata: NodeMetadata::from_rule("natbib.bibliography"),
                children: vec![],
            },
            Block::Algorithm {
                lines,
                io,
                caption,
                number,
                span,
            } => Self {
                id,
                kind: BlockKind::Algorithm(AlgorithmNode {
                    caption: caption.clone(),
                    io: io.clone(),
                    lines: lines.clone(),
                }),
                source_span: Some(*span),
                label: None,
                number: number.clone().map(|value| NumberingValue {
                    value,
                    rule_id: Some("algorithm2e.algorithm".to_string()),
                }),
                style_intent: StyleIntent::profile("JOSCode"),
                layout: LayoutHints {
                    keep_next: true,
                    keep_lines: true,
                    allow_split: false,
                    width: None,
                    alignment: None,
                    spacing: None,
                },
                metadata: NodeMetadata::from_rule("algorithm2e.algorithm"),
                children: vec![],
            },
            Block::CodeBlock {
                language,
                code,
                span,
            } => Self {
                id,
                kind: BlockKind::CodeBlock(CodeBlockNode {
                    language: language.clone(),
                    code: code.clone(),
                    source: CodeBlockSource::Verbatim,
                }),
                source_span: Some(*span),
                label: None,
                number: None,
                style_intent: StyleIntent::profile("JOSCode"),
                layout: LayoutHints {
                    keep_next: true,
                    keep_lines: true,
                    allow_split: false,
                    width: None,
                    alignment: None,
                    spacing: None,
                },
                metadata: NodeMetadata::from_rule("tex.code_block"),
                children: vec![],
            },
            Block::RawFallback { text, span } => Self {
                id,
                kind: BlockKind::RawFallback {
                    raw: text.clone(),
                    reason: "legacy raw fallback".to_string(),
                },
                source_span: Some(*span),
                label: None,
                number: None,
                style_intent: StyleIntent::default(),
                layout: LayoutHints::default(),
                metadata: NodeMetadata::from_rule("tex.raw_fallback"),
                children: vec![],
            },
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self.kind {
            BlockKind::FrontMatter(_) => "front_matter",
            BlockKind::Heading { .. } => "heading",
            BlockKind::Paragraph { .. } => "paragraph",
            BlockKind::Figure(_) => "figure",
            BlockKind::Table(_) => "table",
            BlockKind::Algorithm(_) => "algorithm",
            BlockKind::Equation(_) => "equation",
            BlockKind::TheoremLike(_) => "theorem_like",
            BlockKind::List(_) => "list",
            BlockKind::Bibliography(_) => "bibliography",
            BlockKind::AuthorBio(_) => "author_bio",
            BlockKind::CodeBlock(_) => "code_block",
            BlockKind::RawFallback { .. } => "raw_fallback",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlockKind {
    FrontMatter(FrontMatterKind),
    Heading { level: u8, runs: Vec<InlineNode> },
    Paragraph { runs: Vec<InlineNode> },
    Figure(FigureNode),
    Table(TableNode),
    Algorithm(AlgorithmNode),
    Equation(EquationNode),
    TheoremLike(TheoremLikeNode),
    List(ListNode),
    Bibliography(BibliographyNode),
    AuthorBio(Vec<InlineNode>),
    CodeBlock(CodeBlockNode),
    RawFallback { raw: String, reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FigureNode {
    pub path: String,
    pub caption: Option<String>,
    pub scale: f32,
    pub sizing: Option<FigureSizing>,
    pub label: Option<String>,
    pub text_direction: Option<TextDirection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableNode {
    pub caption: Option<String>,
    pub rows: Vec<TableRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlgorithmNode {
    pub caption: Option<String>,
    pub io: Vec<(String, String)>,
    pub lines: Vec<AlgLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EquationNode {
    pub latex: String,
    pub is_block: bool,
    pub normalized: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TheoremLikeNode {
    pub kind: TheoremLikeKind,
    pub title: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListNode {
    pub ordered: bool,
    pub item_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BibliographyNode {
    pub entries: Vec<BibEntry>,
}

/// A code block from minted or listings environments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeBlockNode {
    /// Programming language hint (e.g., "python", "rust", "c++").
    pub language: Option<String>,
    /// The raw source code content.
    pub code: String,
    /// Which environment or command produced this block.
    pub source: CodeBlockSource,
}

/// Source of a code block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CodeBlockSource {
    /// From a minted LaTeX environment.
    Minted,
    /// From a listings environment.
    Listlings,
    /// From a verbatim environment.
    Verbatim,
    /// From an lstlisting environment.
    Lstlisting,
    /// From a CodeBlock Markdown fence.
    MarkdownFence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FrontMatterKind {
    Title,
    Authors,
    Institute,
    Abstract,
    Keywords,
    Citation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InlineNode {
    Text {
        text: String,
        style: InlineStyle,
        source_span: Option<Span>,
    },
    Citation {
        keys: Vec<String>,
        rendered: String,
        number_refs: Vec<usize>,
    },
    CrossRef {
        label: String,
        target: Option<String>,
        rendered: String,
    },
    MathInline {
        latex: String,
        normalized: String,
        runs: Vec<InlineNode>,
    },
    Link {
        url: String,
        text: String,
    },
}

impl InlineNode {
    fn text(text: String) -> Self {
        Self::Text {
            text,
            style: InlineStyle::default(),
            source_span: None,
        }
    }

    fn from_text_run(run: TextRun) -> Self {
        Self::Text {
            text: run.text,
            style: InlineStyle {
                text_style: format!("{:?}", run.style),
                ..InlineStyle::default()
            },
            source_span: Some(run.span),
        }
    }

    fn plain_text(&self) -> String {
        match self {
            Self::Text { text, .. } => text.clone(),
            Self::Citation { rendered, .. } => rendered.clone(),
            Self::CrossRef { rendered, .. } => rendered.clone(),
            Self::MathInline { normalized, .. } => normalized.clone(),
            Self::Link { text, .. } => text.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct InlineStyle {
    pub text_style: String,
    pub font_hint: Option<String>,
    pub language_hint: Option<String>,
    pub citation_role: Option<String>,
    pub math_role: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StyleIntent {
    pub semantic_role: Option<String>,
    pub profile_style: Option<String>,
    pub font_hint: Option<String>,
    pub paragraph_hint: Option<String>,
}

impl StyleIntent {
    fn profile(style: impl Into<String>) -> Self {
        Self {
            profile_style: Some(style.into()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LayoutHints {
    pub keep_next: bool,
    pub keep_lines: bool,
    pub allow_split: bool,
    pub width: Option<String>,
    pub alignment: Option<String>,
    pub spacing: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NodeMetadata {
    pub rule_ids: Vec<String>,
}

impl NodeMetadata {
    fn from_rule(rule_id: impl Into<String>) -> Self {
        Self {
            rule_ids: vec![rule_id.into()],
        }
    }

    fn from_rules<const N: usize>(rule_ids: [&str; N]) -> Self {
        Self {
            rule_ids: rule_ids.iter().map(|rule_id| rule_id.to_string()).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Diagnostic {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub node_id: Option<String>,
}

fn write_block_markdown(out: &mut String, block: &BlockNode) {
    out.push_str(&format!("### [{}] {}\n\n", block.id, block.kind_name()));
    if let Some(span) = block.source_span {
        out.push_str(&format!("- source_span: `{}..{}`\n", span.start, span.end));
    }
    if let Some(label) = &block.label {
        out.push_str(&format!("- label: `{label}`\n"));
    }
    if let Some(number) = &block.number {
        out.push_str(&format!("- number: `{}`\n", number.value));
        if let Some(rule_id) = &number.rule_id {
            out.push_str(&format!("- number_rule: `{rule_id}`\n"));
        }
    }
    if let Some(style) = &block.style_intent.profile_style {
        out.push_str(&format!("- style_intent: `{style}`\n"));
    }
    if !block.metadata.rule_ids.is_empty() {
        out.push_str(&format!(
            "- rule_ids: `{}`\n",
            block.metadata.rule_ids.join(", ")
        ));
    }
    match &block.kind {
        BlockKind::Heading { level, runs } => {
            out.push_str(&format!("- level: `{level}`\n"));
            out.push_str(&format!("- text: `{}`\n", escape_md(&inline_text(runs))));
        }
        BlockKind::Paragraph { runs } => {
            out.push_str(&format!("- text: `{}`\n", escape_md(&inline_text(runs))));
        }
        BlockKind::Figure(fig) => {
            out.push_str(&format!("- image: `{}`\n", fig.path));
            if let Some(caption) = &fig.caption {
                out.push_str(&format!("- caption: `{}`\n", escape_md(caption)));
            }
        }
        BlockKind::Table(table) => {
            if let Some(caption) = &table.caption {
                out.push_str(&format!("- caption: `{}`\n", escape_md(caption)));
            }
            out.push_str(&format!("- rows: `{}`\n", table.rows.len()));
        }
        BlockKind::Algorithm(alg) => {
            if let Some(caption) = &alg.caption {
                out.push_str(&format!("- caption: `{}`\n", escape_md(caption)));
            }
            out.push_str("\n| line | indent | code | comment |\n|---:|---:|---|---|\n");
            for (idx, line) in alg.lines.iter().enumerate() {
                out.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    idx + 1,
                    line.indent,
                    escape_md(&line.code),
                    escape_md(&line.comment)
                ));
            }
        }
        BlockKind::Equation(eq) => {
            out.push_str(&format!("- latex: `{}`\n", escape_md(&eq.latex)));
        }
        BlockKind::TheoremLike(thm) => {
            out.push_str(&format!(
                "- kind: `{:?}`\n- title: `{}`\n- body: `{}`\n",
                thm.kind,
                thm.title
                    .as_deref()
                    .map(escape_md)
                    .unwrap_or_else(|| "-".to_string()),
                escape_md(&thm.body)
            ));
        }
        BlockKind::List(list) => {
            out.push_str(&format!(
                "- ordered: `{}`\n- item_count: `{}`\n",
                list.ordered, list.item_count
            ));
        }
        BlockKind::Bibliography(bib) => {
            out.push_str(&format!("- entries: `{}`\n", bib.entries.len()));
        }
        BlockKind::RawFallback { raw, reason } => {
            out.push_str(&format!(
                "- reason: `{}`\n- raw: `{}`\n",
                escape_md(reason),
                escape_md(raw)
            ));
        }
        BlockKind::CodeBlock(node) => {
            let lang = node.language.as_deref().unwrap_or("text");
            out.push_str(&format!("**Code block** — `{lang}`\n\n"));
            out.push_str("```");
            out.push_str(lang);
            out.push('\n');
            out.push_str(&escape_md(&node.code));
            out.push_str("\n```\n");
        }
        BlockKind::FrontMatter(_) | BlockKind::AuthorBio(_) => {}
    }
    out.push('\n');
}

fn metadata_row(field: &str, value: Option<&str>) -> String {
    format!(
        "| {field} | {} |\n",
        value.map(escape_md).unwrap_or_else(|| "-".to_string())
    )
}

fn inline_text(runs: &[InlineNode]) -> String {
    runs.iter()
        .map(InlineNode::plain_text)
        .collect::<Vec<_>>()
        .join("")
}

fn escape_md(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SourceId, TextStyle};

    #[test]
    fn standard_document_markdown_contains_blocks() {
        let doc = Document {
            metadata: crate::MetaData {
                title: Some("Title".to_string()),
                ..Default::default()
            },
            blocks: vec![Block::Heading {
                level: 1,
                text: "Intro".to_string(),
                number: Some("1".to_string()),
                span: Span::new(0, 5, SourceId(0)),
            }],
        };
        let standard = StandardDocument::from_legacy_document(
            &doc,
            SourceBundle {
                main_path: "main.tex".to_string(),
                files: vec![],
            },
            "jos-2025",
        );
        let md = standard.to_markdown();
        assert!(md.contains("# AST Dump: main.tex"));
        assert!(md.contains("## Source Files"));
        assert!(md.contains("[B0001] heading"));
        assert!(md.contains("latex2e.section"));
    }

    #[test]
    fn inline_from_text_run_preserves_text() {
        let run = TextRun {
            text: "x".to_string(),
            style: TextStyle::Superscript,
            span: Span::new(0, 1, SourceId(0)),
        };
        assert_eq!(InlineNode::from_text_run(run).plain_text(), "x");
    }

    #[test]
    fn standard_document_preserves_figure_sizing() {
        let sizing = FigureSizing::from_options(Some("width=.8\\textwidth".to_string()));
        let doc = Document {
            metadata: Default::default(),
            blocks: vec![Block::Figure {
                path: "figures/a.png".to_string(),
                caption: Some("Demo".to_string()),
                scale: 0.8,
                sizing: sizing.clone(),
                number: Some("图 1".to_string()),
                label: None,
                text_direction: None,
                span: Span::new(0, 5, SourceId(0)),
            }],
        };
        let standard = StandardDocument::from_legacy_document(
            &doc,
            SourceBundle {
                main_path: "main.tex".to_string(),
                files: vec![],
            },
            "jos-2025",
        );

        assert_eq!(
            standard.blocks[0].layout.width.as_deref(),
            Some("0.8000\\textwidth")
        );
        match &standard.blocks[0].kind {
            BlockKind::Figure(fig) => {
                assert_eq!(
                    fig.sizing
                        .as_ref()
                        .and_then(|s| s.source_options.as_deref()),
                    Some("width=.8\\textwidth")
                );
            }
            _ => panic!("expected figure"),
        }
    }

    #[test]
    fn standard_document_json_roundtrip() {
        let doc = Document {
            metadata: crate::MetaData {
                title: Some("Title".to_string()),
                ..Default::default()
            },
            blocks: vec![Block::Paragraph {
                runs: vec![TextRun {
                    text: "hello".to_string(),
                    style: TextStyle::Plain,
                    span: Span::new(0, 5, SourceId(0)),
                }],
                span: Span::new(0, 5, SourceId(0)),
            }],
        };
        let standard = StandardDocument::from_legacy_document(
            &doc,
            SourceBundle {
                main_path: "main.tex".to_string(),
                files: vec![SourceFile {
                    path: "main.tex".to_string(),
                    hash: Some("blake3:test".to_string()),
                }],
            },
            "jos-2025",
        );
        let json = serde_json::to_string(&standard).unwrap();
        let restored: StandardDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, standard);
    }
}
