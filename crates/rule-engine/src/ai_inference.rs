//! AI-powered macro inference (requires `ai-fallback` feature).
//!
//! When enabled, unknown macros are sent to an OpenAI-compatible API for
//! semantic classification. All AI interactions are recorded in the audit
//! trail with confidence scores and prompt hashes.

use crate::audit::{AuditCache, AuditRecord, DecisionSource};
use crate::rule_output::RuleOutput;

/// Build a classification prompt for the AI model.
pub fn build_prompt(macro_name: &str, arity: usize, context: &str) -> String {
    format!(
        r#"Classify the LaTeX macro `\{}` (arity={}) in the following context:

```
{}
```

Respond with ONLY a JSON object like:
{{"type": "inline_text", "confidence": 0.95}}
or
{{"type": "paragraph", "body_arg": 0, "confidence": 0.88}}

Allowed types:
- "inline_text": inline formatting like \textit, \textbf, \emph
- "paragraph": starts a paragraph like \item, \note
- "heading": section heading like \section, \subsection
- "figure": figure/graphic like \plot, \diagram
- "table": table content like \tabledata
- "ignore": formatting macro to skip like \kern, \hfil
- "verbatim": preserve as-is text
- "unknown": cannot determine

Return only the JSON object, no explanation."#,
        macro_name, arity, context
    )
}

/// AI inference result.
#[derive(Debug)]
pub struct AiInference {
    /// The inferred output type.
    pub output: RuleOutput,
    /// Confidence score [0.0, 1.0].
    pub confidence: f32,
    /// Model name that produced the inference (e.g. "gpt-4o-mini").
    pub model: String,
}

/// Perform AI inference for a macro using an OpenAI-compatible API.
///
/// Uses the blocking `reqwest` client. Fails gracefully if the API is
/// unavailable; the caller should fall back to conservative behavior.
#[cfg(feature = "ai-fallback")]
pub fn infer_macro(
    macro_name: &str,
    arity: usize,
    context: &str,
    api_url: &str,
    api_key: Option<&str>,
) -> Result<AiInference, Box<dyn std::error::Error + Send + Sync>> {
    use reqwest::blocking::Client;

    let client = Client::new();
    let mut req = client
        .post(api_url)
        .header("Content-Type", "application/json");

    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }

    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            {"role": "system", "content": "You are a LaTeX document analysis assistant."},
            {"role": "user", "content": build_prompt(macro_name, arity, context)}
        ],
        "temperature": 0.1,
        "max_tokens": 100,
    });

    let resp = req.json(&body).send()?;
    let json: serde_json::Value = resp.json()?;

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("no content in AI response")?;

    // Parse the JSON response, handling optional markdown code-block wrapping.
    let result: serde_json::Value = serde_json::from_str(content)
        .or_else(|_| {
            let stripped = content
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```");
            let stripped = stripped.trim_end_matches("```").trim();
            serde_json::from_str(stripped)
        })
        .map_err(|e| format!("failed to parse AI response: {e}: {content}"))?;

    let type_str = result["type"].as_str().unwrap_or("unknown");
    let confidence = result["confidence"].as_f64().unwrap_or(0.5) as f32;

    let output = match type_str {
        "inline_text" => RuleOutput::InlineText { content_arg: 0 },
        "paragraph" => RuleOutput::Paragraph {
            body_arg: result.get("body_arg").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
        },
        "heading" => RuleOutput::Heading {
            level: result.get("level").and_then(|v| v.as_u64()).unwrap_or(1) as u8,
            text_arg: 0,
        },
        "figure" => RuleOutput::Figure { body_arg: 0 },
        "table" => RuleOutput::Table { body_arg: 0 },
        "ignore" => RuleOutput::Ignore,
        "verbatim" => RuleOutput::Verbatim,
        _ => RuleOutput::InlineText { content_arg: 0 },
    };

    Ok(AiInference {
        output,
        confidence,
        model: json["model"].as_str().unwrap_or("unknown").to_string(),
    })
}

/// Compute a SHA-256 hash of a prompt for audit-trail deduplication.
#[cfg(feature = "ai-fallback")]
pub fn compute_prompt_hash(prompt: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(prompt.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ─── Stubs when the feature is disabled ───────────────────────────────────────

#[cfg(not(feature = "ai-fallback"))]
pub mod stub {
    use super::*;

    pub fn build_prompt(_macro_name: &str, _arity: usize, _context: &str) -> String {
        String::new()
    }

    pub fn infer_macro(
        _macro_name: &str,
        _arity: usize,
        _context: &str,
        _api_url: &str,
        _api_key: Option<&str>,
    ) -> Result<AiInference, Box<dyn std::error::Error + Send + Sync>> {
        Err("ai-fallback feature not enabled".into())
    }

    pub fn compute_prompt_hash(_prompt: &str) -> String {
        "disabled".to_string()
    }
}
