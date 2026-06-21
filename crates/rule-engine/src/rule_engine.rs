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

    /// Process a macro with optional AI fallback (requires `ai-fallback` feature).
    ///
    /// First checks the rule registry. If the macro is unknown and AI is enabled
    /// and `api_url` is provided, sends the macro to the configured AI API for
    /// semantic classification. Falls back to conservative `InlineText` if AI
    /// is unavailable or confidence is below threshold.
    ///
    /// Returns the `RuleOutput` for the macro.
    #[cfg(feature = "ai-fallback")]
    pub fn process_with_ai(
        &mut self,
        macro_name: &str,
        arity: usize,
        context: &str,
        api_url: &str,
        api_key: Option<&str>,
    ) -> Option<RuleOutput> {
        // 1. Check registry first
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

        // 2. Try AI inference when enabled
        if self.config.enable_ai {
            let prompt = crate::ai_inference::build_prompt(macro_name, arity, context);
            let prompt_hash = crate::ai_inference::compute_prompt_hash(&prompt);

            match crate::ai_inference::infer_macro(macro_name, arity, context, api_url, api_key) {
                Ok(inference) => {
                    if inference.confidence >= self.config.ai_min_confidence {
                        let record = AuditRecord::new(
                            macro_name.to_string(),
                            arity,
                            inference.output.as_str(),
                            inference.confidence,
                            DecisionSource::AI,
                        )
                        .with_prompt_hash(prompt_hash)
                        .with_ai_model(&inference.model);
                        self.audit.record(record);
                        return Some(inference.output);
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[rule-engine] AI inference failed for \\{}: {e}",
                        macro_name
                    );
                }
            }
        }

        // 3. Fallback: conservative text
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

        Some(RuleOutput::InlineText { content_arg: 0 })
    }

    /// Stub for `process_with_ai` when `ai-fallback` is not enabled.
    /// Logs a warning that AI is not available and falls back to `process_unknown`.
    #[cfg(not(feature = "ai-fallback"))]
    pub fn process_with_ai(
        &mut self,
        macro_name: &str,
        arity: usize,
        _context: &str,
        _api_url: &str,
        _api_key: Option<&str>,
    ) -> Option<RuleOutput> {
        if self.config.enable_ai {
            eprintln!(
                "[rule-engine] AI fallback requested but `ai-fallback` feature is not enabled; \
                 falling back to builtin behavior"
            );
        }
        self.process_unknown(macro_name, arity)
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

    #[cfg(feature = "ai-fallback")]
    #[test]
    fn ai_fallback_stub_builds_prompt() {
        let prompt = crate::ai_inference::build_prompt("mycompanytable", 2, "some context");
        assert!(prompt.contains("mycompanytable"));
        assert!(prompt.contains("2"));
        assert!(prompt.contains("some context"));
    }

    #[cfg(feature = "ai-fallback")]
    #[test]
    fn ai_fallback_stub_infer_fails() {
        let result = crate::ai_inference::infer_macro(
            "foo", 1, "ctx", "http://localhost:9999", None,
        );
        // Should fail gracefully (no server running)
        assert!(result.is_err());
    }

    #[cfg(feature = "ai-fallback")]
    #[test]
    fn process_with_ai_registry_hit() {
        let mut engine = RuleEngine::new();
        engine.set_config(RuleEngineConfig {
            enable_ai: true,
            ai_min_confidence: 0.5,
            ..Default::default()
        });
        // textbf is a builtin rule — should be found in registry, no AI call
        let result = engine.process_with_ai("textbf", 1, "", "http://localhost:9999", None);
        assert!(result.is_some());
        let records = engine.audit_cache().records();
        assert_eq!(records[records.len() - 1].source, DecisionSource::Loaded);
    }

    #[cfg(feature = "ai-fallback")]
    #[test]
    fn process_with_ai_unknown_falls_through() {
        let mut engine = RuleEngine::new();
        engine.set_config(RuleEngineConfig {
            enable_ai: true,
            ai_min_confidence: 0.5,
            ..Default::default()
        });
        // No real API server — should fall through to fallback
        let result = engine.process_with_ai(
            "definitelyunknown", 1, "", "http://localhost:9999", None,
        );
        assert!(result.is_some());
        let records = engine.audit_cache().records();
        assert_eq!(records[records.len() - 1].source, DecisionSource::Fallback);
    }

    #[cfg(not(feature = "ai-fallback"))]
    #[test]
    fn process_with_ai_disabled_warns_and_falls_back() {
        let mut engine = RuleEngine::new();
        engine.set_config(RuleEngineConfig {
            enable_ai: true,
            ..Default::default()
        });
        // Should fall back to builtin process_unknown
        let result = engine.process_with_ai("anymacro", 1, "", "", None);
        assert!(result.is_some());
        assert_eq!(engine.audit_count(), 1);
    }

    #[cfg(feature = "ai-fallback")]
    #[test]
    fn compute_prompt_hash_is_deterministic() {
        let h1 = crate::ai_inference::compute_prompt_hash("hello");
        let h2 = crate::ai_inference::compute_prompt_hash("hello");
        let h3 = crate::ai_inference::compute_prompt_hash("world");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert_eq!(h1.len(), 64); // SHA-256 hex length
    }
}
