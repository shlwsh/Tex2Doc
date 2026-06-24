//! P5/P7: Desktop adapter for Tex2Doc cloud conversion APIs.

use std::fs;
use std::io::{Cursor, Write};
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

#[derive(Debug, Error)]
pub enum CloudConvertError {
    #[error("invalid API base URL: {0}")]
    InvalidBaseUrl(String),
    #[error("missing access token; sign in first")]
    MissingAccessToken,
    #[error("project not found: {0}")]
    ProjectNotFound(String),
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

pub fn convert_project_blocking(
    base_url: &str,
    access_token: Option<String>,
    project_path: &Path,
    main_tex: Option<&str>,
    output_docx: &Path,
    profile: &str,
    quality: &str,
) -> Result<CloudConvertResult> {
    let access_token = access_token.ok_or(CloudConvertError::MissingAccessToken)?;
    let base_url = parse_base_url(base_url)?;
    let package = package_project(project_path, main_tex)?;
    let output_docx = output_docx.to_path_buf();
    let report_path = report_path_for(&output_docx);
    let profile = profile.to_string();
    let quality = quality.to_string();
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
        let upload = client
            .upload_project_zip(package.bytes, package.file_name)
            .await?;
        let job = client
            .create_conversion(&CreateConversionRequest {
                upload_id: upload.upload_id,
                main_tex: package.main_tex,
                profile,
                quality,
            })
            .await?;

        let completed = poll_until_ready(&client, &job.job_id).await?;
        let docx = client.download_conversion_docx(&completed.job_id).await?;
        if let Some(parent) = output_docx.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output_docx, &docx)?;

        let report = client.get_conversion_report(&completed.job_id).await?;
        let report_json = serde_json::to_vec_pretty(&report)?;
        fs::write(&report_path, report_json)?;

        Ok(CloudConvertResult {
            job_id: completed.job_id,
            docx_path: output_docx,
            report_path,
            docx_bytes: docx.len(),
            report_text: format_report_line(&report),
        })
    })
}

struct ProjectPackage {
    bytes: Vec<u8>,
    file_name: String,
    main_tex: String,
}

fn package_project(project_path: &Path, main_tex: Option<&str>) -> Result<ProjectPackage> {
    if !project_path.exists() {
        return Err(CloudConvertError::ProjectNotFound(
            project_path.display().to_string(),
        ));
    }

    let main_tex = normalize_main_tex(project_path, main_tex)?;
    if project_path.is_file() {
        let bytes = fs::read(project_path)?;
        let file_name = project_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("project.zip")
            .to_string();
        return Ok(ProjectPackage {
            bytes,
            file_name,
            main_tex,
        });
    }

    let bytes = zip_directory(project_path)?;
    let file_name = project_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("{name}.zip"))
        .unwrap_or_else(|| "project.zip".to_string());
    Ok(ProjectPackage {
        bytes,
        file_name,
        main_tex,
    })
}

fn normalize_main_tex(project_path: &Path, main_tex: Option<&str>) -> Result<String> {
    if let Some(value) = main_tex.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(value.replace('\\', "/"));
    }
    if project_path.is_dir() {
        return find_main_tex(project_path);
    }
    Ok("main.tex".to_string())
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

fn zip_directory(project_root: &Path) -> Result<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut out);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        add_directory_to_zip(project_root, project_root, &mut zip, options)?;
        zip.finish()?;
    }
    Ok(out.into_inner())
}

fn add_directory_to_zip(
    root: &Path,
    current: &Path,
    zip: &mut zip::ZipWriter<&mut Cursor<Vec<u8>>>,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    let mut entries = fs::read_dir(current)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if should_skip_path(&path) {
            continue;
        }
        if path.is_dir() {
            add_directory_to_zip(root, &path, zip, options)?;
            continue;
        }
        if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .map(|path| path.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));
            zip.start_file(relative, options)?;
            zip.write_all(&fs::read(&path)?)?;
        }
    }
    Ok(())
}

fn should_skip_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            matches!(
                name,
                ".git" | "target" | "output" | ".DS_Store" | "__pycache__"
            )
        })
        .unwrap_or(false)
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

fn report_path_for(output_docx: &Path) -> PathBuf {
    let mut path = output_docx.to_path_buf();
    let stem = output_docx
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("conversion");
    path.set_file_name(format!("{stem}.report.json"));
    path
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

    #[test]
    fn report_path_uses_docx_stem() {
        assert_eq!(
            report_path_for(Path::new("/tmp/out.docx")),
            PathBuf::from("/tmp/out.report.json")
        );
    }

    #[test]
    fn explicit_main_tex_is_normalized() {
        assert_eq!(
            normalize_main_tex(Path::new("/tmp/project.zip"), Some("chapters\\main.tex")).unwrap(),
            "chapters/main.tex"
        );
    }
}
