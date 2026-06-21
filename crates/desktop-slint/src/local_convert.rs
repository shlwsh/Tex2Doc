//! P5: Local conversion engine adapter.
//!
//! Wraps the `SemanticTexEngine` for local (offline) conversion
//! that does not consume cloud quotas.

use doc_compiler_engine::{
    CompileOptions, CompileArtifact, EngineError, ProfileRef, SemanticBackendKind, SemanticTexEngine,
};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LocalConvertError {
    #[error("engine error: {0}")]
    Engine(#[from] EngineError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("project not found: {0}")]
    ProjectNotFound(String),
}

pub type Result<T> = std::result::Result<T, LocalConvertError>;

/// P5: High-level local conversion function.
/// Does not require auth, does not consume cloud quota.
pub fn convert(
    project_root: &Path,
    main_tex: &Path,
    output_docx: &Path,
    output_report: Option<&Path>,
    profile: &str,
    quality: &str,
) -> Result<CompileArtifact> {
    if !project_root.is_dir() {
        return Err(LocalConvertError::ProjectNotFound(project_root.display().to_string()));
    }

    let min_score = match quality {
        "preview" => 60u8,
        "strict" => 90u8,
        _ => 75u8,
    };

    let profile_ref = match profile {
        "auto" | "" => ProfileRef::Auto,
        id => ProfileRef::Id(id.to_string()),
    };

    let options = CompileOptions {
        profile_ref: Some(profile_ref),
        semantic_backend: SemanticBackendKind::Auto,
        allow_backend_fallback: true,
        min_compatibility_score_override: Some(min_score),
        ..Default::default()
    };

    let engine = SemanticTexEngine::new();
    let artifact = engine.compile_dir_to_docx(project_root, main_tex, &options)?;

    std::fs::write(output_docx, &artifact.docx)?;

    if let Some(report_path) = output_report {
        let json = serde_json::to_string_pretty(&artifact.report)?;
        std::fs::write(report_path, json)?;
    }

    Ok(artifact)
}
