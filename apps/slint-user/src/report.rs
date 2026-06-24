//! P5: Report display and formatting for the desktop client.
//!
//! Formats `CompileReport` and `QualityGateResult` into human-readable strings
//! for display in the UI status panel.

use doc_compiler_engine::CompileReport;

/// P5: Formatted summary of a conversion report for UI display.
#[derive(Debug, Clone)]
pub struct ReportSummary {
    pub profile: String,
    pub display_name: String,
    pub source: String,
    pub confidence: Option<f32>,
    pub compatibility_score: u8,
    pub backend: String,
    pub quality_status: String,
    pub quality_score: String,
    pub checks_passed: usize,
    pub checks_total: usize,
    pub warnings: Vec<String>,
    pub block_count: usize,
    pub image_count: usize,
    pub citation_count: usize,
    pub docx_bytes: usize,
}

impl ReportSummary {
    /// Build a summary from a `CompileReport`.
    pub fn from_report(report: &CompileReport) -> Self {
        let ap = report.active_profile.as_ref();
        let qg = report.quality_gate.as_ref();

        let checks_passed = qg.map(|q| q.passed_checks).unwrap_or(0);
        let checks_total = qg.map(|q| q.total_checks).unwrap_or(0);
        let warnings: Vec<String> = qg
            .map(|q| {
                q.warnings
                    .iter()
                    .map(|c| format!("{}: {}", c.name, c.message))
                    .collect()
            })
            .unwrap_or_default();

        Self {
            profile: ap
                .map(|p| p.id.clone())
                .unwrap_or_else(|| report.profile.id().to_string()),
            display_name: ap.map(|p| p.display_name.clone()).unwrap_or_default(),
            source: ap.map(|p| p.source.clone()).unwrap_or_default(),
            confidence: ap.and_then(|p| p.confidence),
            compatibility_score: report.compatibility.score,
            backend: report.backend.selected.id().to_string(),
            quality_status: qg
                .map(|q| q.status.as_str().to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            quality_score: qg
                .map(|q| q.score.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            checks_passed,
            checks_total,
            warnings,
            block_count: report.block_count,
            image_count: report.image_asset_count,
            citation_count: report.citation_count,
            docx_bytes: report.docx_bytes,
        }
    }

    /// Format as a multi-line string for display in the UI status panel.
    pub fn format_for_ui(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("=== Conversion Report ==="));
        lines.push(format!("Profile: {} ({})", self.profile, self.display_name));
        lines.push(format!("Source: {}", self.source));
        if let Some(conf) = self.confidence {
            lines.push(format!("Confidence: {:.0}%", conf * 100.0));
        }
        lines.push(String::new());
        lines.push(format!("Compatibility: {}%", self.compatibility_score));
        lines.push(format!("Backend: {}", self.backend));
        lines.push(String::new());
        lines.push(format!(
            "Quality Gate: {} (score={})",
            self.quality_status, self.quality_score
        ));
        lines.push(format!(
            "Checks: {}/{} passed",
            self.checks_passed, self.checks_total
        ));
        for warn in &self.warnings {
            lines.push(format!("  WARN: {}", warn));
        }
        lines.push(String::new());
        lines.push(format!("Blocks: {}", self.block_count));
        lines.push(format!("Images: {}", self.image_count));
        lines.push(format!("Citations: {}", self.citation_count));
        lines.push(format!("DOCX: {} bytes", self.docx_bytes));
        lines.join("\n")
    }
}
