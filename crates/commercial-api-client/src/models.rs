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
    pub period_start: String,
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
