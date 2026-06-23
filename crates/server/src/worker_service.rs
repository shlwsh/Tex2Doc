//! P7 in-memory cloud conversion worker.

use doc_compiler_engine::{CompileOptions, ProfileRef, SemanticBackendKind, SemanticTexEngine};
use doc_core::{convert_zip, ConvertOptions};
use tokio::sync::mpsc;

use crate::state::{ConversionReportRecord, ConversionStatus, ServerState};

#[derive(Debug, Clone)]
pub struct WorkerCommand {
    pub job_id: String,
}

pub fn spawn_worker_state() -> ServerState {
    let (tx, rx) = mpsc::channel(32);
    let state = ServerState::new(tx);
    tokio::spawn(worker_loop(state.clone(), rx));
    state
}

async fn worker_loop(state: ServerState, mut rx: mpsc::Receiver<WorkerCommand>) {
    while let Some(command) = rx.recv().await {
        process_job(state.clone(), command.job_id).await;
    }
}

async fn process_job(state: ServerState, job_id: String) {
    state
        .update_status(&job_id, ConversionStatus::Normalizing)
        .await;

    let Some(job) = state.get_job(&job_id).await else {
        return;
    };
    let Some(upload) = state.get_upload(&job.upload_id).await else {
        state
            .fail_job_with_code(
                &job_id,
                "upload_not_found",
                format!("upload not found: {}", job.upload_id),
            )
            .await;
        return;
    };

    state
        .update_status(&job_id, ConversionStatus::Detecting)
        .await;
    state
        .update_status(&job_id, ConversionStatus::Analyzing)
        .await;
    state
        .update_status(&job_id, ConversionStatus::Compiling)
        .await;

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
        Ok(Err(error)) => {
            state
                .fail_job_with_code(&job_id, "convert_failed", error)
                .await;
            return;
        }
        Err(error) => {
            state
                .fail_job_with_code(
                    &job_id,
                    "worker_join_error",
                    format!("worker join error: {error}"),
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
            .fail_job_with_code(
                &job_id,
                "invalid_docx",
                "generated docx does not have a valid ZIP header".to_string(),
            )
            .await;
        return;
    }

    let report = ConversionReportRecord {
        job_id: job_id.clone(),
        status: ConversionStatus::Completed,
        quality_score: output.quality_score,
        profile: output.profile,
        main_tex: job.main_tex.clone(),
        executor: output.executor,
        backend: output.backend,
        quality_status: output.quality_status,
        compatibility_score: output.compatibility_score,
        docx_bytes: output.docx.len(),
        warnings: output.warnings,
        error_code: None,
        message: format!(
            "Converted upload {} ({}) with profile={} quality={} engine={}",
            upload.upload_id, upload.file_name, job.profile, job.quality, job.engine
        ),
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
    let quality_status = report
        .quality_gate
        .as_ref()
        .map(|quality| quality.status.as_str().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let quality_score = report
        .quality_gate
        .as_ref()
        .map(|quality| quality.score)
        .unwrap_or_else(|| score_from_docx_size(artifact.docx.len()));
    let mut warnings = report
        .diagnostics
        .iter()
        .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
        .collect::<Vec<_>>();
    if let Some(fallback_from) = report.backend.fallback_from {
        warnings.insert(
            0,
            format!(
                "semantic backend fallback: {} -> {}",
                fallback_from.id(),
                report.backend.selected.id()
            ),
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
    })
}

fn execute_legacy(input: ConversionJobInput) -> Result<ConversionJobOutput, String> {
    let result = convert_zip(
        &input.zip_bytes,
        &input.main_tex,
        &ConvertOptions::default(),
    )
    .map_err(|e| e.to_string())?;

    Ok(ConversionJobOutput {
        quality_score: score_from_docx_size(result.docx.len()),
        docx: result.docx,
        executor: "legacy-rule".to_string(),
        backend: "doc-core".to_string(),
        profile: input.profile,
        quality_status: "LegacyHeuristic".to_string(),
        compatibility_score: None,
        warnings: result.warnings,
    })
}

fn parse_profile_ref(profile: &str) -> ProfileRef {
    match profile.trim() {
        "" | "auto" => ProfileRef::Auto,
        id => ProfileRef::Id(id.to_string()),
    }
}

fn min_score_for_quality(quality: &str) -> u8 {
    match quality.trim().to_ascii_lowercase().as_str() {
        "preview" => 60,
        "strict" => 90,
        _ => 75,
    }
}

fn score_from_docx_size(bytes: usize) -> u8 {
    if bytes > 4 * 1024 {
        90
    } else {
        70
    }
}

fn normalize_engine(engine: &str) -> String {
    engine.trim().to_ascii_lowercase().replace('_', "-")
}
