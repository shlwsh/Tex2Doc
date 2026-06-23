//! In-memory commercial API state for P7 cloud conversion jobs.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};

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
    usage: RwLock<HashMap<String, u64>>,
    seq: AtomicU64,
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
        Self {
            inner: Arc::new(ServerStateInner {
                uploads: RwLock::new(HashMap::new()),
                jobs: RwLock::new(HashMap::new()),
                recharges: RwLock::new(HashMap::new()),
                entitlements: RwLock::new(HashMap::new()),
                usage: RwLock::new(HashMap::new()),
                seq: AtomicU64::new(1),
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
