//! In-memory commercial API state for P7 cloud conversion jobs.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, RwLock};

use crate::excel_export;
use crate::feedback_service::FeedbackStore;
use crate::file_storage::FileStorage;
use crate::worker_service::WorkerCommand;

pub const PREVIEW_CLOUD_CONVERSION_LIMIT: u64 = 100;

#[derive(Clone)]
pub struct ServerState {
    inner: Arc<ServerStateInner>,
    queue: mpsc::Sender<WorkerCommand>,
}

struct ServerStateInner {
    uploads: RwLock<HashMap<String, UploadRecord>>,
    jobs: RwLock<HashMap<String, ConversionJobRecord>>,
    recharges: RwLock<HashMap<String, Vec<RechargeRecord>>>,
    entitlements: RwLock<HashMap<String, EntitlementRecord>>,
    redeem_batches: RwLock<HashMap<String, RedeemCodeBatchRecord>>,
    redeem_codes: RwLock<HashMap<String, RedeemCodeRecord>>,
    redeem_events: RwLock<Vec<RedeemCodeEventRecord>>,
    usage: RwLock<HashMap<String, u64>>,
    seq: AtomicU64,
    file_storage: FileStorage,
    feedback_store: FeedbackStore,
}

#[derive(Debug, Clone)]
pub struct UploadRecord {
    pub upload_id: String,
    pub file_name: String,
    pub bytes: Vec<u8>,
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
}

#[derive(Debug, Clone)]
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
    // Session file storage
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
    pub redeemed_by: Option<String>,
    pub redeemed_recharge_id: Option<String>,
    pub redeemed_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemCodeEventRecord {
    pub code_id: Option<String>,
    pub user_id: Option<String>,
    pub event_type: String,
    pub reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn new(queue: mpsc::Sender<WorkerCommand>) -> Self {
        let file_storage = FileStorage::new(std::path::PathBuf::from("sessions"))
            .expect("sessions root directory should be creatable");
        Self {
            inner: Arc::new(ServerStateInner {
                uploads: RwLock::new(HashMap::new()),
                jobs: RwLock::new(HashMap::new()),
                recharges: RwLock::new(HashMap::new()),
                entitlements: RwLock::new(HashMap::new()),
                redeem_batches: RwLock::new(HashMap::new()),
                redeem_codes: RwLock::new(HashMap::new()),
                redeem_events: RwLock::new(Vec::new()),
                usage: RwLock::new(HashMap::new()),
                seq: AtomicU64::new(1),
                file_storage,
                feedback_store: FeedbackStore::new(),
            }),
            queue,
        }
    }

    pub async fn store_upload(&self, file_name: String, bytes: Vec<u8>) -> UploadRecord {
        let upload_id = self.next_id("upload");
        let record = UploadRecord {
            upload_id: upload_id.clone(),
            file_name,
            bytes,
            created_at: now_timestamp(),
        };
        self.inner
            .uploads
            .write()
            .await
            .insert(upload_id, record.clone());
        record
    }

    pub async fn get_upload(&self, upload_id: &str) -> Option<UploadRecord> {
        self.inner.uploads.read().await.get(upload_id).cloned()
    }

    pub async fn create_job(
        &self,
        user_id: String,
        upload_id: String,
        main_tex: String,
        profile: String,
        quality: String,
        engine: String,
    ) -> ConversionJobRecord {
        let job_id = self.next_id("conv");
        let now = now_timestamp();
        let job = ConversionJobRecord {
            job_id: job_id.clone(),
            user_id,
            upload_id,
            main_tex,
            profile,
            quality,
            engine,
            status: ConversionStatus::Queued,
            created_at: now.clone(),
            updated_at: now,
            docx: None,
            report: None,
            error_code: None,
            error: None,
            storage_path: None,
            source_zip_key: None,
            result_docx_key: None,
            result_log_key: None,
            zip_bytes: None,
            docx_bytes: None,
            log_bytes: None,
        };
        self.inner.jobs.write().await.insert(job_id, job.clone());
        job
    }

    pub async fn enqueue_job(&self, job_id: String) -> Result<(), String> {
        self.queue
            .send(WorkerCommand { job_id })
            .await
            .map_err(|e| format!("conversion queue unavailable: {e}"))
    }

    pub async fn get_job(&self, job_id: &str) -> Option<ConversionJobRecord> {
        self.inner.jobs.read().await.get(job_id).cloned()
    }

    pub async fn list_jobs_by_user(&self, user_id: &str) -> Vec<ConversionJobRecord> {
        let mut jobs = self
            .inner
            .jobs
            .read()
            .await
            .values()
            .filter(|job| job.user_id == user_id)
            .cloned()
            .collect::<Vec<_>>();
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        jobs
    }

    pub async fn cloud_conversions_used(&self, user_id: &str) -> u64 {
        self.inner
            .usage
            .read()
            .await
            .get(user_id)
            .copied()
            .unwrap_or_default()
    }

    pub async fn entitlement(&self, user_id: &str) -> EntitlementRecord {
        self.inner
            .entitlements
            .read()
            .await
            .get(user_id)
            .cloned()
            .unwrap_or_else(|| EntitlementRecord {
                user_id: user_id.to_string(),
                updated_at: now_timestamp(),
                ..EntitlementRecord::default()
            })
    }

    pub async fn try_consume_cloud_conversion(&self, user_id: &str) -> Result<u64, u64> {
        {
            let mut entitlements = self.inner.entitlements.write().await;
            if let Some(entitlement) = entitlements.get_mut(user_id) {
                if entitlement
                    .valid_until
                    .as_deref()
                    .and_then(|value| value.parse::<u64>().ok())
                    .is_some_and(|valid_until| valid_until >= now_secs())
                {
                    entitlement.updated_at = now_timestamp();
                    let used = self.cloud_conversions_used(user_id).await;
                    return Ok(used);
                }
                if entitlement.count_balance > 0 {
                    entitlement.count_balance -= 1;
                    entitlement.updated_at = now_timestamp();
                    let used = self.cloud_conversions_used(user_id).await;
                    return Ok(used);
                }
            }
        }

        let mut usage = self.inner.usage.write().await;
        let used = usage.get(user_id).copied().unwrap_or_default();
        if used >= PREVIEW_CLOUD_CONVERSION_LIMIT {
            return Err(used);
        }
        let next = used + 1;
        usage.insert(user_id.to_string(), next);
        Ok(next)
    }

    pub async fn create_recharge(
        &self,
        user_id: String,
        recharge_type: String,
        package_id: String,
        quantity: u64,
        amount_cents: u64,
    ) -> RechargeRecord {
        let recharge_id = self.next_id("recharge");
        let record = RechargeRecord {
            recharge_id: recharge_id.clone(),
            user_id: user_id.clone(),
            recharge_type,
            package_id,
            quantity,
            amount_cents,
            currency: "CNY".to_string(),
            status: "paid_mock".to_string(),
            provider: "mock-pay".to_string(),
            provider_trade_id: format!("mock_trade_{recharge_id}"),
            created_at: now_timestamp(),
        };
        self.inner
            .recharges
            .write()
            .await
            .entry(user_id)
            .or_default()
            .push(record.clone());
        self.apply_recharge_entitlement(&record).await;
        record
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
        let package = redeem_package(package_id).ok_or(RedeemFailure::InvalidCode)?;
        if requested_count == 0 || requested_count > 10_000 {
            return Err(RedeemFailure::InvalidCode);
        }

        let batch_id = self.next_id("redeem_batch");
        let batch_no = format!(
            "RC{}",
            batch_id.trim_start_matches("redeem_batch_").to_uppercase()
        );
        let batch_prefix = batch_no
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<String>();
        let now = now_timestamp();
        let mut codes = Vec::with_capacity(requested_count as usize);
        let mut records = Vec::with_capacity(requested_count as usize);

        for _ in 0..requested_count {
            let code_id = self.next_id("redeem_code");
            let code = generate_redeem_code(&batch_prefix);
            let normalized = normalize_redeem_code(&code).ok_or(RedeemFailure::InvalidCode)?;
            let code_hash = code_hash(&normalized);
            let nonce = random_bytes(12);
            let ciphertext = encrypt_code(&normalized, &nonce);
            let record = RedeemCodeRecord {
                code_id: code_id.clone(),
                batch_id: batch_id.clone(),
                batch_no: batch_no.clone(),
                package_id: package.id.clone(),
                package_name: package.name.clone(),
                recharge_type: package.package_type.clone(),
                quantity: package.quantity,
                code_hash,
                code_ciphertext: ciphertext,
                code_nonce: nonce,
                code_preview: code_preview(&code),
                plaintext_code: code.clone(),
                key_version: "v1".to_string(),
                status: "unused".to_string(),
                redeemed_by: None,
                redeemed_recharge_id: None,
                redeemed_at: None,
                expires_at: expires_at.clone(),
                created_at: now.clone(),
            };
            codes.push(code);
            records.push(record);
        }

        let batch = RedeemCodeBatchRecord {
            batch_id: batch_id.clone(),
            batch_no,
            batch_prefix,
            package_id: package.id,
            package_name: package.name,
            recharge_type: package.package_type,
            quantity: package.quantity,
            generated_count: requested_count,
            exported_count: 0,
            status: "active".to_string(),
            channel,
            note,
            expires_at,
            created_by,
            created_at: now.clone(),
            codes,
        };

        {
            let mut code_map = self.inner.redeem_codes.write().await;
            for record in records {
                code_map.insert(record.code_hash.clone(), record);
            }
        }
        self.inner
            .redeem_batches
            .write()
            .await
            .insert(batch_id, batch.clone());
        self.inner
            .redeem_events
            .write()
            .await
            .push(RedeemCodeEventRecord {
                code_id: None,
                user_id: Some(batch.created_by.clone()),
                event_type: "generated".to_string(),
                reason: Some(format!("batch {}", batch.batch_no)),
                created_at: now,
            });
        Ok(batch)
    }

    pub async fn get_redeem_batch(&self, batch_id: &str) -> Option<RedeemCodeBatchRecord> {
        self.inner
            .redeem_batches
            .read()
            .await
            .get(batch_id)
            .cloned()
    }

    pub async fn mark_redeem_batch_exported(&self, batch_id: &str) {
        if let Some(batch) = self.inner.redeem_batches.write().await.get_mut(batch_id) {
            batch.exported_count = batch.generated_count;
        }
        self.inner
            .redeem_events
            .write()
            .await
            .push(RedeemCodeEventRecord {
                code_id: None,
                user_id: None,
                event_type: "exported".to_string(),
                reason: Some(batch_id.to_string()),
                created_at: now_timestamp(),
            });
    }

    pub async fn redeem_code(
        &self,
        user_id: String,
        input_code: String,
    ) -> Result<RedeemCodeResult, RedeemFailure> {
        let normalized = normalize_redeem_code(&input_code).ok_or(RedeemFailure::InvalidCode)?;
        if !redeem_checksum_valid(&normalized) {
            self.record_redeem_event(None, Some(user_id), "redeem_failed", Some("invalid_code"))
                .await;
            return Err(RedeemFailure::InvalidCode);
        }
        let hash = code_hash(&normalized);
        let now = now_timestamp();

        let snapshot = {
            let mut codes = self.inner.redeem_codes.write().await;
            let record = match codes.get_mut(&hash) {
                Some(record) => record,
                None => {
                    drop(codes);
                    self.record_redeem_event(
                        None,
                        Some(user_id),
                        "redeem_failed",
                        Some("invalid_code"),
                    )
                    .await;
                    return Err(RedeemFailure::InvalidCode);
                }
            };
            match record.status.as_str() {
                "unused" => {}
                "redeemed" => return Err(RedeemFailure::AlreadyRedeemed),
                "voided" => return Err(RedeemFailure::Voided),
                "expired" => return Err(RedeemFailure::Expired),
                _ => return Err(RedeemFailure::InvalidCode),
            }
            if record
                .expires_at
                .as_deref()
                .and_then(|value| value.parse::<u64>().ok())
                .is_some_and(|expires_at| expires_at < now_secs())
            {
                record.status = "expired".to_string();
                return Err(RedeemFailure::Expired);
            }
            record.status = "redeemed".to_string();
            record.redeemed_by = Some(user_id.clone());
            record.redeemed_at = Some(now.clone());
            record.clone()
        };

        let recharge = self
            .create_recharge_from_provider(
                user_id.clone(),
                snapshot.recharge_type.clone(),
                snapshot.package_id.clone(),
                snapshot.quantity,
                0,
                "redeem-code".to_string(),
                snapshot.code_id.clone(),
            )
            .await;

        {
            let mut codes = self.inner.redeem_codes.write().await;
            if let Some(record) = codes.get_mut(&hash) {
                record.redeemed_recharge_id = Some(recharge.recharge_id.clone());
            }
        }
        let entitlement = self.entitlement(&user_id).await;
        self.record_redeem_event(
            Some(snapshot.code_id.clone()),
            Some(user_id),
            "redeem_success",
            None,
        )
        .await;
        Ok(RedeemCodeResult {
            redeem_id: snapshot.code_id,
            recharge_id: recharge.recharge_id,
            package_id: snapshot.package_id,
            package_name: snapshot.package_name,
            recharge_type: snapshot.recharge_type,
            quantity: snapshot.quantity,
            count_balance: entitlement.count_balance,
            date_valid_until: entitlement.valid_until,
            redeemed_at: now,
        })
    }

    pub async fn list_redeem_records(&self, user_id: &str) -> Vec<RedeemCodeRecord> {
        let mut records = self
            .inner
            .redeem_codes
            .read()
            .await
            .values()
            .filter(|record| record.redeemed_by.as_deref() == Some(user_id))
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|a, b| b.redeemed_at.cmp(&a.redeemed_at));
        records
    }

    pub async fn list_recharges(&self, user_id: &str) -> Vec<RechargeRecord> {
        let mut records = self
            .inner
            .recharges
            .read()
            .await
            .get(user_id)
            .cloned()
            .unwrap_or_default();
        records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        records
    }

    pub async fn update_status(&self, job_id: &str, status: ConversionStatus) {
        if let Some(job) = self.inner.jobs.write().await.get_mut(job_id) {
            job.status = status;
            job.updated_at = now_timestamp();
        }
    }

    pub async fn complete_job(&self, job_id: &str, docx: Vec<u8>, report: ConversionReportRecord) {
        if let Some(job) = self.inner.jobs.write().await.get_mut(job_id) {
            job.status = ConversionStatus::Completed;
            job.updated_at = now_timestamp();
            job.docx = Some(docx);
            job.report = Some(report);
            job.error = None;
        }
    }

    pub async fn fail_job_with_code(&self, job_id: &str, error_code: &str, error: String) {
        if let Some(job) = self.inner.jobs.write().await.get_mut(job_id) {
            job.status = ConversionStatus::Failed;
            job.updated_at = now_timestamp();
            job.error_code = Some(error_code.to_string());
            job.report = Some(ConversionReportRecord {
                job_id: job.job_id.clone(),
                status: ConversionStatus::Failed,
                quality_score: 0,
                profile: job.profile.clone(),
                main_tex: job.main_tex.clone(),
                executor: job.engine.clone(),
                backend: "unavailable".to_string(),
                quality_status: "Failed".to_string(),
                compatibility_score: None,
                docx_bytes: 0,
                warnings: Vec::new(),
                error_code: Some(error_code.to_string()),
                message: error.clone(),
            });
            job.error = Some(error);
        }
    }

    fn next_id(&self, prefix: &str) -> String {
        let next = self.inner.seq.fetch_add(1, Ordering::Relaxed);
        format!("{prefix}_{next:016x}")
    }

    async fn apply_recharge_entitlement(&self, record: &RechargeRecord) {
        let mut entitlements = self.inner.entitlements.write().await;
        let entitlement = entitlements
            .entry(record.user_id.clone())
            .or_insert_with(|| EntitlementRecord {
                user_id: record.user_id.clone(),
                updated_at: now_timestamp(),
                ..EntitlementRecord::default()
            });
        match record.recharge_type.as_str() {
            "count" => {
                entitlement.count_balance =
                    entitlement.count_balance.saturating_add(record.quantity);
            }
            "date" => {
                let current_until = entitlement
                    .valid_until
                    .as_deref()
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or_default();
                let base = current_until.max(now_secs());
                let next_until = base.saturating_add(record.quantity.saturating_mul(86_400));
                entitlement.valid_until = Some(next_until.to_string());
            }
            _ => {}
        }
        entitlement.source_order_id = Some(record.recharge_id.clone());
        entitlement.updated_at = now_timestamp();
    }

    async fn create_recharge_from_provider(
        &self,
        user_id: String,
        recharge_type: String,
        package_id: String,
        quantity: u64,
        amount_cents: u64,
        provider: String,
        provider_trade_id: String,
    ) -> RechargeRecord {
        let recharge_id = self.next_id("recharge");
        let record = RechargeRecord {
            recharge_id: recharge_id.clone(),
            user_id: user_id.clone(),
            recharge_type,
            package_id,
            quantity,
            amount_cents,
            currency: "CNY".to_string(),
            status: "paid".to_string(),
            provider,
            provider_trade_id,
            created_at: now_timestamp(),
        };
        self.inner
            .recharges
            .write()
            .await
            .entry(user_id)
            .or_default()
            .push(record.clone());
        self.apply_recharge_entitlement(&record).await;
        record
    }

    async fn record_redeem_event(
        &self,
        code_id: Option<String>,
        user_id: Option<String>,
        event_type: &str,
        reason: Option<&str>,
    ) {
        self.inner
            .redeem_events
            .write()
            .await
            .push(RedeemCodeEventRecord {
                code_id,
                user_id,
                event_type: event_type.to_string(),
                reason: reason.map(str::to_string),
                created_at: now_timestamp(),
            });
    }

    // ─────────────────────────────────────────────────────────────────────────
    // File storage helpers (session-based)
    // ─────────────────────────────────────────────────────────────────────────

    /// Load a session file for a conversion job.
    pub fn load_session_file(&self, job_id: &str, filename: &str) -> Option<Vec<u8>> {
        self.inner.file_storage.load(job_id, filename).ok()
    }

    /// Check if a session file exists for a conversion job.
    pub fn session_file_exists(&self, job_id: &str, filename: &str) -> bool {
        self.inner.file_storage.exists(job_id, filename)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Feedback store passthrough methods
    // ─────────────────────────────────────────────────────────────────────────

    pub fn feedback_store(&self) -> &FeedbackStore {
        &self.inner.feedback_store
    }

    /// Build an Excel export for feedback threads.
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

fn redeem_package(package_id: &str) -> Option<RedeemPackage> {
    redeem_packages()
        .into_iter()
        .find(|package| package.id == package_id)
}

fn random_bytes(len: usize) -> Vec<u8> {
    let mut bytes = vec![0_u8; len];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes
}

fn generate_redeem_code(batch_prefix: &str) -> String {
    let payload = crockford_base32(&random_bytes(12));
    let body = format!("T2D{batch_prefix}{payload}");
    let check = redeem_checksum(&body);
    let raw = format!("{body}{check}");
    group_redeem_code(&raw)
}

fn normalize_redeem_code(code: &str) -> Option<String> {
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

fn redeem_checksum_valid(normalized: &str) -> bool {
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

fn code_hash(normalized: &str) -> String {
    let pepper = std::env::var("REDEEM_CODE_PEPPER")
        .unwrap_or_else(|_| "tex2doc-preview-pepper".to_string());
    let mut hasher = Sha256::new();
    hasher.update(pepper.as_bytes());
    hasher.update(b":");
    hasher.update(normalized.as_bytes());
    hex_lower(&hasher.finalize())
}

fn encrypt_code(normalized: &str, nonce: &[u8]) -> Vec<u8> {
    let key = std::env::var("REDEEM_CODE_MASTER_KEY")
        .unwrap_or_else(|_| "tex2doc-preview-master-key".to_string());
    let mut out = normalized.as_bytes().to_vec();
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

fn code_preview(code: &str) -> String {
    let compact = code.replace('-', "");
    if compact.len() <= 12 {
        return compact;
    }
    format!("{}****{}", &compact[..8], &compact[compact.len() - 4..])
}

fn group_redeem_code(raw: &str) -> String {
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

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

pub fn now_timestamp() -> String {
    now_secs().to_string()
}
