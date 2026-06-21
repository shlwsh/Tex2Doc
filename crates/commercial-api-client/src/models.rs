//! API request/response types.

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("transport error: {0}")]
    Transport(String),
    #[error("HTTP {status}: {body}")]
    Http { status: StatusCode, body: String },
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),
    #[error("decode error: {0}")]
    Decode(String),
    #[error("API error: {code} {message}")]
    Api { code: String, message: String },
}

#[derive(Debug, Serialize)]
pub struct SubmitRequest {
    pub callback_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AnalysisJob {
    pub job_id: String,
    pub status: JobStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Deserialize)]
pub struct AnalysisResult {
    pub job_id: String,
    pub status: JobStatus,
    pub report: Option<DetailedReport>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DetailedReport {
    pub overall_score: f32,
    pub structural_checks: Vec<CheckResult>,
    pub style_checks: Vec<CheckResult>,
    pub reference_checks: Vec<CheckResult>,
}

#[derive(Debug, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub score: f32,
    pub message: String,
}
