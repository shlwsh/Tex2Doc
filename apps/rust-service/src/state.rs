//! Commercial API state backed by PostgreSQL and session file storage.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::db_store::{AppUser, BillingPlanRecord, DbStore, ManualOrderRecord};
use crate::excel_export;
use crate::feedback_service::FeedbackStore;
use crate::file_storage::FileStorage;
use crate::worker_service::WorkerCommand;

pub const PREVIEW_CLOUD_CONVERSION_LIMIT: u64 = 100;

#[derive(Clone)]
pub struct ServerState {
    db: DbStore,
    queue: mpsc::Sender<WorkerCommand>,
    file_storage: FileStorage,
    feedback_store: FeedbackStore,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UploadRecord {
    pub upload_id: String,
    pub file_name: String,
    pub bytes: Vec<u8>,
    pub storage_key: Option<String>,
    pub storage_path: Option<String>,
    pub bytes_size: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversionStatus {
    Queued,
    Normalizing,
    Detecting,
    Analyzing,
    Compiling,
    Rendering,
    Verifying,
    Completed,
    Failed,
    Expired,
}

impl ConversionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Normalizing => "normalizing",
            Self::Detecting => "detecting",
            Self::Analyzing => "analyzing",
            Self::Compiling => "compiling",
            Self::Rendering => "rendering",
            Self::Verifying => "verifying",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Expired => "expired",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Self {
        match value {
            "normalizing" => Self::Normalizing,
            "detecting" => Self::Detecting,
            "analyzing" => Self::Analyzing,
            "compiling" => Self::Compiling,
            "rendering" => Self::Rendering,
            "verifying" => Self::Verifying,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "expired" => Self::Expired,
            _ => Self::Queued,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConversionJobRecord {
    pub job_id: String,
    pub user_id: String,
    pub upload_id: String,
    pub main_tex: String,
    pub profile: String,
    pub quality: String,
    pub engine: String,
    pub status: ConversionStatus,
    pub created_at: String,
    pub updated_at: String,
    pub docx: Option<Vec<u8>>,
    pub report: Option<ConversionReportRecord>,
    pub error_code: Option<String>,
    pub error: Option<String>,
    pub storage_path: Option<String>,
    pub source_zip_key: Option<String>,
    pub result_docx_key: Option<String>,
    pub result_log_key: Option<String>,
    pub zip_bytes: Option<u64>,
    pub docx_bytes: Option<u64>,
    pub log_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RechargeRecord {
    pub recharge_id: String,
    pub user_id: String,
    pub recharge_type: String,
    pub package_id: String,
    pub quantity: u64,
    pub amount_cents: u64,
    pub currency: String,
    pub status: String,
    pub provider: String,
    pub provider_trade_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemPackage {
    pub id: String,
    pub name: String,
    pub package_type: String,
    pub quantity: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemCodeBatchRecord {
    pub batch_id: String,
    pub batch_no: String,
    pub batch_prefix: String,
    pub package_id: String,
    pub package_name: String,
    pub recharge_type: String,
    pub quantity: u64,
    pub generated_count: u64,
    pub exported_count: u64,
    pub status: String,
    pub channel: Option<String>,
    pub note: Option<String>,
    pub expires_at: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemCodeRecord {
    pub code_id: String,
    pub batch_id: String,
    pub batch_no: String,
    pub package_id: String,
    pub package_name: String,
    pub recharge_type: String,
    pub quantity: u64,
    pub code_hash: String,
    pub code_ciphertext: Vec<u8>,
    pub code_nonce: Vec<u8>,
    pub code_preview: String,
    pub plaintext_code: String,
    pub key_version: String,
    pub status: String,
    pub stock_status: String,
    pub stocked_by: Option<String>,
    pub stocked_at: Option<String>,
    pub redeemed_by: Option<String>,
    pub redeemed_recharge_id: Option<String>,
    pub redeemed_at: Option<String>,
    pub restocked_by: Option<String>,
    pub restocked_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct RedeemCodeResult {
    pub redeem_id: String,
    pub recharge_id: String,
    pub package_id: String,
    pub package_name: String,
    pub recharge_type: String,
    pub quantity: u64,
    pub count_balance: u64,
    pub date_valid_until: Option<String>,
    pub redeemed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedeemFailure {
    InvalidCode,
    AlreadyRedeemed,
    Voided,
    Expired,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct EntitlementRecord {
    pub user_id: String,
    pub count_balance: u64,
    pub valid_until: Option<String>,
    pub source_order_id: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionReportRecord {
    pub job_id: String,
    pub status: ConversionStatus,
    pub quality_score: u8,
    pub profile: String,
    pub main_tex: String,
    pub executor: String,
    pub backend: String,
    pub quality_status: String,
    pub compatibility_score: Option<u8>,
    pub docx_bytes: usize,
    pub warnings: Vec<String>,
    pub error_code: Option<String>,
    pub message: String,
}

impl ServerState {
    pub async fn new(queue: mpsc::Sender<WorkerCommand>) -> Result<Self, sqlx::Error> {
        let db = DbStore::connect_from_env().await?;
        let file_storage = FileStorage::new(PathBuf::from("sessions"))
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
        let feedback_store = FeedbackStore::new(db.clone());
        Ok(Self {
            db,
            queue,
            file_storage,
            feedback_store,
        })
    }

    pub async fn register_user(
        &self,
        email: &str,
        display_name: Option<&str>,
        password: &str,
    ) -> Result<AppUser, sqlx::Error> {
        self.db.register_user(email, display_name, password).await
    }

    pub async fn login_user(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<AppUser>, sqlx::Error> {
        self.db.login_user(email, password).await
    }

    pub async fn issue_token(&self, user_id: &str, prefix: &str) -> Result<String, sqlx::Error> {
        self.db.issue_token(user_id, prefix).await
    }

    pub async fn user_for_token(&self, token: &str) -> Result<Option<AppUser>, sqlx::Error> {
        self.db.user_for_token(token).await
    }

    pub async fn user_for_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<AppUser>, sqlx::Error> {
        self.db.user_for_refresh_token(token).await
    }

    pub async fn list_billing_plans(&self) -> Result<Vec<BillingPlanRecord>, sqlx::Error> {
        self.db.list_billing_plans().await
    }

    pub async fn list_redeem_packages(&self) -> Result<Vec<RedeemPackage>, sqlx::Error> {
        self.db.list_redeem_packages().await
    }

    pub async fn latest_release_manifest(
        &self,
        channel: &str,
    ) -> Result<Option<serde_json::Value>, sqlx::Error> {
        self.db.latest_release_manifest(channel).await
    }

    pub async fn store_upload(
        &self,
        user_id: &str,
        file_name: String,
        bytes: Vec<u8>,
    ) -> Result<UploadRecord, String> {
        let upload_id = Uuid::new_v4().to_string();
        let object_key = self.file_storage.file_key(&upload_id, "source.zip");
        self.file_storage
            .store(&upload_id, "source.zip", &bytes)
            .map_err(|e| e.to_string())?;
        self.db
            .store_upload(&upload_id, user_id, file_name, object_key, bytes)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_upload(&self, upload_id: &str) -> Option<UploadRecord> {
        let mut upload = self.db.get_upload(upload_id).await.ok().flatten()?;
        if let Some(key) = upload.storage_key.as_deref() {
            upload.bytes = self.file_storage.load_key(key).ok()?;
        }
        Some(upload)
    }

    pub async fn create_job(
        &self,
        user_id: String,
        upload_id: String,
        main_tex: String,
        profile: String,
        quality: String,
        engine: String,
    ) -> Result<ConversionJobRecord, String> {
        let mut job = self
            .db
            .create_job(
                user_id,
                upload_id.clone(),
                main_tex,
                profile,
                quality,
                engine,
            )
            .await
            .map_err(|e| e.to_string())?;
        if let Some(upload) = self.get_upload(&upload_id).await {
            let source_key = self.file_storage.file_key(&job.job_id, "source.zip");
            self.file_storage
                .store(&job.job_id, "source.zip", &upload.bytes)
                .map_err(|e| e.to_string())?;
            self.db
                .update_job_source_storage(
                    &job.job_id,
                    source_key.clone(),
                    upload.bytes.len() as u64,
                )
                .await
                .map_err(|e| e.to_string())?;
            job.source_zip_key = Some(source_key.clone());
            job.storage_path = Some(source_key);
            job.zip_bytes = Some(upload.bytes.len() as u64);
        }
        Ok(job)
    }

    pub async fn enqueue_job(&self, job_id: String) -> Result<(), String> {
        self.queue
            .send(WorkerCommand { job_id })
            .await
            .map_err(|e| format!("conversion queue unavailable: {e}"))
    }

    pub async fn claim_next_job(&self, worker_id: &str) -> Result<Option<String>, sqlx::Error> {
        self.db.claim_next_job(worker_id).await
    }

    pub async fn recover_stale_jobs(&self) -> Result<u64, sqlx::Error> {
        self.db.recover_stale_jobs().await
    }

    pub async fn get_job(&self, job_id: &str) -> Option<ConversionJobRecord> {
        let mut job = self.db.get_job(job_id).await.ok().flatten()?;
        if let Some(key) = job.result_docx_key.as_deref() {
            job.docx = self.file_storage.load_key(key).ok();
        }
        Some(job)
    }

    pub async fn list_jobs_by_user(&self, user_id: &str) -> Vec<ConversionJobRecord> {
        self.db.list_jobs_by_user(user_id).await.unwrap_or_default()
    }

    pub async fn cloud_conversions_used(&self, user_id: &str) -> u64 {
        self.db
            .cloud_conversions_used(user_id)
            .await
            .unwrap_or_default()
    }

    pub async fn entitlement(&self, user_id: &str) -> EntitlementRecord {
        self.db
            .entitlement(user_id)
            .await
            .unwrap_or_else(|_| EntitlementRecord {
                user_id: user_id.to_string(),
                updated_at: now_timestamp(),
                ..EntitlementRecord::default()
            })
    }

    pub async fn reserve_cloud_conversion(&self, user_id: &str, job_id: &str) -> Result<u64, u64> {
        self.db.reserve_cloud_conversion(user_id, job_id).await
    }

    pub async fn consume_local_conversion(&self, user_id: &str) -> Result<u64, u64> {
        self.db.consume_local_conversion(user_id).await
    }

    pub async fn create_recharge(
        &self,
        user_id: String,
        recharge_type: String,
        package_id: String,
        quantity: u64,
        amount_cents: u64,
    ) -> Result<RechargeRecord, String> {
        self.db
            .create_recharge(
                user_id,
                recharge_type,
                package_id,
                quantity,
                amount_cents,
                "pending_manual",
                "manual-request",
                format!("manual_request_{}", Uuid::new_v4().simple()),
            )
            .await
            .map_err(|e| e.to_string())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_manual_order(
        &self,
        user_id: String,
        recharge_type: String,
        package_id: String,
        quantity: u64,
        amount_cents: u64,
        operator_id: String,
        payment_note: Option<String>,
    ) -> Result<ManualOrderRecord, String> {
        self.db
            .create_manual_order(
                user_id,
                recharge_type,
                package_id,
                quantity,
                amount_cents,
                operator_id,
                payment_note,
            )
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn list_manual_orders(&self) -> Result<Vec<ManualOrderRecord>, sqlx::Error> {
        self.db.list_manual_orders().await
    }

    pub async fn list_users(&self) -> Result<Vec<serde_json::Value>, sqlx::Error> {
        self.db.list_users().await
    }

    pub async fn list_usage_ledger(&self) -> Result<Vec<serde_json::Value>, sqlx::Error> {
        self.db.list_usage_ledger().await
    }

    pub async fn create_waitlist_lead(
        &self,
        email: String,
        identity: Option<String>,
        paper_type: Option<String>,
        current_tool: Option<String>,
        pain_point: Option<String>,
        paid_intent: Option<String>,
    ) -> Result<serde_json::Value, sqlx::Error> {
        self.db
            .create_waitlist_lead(
                email,
                identity,
                paper_type,
                current_tool,
                pain_point,
                paid_intent,
            )
            .await
    }

    pub async fn list_waitlist_leads(&self) -> Result<Vec<serde_json::Value>, sqlx::Error> {
        self.db.list_waitlist_leads().await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn publish_release(
        &self,
        actor: &str,
        channel: String,
        platform: String,
        arch: String,
        version: String,
        download_url: String,
        sha256: String,
        signature: Option<String>,
        file_size_bytes: u64,
        release_title: Option<String>,
        release_notes: Option<String>,
        strategy_type: Option<String>,
    ) -> Result<serde_json::Value, sqlx::Error> {
        self.db
            .publish_release(
                actor,
                channel,
                platform,
                arch,
                version,
                download_url,
                sha256,
                signature,
                file_size_bytes,
                release_title,
                release_notes,
                strategy_type,
            )
            .await
    }

    pub async fn list_release_manifests(&self) -> Result<Vec<serde_json::Value>, sqlx::Error> {
        self.db.list_release_manifests().await
    }

    pub async fn rollback_release(
        &self,
        actor: &str,
        release_id: &str,
        reason: Option<String>,
    ) -> Result<serde_json::Value, sqlx::Error> {
        self.db.rollback_release(actor, release_id, reason).await
    }

    pub async fn list_release_audit(&self) -> Result<Vec<serde_json::Value>, sqlx::Error> {
        self.db.list_release_audit().await
    }

    pub async fn create_redeem_batch(
        &self,
        package_id: &str,
        requested_count: u64,
        channel: Option<String>,
        note: Option<String>,
        expires_at: Option<String>,
        created_by: String,
    ) -> Result<RedeemCodeBatchRecord, RedeemFailure> {
        self.db
            .create_redeem_batch(
                package_id,
                requested_count,
                channel,
                note,
                expires_at,
                created_by,
            )
            .await
    }

    pub async fn list_redeem_batches(&self) -> Vec<RedeemCodeBatchRecord> {
        self.db.list_redeem_batches().await.unwrap_or_default()
    }

    pub async fn get_redeem_batch(&self, batch_id: &str) -> Option<RedeemCodeBatchRecord> {
        self.db
            .get_redeem_batch(batch_id, true)
            .await
            .ok()
            .flatten()
    }

    pub async fn get_redeem_batch_detail(&self, batch_id: &str) -> Option<RedeemCodeBatchRecord> {
        self.db
            .get_redeem_batch(batch_id, true)
            .await
            .ok()
            .flatten()
    }

    pub async fn mark_redeem_batch_exported(&self, batch_id: &str) -> Result<(), String> {
        self.db
            .mark_redeem_batch_exported(batch_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn redeem_code(
        &self,
        user_id: String,
        input_code: String,
    ) -> Result<RedeemCodeResult, RedeemFailure> {
        self.db.redeem_code(user_id, input_code).await
    }

    pub async fn list_redeem_records(&self, user_id: &str) -> Vec<RedeemCodeRecord> {
        self.db
            .list_redeem_records(user_id)
            .await
            .unwrap_or_default()
    }

    /// Admin: list redeem codes with optional filters.
    pub async fn admin_list_redeem_codes(
        &self,
        stock_status: Option<&str>,
        batch_id: Option<&str>,
        package_id: Option<&str>,
        search: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Vec<RedeemCodeRecord> {
        self.db
            .admin_list_redeem_codes(stock_status, batch_id, package_id, search, limit, offset)
            .await
            .unwrap_or_default()
    }

    /// Admin: count redeem codes matched by filters (for pagination).
    pub async fn admin_count_redeem_codes(
        &self,
        stock_status: Option<&str>,
        batch_id: Option<&str>,
        package_id: Option<&str>,
        search: Option<&str>,
    ) -> u64 {
        self.db
            .admin_count_redeem_codes(stock_status, batch_id, package_id, search)
            .await
            .unwrap_or_default()
    }

    /// Admin: bulk mark given codes as "stocked" (上货).
    pub async fn admin_stock_redeem_codes(
        &self,
        admin_id: &str,
        code_ids: &[String],
    ) -> Result<u64, RedeemFailure> {
        self.db.admin_stock_redeem_codes(admin_id, code_ids).await
    }

    /// Admin: reset codes back to "new" (恢复/重置). 文本导入时使用。
    pub async fn admin_restock_redeem_codes(
        &self,
        admin_id: &str,
        codes: &[String],
    ) -> Result<u64, RedeemFailure> {
        self.db.admin_restock_redeem_codes(admin_id, codes).await
    }

    pub async fn list_recharges(&self, user_id: &str) -> Vec<RechargeRecord> {
        self.db.list_recharges(user_id).await.unwrap_or_default()
    }

    pub async fn update_status(&self, job_id: &str, status: ConversionStatus) {
        if let Err(error) = self.db.update_status(job_id, status).await {
            tracing::error!("failed to update conversion status: {error}");
        }
    }

    pub async fn complete_job(&self, job_id: &str, docx: Vec<u8>, report: ConversionReportRecord) {
        let log = FileStorage::build_conversion_log(
            job_id,
            &report.job_id,
            "",
            &report.main_tex,
            &report.profile,
            "",
            &report.executor,
            "completed",
            Some(docx.len()),
            None,
        );
        match (
            self.file_storage.store(job_id, "result.docx", &docx),
            self.file_storage
                .store(job_id, "conversion.log", log.as_bytes()),
        ) {
            (Ok(_), Ok(_)) => {
                let docx_key = self.file_storage.file_key(job_id, "result.docx");
                let log_key = self.file_storage.file_key(job_id, "conversion.log");
                if let Err(error) = self
                    .db
                    .complete_job(
                        job_id,
                        docx_key,
                        docx.len() as u64,
                        log_key,
                        log.len() as u64,
                        &report,
                    )
                    .await
                {
                    tracing::error!("failed to complete conversion job: {error}");
                }
            }
            (docx_result, log_result) => {
                tracing::error!(
                    "failed to persist conversion files: docx={:?} log={:?}",
                    docx_result.err(),
                    log_result.err()
                );
            }
        }
    }

    pub async fn fail_job_with_code(&self, job_id: &str, error_code: &str, error: String) {
        let job = self.db.get_job(job_id).await.ok().flatten();
        let report = ConversionReportRecord {
            job_id: job_id.to_string(),
            status: ConversionStatus::Failed,
            quality_score: 0,
            profile: job.as_ref().map(|j| j.profile.clone()).unwrap_or_default(),
            main_tex: job.as_ref().map(|j| j.main_tex.clone()).unwrap_or_default(),
            executor: job.as_ref().map(|j| j.engine.clone()).unwrap_or_default(),
            backend: "unavailable".to_string(),
            quality_status: "Failed".to_string(),
            compatibility_score: None,
            docx_bytes: 0,
            warnings: Vec::new(),
            error_code: Some(error_code.to_string()),
            message: error.clone(),
        };
        let log = FileStorage::build_conversion_log(
            job_id,
            job.as_ref().map(|j| j.user_id.as_str()).unwrap_or_default(),
            job.as_ref()
                .map(|j| j.upload_id.as_str())
                .unwrap_or_default(),
            &report.main_tex,
            &report.profile,
            "",
            &report.executor,
            "failed",
            None,
            Some(&error),
        );
        let log_key = self.file_storage.file_key(job_id, "conversion.log");
        if let Err(write_error) = self
            .file_storage
            .store(job_id, "conversion.log", log.as_bytes())
        {
            tracing::error!("failed to persist conversion failure log: {write_error}");
        }
        if let Err(db_error) = self
            .db
            .fail_job(
                job_id,
                error_code,
                &error,
                log_key,
                log.len() as u64,
                &report,
            )
            .await
        {
            tracing::error!("failed to mark conversion job failed: {db_error}");
        }
        if let Err(refund_error) = self
            .db
            .refund_cloud_conversion_for_job(job_id, error_code)
            .await
        {
            tracing::error!("failed to refund conversion quota: {refund_error}");
        }
    }

    pub fn load_storage_key(&self, key: &str) -> Option<Vec<u8>> {
        self.file_storage.load_key(key).ok()
    }

    pub fn feedback_store(&self) -> &FeedbackStore {
        &self.feedback_store
    }

    pub fn build_feedback_export(
        &self,
        threads: Vec<crate::feedback_service::FeedbackThreadSummary>,
    ) -> Vec<u8> {
        excel_export::build_admin_feedback_export(&threads)
    }
}

pub fn redeem_packages() -> Vec<RedeemPackage> {
    vec![
        RedeemPackage {
            id: "count_3".to_string(),
            name: "3 次转换包".to_string(),
            package_type: "count".to_string(),
            quantity: 3,
        },
        RedeemPackage {
            id: "count_10".to_string(),
            name: "10 次转换包".to_string(),
            package_type: "count".to_string(),
            quantity: 10,
        },
        RedeemPackage {
            id: "count_30".to_string(),
            name: "30 次转换包".to_string(),
            package_type: "count".to_string(),
            quantity: 30,
        },
    ]
}

pub fn redeem_package(package_id: &str) -> Option<RedeemPackage> {
    redeem_packages()
        .into_iter()
        .find(|package| package.id == package_id)
}

pub fn random_bytes(len: usize) -> Vec<u8> {
    let mut bytes = vec![0_u8; len];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes
}

pub fn generate_redeem_code(batch_prefix: &str) -> String {
    let payload = crockford_base32(&random_bytes(12));
    let body = format!("T2D{batch_prefix}{payload}");
    let check = redeem_checksum(&body);
    let raw = format!("{body}{check}");
    group_redeem_code(&raw)
}

pub fn normalize_redeem_code(code: &str) -> Option<String> {
    let normalized = code
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect::<String>();
    if normalized.starts_with("T2D") && normalized.len() >= 20 {
        Some(normalized)
    } else {
        None
    }
}

pub fn redeem_checksum_valid(normalized: &str) -> bool {
    if normalized.len() < 5 {
        return false;
    }
    let (body, check) = normalized.split_at(normalized.len() - 2);
    redeem_checksum(body) == check
}

fn redeem_checksum(body: &str) -> String {
    let digest = Sha256::digest(body.as_bytes());
    crockford_base32(&digest[..2]).chars().take(2).collect()
}

pub fn code_hash(normalized: &str) -> String {
    let pepper = std::env::var("REDEEM_CODE_PEPPER")
        .unwrap_or_else(|_| "tex2doc-preview-pepper".to_string());
    hash_text(&format!("{pepper}:{normalized}"))
}

pub fn encrypt_code(normalized: &str, nonce: &[u8]) -> Vec<u8> {
    xor_code_stream(normalized.as_bytes(), nonce)
}

pub fn decrypt_code(ciphertext: &[u8], nonce: &[u8]) -> Result<String, String> {
    String::from_utf8(xor_code_stream(ciphertext, nonce)).map_err(|e| e.to_string())
}

fn xor_code_stream(input: &[u8], nonce: &[u8]) -> Vec<u8> {
    let key = std::env::var("REDEEM_CODE_MASTER_KEY")
        .unwrap_or_else(|_| "tex2doc-preview-master-key".to_string());
    let mut out = input.to_vec();
    let mut offset = 0_usize;
    while offset < out.len() {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        hasher.update(nonce);
        hasher.update((offset as u64).to_le_bytes());
        let stream = hasher.finalize();
        for byte in stream {
            if offset >= out.len() {
                break;
            }
            out[offset] ^= byte;
            offset += 1;
        }
    }
    out
}

pub fn code_preview(code: &str) -> String {
    let compact = code.replace('-', "");
    if compact.len() <= 12 {
        return compact;
    }
    format!("{}****{}", &compact[..8], &compact[compact.len() - 4..])
}

pub fn group_redeem_code(raw: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in raw.chars().enumerate() {
        if idx > 0 && idx % 4 == 0 {
            out.push('-');
        }
        out.push(ch);
    }
    out
}

fn crockford_base32(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut out = String::new();
    let mut buffer = 0_u32;
    let mut bits = 0_u8;
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            let idx = ((buffer >> (bits - 5)) & 0x1f) as usize;
            out.push(ALPHABET[idx] as char);
            bits -= 5;
        }
    }
    if bits > 0 {
        let idx = ((buffer << (5 - bits)) & 0x1f) as usize;
        out.push(ALPHABET[idx] as char);
    }
    out
}

pub fn hash_text(value: &str) -> String {
    hex_lower(&Sha256::digest(value.as_bytes()))
}

pub fn hash_bytes(value: &[u8]) -> String {
    hex_lower(&Sha256::digest(value))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

pub fn now_timestamp() -> String {
    now_secs().to_string()
}
