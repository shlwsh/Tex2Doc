//! Rule registry: stores and looks up macro rules.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// A named macro rule: declares how a specific macro should be processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroRule {
    /// Unique identifier for this rule.
    pub id: String,
    /// LaTeX macro name (without backslash), e.g. `"textbf"` or `"mycompanytable"`.
    pub name: String,
    /// How many mandatory arguments this macro expects.
    pub arity: usize,
    /// What to do when this macro is encountered.
    pub output: super::RuleOutput,
    /// Optional description for documentation / audit.
    pub description: Option<String>,
}

/// A rule registry that stores builtin and user-loaded rules.
#[derive(Debug, Default)]
pub struct RuleRegistry {
    rules: HashMap<String, Arc<MacroRule>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a rule. Overwrites any existing rule with the same name.
    pub fn register(&mut self, rule: MacroRule) {
        let key = rule.name.clone();
        self.rules.insert(key, Arc::new(rule));
    }

    /// Look up a rule by macro name. Returns `None` if no rule matches.
    pub fn lookup(&self, name: &str) -> Option<Arc<MacroRule>> {
        self.rules.get(name).cloned()
    }

    /// Returns true if the registry has a rule for the given macro name.
    pub fn has_rule(&self, name: &str) -> bool {
        self.rules.contains_key(name)
    }

    /// Number of registered rules.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Iterate over all rules.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<MacroRule>> {
        self.rules.values()
    }

    /// Remove a rule by name.
    pub fn remove(&mut self, name: &str) -> Option<Arc<MacroRule>> {
        self.rules.remove(name)
    }

    /// Load rules from a JSON array.
    pub fn load_from_json(&mut self, json: &str) -> Result<usize, serde_json::Error> {
        let rules: Vec<MacroRule> = serde_json::from_str(json)?;
        let count = rules.len();
        for rule in rules {
            self.register(rule);
        }
        Ok(count)
    }

    /// Export all rules as a JSON array.
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        let rules: Vec<&MacroRule> = self.rules.values().map(|r| r.as_ref()).collect();
        serde_json::to_string_pretty(&rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuleOutput;

    fn make_rule(name: &str, arity: usize, output: RuleOutput) -> MacroRule {
        MacroRule {
            id: name.to_string(),
            name: name.to_string(),
            arity,
            output,
            description: None,
        }
    }

    #[test]
    fn registry_insert_lookup() {
        let mut reg = RuleRegistry::new();
        reg.register(make_rule(
            "textbf",
            1,
            RuleOutput::InlineText { content_arg: 0 },
        ));
        assert!(reg.has_rule("textbf"));
        assert!(reg.lookup("textbf").is_some());
        assert_eq!(reg.lookup("textbf").unwrap().arity, 1);
        assert!(reg.lookup("unknown").is_none());
    }

    #[test]
    fn registry_json_roundtrip() {
        let mut reg = RuleRegistry::new();
        reg.register(make_rule(
            "mycompanytable",
            2,
            RuleOutput::Table { body_arg: 0 },
        ));
        let json = reg.export_json().unwrap();
        let mut reg2 = RuleRegistry::new();
        reg2.load_from_json(&json).unwrap();
        assert_eq!(reg2.len(), 1);
        let r = reg2.lookup("mycompanytable").unwrap();
        assert_eq!(r.arity, 2);
    }

    #[test]
    fn registry_overwrite() {
        let mut reg = RuleRegistry::new();
        reg.register(make_rule("foo", 1, RuleOutput::Ignore));
        reg.register(make_rule("foo", 2, RuleOutput::Ignore));
        assert_eq!(reg.lookup("foo").unwrap().arity, 2);
    }
}
