//! 质量层核心类型。
//!
//! 设计见 `docs/study/08-pdf-pipeline/04-quality-comparison.md` §4.4。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 三层质量层。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    Structural,
    Textual,
    Visual,
}

impl Layer {
    pub fn as_str(self) -> &'static str {
        match self {
            Layer::Structural => "structural",
            Layer::Textual => "textual",
            Layer::Visual => "visual",
        }
    }
}

/// 检查项的严重级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// 阻断 CI（exit 1）。
    Critical,
    /// 中等，阻断 CI（exit 1）。
    Major,
    /// 仅报告（exit 0/2）。
    Minor,
    /// 仅展示。
    Info,
}

/// 单条检查项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub severity: Severity,
    pub expected: String,
    pub actual: String,
    pub passed: bool,
    pub note: Option<String>,
}

impl Check {
    /// 简便构造。
    pub fn new(
        name: impl Into<String>,
        severity: Severity,
        expected: impl Into<String>,
        actual: impl Into<String>,
        passed: bool,
    ) -> Self {
        Self {
            name: name.into(),
            severity,
            expected: expected.into(),
            actual: actual.into(),
            passed,
            note: None,
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// 单层结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerResult {
    pub layer: Layer,
    pub passed: bool,
    pub checks: Vec<Check>,
}

impl LayerResult {
    pub fn new(layer: Layer, checks: Vec<Check>) -> Self {
        let passed = checks.iter().all(|c| c.passed);
        Self { layer, passed, checks }
    }
}

/// 单个 marker 在三处的命中情况（V2 扩展：含 rust_pdf 命中列）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkerHit {
    pub marker: String,
    pub in_docx: bool,
    pub in_oracle_pdf: bool,
    pub in_rust_pdf: bool,
}

/// V2 顶层报告。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    pub docx: PathBuf,
    pub rust_pdf: PathBuf,
    pub oracle_pdf: PathBuf,
    pub passed: bool,
    pub exit_code: i32,
    pub layer_results: Vec<LayerResult>,
    pub marker_coverage: Vec<MarkerHit>,
    pub docx_chars: usize,
    pub rust_pdf_chars: usize,
    pub oracle_pdf_chars: usize,
    pub char_ratio_docx_to_oracle: f64,
    pub char_ratio_rust_to_oracle: f64,
    pub paragraphs: usize,
}

impl QualityReport {
    /// 收集所有 fail 的检查项。
    pub fn failed_checks(&self) -> Vec<&Check> {
        self.layer_results
            .iter()
            .flat_map(|l| l.checks.iter())
            .filter(|c| !c.passed)
            .collect()
    }
}
