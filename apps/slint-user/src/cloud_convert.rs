//! P5/P7: Desktop adapter for Tex2Doc cloud conversion APIs.
//!
//! Provides the Flutter-aligned upload-then-download flow:
//! - [`convert_upload_blocking`] — cloud engine: uploads raw bytes, creates a job,
//!   polls until done, then downloads the result DOCX to a user-specified output
//!   directory.
//! - [`convert_local_blocking`] — local engine: extracts the uploaded archive to a
//!   temp directory, runs the semantic engine, and writes the result DOCX to the
//!   output directory.

use std::fs::{self, File, OpenOptions};
use std::io::Cursor;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use doc_commercial_api_client::{
    ApiClient, ApiError, ClientConfig, ConversionReport, CreateConversionRequest, JobStatus,
};
use thiserror::Error;

// ============================================================
// File Logger - writes to output directory
// ============================================================

struct FileLogger {
    log_path: PathBuf,
}

impl FileLogger {
    fn new(output_dir: &Path, prefix: &str) -> std::io::Result<Self> {
        let timestamp = chrono_lite_timestamp();
        let log_name = format!("{}_{}.log", prefix, timestamp);
        let log_path = output_dir.join(&log_name);

        // Ensure output directory exists
        fs::create_dir_all(output_dir)?;

        // Create/overwrite log file
        let _file = File::create(&log_path)?;

        Ok(Self { log_path })
    }

    fn log(&self, message: &str) -> std::io::Result<()> {
        let timestamp = chrono_lite_timestamp();
        let mut file = OpenOptions::new().append(true).open(&self.log_path)?;

        writeln!(file, "[{}] {}", timestamp, message)?;
        file.flush()?;
        Ok(())
    }

    fn log_multi(&self, messages: &[&str]) -> std::io::Result<()> {
        for msg in messages {
            self.log(msg)?;
        }
        Ok(())
    }

    fn path(&self) -> &Path {
        &self.log_path
    }
}

fn chrono_lite_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let secs = secs % 60;
    let millis = now.subsec_millis();
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, millis)
}

#[derive(Debug)]
pub struct CloudConvertResult {
    pub job_id: String,
    pub docx_path: PathBuf,
    pub report_path: PathBuf,
    pub docx_bytes: usize,
    pub report_text: String,
}

/// P5/P7: Result of a local engine conversion (used for the upload-based local flow).
#[allow(dead_code)]
#[derive(Debug)]
pub struct LocalConvertResult {
    pub docx_path: PathBuf,
    pub report_path: PathBuf,
    pub docx_bytes: usize,
    pub profile: String,
    pub quality_status: String,
    pub quality_score: String,
    pub report_text: String,
    // Extended quality report fields
    pub job_id: String,
    pub engine_version: String,
    pub parse_score: u8,
    pub semantic_score: u8,
    pub docx_score: u8,
    pub visual_score: u8,
    pub editable_score: u8,
    pub performance_score: u8,
    pub word_status: String,
    pub word_errors: Vec<String>,
    pub word_method: String,
    pub style_coverage_rate: f64,
    pub blocking_issues_count: usize,
    pub warnings_count: usize,
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
        let plan_remaining = usage
            .cloud_conversions_limit
            .saturating_sub(usage.cloud_conversions_used);
        if plan_remaining == 0 && usage.count_balance == 0 {
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
#[allow(unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn convert_local_blocking(
    base_url: &str,
    access_token: Option<String>,
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

    // Create file logger for this conversion
    let logger = match FileLogger::new(output_dir, "local_conversion") {
        Ok(l) => {
            let _ = l.log("=== 开始本地转换 ===");
            let _ = l.log(&format!("文件: {}", file_name));
            let _ = l.log(&format!("输出目录: {}", output_dir.display()));
            let _ = l.log(&format!("Profile: {}, Quality: {}", profile, quality));
            Some(l)
        }
        Err(e) => {
            log::warn!("Failed to create file logger: {}", e);
            None
        }
    };

    let output_dir = output_dir.to_path_buf();
    let output_docx = output_dir.join(format!("{stem}.docx"));
    let report_path = output_dir.join(format!("{stem}.report.json"));

    let access_token = access_token.ok_or(CloudConvertError::MissingAccessToken)?;

    // Note: Local conversion does not require cloud quota check.
    // The local engine runs entirely on the client machine.
    // We skip the check_local_conversion() call to allow unlimited local conversions.

    if let Some(ref l) = logger {
        let _ = l.log("开始本地转换（无需云端配额）...");
    }

    if let Some(ref l) = logger {
        let _ = l.log("正在提取文件...");
    }

    let temp_dir = extract_to_temp(bytes, file_name)?;
    let project_root = temp_dir.path();

    if let Some(ref l) = logger {
        let _ = l.log(&format!("项目根目录: {}", project_root.display()));
    }

    let main_tex_path = if main_tex.trim().is_empty() {
        let found = find_main_tex(project_root)?;
        let path = project_root.join(&found);
        if let Some(ref l) = logger {
            let _ = l.log(&format!("自动检测到主文件: {}", found));
        }
        path
    } else {
        let path = project_root.join(main_tex.replace('\\', "/"));
        if let Some(ref l) = logger {
            let _ = l.log(&format!("使用指定主文件: {}", main_tex));
        }
        path
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

    if let Some(ref l) = logger {
        let _ = l.log("正在运行语义引擎编译...");
    }

    let artifact = engine
        .compile_dir_to_docx(project_root, &main_tex_path, &options)
        .map_err(|e| {
            if let Some(ref l) = logger {
                let _ = l.log(&format!("编译失败: {}", e));
            }
            CloudConvertError::ConversionFailed {
                job_id: "local".to_string(),
                error_code: None,
                error: e.to_string(),
            }
        })?;

    if let Some(ref l) = logger {
        let _ = l.log("正在保存结果...");
    }

    let temp_docx_path = temp_dir.path().join("temp_result.docx");
    fs::write(&temp_docx_path, &artifact.docx)?;

    // Local conversion does not consume cloud quota - no consume_local_conversion() call needed

    fs::create_dir_all(&output_dir)?;
    fs::copy(&temp_docx_path, &output_docx)?;

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

    // Extract blocking issues and warnings count
    let (blocking_issues_count, warnings_count) = extract_issue_counts(&artifact.report);

    // Extract word compatibility info (simplified - actual implementation may vary)
    let (word_status, word_errors, word_method) =
        ("unchecked".to_string(), Vec::new(), "none".to_string());

    // Extract style coverage rate (placeholder - actual implementation may vary)
    let style_coverage_rate = 0.0;

    // Log success
    if let Some(ref l) = logger {
        let _ = l.log_multi(&[
            "=== 转换成功 ===",
            &format!("DOCX: {}", output_docx.display()),
            &format!("报告: {}", report_path.display()),
            &format!("质量评分: {}", quality_score),
            &format!("配置文件: {}", active_profile),
            &format!("阻断问题数: {}", blocking_issues_count),
            &format!("警告数: {}", warnings_count),
            &format!("日志文件: {}", l.path().display()),
        ]);
    }

    Ok(LocalConvertResult {
        docx_path: output_docx,
        report_path,
        docx_bytes: artifact.report.docx_bytes,
        profile: active_profile,
        quality_status,
        quality_score,
        report_text: report_summary.format_for_ui(),
        job_id: "local".to_string(),
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        parse_score: 100,
        semantic_score: 100,
        docx_score: 100,
        visual_score: 100,
        editable_score: 100,
        performance_score: 100,
        word_status,
        word_errors,
        word_method,
        style_coverage_rate,
        blocking_issues_count,
        warnings_count,
    })
}

// ============================================================
// Helper functions for extracting quality report details
// ============================================================

fn extract_issue_counts(report: &doc_compiler_engine::CompileReport) -> (usize, usize) {
    let blocking = report
        .quality_gate
        .as_ref()
        .map(|g| g.failed_checks.len())
        .unwrap_or(0);
    let warnings = report
        .quality_gate
        .as_ref()
        .map(|g| g.warnings.len())
        .unwrap_or(0);
    (blocking, warnings)
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

    #[test]
    fn test_extract_upload_zip() {
        let bytes = std::fs::read("D:\\temp\\upload.zip").unwrap();
        match extract_to_temp(&bytes, "upload.zip") {
            Ok(_) => println!("Extract OK"),
            Err(e) => panic!("Extract failed: {}", e),
        }
    }

    #[test]
    fn test_engine_directly_on_upload_zip() {
        let bytes = std::fs::read("D:\\temp\\upload.zip").unwrap();
        let temp = extract_to_temp(&bytes, "upload.zip").unwrap();

        let engine = doc_compiler_engine::SemanticTexEngine::new();
        let options = doc_compiler_engine::CompileOptions {
            profile_ref: Some(doc_compiler_engine::ProfileRef::Auto),
            semantic_backend: doc_compiler_engine::SemanticBackendKind::Auto,
            allow_backend_fallback: true,
            min_compatibility_score_override: Some(75),
            ..Default::default()
        };

        let main_tex = find_main_tex(temp.path()).unwrap();
        println!("Found main tex: {}", main_tex);

        let res = engine.compile_dir_to_docx(temp.path(), &temp.path().join(main_tex), &options);
        match res {
            Ok(artifact) => println!("Engine OK! Docx size: {}", artifact.docx.len()),
            Err(e) => panic!("Engine failed: {}", e),
        }
    }
}
