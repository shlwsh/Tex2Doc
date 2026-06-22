//! Builtin macro rules for common LaTeX commands and journal-specific extensions.

use super::{MacroRule, RuleOutput};

/// Returns all builtin macro rules.
pub fn builtin_rules() -> Vec<MacroRule> {
    vec![
        // ── Font commands (inline text) ─────────────────────────────────
        rule(
            "textbf",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "Bold text",
        ),
        rule(
            "textit",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "Italic text",
        ),
        rule(
            "texttt",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "Typewriter/monospace text",
        ),
        rule(
            "textsf",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "Sans-serif text",
        ),
        rule(
            "emph",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "Emphasized text",
        ),
        rule(
            "mathbf",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "Bold math text",
        ),
        rule(
            "mathrm",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "Roman (upright) math text",
        ),
        rule(
            "mathsf",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "Sans-serif math text",
        ),
        rule(
            "mathtt",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "Typewriter math text",
        ),
        // ── Font size commands ──────────────────────────────────────────
        rule("tiny", 0, RuleOutput::Ignore, "Tiny font size"),
        rule("small", 0, RuleOutput::Ignore, "Small font size"),
        rule("large", 0, RuleOutput::Ignore, "Large font size"),
        rule("Large", 0, RuleOutput::Ignore, "Large font size"),
        rule("LARGE", 0, RuleOutput::Ignore, "LARGE font size"),
        rule("huge", 0, RuleOutput::Ignore, "Huge font size"),
        rule("Huge", 0, RuleOutput::Ignore, "Huge font size"),
        // ── Spacing commands ───────────────────────────────────────────
        rule("hspace", 1, RuleOutput::Ignore, "Horizontal spacing"),
        rule("vspace", 1, RuleOutput::Ignore, "Vertical spacing"),
        rule("quad", 0, RuleOutput::Ignore, "1em quad spacing"),
        rule("qquad", 0, RuleOutput::Ignore, "2em quad spacing"),
        rule("smallskip", 0, RuleOutput::Ignore, "Small vertical skip"),
        rule("smallskip", 0, RuleOutput::Ignore, "Medium vertical skip"),
        rule("bigskip", 0, RuleOutput::Ignore, "Big vertical skip"),
        rule("newline", 0, RuleOutput::Ignore, "Line break"),
        rule("newpage", 0, RuleOutput::Ignore, "Page break"),
        rule("pagebreak", 0, RuleOutput::Ignore, "Page break"),
        rule("nopagebreak", 0, RuleOutput::Ignore, "Suppress page break"),
        rule(
            "clearpage",
            0,
            RuleOutput::Ignore,
            "Clear all pending floats and page break",
        ),
        rule(
            "cleardoublepage",
            0,
            RuleOutput::Ignore,
            "Clear floats and go to next odd page",
        ),
        // ── Accents / symbols ──────────────────────────────────────────
        rule(
            "underline",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "Underlined text",
        ),
        rule(
            "sout",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "Strikethrough text (ulem)",
        ),
        rule(
            "xout",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "Crossed-out text (ulem)",
        ),
        rule(
            "uwave",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "Wave underline (ulem)",
        ),
        rule(
            "CJKfamily",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "CJK font family switch",
        ),
        rule(
            "CJKunderline",
            1,
            RuleOutput::InlineText { content_arg: 0 },
            "CJK underline",
        ),
        rule(
            "paragraph",
            1,
            RuleOutput::Heading {
                level: 4,
                text_arg: 0,
            },
            "Fourth-level heading",
        ),
        rule(
            "subparagraph",
            1,
            RuleOutput::Heading {
                level: 5,
                text_arg: 0,
            },
            "Fifth-level heading",
        ),
    ]
}

/// Returns journal-specific macro rules for the given profile ID.
/// These supplement (not replace) the builtin rules.
pub fn journal_rules(profile_id: &str) -> Vec<MacroRule> {
    match profile_id {
        "jos-paper" | "jos-paper-toml" => jos_rules(),
        "tacl" | "acl" => tacl_rules(),
        "cvpr" | "iccv" => cvpr_rules(),
        "nature" => nature_rules(),
        "springer" | "svjour3" | "llncs" => springer_rules(),
        "chinese-academic" | "chinese-academic-toml" => chinese_academic_rules(),
        _ => Vec::new(),
    }
}

fn rule(name: &str, arity: usize, output: RuleOutput, desc: &str) -> MacroRule {
    MacroRule {
        id: name.into(),
        name: name.into(),
        arity,
        output,
        description: Some(desc.into()),
    }
}

// ── IEEE / JOS rules ───────────────────────────────────────────────────

fn jos_rules() -> Vec<MacroRule> {
    vec![
        rule(
            "IEEEauthorblockN",
            1,
            RuleOutput::AuthorList { content_arg: 0 },
            "IEEE author name block",
        ),
        rule(
            "IEEEauthorblockA",
            1,
            RuleOutput::Affiliation { content_arg: 0 },
            "IEEE author affiliation block",
        ),
        rule(
            "IEEEkeywords",
            1,
            RuleOutput::KeywordList {
                content_arg: 0,
                separator: ",".into(),
            },
            "IEEE paper keywords",
        ),
        rule(
            "markboth",
            2,
            RuleOutput::MetadataField {
                key: "markboth".into(),
                content_arg: 0,
            },
            "Mark both running heads",
        ),
        rule(
            "IEEEpeerreviewmaketitle",
            0,
            RuleOutput::MetadataField {
                key: "IEEEpeerreviewmaketitle".into(),
                content_arg: 0,
            },
            "IEEE peer review maketitle",
        ),
        rule(
            "citet",
            1,
            RuleOutput::Citation {
                keys_arg: 0,
                style: "textual".into(),
            },
            "IEEE textual citation",
        ),
        rule(
            "citep",
            1,
            RuleOutput::Citation {
                keys_arg: 0,
                style: "parenthetical".into(),
            },
            "IEEE parenthetical citation",
        ),
    ]
}

// ── ACL / TACL rules ────────────────────────────────────────────────────

fn tacl_rules() -> Vec<MacroRule> {
    vec![
        rule(
            "aclfinalcopy",
            0,
            RuleOutput::MetadataField {
                key: "aclfinalcopy".into(),
                content_arg: 0,
            },
            "ACL/TACL final copy marker",
        ),
        rule(
            "aclpaperid",
            0,
            RuleOutput::MetadataField {
                key: "aclpaperid".into(),
                content_arg: 0,
            },
            "ACL/TACL paper ID",
        ),
        rule(
            "citet",
            1,
            RuleOutput::Citation {
                keys_arg: 0,
                style: "textual".into(),
            },
            "ACL textual citation",
        ),
        rule(
            "citep",
            1,
            RuleOutput::Citation {
                keys_arg: 0,
                style: "parenthetical".into(),
            },
            "ACL parenthetical citation",
        ),
        rule(
            "citealp",
            1,
            RuleOutput::Citation {
                keys_arg: 0,
                style: "textual-no-parens".into(),
            },
            "ACL citation without parentheses",
        ),
        rule(
            "shorttitle",
            1,
            RuleOutput::MetadataField {
                key: "shorttitle".into(),
                content_arg: 0,
            },
            "Short title for headers",
        ),
        rule(
            "name",
            1,
            RuleOutput::AuthorList { content_arg: 0 },
            "Author name (TACL)",
        ),
        rule(
            "address",
            1,
            RuleOutput::Affiliation { content_arg: 0 },
            "Author address (TACL)",
        ),
    ]
}

// ── CVPR / ICCV rules ─────────────────────────────────────────────────

fn cvpr_rules() -> Vec<MacroRule> {
    vec![
        rule(
            "cvprfinalcopy",
            0,
            RuleOutput::MetadataField {
                key: "cvprfinalcopy".into(),
                content_arg: 0,
            },
            "CVPR final copy marker",
        ),
        rule(
            "iccvfinalcopy",
            0,
            RuleOutput::MetadataField {
                key: "iccvfinalcopy".into(),
                content_arg: 0,
            },
            "ICCV final copy marker",
        ),
        rule(
            "cvprPaperID",
            1,
            RuleOutput::MetadataField {
                key: "cvprPaperID".into(),
                content_arg: 0,
            },
            "CVPR paper ID",
        ),
        rule(
            "confName",
            1,
            RuleOutput::MetadataField {
                key: "confName".into(),
                content_arg: 0,
            },
            "Conference name",
        ),
        rule(
            "confYear",
            1,
            RuleOutput::MetadataField {
                key: "confYear".into(),
                content_arg: 0,
            },
            "Conference year",
        ),
        rule(
            "author",
            1,
            RuleOutput::AuthorList { content_arg: 0 },
            "Author name (CVPR)",
        ),
        rule(
            "affiliation",
            1,
            RuleOutput::Affiliation { content_arg: 0 },
            "Affiliation (CVPR)",
        ),
    ]
}

// ── Nature rules ───────────────────────────────────────────────────────

fn nature_rules() -> Vec<MacroRule> {
    vec![
        rule(
            "corres",
            0,
            RuleOutput::MetadataField {
                key: "correspondence".into(),
                content_arg: 0,
            },
            "Corresponding author marker",
        ),
        rule(
            "equalcont",
            0,
            RuleOutput::MetadataField {
                key: "equalcont".into(),
                content_arg: 0,
            },
            "Equal contribution marker",
        ),
        rule(
            "affil",
            1,
            RuleOutput::Affiliation { content_arg: 0 },
            "Author affiliation (Nature)",
        ),
        rule(
            "maketitle",
            0,
            RuleOutput::Ignore,
            "Nature maketitle (suppress in DOCX)",
        ),
    ]
}

// ── Springer rules ──────────────────────────────────────────────────────

fn springer_rules() -> Vec<MacroRule> {
    vec![
        rule(
            "institute",
            1,
            RuleOutput::Affiliation { content_arg: 0 },
            "Author institute (Springer)",
        ),
        rule(
            "titlerunning",
            1,
            RuleOutput::MetadataField {
                key: "titlerunning".into(),
                content_arg: 0,
            },
            "Running title",
        ),
        rule(
            "authorrunning",
            1,
            RuleOutput::MetadataField {
                key: "authorrunning".into(),
                content_arg: 0,
            },
            "Running author names",
        ),
        rule(
            "email",
            1,
            RuleOutput::MetadataField {
                key: "email".into(),
                content_arg: 0,
            },
            "Author email",
        ),
        rule(
            "orcidID",
            1,
            RuleOutput::MetadataField {
                key: "orcid".into(),
                content_arg: 0,
            },
            "ORCID identifier",
        ),
        rule(
            "keywords",
            1,
            RuleOutput::KeywordList {
                content_arg: 0,
                separator: ",".into(),
            },
            "Springer keywords",
        ),
    ]
}

// ── Chinese academic rules ──────────────────────────────────────────────

fn chinese_academic_rules() -> Vec<MacroRule> {
    vec![
        rule("zihao", 1, RuleOutput::Ignore, "Chinese font size command"),
        rule("songti", 0, RuleOutput::Ignore, "Chinese Song typeface"),
        rule("heiti", 0, RuleOutput::Ignore, "Chinese Hei typeface"),
        rule("kaishu", 0, RuleOutput::Ignore, "Chinese Kai typeface"),
        rule(
            "fangsong",
            0,
            RuleOutput::Ignore,
            "Chinese FangSong typeface",
        ),
        rule("CTEXsetup", 2, RuleOutput::Ignore, "CTeX setup command"),
        rule("ctexset", 1, RuleOutput::Ignore, "CTeX configuration"),
        rule(
            "keywords",
            1,
            RuleOutput::KeywordList {
                content_arg: 0,
                separator: "\u{ff1b}".into(),
            },
            "Chinese academic keywords (semicolon-separated)",
        ),
        rule(
            "zhabstract",
            1,
            RuleOutput::MetadataField {
                key: "zhabstract".into(),
                content_arg: 0,
            },
            "Chinese abstract",
        ),
        rule(
            "enabstract",
            1,
            RuleOutput::MetadataField {
                key: "enabstract".into(),
                content_arg: 0,
            },
            "English abstract",
        ),
    ]
}
