//! Rule Engine for Tex2Doc
//!
//! Provides deterministic macro rules and optional AI inference for handling
//! unknown LaTeX macros during document conversion.
//!
//! # Default behavior
//!
//! - AI fallback **disabled** by default
//! - Unknown macros produce a warning + conservative text fallback
//! - All decisions are recorded in an audit cache for review
//!
//! # Architecture
//!
//! ```text
//! unknown_macro
//!   -> collect context
//!   -> rule registry lookup (builtin + loaded rules)
//!   -> [optional] AI inference
//!   -> audit record
//!   -> user-reviewable decision
//! ```

mod audit;
mod builtin_rules;
mod registry;
mod rule_output;
mod rule_engine;

pub use audit::{AuditCache, AuditRecord, DecisionSource};
pub use registry::{MacroRule, RuleRegistry};
pub use rule_engine::RuleEngine;
pub use rule_engine::RuleEngineConfig;
pub use rule_output::RuleOutput;
