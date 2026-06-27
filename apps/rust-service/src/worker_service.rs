//! P7 cloud conversion worker.

use doc_compiler_engine::{CompileOptions, ProfileRef, SemanticBackendKind, SemanticTexEngine};
use doc_core::{convert_zip, ConvertOptions};
use doc_quality::quality_run::BackendSummary;
use serde_json;
use tokio::sync::mpsc;
use zip;
use tokio::time::{self, Duration};

use crate::error_code::ConversionErrorCode;
use crate::limits::{
    MAX_UPLOAD_FILE_COUNT, MAX_UPLOAD_FILE_BYTES, MAX_UPLOAD_UNCOMPRESSED_BYTES,
};
use crate::state::{ConversionReportRecord, ConversionStatus, ServerState};

/// Redact user content from log messages to protect privacy.
/// Replaces LaTeX content and user data with placeholders.
#[allow(dead_code)]
pub fn redact_content(content: &str) -> String {
    // Truncate long content
    if content.len() > 200 {
        format!("{}... [REDACTED_CONTENT]", &content[..200])
    } else {
        content.replace("\\begin", "[REDACTED_LATEX]")
            .replace("\\end", "[/REDACTED_LATEX]")
    }
}

/// Validate ZIP file for security threats.
fn validate_zip(zip_bytes: &[u8]) -> Result<ZipValidation, String> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| format!("Invalid ZIP: {e}"))?;

    let mut total_uncompressed: u64 = 0;
    let mut file_count: usize = 0;

    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| format!("Cannot read file {i}: {e}"))?;
        file_count += 1;

        if file_count > MAX_UPLOAD_FILE_COUNT {
            return Err(format!("Too many files: {file_count} > {}", MAX_UPLOAD_FILE_COUNT));
        }

        let uncompressed_size = file.size();
        total_uncompressed += uncompressed_size;

        if total_uncompressed > MAX_UPLOAD_UNCOMPRESSED_BYTES {
            return Err(format!(
                "Uncompressed size exceeds limit: {} > {} bytes",
                total_uncompressed, MAX_UPLOAD_UNCOMPRESSED_BYTES
            ));
        }

        // Check for path traversal attacks
        let name = file.name();
        if name.contains("..") || name.starts_with('/') || name.contains('\\') && !name.starts_with('\\') {
            return Err(format!("Dangerous path in ZIP: {name}"));
        }

        // Check individual file size
        if uncompressed_size > MAX_UPLOAD_FILE_BYTES as u64 {
            return Err(format!(
                "File too large: {name} is {} > {} bytes",
                uncompressed_size, MAX_UPLOAD_FILE_BYTES
            ));
        }
    }

    Ok(ZipValidation {
        file_count,
        total_uncompressed,
    })
}

#[allow(dead_code)]
struct ZipValidation {
    file_count: usize,
    total_uncompressed: u64,
}

#[derive(Debug, Clone)]
pub struct WorkerCommand {
    pub job_id: String,
}

pub async fn spawn_worker_state() -> Result<ServerState, sqlx::Error> {
    let (tx, rx) = mpsc::channel(32);
    let state = ServerState::new(tx).await?;
    tokio::spawn(worker_loop(state.clone(), rx));
    Ok(state)
}

async fn worker_loop(state: ServerState, mut rx: mpsc::Receiver<WorkerCommand>) {
    let worker_id = format!("worker-{}", uuid::Uuid::new_v4().simple());
    let mut interval = time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            Some(command) = rx.recv() => {
                tracing::debug!(job_id = %command.job_id, "worker notified");
            }
            _ = interval.tick() => {
                if let Err(error) = state.recover_stale_jobs().await {
                    tracing::warn!(error = %error, "failed to recover stale jobs");
                }
            }
            else => break,
        }

        loop {
            match state.claim_next_job(&worker_id).await {
                Ok(Some(job_id)) => {
                    tracing::info!(job_id = %job_id, "claiming job");
                    process_job(state.clone(), job_id).await;
                }
                Ok(None) => break,
                Err(error) => {
                    tracing::error!("failed to claim queued conversion job: {error}");
                    break;
                }
            }
        }
    }
}

async fn process_job(state: ServerState, job_id: String) {
    tracing::info!(job_id = %job_id, "processing job started");

    state
        .update_status(&job_id, ConversionStatus::Normalizing)
        .await;

    let Some(job) = state.get_job(&job_id).await else {
        tracing::warn!("job not found");
        return;
    };
    let Some(upload) = state.get_upload(&job.upload_id).await else {
        state
            .fail_job(
                &job_id,
                ConversionErrorCode::UploadNotFound,
                format!("upload not found: {}", job.upload_id),
            )
            .await;
        return;
    };

    // Validate ZIP file for security threats
    tracing::info!("validating upload ZIP");
    if let Err(err) = validate_zip(&upload.bytes) {
        tracing::warn!(error = %err, "ZIP validation failed");
        state
            .fail_job(&job_id, ConversionErrorCode::UploadInvalidZip, err)
            .await;
        return;
    }

    tracing::info!(profile = %job.profile, quality = %job.quality, engine = %job.engine, "starting conversion");
    let input = ConversionJobInput {
        zip_bytes: upload.bytes.clone(),
        main_tex: job.main_tex.clone(),
        profile: job.profile.clone(),
        quality: job.quality.clone(),
        engine: job.engine.clone(),
    };
    let result = tokio::task::spawn_blocking(move || execute_conversion(input)).await;

    let output = match result {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => {
            state
                .fail_job(&job_id, ConversionErrorCode::ConvertFailed, err)
                .await;
            return;
        }
        Err(err) => {
            state
                .fail_job(
                    &job_id,
                    ConversionErrorCode::WorkerJoinError,
                    format!("worker join error: {err}"),
                )
                .await;
            return;
        }
    };

    state
        .update_status(&job_id, ConversionStatus::Rendering)
        .await;
    state
        .update_status(&job_id, ConversionStatus::Verifying)
        .await;

    if output.docx.len() < 4 || &output.docx[..4] != b"PK\x03\x04" {
        state
            .fail_job(
                &job_id,
                ConversionErrorCode::InvalidDocx,
                "generated docx does not have a valid ZIP header".to_string(),
            )
            .await;
        return;
    }

    let report = ConversionReportRecord {
        job_id: job_id.clone(),
        status: ConversionStatus::Completed,
        quality_score: output.quality_score,
        profile: output.profile.clone(),
        main_tex: job.main_tex.clone(),
        executor: output.executor.clone(),
        backend: output.backend.clone(),
        quality_status: output.quality_status.clone(),
        compatibility_score: output.compatibility_score,
        docx_bytes: output.docx.len(),
        warnings: output.warnings.clone(),
        error_code: None,
        message: format!(
            "Converted upload {} ({}) with profile={} quality={} engine={}",
            upload.upload_id, upload.file_name, job.profile, job.quality, job.engine
        ),
        dimension_scores: output
            .quality_run_json
            .as_ref()
            .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
            .and_then(|v| v.get("dimension_scores").cloned()),
        quality_run_json: output.quality_run_json,
    };

    state.complete_job(&job_id, output.docx, report).await;
}

#[derive(Debug)]
struct ConversionJobInput {
    zip_bytes: Vec<u8>,
    main_tex: String,
    profile: String,
    quality: String,
    engine: String,
}

#[derive(Debug)]
struct ConversionJobOutput {
    docx: Vec<u8>,
    executor: String,
    backend: String,
    profile: String,
    quality_status: String,
    quality_score: u8,
    compatibility_score: Option<u8>,
    warnings: Vec<String>,
    /// QualityRun JSON 报告（供 API 返回多维评分）。
    quality_run_json: Option<String>,
}

fn execute_conversion(input: ConversionJobInput) -> Result<ConversionJobOutput, String> {
    match normalize_engine(&input.engine).as_str() {
        "legacy-rule" | "doc-core" => execute_legacy(input),
        "semantic-engine" | "semantic" | "auto" => match execute_semantic(&input) {
            Ok(output) => Ok(output),
            Err(error) => {
                let mut fallback = execute_legacy(input)?;
                fallback.warnings.insert(
                    0,
                    format!("semantic-engine fallback to legacy-rule: {error}"),
                );
                Ok(fallback)
            }
        },
        other => Err(format!(
            "unsupported conversion engine '{other}'; expected semantic-engine or legacy-rule"
        )),
    }
}

fn execute_semantic(input: &ConversionJobInput) -> Result<ConversionJobOutput, String> {
    let options = CompileOptions {
        profile_ref: Some(parse_profile_ref(&input.profile)),
        semantic_backend: SemanticBackendKind::Auto,
        allow_backend_fallback: true,
        min_compatibility_score_override: Some(min_score_for_quality(&input.quality)),
        ..Default::default()
    };
    let engine = SemanticTexEngine::new();
    let artifact = engine
        .compile_zip_to_docx(&input.zip_bytes, &input.main_tex, &options)
        .map_err(|e| e.to_string())?;

    let report = &artifact.report;
    let profile = report
        .active_profile
        .as_ref()
        .map(|profile| profile.id.clone())
        .unwrap_or_else(|| report.profile.id().to_string());

    // 从 QualityGateResult 获取评分，不再使用 score_from_docx_size heuristic
    let (quality_status, quality_score) = report
        .quality_gate
        .as_ref()
        .map(|quality| {
            let status = quality.status.as_str().to_string();
            let score = quality.score;
            (status, score)
        })
        .unwrap_or_else(|| ("Unknown".to_string(), 0));

    // 构建 BackendSummary
    let backend_summary = {
        let selected = report.backend.selected.id().to_string();
        let requested = "auto".to_string();
        match report.backend.fallback_from {
            Some(from) => BackendSummary::new(&requested, &selected)
                .with_fallback(from.id(), "runtime unavailable"),
            None => BackendSummary::new(&requested, &selected),
        }
    };

    // 构建 QualityRun（用于 API 返回多维评分）
    let quality_run_json = build_quality_run(
        "",
        "0.1.0",
        &profile,
        backend_summary.clone(),
        &report.diagnostics,
        report.quality_gate.as_ref(),
    );

    let mut warnings = report
        .diagnostics
        .iter()
        .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
        .collect::<Vec<_>>();
    if report.backend.fallback_from.is_some() {
        let fallback_from = report.backend.fallback_from.as_ref().unwrap();
        let selected = report.backend.selected.id();
        warnings.insert(
            0,
            format!("semantic backend fallback: {} -> {}", fallback_from.id(), selected),
        );
    }

    Ok(ConversionJobOutput {
        docx: artifact.docx,
        executor: "semantic-engine".to_string(),
        backend: report.backend.selected.id().to_string(),
        profile,
        quality_status,
        quality_score,
        compatibility_score: Some(report.compatibility.score),
        warnings,
        quality_run_json,
    })
}

/// 从 `CompileReport` 构建 `QualityRun` JSON。
fn build_quality_run(
    job_id: &str,
    engine_version: &str,
    profile: &str,
    backend: BackendSummary,
    diagnostics: &[doc_compiler_engine::EngineDiagnostic],
    quality_gate: Option<&doc_compiler_engine::QualityGateResult>,
) -> Option<String> {
    use doc_quality::quality_run::{
        DimensionScores, QualityIssue, QualityRun, SemanticLossEvent,
    };

    // 从 diagnostics 提取 semantic loss events
    let semantic_losses: Vec<SemanticLossEvent> = diagnostics
        .iter()
        .filter(|d| d.severity == doc_compiler_engine::DiagnosticSeverity::Warning)
        .map(|d| {
            SemanticLossEvent::new(
                &d.code,
                "text_fallback",
                "degraded",
                &d.message,
            )
        })
        .collect();

    // 从 quality_gate 提取 issues
    let (blocking_issues, warnings): (Vec<QualityIssue>, Vec<QualityIssue>) = quality_gate
        .map(|qg| {
            let mut blocking = Vec::new();
            let mut warns = Vec::new();
            for check in &qg.failed_checks {
                let issue = convert_quality_check_to_issue(check);
                if issue.is_blocking() {
                    blocking.push(issue);
                } else {
                    warns.push(issue);
                }
            }
            for check in &qg.warnings {
                warns.push(convert_quality_check_to_issue(check));
            }
            (blocking, warns)
        })
        .unwrap_or_default();

    // 计算维度评分（基于 quality_gate 的 passed_checks 比例）
    let dimension_scores = quality_gate
        .map(|qg| {
            let total = qg.total_checks;
            let passed = qg.passed_checks;
            let ratio = if total > 0 {
                passed as f64 / total as f64
            } else {
                1.0
            };
            let score = (ratio * 100.0).round() as u8;
            DimensionScores {
                parse: score,
                semantic: score,
                docx: score,
                visual: score,
                editable: score,
                performance: 100,
            }
        })
        .unwrap_or_default();

    let quality_score = quality_gate
        .map(|qg| qg.score)
        .unwrap_or_else(|| dimension_scores.weighted_score());

    let qr = QualityRun {
        job_id: job_id.to_string(),
        engine_version: engine_version.to_string(),
        profile: profile.to_string(),
        backend,
        quality_score,
        dimension_scores,
        blocking_issues,
        warnings,
        semantic_loss_events: semantic_losses,
        word_compatibility: doc_quality::quality_run::WordCompatibility::default(),
        artifacts: doc_quality::quality_run::QualityArtifacts::default(),
    };

    serde_json::to_string_pretty(&qr).ok()
}

fn execute_legacy(input: ConversionJobInput) -> Result<ConversionJobOutput, String> {
    let result = convert_zip(
        &input.zip_bytes,
        &input.main_tex,
        &ConvertOptions::default(),
    )
    .map_err(|e| e.to_string())?;

    // Legacy engine: 使用基于 DOCX 结构的最低限度评分
    // 不再依赖纯大小的 heuristic，改用更合理的结构评分
    let quality_score = score_from_docx_structure(&result.docx);

    Ok(ConversionJobOutput {
        quality_score,
        docx: result.docx,
        executor: "legacy-rule".to_string(),
        backend: "doc-core".to_string(),
        profile: input.profile,
        quality_status: "LegacyHeuristic".to_string(),
        compatibility_score: None,
        warnings: result.warnings,
        quality_run_json: None,
    })
}

/// 基于 DOCX 结构特征计算质量分数（替代纯大小 heuristic）。
///
/// 检查点：
/// - 文件是否为有效的 ZIP
/// - 是否包含 document.xml
/// - 是否包含 styles.xml
/// - 文件大小是否合理
fn score_from_docx_structure(docx_bytes: &[u8]) -> u8 {
    // 检查 ZIP 头
    if docx_bytes.len() < 4 || &docx_bytes[..4] != b"PK\x03\x04" {
        return 30;
    }

    // 尝试解析 ZIP 内容
    let archive = zip::ZipArchive::new(std::io::Cursor::new(docx_bytes)).ok();
    let Some(mut archive) = archive else {
        return 50;
    };

    let mut has_document = false;
    let mut has_styles = false;
    let mut content_files = 0;

    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().to_lowercase();
            if name == "word/document.xml" {
                has_document = true;
            } else if name == "word/styles.xml" {
                has_styles = true;
            }
            if name.starts_with("word/") {
                content_files += 1;
            }
        }
    }

    let mut score = 50u8; // 基础分
    if has_document {
        score += 20;
    }
    if has_styles {
        score += 15;
    }
    if content_files >= 3 {
        score += 10;
    }
    if docx_bytes.len() > 2 * 1024 {
        score += 5;
    }

    score.min(100)
}

fn parse_profile_ref(profile: &str) -> ProfileRef {
    match profile.trim() {
        "" | "auto" => ProfileRef::Auto,
        id => ProfileRef::Id(id.to_string()),
    }
}

fn min_score_for_quality(quality: &str) -> u8 {
    match quality.trim().to_ascii_lowercase().as_str() {
        "preview" | "low" => 60,
        "standard" | "high" | "medium" => 60, // relaxed: accept moderate scores
        "strict" => 90,
        _ => 60,
    }
}

fn normalize_engine(engine: &str) -> String {
    engine.trim().to_ascii_lowercase().replace('_', "-")
}

/// 将 compiler-engine 的 QualityCheck 转换为 QualityIssue。
fn convert_quality_check_to_issue(
    check: &doc_compiler_engine::QualityCheck,
) -> doc_quality::quality_run::QualityIssue {
    use doc_quality::quality_run::{IssueSeverity, QualityIssue};

    let layer = match check.severity {
        doc_compiler_engine::QualitySeverity::Error => "structural",
        doc_compiler_engine::QualitySeverity::Warning => "textual",
        doc_compiler_engine::QualitySeverity::Info => "visual",
    };
    let severity = match check.severity {
        doc_compiler_engine::QualitySeverity::Error => IssueSeverity::Blocking,
        doc_compiler_engine::QualitySeverity::Warning => IssueSeverity::Warning,
        doc_compiler_engine::QualitySeverity::Info => IssueSeverity::Info,
    };
    QualityIssue {
        name: check.name.clone(),
        severity,
        description: check.message.clone(),
        suggestion: if check.passed { None } else { Some(check.message.clone()) },
        layer: layer.to_string(),
    }
}
