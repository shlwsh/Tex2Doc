//! P5/P7: Desktop adapter for Tex2Doc cloud conversion APIs.
//!
//! Provides the Flutter-aligned upload-then-download flow:
//! - [`convert_upload_blocking`] — cloud engine: uploads raw bytes, creates a job,
//!   polls until done, then downloads the result DOCX to a user-specified output
//!   directory.
//! - [`convert_local_blocking`] — local engine: extracts the uploaded archive to a
//!   temp directory, runs the semantic engine, and writes the result DOCX to the
//!   output directory.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Duration;

use doc_commercial_api_client::{
    ApiClient, ApiError, ClientConfig, ConversionReport, CreateConversionRequest, JobStatus,
};
use thiserror::Error;

#[derive(Debug)]
pub struct CloudConvertResult {
    pub job_id: String,
    pub docx_path: PathBuf,
    pub report_path: PathBuf,
    pub docx_bytes: usize,
    pub report_text: String,
}

/// P5/P7: Result of a local engine conversion (used for the upload-based local flow).
#[derive(Debug)]
pub struct LocalConvertResult {
    pub docx_path: PathBuf,
    pub report_path: PathBuf,
    pub docx_bytes: usize,
    pub profile: String,
    pub quality_status: String,
    pub quality_score: String,
    pub report_text: String,
}

pub struct CloudUploadRequest<'a> {
    pub base_url: &'a str,
    pub access_token: Option<String>,
    pub zip_bytes: Vec<u8>,
    pub file_name: &'a str,
    pub main_tex: &'a str,
    pub output_dir: &'a Path,
    pub profile: &'a str,
    pub quality: &'a str,
}

#[derive(Debug, Error)]
pub enum CloudConvertError {
    #[error("invalid API base URL: {0}")]
    InvalidBaseUrl(String),
    #[error("missing access token; sign in first")]
    MissingAccessToken,
    #[error("main tex not found in: {0}")]
    MainTexNotFound(String),
    #[error("cloud conversion timed out for job {0}")]
    Timeout(String),
    #[error("cloud conversion failed for job {job_id}: {error}")]
    ConversionFailed {
        job_id: String,
        error_code: Option<String>,
        error: String,
    },
    #[error("cloud conversion quota exceeded: used={used}, limit={limit}")]
    QuotaExceeded { used: u32, limit: u32 },
    #[error("runtime error: {0}")]
    Runtime(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("API error: {0}")]
    Api(#[from] ApiError),
}

pub type Result<T> = std::result::Result<T, CloudConvertError>;

/// P5/P7: Upload-based cloud conversion (Flutter-aligned flow).
///
/// Upload a project archive (bytes), create a cloud conversion job, poll until
/// completion, then download the result DOCX to `output_dir/<file_stem>.docx`.
/// Also fetches and saves a `.report.json` next to the output docx.
pub fn convert_upload_blocking(request: CloudUploadRequest<'_>) -> Result<CloudConvertResult> {
    let CloudUploadRequest {
        base_url,
        access_token,
        zip_bytes,
        file_name,
        main_tex,
        output_dir,
        profile,
        quality,
    } = request;
    let access_token = access_token.ok_or(CloudConvertError::MissingAccessToken)?;
    let base_url = parse_base_url(base_url)?;
    let output_dir = output_dir.to_path_buf();

    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("conversion");

    let runtime = runtime()?;

    runtime.block_on(async move {
        let client = authenticated_client(base_url, &access_token)?;

        let usage = client.usage().await?;
        if usage.cloud_conversions_used >= usage.cloud_conversions_limit {
            return Err(CloudConvertError::QuotaExceeded {
                used: usage.cloud_conversions_used,
                limit: usage.cloud_conversions_limit,
            });
        }

        let upload = client.upload_project_zip(zip_bytes, file_name).await?;

        let job = client
            .create_conversion(&CreateConversionRequest {
                upload_id: upload.upload_id,
                main_tex: main_tex.to_string(),
                profile: profile.to_string(),
                quality: quality.to_string(),
            })
            .await?;

        let completed = poll_until_ready(&client, &job.job_id).await?;

        let docx = client.download_conversion_docx(&completed.job_id).await?;

        fs::create_dir_all(&output_dir)?;
        let docx_path = output_dir.join(format!("{stem}.docx"));
        let report_path = output_dir.join(format!("{stem}.report.json"));

        fs::write(&docx_path, &docx)?;

        let report = client.get_conversion_report(&completed.job_id).await?;
        let report_json = serde_json::to_vec_pretty(&report)?;
        fs::write(&report_path, report_json)?;

        Ok(CloudConvertResult {
            job_id: completed.job_id,
            docx_path,
            report_path,
            docx_bytes: docx.len(),
            report_text: format_report_line(&report),
        })
    })
}

/// P5/P7: Local engine conversion from upload bytes (Flutter-aligned flow).
///
/// Accepts raw bytes of an archive (zip or flat files), extracts to a temp
/// directory, runs the semantic engine, and writes the result DOCX to
/// `output_dir/<file_stem>.docx`.
pub fn convert_local_blocking(
    bytes: &[u8],
    file_name: &str,
    main_tex: &str,
    output_dir: &Path,
    profile: &str,
    quality: &str,
) -> Result<LocalConvertResult> {
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("conversion");
    let output_dir = output_dir.to_path_buf();
    let output_docx = output_dir.join(format!("{stem}.docx"));
    let report_path = output_dir.join(format!("{stem}.report.json"));

    let temp_dir = extract_to_temp(bytes, file_name)?;
    let project_root = temp_dir.path();

    let main_tex_path = if main_tex.trim().is_empty() {
        let found = find_main_tex(project_root)?;
        project_root.join(found)
    } else {
        project_root.join(main_tex.replace('\\', "/"))
    };

    let min_score = match quality {
        "preview" => 60u8,
        "strict" => 90u8,
        _ => 75u8,
    };

    let profile_ref = if profile.is_empty() || profile == "auto" {
        doc_compiler_engine::ProfileRef::Auto
    } else {
        doc_compiler_engine::ProfileRef::Id(profile.to_string())
    };

    let options = doc_compiler_engine::CompileOptions {
        profile_ref: Some(profile_ref),
        semantic_backend: doc_compiler_engine::SemanticBackendKind::Auto,
        allow_backend_fallback: true,
        min_compatibility_score_override: Some(min_score),
        ..Default::default()
    };

    let engine = doc_compiler_engine::SemanticTexEngine::new();
    let artifact = engine
        .compile_dir_to_docx(project_root, &main_tex_path, &options)
        .map_err(|e| CloudConvertError::ConversionFailed {
            job_id: "local".to_string(),
            error_code: None,
            error: e.to_string(),
        })?;

    fs::create_dir_all(&output_dir)?;
    fs::write(&output_docx, &artifact.docx)?;

    let report_json = serde_json::to_vec_pretty(&artifact.report)?;
    fs::write(&report_path, report_json)?;

    let report_summary = crate::report::ReportSummary::from_report(&artifact.report);
    let quality_status = artifact
        .report
        .quality_gate
        .as_ref()
        .map(|qg| qg.status.as_str().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let quality_score = artifact
        .report
        .quality_gate
        .as_ref()
        .map(|qg| qg.score.to_string())
        .unwrap_or_else(|| "N/A".to_string());

    let active_profile = artifact
        .report
        .active_profile
        .as_ref()
        .map(|ap| ap.id.clone())
        .unwrap_or_else(|| artifact.report.profile.id().to_string());

    Ok(LocalConvertResult {
        docx_path: output_docx,
        report_path,
        docx_bytes: artifact.report.docx_bytes,
        profile: active_profile,
        quality_status,
        quality_score,
        report_text: report_summary.format_for_ui(),
    })
}

/// Extract an archive (zip or flat file) to a temporary directory.
/// Returns the temp directory handle; it is automatically deleted when dropped.
fn extract_to_temp(bytes: &[u8], file_name: &str) -> Result<tempfile::TempDir> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| CloudConvertError::Runtime(format!("failed to create temp dir: {}", e)))?;

    // Check magic bytes to distinguish zip from flat files
    if bytes.len() >= 2 && bytes[0] == 0x50 && bytes[1] == 0x4b {
        extract_zip(bytes, temp_dir.path())?;
    } else {
        let dest = temp_dir.path().join(
            Path::new(file_name)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("source"),
        );
        fs::write(&dest, bytes)?;
    }

    Ok(temp_dir)
}

fn extract_zip(bytes: &[u8], dest: &Path) -> Result<()> {
    let reader = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = dest.join(file.name());

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}

fn find_main_tex(project_root: &Path) -> Result<String> {
    for candidate in [
        "main.tex",
        "main-jos.tex",
        "minimal.tex",
        "paper.tex",
        "article.tex",
    ] {
        if project_root.join(candidate).is_file() {
            return Ok(candidate.to_string());
        }
    }
    Err(CloudConvertError::MainTexNotFound(
        project_root.display().to_string(),
    ))
}

async fn poll_until_ready(
    client: &ApiClient,
    job_id: &str,
) -> Result<doc_commercial_api_client::ConversionJob> {
    for _ in 0..120 {
        let job = client.get_conversion(job_id).await?;
        match job.status {
            JobStatus::Completed if job.docx_ready && job.report_ready => return Ok(job),
            JobStatus::Failed | JobStatus::Expired => {
                return Err(CloudConvertError::ConversionFailed {
                    job_id: job.job_id,
                    error_code: job.error_code,
                    error: job.error.unwrap_or_else(|| "unknown error".to_string()),
                });
            }
            _ => tokio::time::sleep(Duration::from_millis(500)).await,
        }
    }
    Err(CloudConvertError::Timeout(job_id.to_string()))
}

fn format_report_line(report: &ConversionReport) -> String {
    format!(
        "Cloud job {} completed: profile={}, score={}, backend={}, docx_bytes={}",
        report.job_id,
        report.profile,
        report.quality_score,
        report.backend.as_deref().unwrap_or("-"),
        report
            .docx_bytes
            .map(|bytes| bytes.to_string())
            .unwrap_or_else(|| "-".to_string())
    )
}

fn authenticated_client(base_url: url::Url, access_token: &str) -> Result<ApiClient> {
    ApiClient::new(ClientConfig {
        base_url,
        api_key: access_token.to_string(),
        timeout: Duration::from_secs(30),
    })
    .map_err(CloudConvertError::from)
}

fn parse_base_url(value: &str) -> Result<url::Url> {
    value
        .trim()
        .parse()
        .map_err(|e: url::ParseError| CloudConvertError::InvalidBaseUrl(e.to_string()))
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CloudConvertError::Runtime(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn extract_to_temp_writes_single_file_for_non_zip() {
        let bytes = b"% hello world";
        let temp = extract_to_temp(bytes, "main.tex").unwrap();
        let file_path = temp.path().join("main.tex");
        assert!(file_path.is_file());
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "% hello world");
    }

    #[test]
    fn extract_to_temp_writes_zip_contents() {
        // Build a tiny zip in memory
        let mut buf = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut zip_writer = zip::ZipWriter::new(cursor);
            let options = zip::write::SimpleFileOptions::default();
            zip_writer.start_file("main.tex", options).unwrap();
            zip_writer.write_all(b"\\documentclass{article}").unwrap();
            zip_writer.finish().unwrap();
        }
        let temp = extract_to_temp(&buf, "archive.zip").unwrap();
        assert!(temp.path().join("main.tex").is_file());
    }
}
