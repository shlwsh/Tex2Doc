//! Routing: RuleOutput → Block conversion.
//!
//! When the rule engine resolves an unknown macro to a `RuleOutput`,
//! this module converts it to the appropriate `Block` variant.

use doc_semantic_ast::{Block, Span, TextRun, TextStyle};

use crate::RuleOutput;

/// Configuration for routing.
#[derive(Debug, Clone)]
pub struct RoutingConfig {
    /// Default heading level for macros resolved as headings without a level.
    pub default_heading_level: u8,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            default_heading_level: 1,
        }
    }
}

/// Convert a `RuleOutput` into a `Block`, given the macro's arguments.
///
/// `args` is the list of macro arguments extracted by the parser.
pub fn route_rule_output(
    output: &RuleOutput,
    args: &[String],
    config: &RoutingConfig,
) -> Option<Block> {
    match output {
        RuleOutput::Heading { level, text_arg } => {
            let level = if *level == 0 {
                config.default_heading_level
            } else {
                *level
            };
            let text = args.get(*text_arg).cloned().unwrap_or_default();
            Some(Block::Heading {
                level,
                text,
                number: None,
                span: Span::default(),
            })
        }
        RuleOutput::Paragraph { body_arg } => {
            let body = args.get(*body_arg).cloned().unwrap_or_default();
            Some(Block::Paragraph {
                runs: vec![TextRun {
                    text: body,
                    style: TextStyle::Plain,
                    span: Span::default(),
                }],
                span: Span::default(),
            })
        }
        RuleOutput::InlineText { .. } => {
            // Inline text doesn't create a block; caller handles it as a run
            None
        }
        RuleOutput::Table { .. } => {
            // Caller handles table routing (requires table parsing logic)
            None
        }
        RuleOutput::Figure { .. } => {
            // Caller handles figure routing (requires figure path extraction)
            None
        }
        RuleOutput::Ignore => None,
        RuleOutput::Verbatim => {
            let text = args.first().cloned().unwrap_or_default();
            Some(Block::RawFallback {
                text,
                span: Span::default(),
            })
        }
        // Journal-specific variants: route to Paragraph-like blocks for now.
        // The actual semantic interpretation (citation/reference/metadata) is handled
        // downstream in the docx renderer.
        RuleOutput::Citation { keys_arg, style } => {
            let keys = args.get(*keys_arg).cloned().unwrap_or_default();
            Some(Block::Paragraph {
                runs: vec![TextRun {
                    text: format!("[citation:{}:{}]", style, keys),
                    style: TextStyle::Plain,
                    span: Span::default(),
                }],
                span: Span::default(),
            })
        }
        RuleOutput::MetadataField { key, content_arg } => {
            let value = args.get(*content_arg).cloned().unwrap_or_default();
            Some(Block::Paragraph {
                runs: vec![TextRun {
                    text: format!("[metadata:{}={}]", key, value),
                    style: TextStyle::Plain,
                    span: Span::default(),
                }],
                span: Span::default(),
            })
        }
        RuleOutput::AuthorList { content_arg } => {
            let value = args.get(*content_arg).cloned().unwrap_or_default();
            Some(Block::Paragraph {
                runs: vec![TextRun {
                    text: format!("[author:{}]", value),
                    style: TextStyle::Plain,
                    span: Span::default(),
                }],
                span: Span::default(),
            })
        }
        RuleOutput::Affiliation { content_arg } => {
            let value = args.get(*content_arg).cloned().unwrap_or_default();
            Some(Block::Paragraph {
                runs: vec![TextRun {
                    text: format!("[affiliation:{}]", value),
                    style: TextStyle::Plain,
                    span: Span::default(),
                }],
                span: Span::default(),
            })
        }
        RuleOutput::KeywordList {
            content_arg,
            separator,
        } => {
            let value = args.get(*content_arg).cloned().unwrap_or_default();
            Some(Block::Paragraph {
                runs: vec![TextRun {
                    text: format!("[keywords{}:{}]", separator, value),
                    style: TextStyle::Plain,
                    span: Span::default(),
                }],
                span: Span::default(),
            })
        }
    }
}

/// Extract the text of a macro argument, stripping outer braces.
pub fn extract_arg_text(arg: &str) -> String {
    let s = arg.trim();
    if s.starts_with('{') && s.ends_with('}') && s.len() >= 2 {
        s[1..s.len() - 1].trim().to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod routing_tests {
    use super::*;

    #[test]
    fn heading_output_becomes_heading_block() {
        let output = RuleOutput::Heading {
            level: 2,
            text_arg: 0,
        };
        let args = vec!["Introduction".to_string()];
        let block = route_rule_output(&output, &args, &RoutingConfig::default());
        match block {
            Some(Block::Heading { level, text, .. }) => {
                assert_eq!(level, 2);
                assert_eq!(text, "Introduction");
            }
            _ => panic!("expected Heading block, got {:?}", block),
        }
    }

    #[test]
    fn heading_with_zero_level_uses_default() {
        let output = RuleOutput::Heading {
            level: 0,
            text_arg: 0,
        };
        let args = vec!["Default Level".to_string()];
        let block = route_rule_output(&output, &args, &RoutingConfig::default());
        match block {
            Some(Block::Heading { level, .. }) => {
                assert_eq!(level, 1); // default is 1
            }
            _ => panic!("expected Heading block"),
        }
    }

    #[test]
    fn paragraph_output_becomes_paragraph_block() {
        let output = RuleOutput::Paragraph { body_arg: 0 };
        let args = vec!["This is a paragraph.".to_string()];
        let block = route_rule_output(&output, &args, &RoutingConfig::default());
        match block {
            Some(Block::Paragraph { runs, .. }) => {
                assert_eq!(runs.len(), 1);
                assert_eq!(runs[0].text, "This is a paragraph.");
                assert_eq!(runs[0].style, TextStyle::Plain);
            }
            _ => panic!("expected Paragraph block, got {:?}", block),
        }
    }

    #[test]
    fn inline_text_returns_none() {
        let output = RuleOutput::InlineText { content_arg: 0 };
        let block = route_rule_output(&output, &[], &RoutingConfig::default());
        assert!(block.is_none());
    }

    #[test]
    fn ignore_returns_none() {
        let output = RuleOutput::Ignore;
        let block = route_rule_output(&output, &[], &RoutingConfig::default());
        assert!(block.is_none());
    }

    #[test]
    fn verbatim_returns_raw_fallback() {
        let output = RuleOutput::Verbatim;
        let args = vec!["\\verbatim{content}".to_string()];
        let block = route_rule_output(&output, &args, &RoutingConfig::default());
        match block {
            Some(Block::RawFallback { text, .. }) => {
                assert_eq!(text, "\\verbatim{content}");
            }
            _ => panic!("expected RawFallback block, got {:?}", block),
        }
    }

    #[test]
    fn table_returns_none() {
        let output = RuleOutput::Table { body_arg: 0 };
        let block = route_rule_output(&output, &[], &RoutingConfig::default());
        assert!(block.is_none());
    }

    #[test]
    fn figure_returns_none() {
        let output = RuleOutput::Figure { body_arg: 0 };
        let block = route_rule_output(&output, &[], &RoutingConfig::default());
        assert!(block.is_none());
    }

    #[test]
    fn extract_arg_text_strips_braces() {
        assert_eq!(extract_arg_text("{hello world}"), "hello world");
        assert_eq!(extract_arg_text("plain"), "plain");
        assert_eq!(extract_arg_text("  {spaced}  "), "spaced");
    }

    #[test]
    fn extract_arg_text_handles_nested_braces() {
        // Only strips outermost braces
        assert_eq!(extract_arg_text("{{nested}}"), "{nested}");
        assert_eq!(extract_arg_text(""), "");
    }

    #[test]
    fn route_with_missing_arg_returns_empty() {
        let output = RuleOutput::Heading {
            level: 1,
            text_arg: 5, // out of bounds
        };
        let args = vec!["Only one arg".to_string()];
        let block = route_rule_output(&output, &args, &RoutingConfig::default());
        match block {
            Some(Block::Heading { text, .. }) => {
                assert_eq!(text, ""); // empty string for missing arg
            }
            _ => panic!("expected Heading block"),
        }
    }
}
