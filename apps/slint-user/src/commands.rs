//! P5: CLI command wrappers for the desktop client.
//!
//! Provides profile detection and ID generation helpers used by the UI layer.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// P5: Errors that can occur during desktop commands.
#[derive(Error, Debug)]
#[allow(dead_code)]
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

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        Self::OutputWriteFailed(e.to_string())
    }
}

impl From<zip::result::ZipError> for CommandError {
    fn from(e: zip::result::ZipError) -> Self {
        Self::ConversionFailed(format!("zip error: {}", e))
    }
}

/// P5: Result type for desktop commands.
pub type CommandResult<T> = Result<T, CommandError>;

/// P5: Detect the journal/profile for an uploaded TeX archive.
/// Reads the upload file, extracts it to a temp directory if it's a zip,
/// then runs the profile detection engine.
pub fn detect_profile_from_upload(upload_path: &str) -> CommandResult<String> {
    if upload_path.trim().is_empty() {
        return Err(CommandError::ProjectNotFound("(empty upload path)".to_string()));
    }
    let path = Path::new(upload_path);
    if !path.is_file() {
        return Err(CommandError::ProjectNotFound(upload_path.to_string()));
    }

    let bytes = std::fs::read(path)
        .map_err(|e| CommandError::ProjectNotFound(format!("{}: {}", upload_path, e)))?;

    // Determine if it's a zip by checking magic bytes
    let is_zip = bytes.len() >= 2 && bytes[0] == 0x50 && bytes[1] == 0x4b;

    let temp_dir = tempfile::tempdir()
        .map_err(|e| CommandError::ConversionFailed(format!("temp dir: {}", e)))?;

    if is_zip {
        let reader = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(reader)?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = temp_dir.path().join(file.name());
            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }
    } else {
        // Single file - write with original name in temp
        let dest = temp_dir.path().join(
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("main.tex"),
        );
        std::fs::write(&dest, &bytes)?;
    }

    detect_profile(temp_dir.path())
}

/// P5: Detect the journal/profile for a TeX project directory.
pub fn detect_profile(project_root: &Path) -> CommandResult<String> {
    use doc_compiler_engine::{CompileOptions, ProfileRef, SemanticBackendKind, SemanticTexEngine};

    if !project_root.is_dir() {
        return Err(CommandError::ProjectNotFound(
            project_root.display().to_string(),
        ));
    }

    let main_tex = find_main_tex(project_root)?;

    let options = CompileOptions {
        profile_ref: Some(ProfileRef::Auto),
        semantic_backend: SemanticBackendKind::RuleBased,
        ..Default::default()
    };

    let engine = SemanticTexEngine::new();
    let artifact = engine.compile_dir_to_docx(project_root, &main_tex, &options)?;

    let profile = artifact
        .report
        .active_profile
        .as_ref()
        .map(|ap| ap.id.clone())
        .unwrap_or_else(|| artifact.report.profile.id().to_string());

    Ok(profile)
}

/// P5: Generate a job ID.
pub fn generate_job_id() -> String {
    uuid_simple()
}

/// Find the main .tex file in a project root.
///
/// Strategy (tiered):
/// 1. Exact-name matches for common conventions (`main.tex`, etc.)
/// 2. Files with `\documentclass` — pick the one with the most content
/// 3. Largest `.tex` file by line count as last resort
fn find_main_tex(project_root: &Path) -> CommandResult<PathBuf> {
    let entries = walkdir(project_root)?;

    let candidates: Vec<PathBuf> = entries
        .into_iter()
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()) == Some("tex")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| {
                        !n.starts_with('.') && !n.ends_with(".sty.tex") && !n.ends_with(".def.tex")
                    })
                    .unwrap_or(false)
        })
        .collect();

    if candidates.is_empty() {
        return Err(CommandError::MainTexNotFound(
            project_root.display().to_string(),
        ));
    }

    // Tier 1: exact-name matches (highest priority)
    let exact_names = ["main.tex", "minimal.tex", "paper.tex", "article.tex"];
    for candidate in &candidates {
        let name = candidate.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if exact_names.contains(&name) {
            return Ok(candidate.clone());
        }
    }

    // Tier 2: files containing \documentclass — pick the one with the most lines
    let with_docclass: Vec<PathBuf> = candidates
        .iter()
        .filter(|p| has_document_class(p))
        .cloned()
        .collect();

    if !with_docclass.is_empty() {
        return Ok(best_candidate_by_lines(&with_docclass));
    }

    // Tier 3: fallback to largest file by line count
    Ok(best_candidate_by_lines(&candidates))
}

/// Returns the candidate with the most lines (proxy for "most likely to be the main file").
fn best_candidate_by_lines(candidates: &[PathBuf]) -> PathBuf {
    candidates
        .iter()
        .map(|p| {
            let lines = std::fs::read_to_string(p)
                .map(|s| s.lines().count())
                .unwrap_or(0);
            (p.clone(), lines)
        })
        .max_by_key(|(_, lines)| *lines)
        .map(|(p, _)| p)
        .unwrap_or_else(|| candidates[0].clone())
}

/// Check whether a .tex file contains \documentclass (ignoring comments).
fn has_document_class(path: &Path) -> bool {
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('%') {
            continue;
        }
        if trimmed.contains(r"\documentclass") || trimmed.contains("\\documentclass") {
            return true;
        }
    }
    false
}

/// Recursively walk a directory, collecting all file paths (not dirs).
fn walkdir(root: &Path) -> CommandResult<Vec<PathBuf>> {
    let mut results = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(current) = stack.pop() {
        let entries = std::fs::read_dir(&current)
            .map_err(|e| CommandError::ProjectNotFound(format!("{}: {}", current.display(), e)))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                results.push(path);
            }
        }
    }

    Ok(results)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_profile_from_directory_resolves_main_tex() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let project_root = repo_root.join("examples/journals/generic");
        let profile = detect_profile(&project_root).unwrap();
        assert!(!profile.is_empty());
    }
}
