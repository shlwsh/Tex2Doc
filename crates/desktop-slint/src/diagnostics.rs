//! P5: Diagnostic bundle export for desktop support workflows.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

#[derive(Debug, Clone)]
pub struct DiagnosticInput {
    pub project_path: String,
    pub output_path: String,
    pub api_base_url: String,
    pub profile: String,
    pub quality: String,
    pub status_text: String,
    pub recent_jobs: String,
    pub update_status: String,
    pub app_version: String,
}

#[derive(Debug, Error)]
pub enum DiagnosticError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("serialize error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("clock error: {0}")]
    Clock(String),
}

pub type Result<T> = std::result::Result<T, DiagnosticError>;

#[derive(Debug, Serialize)]
struct DiagnosticManifest<'a> {
    app_version: &'a str,
    platform: &'static str,
    arch: &'static str,
    generated_at_unix: u64,
    project_path: &'a str,
    output_path: &'a str,
    api_base_url: &'a str,
    profile: &'a str,
    quality: &'a str,
    update_status: &'a str,
    report_path: Option<String>,
    report_included: bool,
}

pub fn export_diagnostic_bundle(input: &DiagnosticInput) -> Result<PathBuf> {
    let generated_at_unix = unix_timestamp()?;
    let bundle_path =
        diagnostic_bundle_path(&input.output_path, &input.project_path, generated_at_unix)?;
    let report_path = report_path_for_output(&input.output_path);
    let report_included = report_path
        .as_ref()
        .map(|path| path.is_file())
        .unwrap_or(false);
    if let Some(parent) = bundle_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let manifest = DiagnosticManifest {
        app_version: &input.app_version,
        platform: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        generated_at_unix,
        project_path: &input.project_path,
        output_path: &input.output_path,
        api_base_url: &input.api_base_url,
        profile: &input.profile,
        quality: &input.quality,
        update_status: &input.update_status,
        report_path: report_path.as_ref().map(|path| path.display().to_string()),
        report_included,
    };

    let file = File::create(&bundle_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("diagnostics.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;

    zip.start_file("status.txt", options)?;
    zip.write_all(input.status_text.as_bytes())?;

    zip.start_file("recent_jobs.txt", options)?;
    zip.write_all(input.recent_jobs.as_bytes())?;

    if let Some(report_path) = report_path.filter(|path| path.is_file()) {
        zip.start_file("compile-report.json", options)?;
        let report = fs::read(report_path)?;
        zip.write_all(&report)?;
    }

    zip.finish()?;
    Ok(bundle_path)
}

fn diagnostic_bundle_path(
    output_path: &str,
    project_path: &str,
    generated_at_unix: u64,
) -> Result<PathBuf> {
    let stem = diagnostic_stem(output_path, project_path);
    let file_name = format!("{stem}-diagnostics-{generated_at_unix}.zip");

    if let Some(base) = output_base_dir(output_path) {
        return Ok(base.join("diagnostics").join(file_name));
    }

    if let Some(base) = project_base_dir(project_path) {
        return Ok(base
            .join("output")
            .join("to-docx")
            .join("diagnostics")
            .join(file_name));
    }

    Ok(std::env::current_dir()
        .map_err(DiagnosticError::Io)?
        .join(file_name))
}

fn output_base_dir(output_path: &str) -> Option<PathBuf> {
    let path = Path::new(output_path.trim());
    if path.as_os_str().is_empty() {
        return None;
    }
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .or_else(|| Some(PathBuf::from(".")))
}

fn project_base_dir(project_path: &str) -> Option<PathBuf> {
    let path = Path::new(project_path.trim());
    if path.as_os_str().is_empty() {
        return None;
    }
    if path.extension().and_then(|ext| ext.to_str()) == Some("zip") {
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .or_else(|| Some(PathBuf::from(".")))
    } else {
        Some(path.to_path_buf())
    }
}

fn diagnostic_stem(output_path: &str, project_path: &str) -> String {
    file_stem(output_path)
        .or_else(|| file_stem(project_path))
        .unwrap_or_else(|| "tex2doc".to_string())
}

fn file_stem(path: &str) -> Option<String> {
    Path::new(path.trim())
        .file_stem()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            name.chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                        ch
                    } else {
                        '-'
                    }
                })
                .collect()
        })
}

fn report_path_for_output(output_path: &str) -> Option<PathBuf> {
    let output = Path::new(output_path.trim());
    if output.as_os_str().is_empty() {
        return None;
    }
    let stem = output
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("conversion");
    let mut report = output.to_path_buf();
    report.set_file_name(format!("{stem}.report.json"));
    Some(report)
}

fn unix_timestamp() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| DiagnosticError::Clock(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::ZipArchive;

    #[test]
    fn bundle_path_prefers_output_docx_parent() {
        let path = diagnostic_bundle_path("/tmp/out/paper.docx", "/tmp/project", 42).unwrap();

        assert_eq!(
            path,
            PathBuf::from("/tmp/out/diagnostics/paper-diagnostics-42.zip")
        );
    }

    #[test]
    fn bundle_path_falls_back_to_project_to_docx() {
        let path = diagnostic_bundle_path("", "/tmp/project", 42).unwrap();

        assert_eq!(
            path,
            PathBuf::from("/tmp/project/output/to-docx/diagnostics/project-diagnostics-42.zip")
        );
    }

    #[test]
    fn stem_is_sanitized() {
        assert_eq!(diagnostic_stem("/tmp/My Paper!.docx", ""), "My-Paper-");
    }

    #[test]
    fn export_writes_expected_zip_entries() {
        let dir =
            std::env::temp_dir().join(format!("tex2doc-diagnostics-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let output_path = dir.join("paper.docx");
        let report_path = dir.join("paper.report.json");
        std::fs::write(&report_path, br#"{"quality":"passed"}"#).unwrap();
        let input = DiagnosticInput {
            project_path: dir.display().to_string(),
            output_path: output_path.display().to_string(),
            api_base_url: "https://api.tex2doc.cn/v1/".to_string(),
            profile: "auto".to_string(),
            quality: "standard".to_string(),
            status_text: "status".to_string(),
            recent_jobs: "jobs".to_string(),
            update_status: "updates".to_string(),
            app_version: "0.1.0".to_string(),
        };

        let bundle = export_diagnostic_bundle(&input).unwrap();
        let file = File::open(&bundle).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();

        assert!(archive.by_name("diagnostics.json").is_ok());
        assert!(archive.by_name("status.txt").is_ok());
        assert!(archive.by_name("recent_jobs.txt").is_ok());
        assert!(archive.by_name("compile-report.json").is_ok());

        let _ = std::fs::remove_file(bundle);
        let _ = std::fs::remove_file(report_path);
        let _ = std::fs::remove_dir_all(dir.join("diagnostics"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn report_path_for_output_uses_docx_stem() {
        assert_eq!(
            report_path_for_output("/tmp/out/paper.docx").unwrap(),
            PathBuf::from("/tmp/out/paper.report.json")
        );
    }
}
