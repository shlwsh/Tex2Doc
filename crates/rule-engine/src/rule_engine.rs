//! Main rule engine: orchestrates rule lookup, AI inference, and audit recording.

use super::audit::AuditCache;
use super::builtin_rules::builtin_rules;
use super::registry::RuleRegistry;
use super::rule_output::RuleOutput;
use super::audit::AuditRecord;
use super::audit::DecisionSource;

/// Configuration for the rule engine.
#[derive(Debug, Clone)]
pub struct RuleEngineConfig {
    /// Whether to enable AI inference (requires network).
    pub enable_ai: bool,
    /// Minimum confidence threshold for accepting an AI inference.
    pub ai_min_confidence: f32,
    /// Emit warnings for unknown macros.
    pub warn_on_unknown: bool,
}

impl Default for RuleEngineConfig {
    fn default() -> Self {
        Self {
            enable_ai: false,
            ai_min_confidence: 0.7,
            warn_on_unknown: true,
        }
    }
}

/// Main rule engine for processing unknown LaTeX macros.
#[derive(Debug)]
pub struct RuleEngine {
    registry: RuleRegistry,
    audit: AuditCache,
    config: RuleEngineConfig,
}

impl RuleEngine {
    /// Create a new rule engine with builtin rules loaded.
    pub fn new() -> Self {
        let mut engine = Self {
            registry: RuleRegistry::new(),
            audit: AuditCache::new(),
            config: RuleEngineConfig::default(),
        };
        for rule in builtin_rules() {
            engine.registry.register(rule);
        }
        engine
    }

    /// Create with an existing registry (no builtin rules loaded).
    pub fn with_registry(registry: RuleRegistry) -> Self {
        Self {
            registry,
            audit: AuditCache::new(),
            config: RuleEngineConfig::default(),
        }
    }

    /// Update the engine configuration.
    pub fn set_config(&mut self, config: RuleEngineConfig) {
        self.config = config;
    }

    /// Current configuration.
    pub fn config(&self) -> &RuleEngineConfig {
        &self.config
    }

    /// Reference to the audit cache.
    pub fn audit_cache(&self) -> &AuditCache {
        &self.audit
    }

    /// Mutable reference to the audit cache.
    pub fn audit_cache_mut(&mut self) -> &mut AuditCache {
        &mut self.audit
    }

    /// Reference to the rule registry.
    pub fn registry(&self) -> &RuleRegistry {
        &self.registry
    }

    /// Mutable reference to the rule registry.
    pub fn registry_mut(&mut self) -> &mut RuleRegistry {
        &mut self.registry
    }

    /// Load additional rules from JSON.
    pub fn load_rules(&mut self, json: &str) -> Result<usize, serde_json::Error> {
        self.registry.load_from_json(json)
    }

    /// Export the current audit log as JSON.
    pub fn export_audit(&self) -> Result<String, serde_json::Error> {
        self.audit.to_json()
    }

    /// Process an unknown macro and return the rule output.
    /// Records the decision in the audit cache.
    ///
    /// Returns `None` if the macro is unknown and AI is disabled.
    pub fn process_unknown(
        &mut self,
        macro_name: &str,
        arity: usize,
    ) -> Option<RuleOutput> {
        // 1. Check registry
        if let Some(rule) = self.registry.lookup(macro_name) {
            let record = AuditRecord::new(
                macro_name.to_string(),
                arity,
                rule.output.as_str(),
                1.0,
                DecisionSource::Loaded,
            );
            self.audit.record(record);
            return Some(rule.output.clone());
        }

        // 2. Fallback: conservative text
        let record = AuditRecord::new(
            macro_name.to_string(),
            arity,
            "verbatim",
            1.0,
            DecisionSource::Fallback,
        );
        self.audit.record(record);

        if self.config.warn_on_unknown {
            eprintln!(
                "[rule-engine] unknown macro: \\{} (arity={}) — using verbatim fallback",
                macro_name, arity
            );
        }

        // Return a static fallback: verbatim (keep as text)
        Some(RuleOutput::InlineText { content_arg: 0 })
    }

    /// Number of audit records.
    pub fn audit_count(&self) -> usize {
        self.audit.len()
    }

    /// Whether the engine has encountered any unknown macros.
    pub fn has_unknown(&self) -> bool {
        !self.audit.is_empty()
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{AuditCache, AuditRecord, DecisionSource, MacroRule, RuleRegistry, RuleEngine, RuleEngineConfig, RuleOutput};

    #[test]
    fn builtin_rules_loaded() {
        let engine = RuleEngine::new();
        // textbf should be in the registry
        assert!(engine.registry().has_rule("textbf"));
        // A custom unknown macro should not be
        assert!(!engine.registry().has_rule("mycompanytable"));
    }

    #[test]
    fn process_unknown_fallback() {
        let mut engine = RuleEngine::new();
        let result = engine.process_unknown("myunknown", 1);
        assert!(result.is_some());
        assert_eq!(engine.audit_count(), 1);
        let records = engine.audit_cache().records();
        assert_eq!(records[0].macro_name, "myunknown");
        assert_eq!(records[0].source, DecisionSource::Fallback);
    }

    #[test]
    fn process_builtin_rule() {
        let mut engine = RuleEngine::new();
        let result = engine.process_unknown("textbf", 1);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), RuleOutput::InlineText { content_arg: 0 }));
        assert_eq!(engine.audit_count(), 1);
        // Source should be Loaded (since it came from registry)
        assert_eq!(engine.audit_cache().records()[0].source, DecisionSource::Loaded);
    }

    #[test]
    fn load_custom_rules() {
        let mut engine = RuleEngine::new();
        let json = r#"[
            {"id":"custom-rule","name":"mycompanytable","arity":2,"output":{"Table":{"body_arg":0}},"description":"Custom company table"}
        ]"#;
        engine.load_rules(json).unwrap();
        assert!(engine.registry().has_rule("mycompanytable"));
        let result = engine.process_unknown("mycompanytable", 2);
        assert!(matches!(result.unwrap(), RuleOutput::Table { .. }));
    }

    #[test]
    fn export_audit() {
        let mut engine = RuleEngine::new();
        engine.process_unknown("foo", 1);
        let json = engine.export_audit().unwrap();
        assert!(json.contains("foo"));
        assert!(json.contains("Fallback"));
    }

    #[test]
    fn disable_warnings() {
        let mut engine = RuleEngine::new();
        engine.set_config(RuleEngineConfig {
            warn_on_unknown: false,
            ..Default::default()
        });
        // Should not panic even with stderr
        engine.process_unknown("silent", 1);
        assert_eq!(engine.audit_count(), 1);
    }
}
