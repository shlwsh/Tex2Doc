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

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisJob {
    pub job_id: String,
    pub status: JobStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Normalizing,
    Detecting,
    Analyzing,
    Compiling,
    Rendering,
    Verifying,
    Pending,
    Processing,
    Completed,
    Failed,
    Expired,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub job_id: String,
    pub status: JobStatus,
    pub report: Option<DetailedReport>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DetailedReport {
    pub overall_score: f32,
    pub structural_checks: Vec<CheckResult>,
    pub style_checks: Vec<CheckResult>,
    pub reference_checks: Vec<CheckResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub score: f32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserProfile,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub plan_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageSummary {
    pub plan_id: String,
    pub cloud_conversions_used: u32,
    pub cloud_conversions_limit: u32,
    #[serde(default)]
    pub count_balance: u32,
    #[serde(default)]
    pub date_valid_until: Option<String>,
    #[serde(default)]
    pub entitlement_source_order_id: Option<String>,
    pub storage_bytes_used: u64,
    pub storage_bytes_limit: u64,
    #[serde(default)]
    pub period_start: String,
    #[serde(default)]
    pub period_end: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlanSummary {
    pub id: String,
    pub name: String,
    pub price_cents: u32,
    pub currency: String,
    pub monthly_conversions: u32,
    pub features: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckoutRequest {
    pub plan_id: String,
    pub success_url: String,
    pub cancel_url: String,
}

#[derive(Debug, Serialize)]
pub struct BillingPortalRequest {
    pub return_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BillingSession {
    pub url: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RechargeRecord {
    pub recharge_id: String,
    pub recharge_type: String,
    pub package_id: String,
    pub quantity: u32,
    pub amount_cents: u32,
    pub currency: String,
    pub status: String,
    pub provider: String,
    pub provider_trade_id: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedeemCodeOptions {
    pub enabled: bool,
    pub provider: String,
    pub code_format_hint: String,
    pub support_text: String,
    #[serde(default)]
    pub packages: Vec<RedeemPackageSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedeemPackageSummary {
    pub id: String,
    pub name: String,
    pub recharge_type: String,
    pub quantity: u32,
}

#[derive(Debug, Serialize)]
pub struct RedeemCodeRequest {
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedeemCodeResult {
    pub redeem_id: String,
    pub recharge_id: String,
    pub package_id: String,
    pub package_name: String,
    pub recharge_type: String,
    pub quantity: u32,
    pub count_balance: u32,
    pub date_valid_until: Option<String>,
    pub redeemed_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedeemCodeRecord {
    pub redeem_id: String,
    pub batch_id: String,
    pub batch_no: String,
    pub code_preview: String,
    pub package_id: String,
    pub package_name: String,
    pub recharge_type: String,
    pub quantity: u32,
    pub status: String,
    pub redeemed_recharge_id: Option<String>,
    pub redeemed_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub upload_id: String,
    pub status: String,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct CreateConversionRequest {
    pub upload_id: String,
    pub main_tex: String,
    pub profile: String,
    pub quality: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversionJob {
    pub job_id: String,
    pub upload_id: Option<String>,
    pub main_tex: Option<String>,
    pub profile: Option<String>,
    pub quality: Option<String>,
    pub engine: Option<String>,
    pub status: JobStatus,
    pub created_at: String,
    pub updated_at: String,
    pub docx_ready: bool,
    pub report_ready: bool,
    pub error_code: Option<String>,
    pub error: Option<String>,
    #[serde(default, alias = "storage")]
    pub storage_info: Option<ConversionStorageInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversionReport {
    pub job_id: String,
    pub status: JobStatus,
    pub quality_score: u8,
    pub profile: String,
    pub main_tex: Option<String>,
    pub executor: Option<String>,
    pub backend: Option<String>,
    pub quality_status: Option<String>,
    pub compatibility_score: Option<u8>,
    pub docx_bytes: Option<usize>,
    pub warnings: Option<Vec<String>>,
    pub error_code: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub version: String,
    pub channel: String,
    pub download_url: String,
    pub sha256: String,
    pub signature: String,
    pub release_notes: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Feedback module models
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackThread {
    pub thread_id: String,
    pub user_id: Option<String>,
    pub conversion_job_id: Option<String>,
    pub title: String,
    pub feedback_type: String,
    pub status: String,
    pub priority: String,
    pub admin_assignee: Option<String>,
    pub message_count: Option<u32>,
    pub latest_message_at: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackMessage {
    pub message_id: String,
    pub thread_id: String,
    pub parent_message_id: Option<String>,
    pub sender_user_id: Option<String>,
    pub sender_type: String,
    pub content: String,
    pub attachments: Vec<FeedbackAttachment>,
    pub is_internal: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackAttachment {
    pub filename: String,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateFeedbackRequest {
    pub title: String,
    pub feedback_type: String,
    pub content: String,
    pub conversion_job_id: Option<String>,
    pub priority: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateFeedbackResponse {
    pub thread_id: String,
    pub status: String,
    pub created_at: String,
    pub message_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddMessageRequest {
    pub content: String,
    pub parent_message_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackThreadDetail {
    pub thread: FeedbackThread,
    pub messages: Vec<FeedbackMessage>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Session file storage models
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionStorageInfo {
    pub path: Option<String>,
    #[serde(default)]
    pub source_zip: Option<FileMeta>,
    #[serde(default)]
    pub source_zip_key: Option<String>,
    #[serde(default)]
    pub zip_bytes: Option<u64>,
    #[serde(default)]
    pub result_docx: Option<FileMeta>,
    #[serde(default)]
    pub result_docx_key: Option<String>,
    #[serde(default)]
    pub docx_bytes: Option<u64>,
    #[serde(default)]
    pub conversion_log: Option<FileMeta>,
    #[serde(default, alias = "result_log_key")]
    pub conversion_log_key: Option<String>,
    #[serde(default)]
    pub log_bytes: Option<u64>,
}

impl ConversionStorageInfo {
    #[inline]
    #[allow(dead_code)]
    pub fn has_docx(&self) -> bool {
        self.result_docx.is_some() || self.result_docx_key.is_some()
    }

    #[inline]
    #[allow(dead_code)]
    pub fn has_zip(&self) -> bool {
        self.source_zip.is_some() || self.source_zip_key.is_some()
    }

    #[inline]
    #[allow(dead_code)]
    pub fn has_log(&self) -> bool {
        self.conversion_log.is_some() || self.conversion_log_key.is_some()
    }

    #[inline]
    #[allow(dead_code)]
    pub fn docx_size(&self) -> Option<u64> {
        self.result_docx
            .as_ref()
            .and_then(|d| d.bytes)
            .or(self.docx_bytes)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn zip_size(&self) -> Option<u64> {
        self.source_zip
            .as_ref()
            .and_then(|d| d.bytes)
            .or(self.zip_bytes)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn log_size(&self) -> Option<u64> {
        self.conversion_log
            .as_ref()
            .and_then(|d| d.bytes)
            .or(self.log_bytes)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn has_any(&self) -> bool {
        self.has_zip() || self.has_docx() || self.has_log()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMeta {
    pub key: String,
    pub bytes: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalQuotaCheckResponse {
    pub allowed: bool,
    pub valid_until_active: bool,
    pub count_balance: u32,
    pub used: u32,
    pub limit: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalQuotaConsumeResponse {
    pub consumed: bool,
    pub balance: u32,
}
