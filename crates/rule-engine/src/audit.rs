//! Audit cache for rule engine decisions.

use serde::{Deserialize, Serialize};

/// Source of the rule decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DecisionSource {
    /// Builtin fallback (conservative text)
    Fallback,
    /// Matched a builtin rule
    Builtin,
    /// Matched a user-loaded rule
    Loaded,
    /// Inferred by optional AI engine
    AI,
    /// User explicitly accepted/rejected
    UserOverride,
}

/// A single audit record for one unknown macro decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    /// LaTeX macro name (without backslash).
    pub macro_name: String,
    /// Number of arguments the macro takes (if known).
    pub arity: usize,
    /// What the engine decided to do with this macro.
    pub decision: String,
    /// Confidence score [0.0, 1.0].
    pub confidence: f32,
    /// Source of the decision.
    pub source: DecisionSource,
    /// SHA-256 hash of the AI prompt (if applicable).
    pub prompt_hash: Option<String>,
    /// Whether this decision was accepted by the user.
    pub accepted: Option<bool>,
    /// Source file where the macro was encountered.
    pub source_file: Option<String>,
    /// Source line number.
    pub source_line: Option<u32>,
    /// Optional rule ID (for loaded rules).
    pub rule_id: Option<String>,
}

impl AuditRecord {
    /// Create a new audit record.
    pub fn new(
        macro_name: String,
        arity: usize,
        decision: &str,
        confidence: f32,
        source: DecisionSource,
    ) -> Self {
        Self {
            macro_name,
            arity,
            decision: decision.to_string(),
            confidence,
            source,
            prompt_hash: None,
            accepted: None,
            source_file: None,
            source_line: None,
            rule_id: None,
        }
    }

    /// Mark this record as accepted or rejected by the user.
    pub fn with_user_decision(mut self, accepted: bool) -> Self {
        self.accepted = Some(accepted);
        self.source = DecisionSource::UserOverride;
        self
    }

    /// Add source location context.
    pub fn with_location(mut self, file: String, line: u32) -> Self {
        self.source_file = Some(file);
        self.source_line = Some(line);
        self
    }
}

/// In-memory audit cache for the current conversion session.
#[derive(Debug, Default)]
pub struct AuditCache {
    records: Vec<AuditRecord>,
}

impl AuditCache {
    pub fn new() -> Self {
        Self { records: Vec::new() }
    }

    /// Record a decision.
    pub fn record(&mut self, record: AuditRecord) {
        self.records.push(record);
    }

    /// Iterate over all records.
    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }

    /// Iterate over all records mutably.
    pub fn records_mut(&mut self) -> &mut Vec<AuditRecord> {
        &mut self.records
    }

    /// Number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Serialize the audit cache to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.records)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let records: Vec<AuditRecord> = serde_json::from_str(json)?;
        Ok(Self { records })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_record_basics() {
        let record = AuditRecord::new(
            "mycompanytable".to_string(),
            2,
            "table",
            0.72,
            DecisionSource::AI,
        );
        assert_eq!(record.macro_name, "mycompanytable");
        assert_eq!(record.decision, "table");
        assert!(record.accepted.is_none());

        let accepted = record.clone().with_user_decision(true);
        assert_eq!(accepted.accepted, Some(true));
        assert_eq!(accepted.source, DecisionSource::UserOverride);

        let with_loc = record.with_location("main.tex".to_string(), 42);
        assert_eq!(with_loc.source_file, Some("main.tex".to_string()));
        assert_eq!(with_loc.source_line, Some(42));
    }

    #[test]
    fn audit_cache_roundtrip() {
        let mut cache = AuditCache::new();
        cache.record(AuditRecord::new(
            "foo".to_string(),
            1,
            "paragraph",
            0.9,
            DecisionSource::Builtin,
        ));
        let json = cache.to_json().unwrap();
        let cache2 = AuditCache::from_json(&json).unwrap();
        assert_eq!(cache2.len(), 1);
        assert_eq!(cache2.records[0].macro_name, "foo");
    }
}
