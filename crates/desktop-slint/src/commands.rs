//! P5: CLI command wrappers for the desktop client.
//!
//! Wraps `SemanticTexEngine` and `doc-core` conversion functions
//! for use by the UI layer.

use crate::app_state::{AppState, JobEntry, JobStatus, JobUpdate};
use doc_compiler_engine::{
    CompileOptions, ProfileRef, SemanticBackendKind, SemanticTexEngine,
};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// P5: Errors that can occur during desktop commands.
#[derive(Error, Debug)]
pub enum CommandError {
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    #[error("main.tex not found in: {0}")]
    MainTexNotFound(String),
    #[error("conversion failed: {0}")]
    ConversionFailed(String),
    #[error("output write failed: {0}")]
    OutputWriteFailed(String),
    #[error("report parse failed: {0}")]
    ReportParseFailed(String),
}

impl From<doc_compiler_engine::EngineError> for CommandError {
    fn from(e: doc_compiler_engine::EngineError) -> Self {
        Self::ConversionFailed(e.to_string())
    }
}

/// P5: Result type for desktop commands.
pub type CommandResult<T> = Result<T, CommandError>;

/// P5: Result of a local conversion.
#[derive(Debug, Clone)]
pub struct LocalConvertResult {
    pub docx_path: PathBuf,
    pub profile: String,
    pub compatibility_score: u8,
    pub quality_status: String,
    pub docx_bytes: usize,
}

/// P5: Detect the journal/profile for a TeX project.
pub fn detect_profile(project_root: &Path) -> CommandResult<String> {
    if !project_root.is_dir() {
        return Err(CommandError::ProjectNotFound(project_root.display().to_string()));
    }

    // Look for main.tex or minimal.tex
    let main_tex = find_main_tex(project_root)?;

    let options = CompileOptions {
        profile_ref: Some(ProfileRef::Auto),
        semantic_backend: SemanticBackendKind::RuleBased,
        ..Default::default()
    };

    let engine = SemanticTexEngine::new();
    let artifact = engine.compile_dir_to_docx(project_root, &main_tex, &options)?;

    let profile = artifact.report.active_profile
        .as_ref()
        .map(|ap| ap.id.clone())
        .unwrap_or_else(|| artifact.report.profile.id().to_string());

    Ok(profile)
}

/// P5: Run a local conversion.
pub fn run_local_convert(
    project_root: &Path,
    output_path: &Path,
    quality: &str,
    app_state: &AppState,
) -> CommandResult<LocalConvertResult> {
    if !project_root.is_dir() {
        return Err(CommandError::ProjectNotFound(project_root.display().to_string()));
    }

    let main_tex = find_main_tex(project_root)?;

    // Determine min score from quality level
    let min_score = match quality {
        "preview" => 60u8,
        "strict" => 90u8,
        _ => 75u8,
    };

    let options = CompileOptions {
        profile_ref: Some(ProfileRef::Auto),
        semantic_backend: SemanticBackendKind::Auto,
        allow_backend_fallback: true,
        min_compatibility_score_override: Some(min_score),
        ..Default::default()
    };

    let engine = SemanticTexEngine::new();
    let artifact = engine.compile_dir_to_docx(project_root, &main_tex, &options)?;

    // Write DOCX
    std::fs::write(output_path, &artifact.docx)
        .map_err(|e| CommandError::OutputWriteFailed(e.to_string()))?;

    let profile = artifact.report.active_profile
        .as_ref()
        .map(|ap| ap.id.clone())
        .unwrap_or_else(|| artifact.report.profile.id().to_string());

    let quality_status = artifact.report.quality_gate
        .as_ref()
        .map(|qg| qg.status.as_str().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    // Record job
    let job = JobEntry {
        id: uuid_simple(),
        project_path: project_root.display().to_string(),
        profile: profile.clone(),
        status: JobStatus::Succeeded,
        output_path: Some(output_path.display().to_string()),
        error: None,
        created_at: chrono_now(),
    };
    app_state.add_job(job);

    Ok(LocalConvertResult {
        docx_path: output_path.to_path_buf(),
        profile,
        compatibility_score: artifact.report.compatibility.score,
        quality_status,
        docx_bytes: artifact.report.docx_bytes,
    })
}

/// P5: Generate a job ID.
pub fn generate_job_id() -> String {
    uuid_simple()
}

/// Find main.tex or minimal.tex in project root.
fn find_main_tex(project_root: &Path) -> CommandResult<PathBuf> {
    let candidates = ["main.tex", "minimal.tex", "paper.tex", "article.tex"];
    for candidate in &candidates {
        let path = project_root.join(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(CommandError::MainTexNotFound(project_root.display().to_string()))
}

/// Simple UUID-like ID generator.
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", now)
}

/// Current timestamp as ISO string.
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // Simple UTC timestamp
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let mins = (remaining % 3600) / 60;
    let seconds = remaining % 60;
    format!("{}d{:02}h{:02}m{:02}s", days, hours, mins, seconds)
}
