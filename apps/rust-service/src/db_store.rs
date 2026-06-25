//! PostgreSQL-backed commercial state.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

static SCHEMA_INIT_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
static SCHEMA_READY: AtomicBool = AtomicBool::new(false);

use crate::feedback_service::{
    FeedbackMessage, FeedbackPriority, FeedbackStatus, FeedbackThread, FeedbackThreadSummary,
    FeedbackType, SenderType, ThreadFilters,
};
use crate::state::{
    code_hash, decrypt_code, encrypt_code, generate_redeem_code, group_redeem_code, hash_bytes,
    hash_text, normalize_redeem_code, now_timestamp, random_bytes, redeem_checksum_valid,
    redeem_package, ConversionJobRecord, ConversionReportRecord, ConversionStatus,
    EntitlementRecord, RechargeRecord, RedeemCodeBatchRecord, RedeemCodeRecord, RedeemCodeResult,
    RedeemFailure, RedeemPackage, UploadRecord, PREVIEW_CLOUD_CONVERSION_LIMIT,
};

const BUSINESS_SCHEMA: &str = include_str!("../../../docs-zh/money/001_docdb_business_schema.sql");
const REDEEM_STOCK_SCHEMA: &str =
    include_str!("../../../docs-zh/money/002_redeem_codes_stock_status.sql");
const FEEDBACK_SCHEMA: &str =
    include_str!("../../../docs-zh/money/003_feedback_and_session_storage.sql");

#[derive(Debug, Clone)]
pub struct DbStore {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct AppUser {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub plan_id: String,
    pub role: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct BillingPlanRecord {
    pub id: String,
    pub name: String,
    pub currency: String,
    pub price_cents: u64,
    pub monthly_conversions: u64,
    pub storage_bytes: u64,
    pub features: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ManualOrderRecord {
    pub order_id: String,
    pub user_id: String,
    pub recharge_id: Option<String>,
    pub recharge_type: String,
    pub package_id: String,
    pub quantity: u64,
    pub amount_cents: u64,
    pub currency: String,
    pub status: String,
    pub operator_id: Option<String>,
    pub payment_note: Option<String>,
    pub created_at: String,
}

impl DbStore {
    pub async fn connect_from_env() -> Result<Self, sqlx::Error> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/docdb".to_string());
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await?;
        let store = Self { pool };
        if !SCHEMA_READY.load(Ordering::Acquire) {
            let _guard = SCHEMA_INIT_LOCK
                .get_or_init(|| tokio::sync::Mutex::new(()))
                .lock()
                .await;
            if !SCHEMA_READY.load(Ordering::Acquire) {
                store.init_schema().await?;
                SCHEMA_READY.store(true, Ordering::Release);
            }
        }
        Ok(store)
    }

    async fn init_schema(&self) -> Result<(), sqlx::Error> {
        sqlx::raw_sql(BUSINESS_SCHEMA).execute(&self.pool).await?;
        sqlx::raw_sql(REDEEM_STOCK_SCHEMA).execute(&self.pool).await?;
        sqlx::raw_sql(FEEDBACK_SCHEMA).execute(&self.pool).await?;
        self.seed_admin_from_env().await?;
        Ok(())
    }

    async fn seed_admin_from_env(&self) -> Result<(), sqlx::Error> {
        let Ok(email) = std::env::var("TEX2DOC_BOOTSTRAP_ADMIN_EMAIL") else {
            return Ok(());
        };
        let Ok(password) = std::env::var("TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD") else {
            return Ok(());
        };
        if email.trim().is_empty() || password.trim().is_empty() {
            return Ok(());
        }
        sqlx::query(
            r#"
            INSERT INTO app_users (email, password_hash, display_name, default_plan_id, role, status)
            VALUES ($1, $2, 'Tex2Doc Admin', 'preview', 'admin', 'active')
            ON CONFLICT (email) DO UPDATE SET
                password_hash = EXCLUDED.password_hash,
                role = 'admin',
                status = 'active',
                updated_at = now()
            "#,
        )
        .bind(email.trim())
        .bind(hash_text(password.trim()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn register_user(
        &self,
        email: &str,
        display_name: Option<&str>,
        password: &str,
    ) -> Result<AppUser, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO app_users (email, password_hash, display_name, default_plan_id, status)
            VALUES ($1, $2, $3, 'preview', 'active')
            RETURNING id::text, email, display_name, default_plan_id, role, status
            "#,
        )
        .bind(email)
        .bind(hash_text(password))
        .bind(display_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(app_user_from_row(&row))
    }

    pub async fn login_user(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<AppUser>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id::text, email, display_name, default_plan_id, role, status, password_hash
            FROM app_users
            WHERE email = $1 AND status = 'active'
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let stored_hash: String = row.get("password_hash");
        if stored_hash != hash_text(password) {
            return Ok(None);
        }
        sqlx::query("UPDATE app_users SET last_login_at = now(), updated_at = now() WHERE id = $1")
            .bind(parse_uuid(row.get::<String, _>("id").as_str())?)
            .execute(&self.pool)
            .await?;
        Ok(Some(app_user_from_row(&row)))
    }

    pub async fn issue_token(&self, user_id: &str, prefix: &str) -> Result<String, sqlx::Error> {
        let token = format!("{prefix}-{}", Uuid::new_v4().simple());
        if prefix.contains("refresh") {
            sqlx::query(
                r#"
                INSERT INTO auth_refresh_tokens (user_id, token_hash, expires_at)
                VALUES ($1, $2, now() + interval '30 days')
                "#,
            )
            .bind(parse_uuid(user_id)?)
            .bind(hash_text(&token))
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                INSERT INTO app_access_tokens (token_hash, user_id, expires_at)
                VALUES ($1, $2, now() + interval '12 hours')
                "#,
            )
            .bind(hash_text(&token))
            .bind(parse_uuid(user_id)?)
            .execute(&self.pool)
            .await?;
        }
        Ok(token)
    }

    pub async fn user_for_token(&self, token: &str) -> Result<Option<AppUser>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT u.id::text, u.email, u.display_name, u.default_plan_id, u.role, u.status
            FROM app_access_tokens t
            JOIN app_users u ON u.id = t.user_id
            WHERE t.token_hash = $1
              AND (t.expires_at IS NULL OR t.expires_at > now())
              AND t.revoked_at IS NULL
              AND u.status = 'active'
            "#,
        )
        .bind(hash_text(token))
        .fetch_optional(&self.pool)
        .await?;
        if row.is_some() {
            sqlx::query("UPDATE app_access_tokens SET last_used_at = now() WHERE token_hash = $1")
                .bind(hash_text(token))
                .execute(&self.pool)
                .await?;
        }
        Ok(row.as_ref().map(app_user_from_row))
    }

    pub async fn user_for_refresh_token(
        &self,
        token: &str,
    ) -> Result<Option<AppUser>, sqlx::Error> {
        let token_hash = hash_text(token);
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            r#"
            SELECT u.id::text, u.email, u.display_name, u.default_plan_id, u.role, u.status
            FROM auth_refresh_tokens t
            JOIN app_users u ON u.id = t.user_id
            WHERE t.token_hash = $1
              AND t.revoked_at IS NULL
              AND t.expires_at > now()
              AND u.status = 'active'
            "#,
        )
        .bind(&token_hash)
        .fetch_optional(&mut *tx)
        .await?;
        if row.is_some() {
            sqlx::query(
                "UPDATE auth_refresh_tokens SET last_used_at = now(), revoked_at = now() WHERE token_hash = $1",
            )
            .bind(&token_hash)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(row.as_ref().map(app_user_from_row))
    }

    pub async fn list_billing_plans(&self) -> Result<Vec<BillingPlanRecord>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, currency, price_cents, monthly_conversions, storage_bytes, features
            FROM billing_plans
            WHERE active = true
            ORDER BY price_cents ASC, id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(billing_plan_from_row).collect())
    }

    pub async fn list_redeem_packages(&self) -> Result<Vec<RedeemPackage>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, package_type, quantity
            FROM redeem_packages
            WHERE active = true
            ORDER BY quantity ASC, id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|row| RedeemPackage {
                id: row.get("id"),
                name: row.get("name"),
                package_type: row.get("package_type"),
                quantity: row.get::<i32, _>("quantity").max(0) as u64,
            })
            .collect())
    }

    pub async fn latest_release_manifest(
        &self,
        channel: &str,
    ) -> Result<Option<Value>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT r.id::text, r.channel, r.platform, r.arch, r.version, r.download_url,
                   r.sha256, r.signature, r.signature_algorithm, r.file_size_bytes,
                   r.release_title, r.release_notes, r.published_by, r.is_prerelease,
                   EXTRACT(EPOCH FROM r.published_at)::bigint AS published_at_secs,
                   s.strategy_type, s.min_required_version,
                   EXTRACT(EPOCH FROM s.force_deadline_at)::bigint AS force_deadline_secs,
                   s.block_if_outdated, s.rollout_percentage, s.prompt_title,
                   s.prompt_message, s.prompt_dismissable
            FROM release_manifests r
            LEFT JOIN release_strategies s ON s.release_id = r.id AND s.is_active = true
            WHERE r.channel = $1 AND r.active = true
            ORDER BY r.published_at DESC, r.version DESC
            LIMIT 1
            "#,
        )
        .bind(channel)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| {
            serde_json::json!({
                "version": row.get::<String, _>("version"),
                "channel": row.get::<String, _>("channel"),
                "platform": row.get::<String, _>("platform"),
                "arch": row.get::<String, _>("arch"),
                "download_url": row.get::<String, _>("download_url"),
                "sha256": row.get::<String, _>("sha256"),
                "signature": row.get::<String, _>("signature"),
                "signature_algorithm": row.get::<String, _>("signature_algorithm"),
                "file_size_bytes": row.get::<i64, _>("file_size_bytes").max(0),
                "release_title": row.get::<String, _>("release_title"),
                "release_notes": row.get::<String, _>("release_notes"),
                "published_by": row.get::<String, _>("published_by"),
                "is_prerelease": row.get::<bool, _>("is_prerelease"),
                "published_at": epoch_col(&row, "published_at_secs"),
                "strategy": row.try_get::<Option<String>, _>("strategy_type").ok().flatten().map(|strategy_type| serde_json::json!({
                    "type": strategy_type,
                    "min_required_version": row.try_get::<Option<String>, _>("min_required_version").ok().flatten(),
                    "force_deadline_at": epoch_col(&row, "force_deadline_secs"),
                    "block_if_outdated": row.try_get::<bool, _>("block_if_outdated").unwrap_or(false),
                    "rollout_percentage": row.try_get::<i32, _>("rollout_percentage").unwrap_or(100),
                    "prompt_title": row.try_get::<String, _>("prompt_title").unwrap_or_default(),
                    "prompt_message": row.try_get::<String, _>("prompt_message").unwrap_or_default(),
                    "prompt_dismissable": row.try_get::<bool, _>("prompt_dismissable").unwrap_or(true),
                })),
            })
        }))
    }

    pub async fn store_upload(
        &self,
        upload_id: &str,
        user_id: &str,
        file_name: String,
        object_key: String,
        bytes: Vec<u8>,
    ) -> Result<UploadRecord, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO uploads (id, user_id, file_name, object_key, bytes, sha256, status)
            VALUES ($1, $2, $3, $4, $5, $6, 'stored')
            RETURNING id::text, file_name, object_key, bytes,
                      EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
            "#,
        )
        .bind(parse_uuid(upload_id)?)
        .bind(parse_uuid(user_id)?)
        .bind(&file_name)
        .bind(&object_key)
        .bind(bytes.len() as i64)
        .bind(hash_bytes(&bytes))
        .fetch_one(&self.pool)
        .await?;
        Ok(upload_from_row(&row, bytes))
    }

    pub async fn get_upload(&self, upload_id: &str) -> Result<Option<UploadRecord>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id::text, file_name, object_key, bytes,
                   EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
            FROM uploads WHERE id = $1 AND status = 'stored'
            "#,
        )
        .bind(parse_uuid(upload_id)?)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| upload_from_row(&row, Vec::new())))
    }

    pub async fn create_job(
        &self,
        user_id: String,
        upload_id: String,
        main_tex: String,
        profile: String,
        quality: String,
        engine: String,
    ) -> Result<ConversionJobRecord, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO conversion_jobs (user_id, upload_id, main_tex, profile, quality, engine, status)
            VALUES ($1, $2, $3, $4, $5, $6, 'queued')
            RETURNING id::text, user_id::text, upload_id::text, main_tex, profile, quality, engine, status,
                      result_docx_key, result_report_key, report_json, source_zip_key, result_log_key, storage_path,
                      zip_bytes, docx_bytes, log_bytes, error_code, error_message,
                      EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs,
                      EXTRACT(EPOCH FROM updated_at)::bigint AS updated_at_secs
            "#,
        )
        .bind(parse_uuid(&user_id)?)
        .bind(parse_uuid(&upload_id)?)
        .bind(&main_tex)
        .bind(&profile)
        .bind(&quality)
        .bind(&engine)
        .fetch_one(&self.pool)
        .await?;
        Ok(job_from_row(&row, None, None))
    }

    pub async fn update_job_source_storage(
        &self,
        job_id: &str,
        source_zip_key: String,
        zip_bytes: u64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE conversion_jobs
            SET source_zip_key = $2, storage_path = $2, zip_bytes = $3, updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(parse_uuid(job_id)?)
        .bind(source_zip_key)
        .bind(zip_bytes as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_job(&self, job_id: &str) -> Result<Option<ConversionJobRecord>, sqlx::Error> {
        let row = sqlx::query(&job_select_sql("WHERE id = $1"))
            .bind(parse_uuid(job_id)?)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| job_from_row(&row, None, report_from_row(&row))))
    }

    pub async fn list_jobs_by_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<ConversionJobRecord>, sqlx::Error> {
        let rows = sqlx::query(&job_select_sql(
            "WHERE user_id = $1 ORDER BY created_at DESC",
        ))
        .bind(parse_uuid(user_id)?)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|row| job_from_row(row, None, report_from_row(row)))
            .collect())
    }

    pub async fn update_status(
        &self,
        job_id: &str,
        status: ConversionStatus,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE conversion_jobs SET status = $2, updated_at = now() WHERE id = $1")
            .bind(parse_uuid(job_id)?)
            .bind(status.as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn complete_job(
        &self,
        job_id: &str,
        docx_key: String,
        docx_bytes: u64,
        log_key: String,
        log_bytes: u64,
        report: &ConversionReportRecord,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE conversion_jobs
            SET status = 'completed',
                result_docx_key = $2,
                report_json = $3,
                result_log_key = $4,
                docx_bytes = $5,
                log_bytes = $6,
                error_code = NULL,
                error_message = NULL,
                updated_at = now(),
                completed_at = now()
            WHERE id = $1
            "#,
        )
        .bind(parse_uuid(job_id)?)
        .bind(docx_key)
        .bind(serde_json::to_value(report).unwrap_or_else(|_| Value::Object(Default::default())))
        .bind(log_key)
        .bind(docx_bytes as i64)
        .bind(log_bytes as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn fail_job(
        &self,
        job_id: &str,
        error_code: &str,
        error: &str,
        log_key: String,
        log_bytes: u64,
        report: &ConversionReportRecord,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE conversion_jobs
            SET status = 'failed',
                error_code = $2,
                error_message = $3,
                result_log_key = $4,
                log_bytes = $5,
                report_json = $6,
                failed_at = now(),
                updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(parse_uuid(job_id)?)
        .bind(error_code)
        .bind(error)
        .bind(log_key)
        .bind(log_bytes as i64)
        .bind(serde_json::to_value(report).unwrap_or_else(|_| Value::Object(Default::default())))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn claim_next_job(&self, worker_id: &str) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            WITH picked AS (
                SELECT id
                FROM conversion_jobs
                WHERE status = 'queued'
                  AND next_run_at <= now()
                ORDER BY created_at ASC
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            UPDATE conversion_jobs j
            SET status = 'normalizing',
                worker_id = $1,
                locked_at = now(),
                started_at = COALESCE(started_at, now()),
                attempts = attempts + 1,
                updated_at = now()
            FROM picked
            WHERE j.id = picked.id
            RETURNING j.id::text
            "#,
        )
        .bind(worker_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| row.get("id")))
    }

    pub async fn recover_stale_jobs(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE conversion_jobs
            SET status = CASE WHEN attempts >= 3 THEN 'failed' ELSE 'queued' END,
                error_code = CASE WHEN attempts >= 3 THEN 'worker_timeout' ELSE error_code END,
                error_message = CASE WHEN attempts >= 3 THEN 'worker timed out before completing this conversion' ELSE error_message END,
                failed_at = CASE WHEN attempts >= 3 THEN now() ELSE failed_at END,
                worker_id = NULL,
                locked_at = NULL,
                next_run_at = CASE
                    WHEN attempts >= 3 THEN next_run_at
                    ELSE now() + make_interval(secs => LEAST(60, GREATEST(1, attempts * 5)))
                END,
                updated_at = now()
            WHERE status IN ('normalizing', 'detecting', 'analyzing', 'compiling', 'rendering', 'verifying')
              AND locked_at IS NOT NULL
              AND locked_at < now() - interval '15 minutes'
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn cloud_conversions_used(&self, user_id: &str) -> Result<u64, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*)::bigint AS ledger_count,
                   COALESCE(SUM(
                       CASE
                           WHEN event_type = 'reserve' THEN quantity
                           WHEN event_type = 'refund' THEN -quantity
                           ELSE 0
                       END
                   ), 0)::bigint AS ledger_used
            FROM usage_ledger
            WHERE user_id = $1 AND source = 'preview'
            "#,
        )
        .bind(parse_uuid(user_id)?)
        .fetch_one(&self.pool)
        .await?;
        if row.get::<i64, _>("ledger_count") > 0 {
            return Ok(row.get::<i64, _>("ledger_used").max(0) as u64);
        }

        let row = sqlx::query(
            r#"
            SELECT COALESCE(SUM(quantity), 0)::bigint AS used
            FROM usage_events
            WHERE user_id = $1 AND event_type = 'cloud_conversion'
            "#,
        )
        .bind(parse_uuid(user_id)?)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get::<i64, _>("used").max(0) as u64)
    }

    pub async fn entitlement(&self, user_id: &str) -> Result<EntitlementRecord, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO commercial_entitlements (user_id)
            VALUES ($1)
            ON CONFLICT (user_id) DO UPDATE SET user_id = EXCLUDED.user_id
            RETURNING user_id::text, count_balance,
                      EXTRACT(EPOCH FROM valid_until)::bigint AS valid_until_secs,
                      source_order_id,
                      EXTRACT(EPOCH FROM updated_at)::bigint AS updated_at_secs
            "#,
        )
        .bind(parse_uuid(user_id)?)
        .fetch_one(&self.pool)
        .await?;
        Ok(entitlement_from_row(&row))
    }

    pub async fn reserve_cloud_conversion(&self, user_id: &str, job_id: &str) -> Result<u64, u64> {
        let user_uuid = parse_uuid(user_id).map_err(|_| 0_u64)?;
        let job_uuid = parse_uuid(job_id).map_err(|_| 0_u64)?;
        let mut tx = self.pool.begin().await.map_err(|_| 0_u64)?;
        let entitlement = sqlx::query(
            r#"
            INSERT INTO commercial_entitlements (user_id)
            VALUES ($1)
            ON CONFLICT (user_id) DO UPDATE SET user_id = EXCLUDED.user_id
            RETURNING count_balance, valid_until
            "#,
        )
        .bind(user_uuid)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| 0_u64)?;
        let count_balance = entitlement.get::<i64, _>("count_balance");
        let valid_until_active = entitlement
            .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("valid_until")
            .ok()
            .flatten()
            .is_some_and(|value| value >= chrono::Utc::now());
        let used = preview_conversions_used_tx(&mut tx, user_uuid)
            .await
            .map_err(|_| 0_u64)?;

        if valid_until_active {
            insert_usage_ledger(
                &mut tx,
                user_uuid,
                Some(job_uuid),
                "reserve",
                0,
                None,
                "date_entitlement",
                Some("date entitlement active"),
            )
            .await
            .map_err(|_| used)?;
            tx.commit().await.map_err(|_| used)?;
            return Ok(used);
        }

        if count_balance > 0 {
            let new_balance = count_balance - 1;
            sqlx::query(
                "UPDATE commercial_entitlements SET count_balance = $2, updated_at = now() WHERE user_id = $1",
            )
            .bind(user_uuid)
            .bind(new_balance)
            .execute(&mut *tx)
            .await
            .map_err(|_| used)?;
            insert_usage_ledger(
                &mut tx,
                user_uuid,
                Some(job_uuid),
                "reserve",
                1,
                Some(new_balance),
                "entitlement",
                Some("count entitlement reserved for conversion"),
            )
            .await
            .map_err(|_| used)?;
            tx.commit().await.map_err(|_| used)?;
            return Ok(used);
        }

        if used >= PREVIEW_CLOUD_CONVERSION_LIMIT {
            tx.rollback().await.ok();
            return Err(used);
        }

        let period_id = ensure_usage_period(&mut tx, user_uuid)
            .await
            .map_err(|_| used)?;
        sqlx::query(
            "INSERT INTO usage_events (user_id, usage_period_id, event_type, quantity, source_id) VALUES ($1, $2, 'cloud_conversion', 1, $3)",
        )
        .bind(user_uuid)
        .bind(period_id)
        .bind(job_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| used)?;
        insert_usage_ledger(
            &mut tx,
            user_uuid,
            Some(job_uuid),
            "reserve",
            1,
            None,
            "preview",
            Some("preview quota reserved for conversion"),
        )
        .await
        .map_err(|_| used)?;
        tx.commit().await.map_err(|_| used)?;
        Ok(used + 1)
    }

    pub async fn refund_cloud_conversion_for_job(
        &self,
        job_id: &str,
        reason: &str,
    ) -> Result<(), sqlx::Error> {
        let job_uuid = parse_uuid(job_id)?;
        let mut tx = self.pool.begin().await?;
        let existing_refund = sqlx::query(
            "SELECT id FROM usage_ledger WHERE conversion_job_id = $1 AND event_type = 'refund' LIMIT 1",
        )
        .bind(job_uuid)
        .fetch_optional(&mut *tx)
        .await?;
        if existing_refund.is_some() {
            tx.commit().await?;
            return Ok(());
        }

        let reserve = sqlx::query(
            r#"
            SELECT user_id, quantity, source
            FROM usage_ledger
            WHERE conversion_job_id = $1 AND event_type = 'reserve'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(job_uuid)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(reserve) = reserve else {
            tx.commit().await?;
            return Ok(());
        };
        let user_uuid: Uuid = reserve.get("user_id");
        let quantity = reserve.get::<i64, _>("quantity").max(0);
        let source: String = reserve.get("source");
        if quantity == 0 {
            tx.commit().await?;
            return Ok(());
        }

        let balance_after = if source == "entitlement" {
            let row = sqlx::query(
                r#"
                UPDATE commercial_entitlements
                SET count_balance = count_balance + $2, updated_at = now()
                WHERE user_id = $1
                RETURNING count_balance
                "#,
            )
            .bind(user_uuid)
            .bind(quantity)
            .fetch_one(&mut *tx)
            .await?;
            Some(row.get::<i64, _>("count_balance"))
        } else {
            None
        };

        insert_usage_ledger(
            &mut tx,
            user_uuid,
            Some(job_uuid),
            "refund",
            quantity,
            balance_after,
            &source,
            Some(reason),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_recharge(
        &self,
        user_id: String,
        recharge_type: String,
        package_id: String,
        quantity: u64,
        amount_cents: u64,
        status: &str,
        provider: &str,
        provider_trade_id: String,
    ) -> Result<RechargeRecord, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let recharge = insert_recharge(
            &mut tx,
            parse_uuid(&user_id)?,
            &recharge_type,
            &package_id,
            quantity,
            amount_cents,
            status,
            provider,
            &provider_trade_id,
        )
        .await?;
        let user_uuid = parse_uuid(&user_id)?;
        apply_entitlement_sql(&mut tx, user_uuid, &recharge).await?;
        insert_grant_ledger(&mut tx, user_uuid, &recharge, provider).await?;
        tx.commit().await?;
        Ok(recharge)
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
    ) -> Result<ManualOrderRecord, sqlx::Error> {
        let user_uuid = parse_uuid(&user_id)?;
        let operator_uuid = parse_uuid(&operator_id)?;
        let mut tx = self.pool.begin().await?;
        let provider_trade_id = format!("manual_order_{}", Uuid::new_v4().simple());
        let recharge = insert_recharge(
            &mut tx,
            user_uuid,
            &recharge_type,
            &package_id,
            quantity,
            amount_cents,
            "paid",
            "manual-order",
            &provider_trade_id,
        )
        .await?;
        apply_entitlement_sql(&mut tx, user_uuid, &recharge).await?;
        insert_grant_ledger(&mut tx, user_uuid, &recharge, "manual-order").await?;
        let row = sqlx::query(
            r#"
            INSERT INTO manual_orders
                (user_id, recharge_id, recharge_type, package_id, quantity, amount_cents,
                 status, operator_id, payment_note)
            VALUES ($1, $2, $3, $4, $5, $6, 'paid', $7, $8)
            RETURNING id::text, user_id::text, recharge_id::text, recharge_type, package_id,
                      quantity, amount_cents, currency, status, operator_id::text, payment_note,
                      EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
            "#,
        )
        .bind(user_uuid)
        .bind(parse_uuid(&recharge.recharge_id)?)
        .bind(&recharge_type)
        .bind(&package_id)
        .bind(quantity as i32)
        .bind(amount_cents as i32)
        .bind(operator_uuid)
        .bind(&payment_note)
        .fetch_one(&mut *tx)
        .await?;
        self.insert_audit_log_tx(
            &mut tx,
            &operator_id,
            "manual_order.create",
            None,
            serde_json::json!({
                "manual_order_id": row.get::<String, _>("id"),
                "user_id": user_id,
                "recharge_id": recharge.recharge_id,
                "package_id": package_id,
                "quantity": quantity,
                "amount_cents": amount_cents,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(manual_order_from_row(&row))
    }

    pub async fn list_manual_orders(&self) -> Result<Vec<ManualOrderRecord>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text, user_id::text, recharge_id::text, recharge_type, package_id,
                   quantity, amount_cents, currency, status, operator_id::text, payment_note,
                   EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
            FROM manual_orders
            ORDER BY created_at DESC
            LIMIT 200
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(manual_order_from_row).collect())
    }

    pub async fn list_recharges(&self, user_id: &str) -> Result<Vec<RechargeRecord>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text, user_id::text, recharge_type, package_id, quantity, amount_cents,
                   currency, status, provider, provider_trade_id,
                   EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
            FROM recharges
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(parse_uuid(user_id)?)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(recharge_from_row).collect())
    }

    pub async fn list_usage_ledger(&self) -> Result<Vec<Value>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT l.id::text, l.user_id::text, u.email, l.conversion_job_id::text,
                   l.event_type, l.quantity, l.balance_after, l.source, l.reason,
                   EXTRACT(EPOCH FROM l.created_at)::bigint AS created_at_secs
            FROM usage_ledger l
            JOIN app_users u ON u.id = l.user_id
            ORDER BY l.created_at DESC
            LIMIT 200
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "id": row.get::<String, _>("id"),
                    "user_id": row.get::<String, _>("user_id"),
                    "email": row.get::<String, _>("email"),
                    "conversion_job_id": row.get::<Option<String>, _>("conversion_job_id"),
                    "event_type": row.get::<String, _>("event_type"),
                    "quantity": row.get::<i64, _>("quantity"),
                    "balance_after": row.get::<Option<i64>, _>("balance_after"),
                    "source": row.get::<String, _>("source"),
                    "reason": row.get::<Option<String>, _>("reason"),
                    "created_at": epoch_col(row, "created_at_secs"),
                })
            })
            .collect())
    }

    pub async fn list_users(&self) -> Result<Vec<Value>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text, email, display_name, default_plan_id, role, status,
                   EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs,
                   EXTRACT(EPOCH FROM last_login_at)::bigint AS last_login_secs
            FROM app_users
            ORDER BY created_at DESC
            LIMIT 200
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "id": row.get::<String, _>("id"),
                    "email": row.get::<String, _>("email"),
                    "display_name": row.get::<Option<String>, _>("display_name"),
                    "plan_id": row.get::<String, _>("default_plan_id"),
                    "role": row.get::<String, _>("role"),
                    "status": row.get::<String, _>("status"),
                    "created_at": epoch_col(row, "created_at_secs"),
                    "last_login_at": epoch_col(row, "last_login_secs"),
                })
            })
            .collect())
    }

    pub async fn create_waitlist_lead(
        &self,
        email: String,
        identity: Option<String>,
        paper_type: Option<String>,
        current_tool: Option<String>,
        pain_point: Option<String>,
        paid_intent: Option<String>,
    ) -> Result<Value, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO waitlist_leads
                (email, identity, paper_type, current_tool, pain_point, paid_intent)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id::text, email, identity, paper_type, current_tool, pain_point,
                      paid_intent, status, EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
            "#,
        )
        .bind(email)
        .bind(identity)
        .bind(paper_type)
        .bind(current_tool)
        .bind(pain_point)
        .bind(paid_intent)
        .fetch_one(&self.pool)
        .await?;
        Ok(waitlist_from_row(&row))
    }

    pub async fn list_waitlist_leads(&self) -> Result<Vec<Value>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text, email, identity, paper_type, current_tool, pain_point,
                   paid_intent, status, follow_up_note,
                   EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
            FROM waitlist_leads
            ORDER BY created_at DESC
            LIMIT 200
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(waitlist_from_row).collect())
    }

    pub async fn list_release_manifests(&self) -> Result<Vec<Value>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text, channel, platform, arch, version, download_url, sha256,
                   signature, signature_algorithm, file_size_bytes, release_title,
                   release_notes, published_by, is_prerelease, active,
                   EXTRACT(EPOCH FROM published_at)::bigint AS published_at_secs
            FROM release_manifests
            ORDER BY published_at DESC, version DESC
            LIMIT 200
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(release_from_row).collect())
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
    ) -> Result<Value, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE release_manifests SET active = false WHERE channel = $1 AND platform = $2 AND arch = $3",
        )
        .bind(&channel)
        .bind(&platform)
        .bind(&arch)
        .execute(&mut *tx)
        .await?;
        let row = sqlx::query(
            r#"
            INSERT INTO release_manifests
                (channel, platform, arch, version, download_url, sha256, signature,
                 file_size_bytes, release_title, release_notes, published_by, active)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, true)
            ON CONFLICT (channel, platform, arch, version) DO UPDATE SET
                download_url = EXCLUDED.download_url,
                sha256 = EXCLUDED.sha256,
                signature = EXCLUDED.signature,
                file_size_bytes = EXCLUDED.file_size_bytes,
                release_title = EXCLUDED.release_title,
                release_notes = EXCLUDED.release_notes,
                published_by = EXCLUDED.published_by,
                active = true,
                deprecated_at = NULL,
                deprecation_reason = NULL,
                published_at = now()
            RETURNING id::text, channel, platform, arch, version, download_url, sha256,
                      signature, signature_algorithm, file_size_bytes, release_title,
                      release_notes, published_by, is_prerelease, active,
                      EXTRACT(EPOCH FROM published_at)::bigint AS published_at_secs
            "#,
        )
        .bind(&channel)
        .bind(&platform)
        .bind(&arch)
        .bind(&version)
        .bind(&download_url)
        .bind(&sha256)
        .bind(signature.unwrap_or_default())
        .bind(file_size_bytes as i64)
        .bind(release_title.unwrap_or_else(|| format!("Tex2Doc {version}")))
        .bind(release_notes.unwrap_or_default())
        .bind(actor)
        .fetch_one(&mut *tx)
        .await?;
        let release_id = parse_uuid(&row.get::<String, _>("id"))?;
        let strategy = strategy_type.unwrap_or_else(|| "recommended".to_string());
        let strategy_row = sqlx::query(
            r#"
            INSERT INTO release_strategies
                (release_id, strategy_type, prompt_title, prompt_message, created_by)
            VALUES ($1, $2, '发现新版本', '建议升级到最新版本以获得更稳定的转换体验。', $3)
            ON CONFLICT (release_id, strategy_type) DO UPDATE SET
                is_active = true,
                updated_at = now()
            RETURNING id
            "#,
        )
        .bind(release_id)
        .bind(&strategy)
        .bind(actor)
        .fetch_one(&mut *tx)
        .await?;
        self.insert_audit_log_tx(
            &mut tx,
            actor,
            "release.publish",
            Some(release_id),
            serde_json::json!({
                "channel": channel,
                "platform": platform,
                "arch": arch,
                "version": version,
                "strategy": strategy,
            }),
        )
        .await?;
        sqlx::query(
            r#"
            INSERT INTO release_rollout_events
                (release_id, strategy_id, event_type, new_percentage, event_reason, triggered_by)
            VALUES ($1, $2, 'rollout_started', 100, 'release published', $3)
            "#,
        )
        .bind(release_id)
        .bind(strategy_row.get::<Uuid, _>("id"))
        .bind(actor)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(release_from_row(&row))
    }

    pub async fn rollback_release(
        &self,
        actor: &str,
        release_id: &str,
        reason: Option<String>,
    ) -> Result<Value, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let current = sqlx::query(
            "SELECT id, channel, platform, arch, version FROM release_manifests WHERE id = $1",
        )
        .bind(parse_uuid(release_id)?)
        .fetch_one(&mut *tx)
        .await?;
        let target = sqlx::query(
            r#"
            SELECT id::text, channel, platform, arch, version, download_url, sha256,
                   signature, signature_algorithm, file_size_bytes, release_title,
                   release_notes, published_by, is_prerelease, active,
                   EXTRACT(EPOCH FROM published_at)::bigint AS published_at_secs
            FROM release_manifests
            WHERE channel = $1 AND platform = $2 AND arch = $3 AND id <> $4
            ORDER BY published_at DESC, version DESC
            LIMIT 1
            "#,
        )
        .bind(current.get::<String, _>("channel"))
        .bind(current.get::<String, _>("platform"))
        .bind(current.get::<String, _>("arch"))
        .bind(current.get::<Uuid, _>("id"))
        .fetch_one(&mut *tx)
        .await?;
        let target_id = parse_uuid(&target.get::<String, _>("id"))?;
        sqlx::query("UPDATE release_manifests SET active = false, deprecated_at = now(), deprecated_by = $2, deprecation_reason = $3 WHERE id = $1")
            .bind(current.get::<Uuid, _>("id"))
            .bind(actor)
            .bind(reason.as_deref())
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE release_manifests SET active = true, deprecated_at = NULL, deprecation_reason = NULL WHERE id = $1")
            .bind(target_id)
            .execute(&mut *tx)
            .await?;
        self.insert_audit_log_tx(
            &mut tx,
            actor,
            "release.rollback",
            Some(current.get::<Uuid, _>("id")),
            serde_json::json!({
                "from": current.get::<String, _>("version"),
                "to": target.get::<String, _>("version"),
                "reason": reason,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(release_from_row(&target))
    }

    pub async fn list_release_audit(&self) -> Result<Vec<Value>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text, actor, action, target_release_id::text, details,
                   EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
            FROM release_audit_log
            ORDER BY created_at DESC
            LIMIT 200
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "id": row.get::<String, _>("id"),
                    "actor": row.get::<String, _>("actor"),
                    "action": row.get::<String, _>("action"),
                    "target_release_id": row.get::<Option<String>, _>("target_release_id"),
                    "details": row.get::<Value, _>("details"),
                    "created_at": epoch_col(row, "created_at_secs"),
                })
            })
            .collect())
    }

    async fn insert_audit_log_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        actor: &str,
        action: &str,
        target_release_id: Option<Uuid>,
        details: Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO release_audit_log (actor, action, target_release_id, details)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(actor)
        .bind(action)
        .bind(target_release_id)
        .bind(details)
        .execute(&mut **tx)
        .await?;
        Ok(())
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
        if requested_count == 0 || requested_count > 10_000 {
            return Err(RedeemFailure::InvalidCode);
        }
        let package = redeem_package(package_id).ok_or(RedeemFailure::InvalidCode)?;
        let expires_sql =
            parse_optional_epoch(&expires_at).map_err(|_| RedeemFailure::InvalidCode)?;
        let created_by_uuid = parse_uuid(&created_by).map_err(|_| RedeemFailure::InvalidCode)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| RedeemFailure::InvalidCode)?;
        let batch_no = format!(
            "RC{}",
            Uuid::new_v4().simple().to_string()[..10].to_uppercase()
        );
        let batch_row = sqlx::query(
            r#"
            INSERT INTO redeem_code_batches
                (batch_no, package_id, quantity, generated_count, exported_count, status, channel, note, expires_at, created_by)
            VALUES ($1, $2, $3, $4, 0, 'active', $5, $6, to_timestamp($7), $8)
            RETURNING id::text, batch_no, EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
            "#,
        )
        .bind(&batch_no)
        .bind(&package.id)
        .bind(package.quantity as i32)
        .bind(requested_count as i32)
        .bind(&channel)
        .bind(&note)
        .bind(expires_sql)
        .bind(created_by_uuid)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| RedeemFailure::InvalidCode)?;
        let batch_id: String = batch_row.get("id");
        let batch_prefix = batch_prefix(&batch_no);
        let created_at = epoch_col(&batch_row, "created_at_secs").unwrap_or_else(now_timestamp);
        let mut codes = Vec::with_capacity(requested_count as usize);

        for _ in 0..requested_count {
            let code = generate_redeem_code(&batch_prefix);
            let normalized = normalize_redeem_code(&code).ok_or(RedeemFailure::InvalidCode)?;
            let nonce = random_bytes(12);
            sqlx::query(
                r#"
                INSERT INTO redeem_codes
                    (batch_id, package_id, code_hash, code_ciphertext, code_nonce, code_preview, key_version, status, expires_at)
                VALUES ($1, $2, $3, $4, $5, $6, 'v1', 'unused', to_timestamp($7))
                "#,
            )
            .bind(parse_uuid(&batch_id).map_err(|_| RedeemFailure::InvalidCode)?)
            .bind(&package.id)
            .bind(code_hash(&normalized))
            .bind(encrypt_code(&normalized, &nonce))
            .bind(nonce)
            .bind(crate::state::code_preview(&code))
            .bind(expires_sql)
            .execute(&mut *tx)
            .await
            .map_err(|_| RedeemFailure::InvalidCode)?;
            codes.push(code);
        }

        insert_redeem_event(
            &mut tx,
            None,
            Some(created_by_uuid),
            "generated",
            Some(&format!("batch {batch_no}")),
        )
        .await
        .map_err(|_| RedeemFailure::InvalidCode)?;
        tx.commit().await.map_err(|_| RedeemFailure::InvalidCode)?;

        Ok(RedeemCodeBatchRecord {
            batch_id,
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
            created_at,
            codes,
        })
    }

    pub async fn list_redeem_batches(&self) -> Result<Vec<RedeemCodeBatchRecord>, sqlx::Error> {
        let rows = sqlx::query(&redeem_batch_select_sql(""))
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .iter()
            .map(|row| redeem_batch_from_row(row, Vec::new()))
            .collect())
    }

    pub async fn get_redeem_batch(
        &self,
        batch_id: &str,
        include_codes: bool,
    ) -> Result<Option<RedeemCodeBatchRecord>, sqlx::Error> {
        let row = sqlx::query(&redeem_batch_select_sql("WHERE b.id = $1"))
            .bind(parse_uuid(batch_id)?)
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let codes = if include_codes {
            self.codes_for_batch(batch_id).await?
        } else {
            Vec::new()
        };
        Ok(Some(redeem_batch_from_row(&row, codes)))
    }

    pub async fn codes_for_batch(&self, batch_id: &str) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT code_ciphertext, code_nonce FROM redeem_codes WHERE batch_id = $1 ORDER BY created_at ASC",
        )
        .bind(parse_uuid(batch_id)?)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .filter_map(|row| {
                let ciphertext: Vec<u8> = row.get("code_ciphertext");
                let nonce: Vec<u8> = row.get("code_nonce");
                decrypt_code(&ciphertext, &nonce).ok()
            })
            .map(|code| group_redeem_code(&code))
            .collect())
    }

    pub async fn mark_redeem_batch_exported(&self, batch_id: &str) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE redeem_code_batches SET exported_count = generated_count, updated_at = now() WHERE id = $1",
        )
        .bind(parse_uuid(batch_id)?)
        .execute(&mut *tx)
        .await?;
        insert_redeem_event(&mut tx, None, None, "exported", Some(batch_id)).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn redeem_code(
        &self,
        user_id: String,
        input_code: String,
    ) -> Result<RedeemCodeResult, RedeemFailure> {
        let normalized = normalize_redeem_code(&input_code).ok_or(RedeemFailure::InvalidCode)?;
        let user_uuid = parse_uuid(&user_id).map_err(|_| RedeemFailure::InvalidCode)?;
        if !redeem_checksum_valid(&normalized) {
            let _ = self.record_redeem_failure(user_uuid, "invalid_code").await;
            return Err(RedeemFailure::InvalidCode);
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| RedeemFailure::InvalidCode)?;
        let row = sqlx::query(
            r#"
            SELECT c.id::text, c.batch_id::text, b.batch_no, c.package_id, p.name AS package_name,
                   p.package_type, p.quantity, c.code_hash, c.code_ciphertext, c.code_nonce,
                   c.code_preview, c.key_version, c.status,
                   c.stock_status,
                   c.stocked_by::text,
                   EXTRACT(EPOCH FROM c.stocked_at)::bigint AS stocked_at_secs,
                   c.redeemed_by::text,
                   c.redeemed_recharge_id::text,
                   EXTRACT(EPOCH FROM c.redeemed_at)::bigint AS redeemed_at_secs,
                   c.restocked_by::text,
                   EXTRACT(EPOCH FROM c.restocked_at)::bigint AS restocked_at_secs,
                   EXTRACT(EPOCH FROM c.expires_at)::bigint AS expires_at_secs,
                   EXTRACT(EPOCH FROM c.created_at)::bigint AS created_at_secs
            FROM redeem_codes c
            JOIN redeem_code_batches b ON b.id = c.batch_id
            JOIN redeem_packages p ON p.id = c.package_id
            WHERE c.code_hash = $1
            FOR UPDATE OF c
            "#,
        )
        .bind(code_hash(&normalized))
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| RedeemFailure::InvalidCode)?;
        let Some(row) = row else {
            insert_redeem_event(
                &mut tx,
                None,
                Some(user_uuid),
                "redeem_failed",
                Some("invalid_code"),
            )
            .await
            .ok();
            tx.commit().await.ok();
            return Err(RedeemFailure::InvalidCode);
        };
        let mut record = redeem_code_from_row(&row);
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
            sqlx::query(
                "UPDATE redeem_codes SET status = 'expired', updated_at = now() WHERE id = $1",
            )
            .bind(parse_uuid(&record.code_id).map_err(|_| RedeemFailure::InvalidCode)?)
            .execute(&mut *tx)
            .await
            .map_err(|_| RedeemFailure::InvalidCode)?;
            insert_redeem_event(
                &mut tx,
                Some(parse_uuid(&record.code_id).map_err(|_| RedeemFailure::InvalidCode)?),
                Some(user_uuid),
                "expired",
                Some("expired"),
            )
            .await
            .ok();
            tx.commit().await.ok();
            return Err(RedeemFailure::Expired);
        }

        let recharge = insert_recharge(
            &mut tx,
            user_uuid,
            &record.recharge_type,
            &record.package_id,
            record.quantity,
            0,
            "paid",
            "redeem-code",
            &record.code_id,
        )
        .await
        .map_err(|_| RedeemFailure::InvalidCode)?;
        apply_entitlement_sql(&mut tx, user_uuid, &recharge)
            .await
            .map_err(|_| RedeemFailure::InvalidCode)?;
        insert_grant_ledger(&mut tx, user_uuid, &recharge, "redeem-code")
            .await
            .map_err(|_| RedeemFailure::InvalidCode)?;
        sqlx::query(
            "UPDATE redeem_codes SET status = 'redeemed', stock_status = 'redeemed', redeemed_by = $2, redeemed_recharge_id = $3, redeemed_at = now(), updated_at = now() WHERE id = $1",
        )
        .bind(parse_uuid(&record.code_id).map_err(|_| RedeemFailure::InvalidCode)?)
        .bind(user_uuid)
        .bind(parse_uuid(&recharge.recharge_id).map_err(|_| RedeemFailure::InvalidCode)?)
        .execute(&mut *tx)
        .await
        .map_err(|_| RedeemFailure::InvalidCode)?;
        insert_redeem_event(
            &mut tx,
            Some(parse_uuid(&record.code_id).map_err(|_| RedeemFailure::InvalidCode)?),
            Some(user_uuid),
            "redeem_success",
            None,
        )
        .await
        .map_err(|_| RedeemFailure::InvalidCode)?;
        tx.commit().await.map_err(|_| RedeemFailure::InvalidCode)?;

        record.status = "redeemed".to_string();
        let entitlement = self
            .entitlement(&user_id)
            .await
            .map_err(|_| RedeemFailure::InvalidCode)?;
        Ok(RedeemCodeResult {
            redeem_id: record.code_id,
            recharge_id: recharge.recharge_id,
            package_id: record.package_id,
            package_name: record.package_name,
            recharge_type: record.recharge_type,
            quantity: record.quantity,
            count_balance: entitlement.count_balance,
            date_valid_until: entitlement.valid_until,
            redeemed_at: now_timestamp(),
        })
    }

    pub async fn list_redeem_records(
        &self,
        user_id: &str,
    ) -> Result<Vec<RedeemCodeRecord>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT c.id::text, c.batch_id::text, b.batch_no, c.package_id, p.name AS package_name,
                   p.package_type, p.quantity, c.code_hash, c.code_ciphertext, c.code_nonce,
                   c.code_preview, c.key_version, c.status,
                   c.stock_status,
                   c.stocked_by::text,
                   EXTRACT(EPOCH FROM c.stocked_at)::bigint AS stocked_at_secs,
                   c.redeemed_by::text,
                   c.redeemed_recharge_id::text,
                   EXTRACT(EPOCH FROM c.redeemed_at)::bigint AS redeemed_at_secs,
                   c.restocked_by::text,
                   EXTRACT(EPOCH FROM c.restocked_at)::bigint AS restocked_at_secs,
                   EXTRACT(EPOCH FROM c.expires_at)::bigint AS expires_at_secs,
                   EXTRACT(EPOCH FROM c.created_at)::bigint AS created_at_secs
            FROM redeem_codes c
            JOIN redeem_code_batches b ON b.id = c.batch_id
            JOIN redeem_packages p ON p.id = c.package_id
            WHERE c.redeemed_by = $1
            ORDER BY c.redeemed_at DESC
            "#,
        )
        .bind(parse_uuid(user_id)?)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(redeem_code_from_row).collect())
    }

    // ─── Admin: redeem code lifecycle (上货/使用/重置) ─────────────────────

    pub async fn admin_list_redeem_codes(
        &self,
        stock_status: Option<&str>,
        batch_id: Option<&str>,
        package_id: Option<&str>,
        search: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<RedeemCodeRecord>, sqlx::Error> {
        // Always bind 6 positional params; use COALESCE so absent filters match all rows.
        let s_status = stock_status.unwrap_or("");
        let b_id = batch_id.unwrap_or("");
        let p_id = package_id.unwrap_or("");
        let search_term = search.unwrap_or("");

        let rows = sqlx::query(
            r#"
            SELECT c.id::text, c.batch_id::text, b.batch_no, c.package_id, p.name AS package_name,
                   p.package_type, p.quantity, c.code_hash, c.code_ciphertext, c.code_nonce,
                   c.code_preview, c.key_version, c.status,
                   COALESCE(c.stock_status, 'new') AS stock_status,
                   c.stocked_by::text,
                   EXTRACT(EPOCH FROM c.stocked_at)::bigint AS stocked_at_secs,
                   c.redeemed_by::text,
                   c.redeemed_recharge_id::text,
                   EXTRACT(EPOCH FROM c.redeemed_at)::bigint AS redeemed_at_secs,
                   c.restocked_by::text,
                   EXTRACT(EPOCH FROM c.restocked_at)::bigint AS restocked_at_secs,
                   EXTRACT(EPOCH FROM c.expires_at)::bigint AS expires_at_secs,
                   EXTRACT(EPOCH FROM c.created_at)::bigint AS created_at_secs
            FROM redeem_codes c
            JOIN redeem_code_batches b ON b.id = c.batch_id
            JOIN redeem_packages p ON p.id = c.package_id
            WHERE ($1 = '' OR c.stock_status = $1)
              AND ($2 = '' OR c.batch_id::text = $2)
              AND ($3 = '' OR c.package_id = $3)
              AND ($4 = '' OR c.code_preview ILIKE '%' || $4 || '%' OR b.batch_no ILIKE '%' || $4 || '%')
            ORDER BY c.created_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(s_status)
        .bind(b_id)
        .bind(p_id)
        .bind(search_term)
        .bind(limit.unwrap_or(100).clamp(1, 1000) as i64)
        .bind(offset.unwrap_or(0) as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(redeem_code_from_row).collect())
    }

    pub async fn admin_count_redeem_codes(
        &self,
        stock_status: Option<&str>,
        batch_id: Option<&str>,
        package_id: Option<&str>,
        search: Option<&str>,
    ) -> Result<u64, sqlx::Error> {
        let s_status = stock_status.unwrap_or("");
        let b_id = batch_id.unwrap_or("");
        let p_id = package_id.unwrap_or("");
        let search_term = search.unwrap_or("");

        let row = sqlx::query(
            r#"
            SELECT COUNT(*)::bigint AS total
            FROM redeem_codes c
            JOIN redeem_code_batches b ON b.id = c.batch_id
            WHERE ($1 = '' OR c.stock_status = $1)
              AND ($2 = '' OR c.batch_id::text = $2)
              AND ($3 = '' OR c.package_id = $3)
              AND ($4 = '' OR c.code_preview ILIKE '%' || $4 || '%' OR b.batch_no ILIKE '%' || $4 || '%')
            "#,
        )
        .bind(&s_status)
        .bind(&b_id)
        .bind(&p_id)
        .bind(&search_term)
        .fetch_one(&self.pool)
        .await?;
        let total: i64 = row.try_get("total").unwrap_or(0);
        Ok(total.max(0) as u64)
    }

    pub async fn admin_stock_redeem_codes(
        &self,
        admin_id: &str,
        code_ids: &[String],
    ) -> Result<u64, RedeemFailure> {
        if code_ids.is_empty() {
            return Ok(0);
        }
        let mut tx = self.pool.begin().await.map_err(|_| RedeemFailure::InvalidCode)?;
        let mut affected: u64 = 0;
        for code_id in code_ids {
            let uuid = parse_uuid(code_id).map_err(|_| RedeemFailure::InvalidCode)?;
            let result = sqlx::query(
                "UPDATE redeem_codes \
                 SET stock_status = 'stocked', stocked_at = now(), stocked_by = $2, updated_at = now() \
                 WHERE id = $1 AND stock_status IN ('new', 'restocked')",
            )
            .bind(uuid)
            .bind(parse_uuid(admin_id).map_err(|_| RedeemFailure::InvalidCode)?)
            .execute(&mut *tx)
            .await
            .map_err(|_| RedeemFailure::InvalidCode)?;
            let rows = result.rows_affected();
            if rows > 0 {
                affected += rows;
                insert_redeem_event(
                    &mut tx,
                    Some(uuid),
                    Some(parse_uuid(admin_id).map_err(|_| RedeemFailure::InvalidCode)?),
                    "stocked",
                    Some("admin_bulk_stock"),
                )
                .await
                .ok();
            }
        }
        tx.commit().await.map_err(|_| RedeemFailure::InvalidCode)?;
        Ok(affected)
    }

    pub async fn admin_restock_redeem_codes(
        &self,
        admin_id: &str,
        codes: &[String],
    ) -> Result<u64, RedeemFailure> {
        if codes.is_empty() {
            return Ok(0);
        }
        let mut tx = self.pool.begin().await.map_err(|_| RedeemFailure::InvalidCode)?;
        let mut affected: u64 = 0;
        let admin_uuid = parse_uuid(admin_id).map_err(|_| RedeemFailure::InvalidCode)?;
        for raw in codes {
            let normalized = match normalize_redeem_code(raw) {
                Some(v) => v,
                None => continue,
            };
            let hash = code_hash(&normalized);
            let result = sqlx::query(
                "UPDATE redeem_codes \
                 SET stock_status = 'new', restocked_at = now(), restocked_by = $2, \
                     stocked_at = NULL, stocked_by = NULL, updated_at = now() \
                 WHERE code_hash = $1 AND stock_status IN ('stocked', 'restocked')",
            )
            .bind(&hash)
            .bind(admin_uuid)
            .execute(&mut *tx)
            .await
            .map_err(|_| RedeemFailure::InvalidCode)?;
            let rows = result.rows_affected();
            if rows > 0 {
                affected += rows;
                insert_redeem_event(
                    &mut tx,
                    None,
                    Some(admin_uuid),
                    "restocked",
                    Some("admin_bulk_restock"),
                )
                .await
                .ok();
            }
        }
        tx.commit().await.map_err(|_| RedeemFailure::InvalidCode)?;
        Ok(affected)
    }

    async fn record_redeem_failure(&self, user_id: Uuid, reason: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO redeem_code_events (user_id, event_type, reason) VALUES ($1, 'redeem_failed', $2)",
        )
        .bind(user_id)
        .bind(reason)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn create_feedback_thread(
        &self,
        user_id: String,
        req: crate::feedback_service::CreateThreadRequest,
    ) -> Result<(FeedbackThread, FeedbackMessage), crate::feedback_service::FeedbackError> {
        if req.title.trim().is_empty() {
            return Err(crate::feedback_service::FeedbackError::Validation(
                "title is required".into(),
            ));
        }
        if req.content.trim().is_empty() {
            return Err(crate::feedback_service::FeedbackError::Validation(
                "content is required".into(),
            ));
        }
        let feedback_type: FeedbackType = req
            .feedback_type
            .parse()
            .map_err(crate::feedback_service::FeedbackError::Validation)?;
        let priority: FeedbackPriority = req
            .priority
            .as_deref()
            .unwrap_or("normal")
            .parse()
            .map_err(crate::feedback_service::FeedbackError::Validation)?;
        let user_uuid = parse_uuid(&user_id)
            .map_err(|_| crate::feedback_service::FeedbackError::Unauthorized)?;
        let job_uuid = req
            .conversion_job_id
            .as_deref()
            .map(parse_uuid)
            .transpose()
            .map_err(|_| {
                crate::feedback_service::FeedbackError::Validation(
                    "invalid conversion_job_id".into(),
                )
            })?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        let thread_row = sqlx::query(
            r#"
            INSERT INTO feedback_threads (user_id, conversion_job_id, title, feedback_type, priority)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id::text, user_id::text, conversion_job_id::text, title, feedback_type, status,
                      priority, admin_assignee::text,
                      EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs,
                      EXTRACT(EPOCH FROM updated_at)::bigint AS updated_at_secs
            "#,
        )
        .bind(user_uuid)
        .bind(job_uuid)
        .bind(&req.title)
        .bind(feedback_type.as_str())
        .bind(priority.as_str())
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        let thread_id: String = thread_row.get("id");
        let msg = insert_feedback_message(
            &mut tx,
            parse_uuid(&thread_id).map_err(|_| {
                crate::feedback_service::FeedbackError::Validation("invalid thread id".into())
            })?,
            None,
            Some(user_uuid),
            SenderType::User,
            &req.content,
            false,
        )
        .await
        .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        let thread = feedback_thread_from_row(&thread_row, 1, Some(msg.created_at.clone()));
        Ok((thread, msg))
    }

    pub async fn add_feedback_message(
        &self,
        user_id: String,
        thread_id: &str,
        req: crate::feedback_service::AddMessageRequest,
    ) -> Result<FeedbackMessage, crate::feedback_service::FeedbackError> {
        if req.content.trim().is_empty() {
            return Err(crate::feedback_service::FeedbackError::Validation(
                "content is required".into(),
            ));
        }
        let user_uuid = parse_uuid(&user_id)
            .map_err(|_| crate::feedback_service::FeedbackError::Unauthorized)?;
        let thread_uuid =
            parse_uuid(thread_id).map_err(|_| crate::feedback_service::FeedbackError::NotFound)?;
        let owner = sqlx::query("SELECT user_id FROM feedback_threads WHERE id = $1")
            .bind(thread_uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        let Some(owner) = owner else {
            return Err(crate::feedback_service::FeedbackError::NotFound);
        };
        if owner.get::<Uuid, _>("user_id") != user_uuid {
            return Err(crate::feedback_service::FeedbackError::Forbidden);
        }
        let parent_uuid = req
            .parent_message_id
            .as_deref()
            .map(parse_uuid)
            .transpose()
            .map_err(|_| {
                crate::feedback_service::FeedbackError::Validation(
                    "invalid parent_message_id".into(),
                )
            })?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        let msg = insert_feedback_message(
            &mut tx,
            thread_uuid,
            parent_uuid,
            Some(user_uuid),
            SenderType::User,
            &req.content,
            false,
        )
        .await
        .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        sqlx::query("UPDATE feedback_threads SET updated_at = now() WHERE id = $1")
            .bind(thread_uuid)
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        Ok(msg)
    }

    pub async fn admin_reply_feedback_message(
        &self,
        admin_id: String,
        thread_id: &str,
        req: crate::feedback_service::AdminReplyRequest,
    ) -> Result<FeedbackMessage, crate::feedback_service::FeedbackError> {
        if req.content.trim().is_empty() {
            return Err(crate::feedback_service::FeedbackError::Validation(
                "content is required".into(),
            ));
        }
        let admin_uuid = parse_uuid(&admin_id)
            .map_err(|_| crate::feedback_service::FeedbackError::Unauthorized)?;
        let thread_uuid =
            parse_uuid(thread_id).map_err(|_| crate::feedback_service::FeedbackError::NotFound)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        let updated = sqlx::query(
            "UPDATE feedback_threads SET status = CASE WHEN status = 'open' THEN 'in_progress' ELSE status END, updated_at = now() WHERE id = $1",
        )
        .bind(thread_uuid)
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        if updated.rows_affected() == 0 {
            return Err(crate::feedback_service::FeedbackError::NotFound);
        }
        let msg = insert_feedback_message(
            &mut tx,
            thread_uuid,
            None,
            Some(admin_uuid),
            SenderType::Admin,
            &req.content,
            req.is_internal.unwrap_or(false),
        )
        .await
        .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        Ok(msg)
    }

    pub async fn admin_update_feedback_thread(
        &self,
        thread_id: &str,
        req: crate::feedback_service::AdminUpdateThreadRequest,
    ) -> Result<FeedbackThread, crate::feedback_service::FeedbackError> {
        let thread_uuid =
            parse_uuid(thread_id).map_err(|_| crate::feedback_service::FeedbackError::NotFound)?;
        let status = req
            .status
            .as_deref()
            .map(str::parse::<FeedbackStatus>)
            .transpose()
            .map_err(crate::feedback_service::FeedbackError::Validation)?;
        let priority = req
            .priority
            .as_deref()
            .map(str::parse::<FeedbackPriority>)
            .transpose()
            .map_err(crate::feedback_service::FeedbackError::Validation)?;
        let assignee = req
            .admin_assignee
            .as_deref()
            .map(parse_uuid)
            .transpose()
            .map_err(|_| {
                crate::feedback_service::FeedbackError::Validation("invalid admin_assignee".into())
            })?;
        let row = sqlx::query(
            r#"
            UPDATE feedback_threads
            SET status = COALESCE($2, status),
                priority = COALESCE($3, priority),
                admin_assignee = COALESCE($4, admin_assignee),
                updated_at = now()
            WHERE id = $1
            RETURNING id::text, user_id::text, conversion_job_id::text, title, feedback_type, status,
                      priority, admin_assignee::text,
                      EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs,
                      EXTRACT(EPOCH FROM updated_at)::bigint AS updated_at_secs
            "#,
        )
        .bind(thread_uuid)
        .bind(status.as_ref().map(FeedbackStatus::as_str))
        .bind(priority.as_ref().map(FeedbackPriority::as_str))
        .bind(assignee)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        let Some(row) = row else {
            return Err(crate::feedback_service::FeedbackError::NotFound);
        };
        let summary = self
            .feedback_counts(thread_uuid)
            .await
            .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        Ok(feedback_thread_from_row(&row, summary.0, summary.1))
    }

    pub async fn list_user_feedback_threads(
        &self,
        user_id: &str,
    ) -> Result<Vec<FeedbackThreadSummary>, sqlx::Error> {
        let rows = sqlx::query(&feedback_summary_sql("WHERE t.user_id = $1"))
            .bind(parse_uuid(user_id)?)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.iter().map(feedback_summary_from_row).collect())
    }

    pub async fn admin_list_feedback_threads(
        &self,
        filters: &ThreadFilters,
    ) -> Result<Vec<FeedbackThreadSummary>, sqlx::Error> {
        let mut rows = sqlx::query(&feedback_summary_sql(""))
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| feedback_summary_from_row(&row))
            .collect::<Vec<_>>();
        if let Some(status) = &filters.status {
            rows.retain(|row| &row.status == status);
        }
        if let Some(feedback_type) = &filters.feedback_type {
            rows.retain(|row| &row.feedback_type == feedback_type);
        }
        if let Some(date_from) = &filters.date_from {
            rows.retain(|row| &row.created_at >= date_from);
        }
        if let Some(date_to) = &filters.date_to {
            rows.retain(|row| &row.created_at <= date_to);
        }
        let page = filters.page.unwrap_or(1).max(1);
        let page_size = filters.page_size.unwrap_or(20).min(100);
        let start = ((page - 1) * page_size) as usize;
        Ok(rows
            .into_iter()
            .skip(start)
            .take(page_size as usize)
            .collect())
    }

    pub async fn get_feedback_thread_for_user(
        &self,
        user_id: &str,
        thread_id: &str,
    ) -> Result<(FeedbackThread, Vec<FeedbackMessage>), crate::feedback_service::FeedbackError>
    {
        let user_uuid = parse_uuid(user_id)
            .map_err(|_| crate::feedback_service::FeedbackError::Unauthorized)?;
        let thread_uuid =
            parse_uuid(thread_id).map_err(|_| crate::feedback_service::FeedbackError::NotFound)?;
        let row = sqlx::query(&feedback_thread_sql("WHERE id = $1"))
            .bind(thread_uuid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        let Some(row) = row else {
            return Err(crate::feedback_service::FeedbackError::NotFound);
        };
        if row.get::<Uuid, _>("user_id_raw") != user_uuid {
            return Err(crate::feedback_service::FeedbackError::Forbidden);
        }
        let messages = self
            .feedback_messages(thread_uuid, false)
            .await
            .map_err(|e| crate::feedback_service::FeedbackError::Validation(e.to_string()))?;
        let thread = feedback_thread_from_row(
            &row,
            messages.len() as u32,
            messages.last().map(|m| m.created_at.clone()),
        );
        Ok((thread, messages))
    }

    async fn feedback_counts(&self, thread_id: Uuid) -> Result<(u32, Option<String>), sqlx::Error> {
        let row = sqlx::query(
            "SELECT COUNT(*)::bigint AS count, EXTRACT(EPOCH FROM MAX(created_at))::bigint AS latest_secs FROM feedback_messages WHERE thread_id = $1",
        )
        .bind(thread_id)
        .fetch_one(&self.pool)
        .await?;
        Ok((
            row.get::<i64, _>("count").max(0) as u32,
            epoch_col(&row, "latest_secs"),
        ))
    }

    async fn feedback_messages(
        &self,
        thread_id: Uuid,
        include_internal: bool,
    ) -> Result<Vec<FeedbackMessage>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text, thread_id::text, parent_message_id::text, sender_user_id::text,
                   sender_type, content, attachments, is_internal,
                   EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
            FROM feedback_messages
            WHERE thread_id = $1 AND ($2 OR is_internal = false)
            ORDER BY created_at ASC
            "#,
        )
        .bind(thread_id)
        .bind(include_internal)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(feedback_message_from_row).collect())
    }
}

fn app_user_from_row(row: &PgRow) -> AppUser {
    AppUser {
        id: row.get("id"),
        email: row.get("email"),
        display_name: row.get("display_name"),
        plan_id: row.get("default_plan_id"),
        role: row.get("role"),
        status: row
            .try_get("status")
            .unwrap_or_else(|_| "active".to_string()),
    }
}

fn billing_plan_from_row(row: &PgRow) -> BillingPlanRecord {
    let features: Value = row.get("features");
    BillingPlanRecord {
        id: row.get("id"),
        name: row.get("name"),
        currency: row.get("currency"),
        price_cents: row.get::<i32, _>("price_cents").max(0) as u64,
        monthly_conversions: row.get::<i32, _>("monthly_conversions").max(0) as u64,
        storage_bytes: row.get::<i64, _>("storage_bytes").max(0) as u64,
        features: serde_json::from_value(features).unwrap_or_default(),
    }
}

fn parse_uuid(value: &str) -> Result<Uuid, sqlx::Error> {
    Uuid::parse_str(value).map_err(|e| sqlx::Error::Protocol(e.to_string()))
}

fn parse_optional_epoch(value: &Option<String>) -> Result<Option<f64>, String> {
    value
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().parse::<f64>().map_err(|e| e.to_string()))
        .transpose()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

fn epoch_col(row: &PgRow, name: &str) -> Option<String> {
    row.try_get::<Option<i64>, _>(name)
        .ok()
        .flatten()
        .map(|value| value.to_string())
}

fn upload_from_row(row: &PgRow, bytes: Vec<u8>) -> UploadRecord {
    let object_key: String = row.get("object_key");
    UploadRecord {
        upload_id: row.get("id"),
        file_name: row.get("file_name"),
        bytes,
        storage_key: Some(object_key.clone()),
        storage_path: Some(object_key),
        bytes_size: row.get::<i64, _>("bytes").max(0) as u64,
        created_at: epoch_col(row, "created_at_secs").unwrap_or_else(now_timestamp),
    }
}

fn job_select_sql(tail: &str) -> String {
    format!(
        r#"
        SELECT id::text, user_id::text, upload_id::text, main_tex, profile, quality, engine, status,
               result_docx_key, result_report_key, report_json, source_zip_key, result_log_key, storage_path,
               zip_bytes, docx_bytes, log_bytes, error_code, error_message,
               EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs,
               EXTRACT(EPOCH FROM updated_at)::bigint AS updated_at_secs
        FROM conversion_jobs {tail}
        "#
    )
}

fn job_from_row(
    row: &PgRow,
    docx: Option<Vec<u8>>,
    report: Option<ConversionReportRecord>,
) -> ConversionJobRecord {
    let status_text: String = row.get("status");
    ConversionJobRecord {
        job_id: row.get("id"),
        user_id: row.get("user_id"),
        upload_id: row.get("upload_id"),
        main_tex: row.get("main_tex"),
        profile: row.get("profile"),
        quality: row.get("quality"),
        engine: row.get("engine"),
        status: ConversionStatus::from_str(&status_text),
        created_at: epoch_col(row, "created_at_secs").unwrap_or_else(now_timestamp),
        updated_at: epoch_col(row, "updated_at_secs").unwrap_or_else(now_timestamp),
        docx,
        report,
        error_code: row.get("error_code"),
        error: row.get("error_message"),
        storage_path: row.get("storage_path"),
        source_zip_key: row.get("source_zip_key"),
        result_docx_key: row.get("result_docx_key"),
        result_log_key: row.get("result_log_key"),
        zip_bytes: row
            .try_get::<Option<i64>, _>("zip_bytes")
            .ok()
            .flatten()
            .map(|v| v.max(0) as u64),
        docx_bytes: row
            .try_get::<Option<i64>, _>("docx_bytes")
            .ok()
            .flatten()
            .map(|v| v.max(0) as u64),
        log_bytes: row
            .try_get::<Option<i64>, _>("log_bytes")
            .ok()
            .flatten()
            .map(|v| v.max(0) as u64),
    }
}

fn report_from_row(row: &PgRow) -> Option<ConversionReportRecord> {
    let report_json = row.try_get::<Value, _>("report_json").ok();
    if let Some(value) = report_json {
        if !value.is_null() && value != Value::Object(Default::default()) {
            if let Ok(report) = serde_json::from_value(value) {
                return Some(report);
            }
        }
    }
    let raw: Option<String> = row.try_get("result_report_key").ok().flatten();
    raw.and_then(|value| serde_json::from_str(&value).ok())
}

fn recharge_from_row(row: &PgRow) -> RechargeRecord {
    RechargeRecord {
        recharge_id: row.get("id"),
        user_id: row.get("user_id"),
        recharge_type: row.get("recharge_type"),
        package_id: row.get("package_id"),
        quantity: row.get::<i32, _>("quantity").max(0) as u64,
        amount_cents: row.get::<i32, _>("amount_cents").max(0) as u64,
        currency: row.get("currency"),
        status: row.get("status"),
        provider: row.get("provider"),
        provider_trade_id: row.get("provider_trade_id"),
        created_at: epoch_col(row, "created_at_secs").unwrap_or_else(now_timestamp),
    }
}

fn manual_order_from_row(row: &PgRow) -> ManualOrderRecord {
    ManualOrderRecord {
        order_id: row.get("id"),
        user_id: row.get("user_id"),
        recharge_id: row.get("recharge_id"),
        recharge_type: row.get("recharge_type"),
        package_id: row.get("package_id"),
        quantity: row.get::<i32, _>("quantity").max(0) as u64,
        amount_cents: row.get::<i32, _>("amount_cents").max(0) as u64,
        currency: row.get("currency"),
        status: row.get("status"),
        operator_id: row.get("operator_id"),
        payment_note: row.get("payment_note"),
        created_at: epoch_col(row, "created_at_secs").unwrap_or_else(now_timestamp),
    }
}

fn waitlist_from_row(row: &PgRow) -> Value {
    serde_json::json!({
        "id": row.get::<String, _>("id"),
        "email": row.get::<String, _>("email"),
        "identity": row.get::<Option<String>, _>("identity"),
        "paper_type": row.get::<Option<String>, _>("paper_type"),
        "current_tool": row.get::<Option<String>, _>("current_tool"),
        "pain_point": row.get::<Option<String>, _>("pain_point"),
        "paid_intent": row.get::<Option<String>, _>("paid_intent"),
        "status": row.get::<String, _>("status"),
        "follow_up_note": row.try_get::<Option<String>, _>("follow_up_note").ok().flatten(),
        "created_at": epoch_col(row, "created_at_secs"),
    })
}

fn release_from_row(row: &PgRow) -> Value {
    serde_json::json!({
        "id": row.get::<String, _>("id"),
        "channel": row.get::<String, _>("channel"),
        "platform": row.get::<String, _>("platform"),
        "arch": row.get::<String, _>("arch"),
        "version": row.get::<String, _>("version"),
        "download_url": row.get::<String, _>("download_url"),
        "sha256": row.get::<String, _>("sha256"),
        "signature": row.get::<String, _>("signature"),
        "signature_algorithm": row.get::<String, _>("signature_algorithm"),
        "file_size_bytes": row.get::<i64, _>("file_size_bytes").max(0),
        "release_title": row.get::<String, _>("release_title"),
        "release_notes": row.get::<String, _>("release_notes"),
        "published_by": row.get::<String, _>("published_by"),
        "is_prerelease": row.get::<bool, _>("is_prerelease"),
        "active": row.get::<bool, _>("active"),
        "published_at": epoch_col(row, "published_at_secs"),
    })
}

#[allow(clippy::too_many_arguments)]
async fn insert_recharge(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    recharge_type: &str,
    package_id: &str,
    quantity: u64,
    amount_cents: u64,
    status: &str,
    provider: &str,
    provider_trade_id: &str,
) -> Result<RechargeRecord, sqlx::Error> {
    let row = sqlx::query(
        r#"
        INSERT INTO recharges
            (user_id, recharge_type, package_id, quantity, amount_cents, currency, status, provider, provider_trade_id)
        VALUES ($1, $2, $3, $4, $5, 'CNY', $6, $7, $8)
        RETURNING id::text, user_id::text, recharge_type, package_id, quantity, amount_cents,
                  currency, status, provider, provider_trade_id,
                  EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
        "#,
    )
    .bind(user_id)
    .bind(recharge_type)
    .bind(package_id)
    .bind(quantity as i32)
    .bind(amount_cents as i32)
    .bind(status)
    .bind(provider)
    .bind(provider_trade_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(recharge_from_row(&row))
}

async fn apply_entitlement_sql(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    record: &RechargeRecord,
) -> Result<(), sqlx::Error> {
    match record.recharge_type.as_str() {
        "count" => {
            sqlx::query(
                r#"
                INSERT INTO commercial_entitlements (user_id, count_balance, source_order_id)
                VALUES ($1, $2, $3)
                ON CONFLICT (user_id) DO UPDATE SET
                    count_balance = commercial_entitlements.count_balance + EXCLUDED.count_balance,
                    source_order_id = EXCLUDED.source_order_id,
                    updated_at = now()
                "#,
            )
            .bind(user_id)
            .bind(record.quantity as i64)
            .bind(&record.recharge_id)
            .execute(&mut **tx)
            .await?;
        }
        "date" => {
            sqlx::query(
                r#"
                INSERT INTO commercial_entitlements (user_id, valid_until, source_order_id)
                VALUES ($1, now() + ($2::text || ' days')::interval, $3)
                ON CONFLICT (user_id) DO UPDATE SET
                    valid_until = GREATEST(COALESCE(commercial_entitlements.valid_until, now()), now()) + ($2::text || ' days')::interval,
                    source_order_id = EXCLUDED.source_order_id,
                    updated_at = now()
                "#,
            )
            .bind(user_id)
            .bind(record.quantity as i64)
            .bind(&record.recharge_id)
            .execute(&mut **tx)
            .await?;
        }
        _ => {}
    }
    Ok(())
}

async fn insert_grant_ledger(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    record: &RechargeRecord,
    source: &str,
) -> Result<(), sqlx::Error> {
    let (quantity, balance_after) = match record.recharge_type.as_str() {
        "count" => {
            let row =
                sqlx::query("SELECT count_balance FROM commercial_entitlements WHERE user_id = $1")
                    .bind(user_id)
                    .fetch_one(&mut **tx)
                    .await?;
            (
                record.quantity as i64,
                Some(row.get::<i64, _>("count_balance")),
            )
        }
        "date" => (record.quantity as i64, None),
        _ => (0, None),
    };
    if quantity > 0 {
        insert_usage_ledger(
            tx,
            user_id,
            None,
            "grant",
            quantity,
            balance_after,
            source,
            Some(&record.recharge_id),
        )
        .await?;
    }
    Ok(())
}

fn entitlement_from_row(row: &PgRow) -> EntitlementRecord {
    EntitlementRecord {
        user_id: row.get("user_id"),
        count_balance: row.get::<i64, _>("count_balance").max(0) as u64,
        valid_until: epoch_col(row, "valid_until_secs"),
        source_order_id: row.get("source_order_id"),
        updated_at: epoch_col(row, "updated_at_secs").unwrap_or_else(now_timestamp),
    }
}

async fn cloud_conversions_used_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<u64, sqlx::Error> {
    let row = sqlx::query(
        "SELECT COALESCE(SUM(quantity), 0)::bigint AS used FROM usage_events WHERE user_id = $1 AND event_type = 'cloud_conversion'",
    )
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.get::<i64, _>("used").max(0) as u64)
}

async fn preview_conversions_used_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<u64, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*)::bigint AS ledger_count,
               COALESCE(SUM(
                   CASE
                       WHEN event_type = 'reserve' THEN quantity
                       WHEN event_type = 'refund' THEN -quantity
                       ELSE 0
                   END
               ), 0)::bigint AS ledger_used
        FROM usage_ledger
        WHERE user_id = $1 AND source = 'preview'
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await?;
    if row.get::<i64, _>("ledger_count") > 0 {
        return Ok(row.get::<i64, _>("ledger_used").max(0) as u64);
    }
    cloud_conversions_used_tx(tx, user_id).await
}

#[allow(clippy::too_many_arguments)]
async fn insert_usage_ledger(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    conversion_job_id: Option<Uuid>,
    event_type: &str,
    quantity: i64,
    balance_after: Option<i64>,
    source: &str,
    reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO usage_ledger
            (user_id, conversion_job_id, event_type, quantity, balance_after, source, reason)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(user_id)
    .bind(conversion_job_id)
    .bind(event_type)
    .bind(quantity)
    .bind(balance_after)
    .bind(source)
    .bind(reason)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn ensure_usage_period(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<Uuid, sqlx::Error> {
    let row = sqlx::query(
        r#"
        INSERT INTO usage_periods (user_id, plan_id, period_start, period_end, cloud_conversions_limit, storage_bytes_limit)
        VALUES ($1, 'preview', date_trunc('month', now()), date_trunc('month', now()) + interval '1 month', $2, 1073741824)
        ON CONFLICT (user_id, period_start, period_end) DO UPDATE SET user_id = EXCLUDED.user_id
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(PREVIEW_CLOUD_CONVERSION_LIMIT as i32)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.get("id"))
}

fn batch_prefix(batch_no: &str) -> String {
    batch_no
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn redeem_batch_select_sql(tail: &str) -> String {
    format!(
        r#"
        SELECT b.id::text, b.batch_no, b.package_id, p.name AS package_name, p.package_type,
               b.quantity, b.generated_count, b.exported_count, b.status, b.channel, b.note,
               EXTRACT(EPOCH FROM b.expires_at)::bigint AS expires_at_secs,
               COALESCE(b.created_by::text, '') AS created_by,
               EXTRACT(EPOCH FROM b.created_at)::bigint AS created_at_secs
        FROM redeem_code_batches b
        JOIN redeem_packages p ON p.id = b.package_id
        {tail}
        ORDER BY b.created_at DESC
        "#
    )
}

fn redeem_batch_from_row(row: &PgRow, codes: Vec<String>) -> RedeemCodeBatchRecord {
    let batch_no: String = row.get("batch_no");
    RedeemCodeBatchRecord {
        batch_id: row.get("id"),
        batch_prefix: batch_prefix(&batch_no),
        batch_no,
        package_id: row.get("package_id"),
        package_name: row.get("package_name"),
        recharge_type: row.get("package_type"),
        quantity: row.get::<i32, _>("quantity").max(0) as u64,
        generated_count: row.get::<i32, _>("generated_count").max(0) as u64,
        exported_count: row.get::<i32, _>("exported_count").max(0) as u64,
        status: row.get("status"),
        channel: row.get("channel"),
        note: row.get("note"),
        expires_at: epoch_col(row, "expires_at_secs"),
        created_by: row.get("created_by"),
        created_at: epoch_col(row, "created_at_secs").unwrap_or_else(now_timestamp),
        codes,
    }
}

fn redeem_code_from_row(row: &PgRow) -> RedeemCodeRecord {
    RedeemCodeRecord {
        code_id: row.get("id"),
        batch_id: row.get("batch_id"),
        batch_no: row.get("batch_no"),
        package_id: row.get("package_id"),
        package_name: row.get("package_name"),
        recharge_type: row.get("package_type"),
        quantity: row.get::<i32, _>("quantity").max(0) as u64,
        code_hash: row.get("code_hash"),
        code_ciphertext: row.get("code_ciphertext"),
        code_nonce: row.get("code_nonce"),
        code_preview: row.get("code_preview"),
        plaintext_code: String::new(),
        key_version: row.get("key_version"),
        status: row.get("status"),
        stock_status: row
            .try_get::<Option<String>, _>("stock_status")
            .ok()
            .flatten()
            .unwrap_or_else(|| "new".to_string()),
        stocked_by: row
            .try_get::<Option<String>, _>("stocked_by")
            .ok()
            .flatten(),
        stocked_at: epoch_col(row, "stocked_at_secs"),
        redeemed_by: row.get("redeemed_by"),
        redeemed_recharge_id: row.get("redeemed_recharge_id"),
        redeemed_at: epoch_col(row, "redeemed_at_secs"),
        restocked_by: row
            .try_get::<Option<String>, _>("restocked_by")
            .ok()
            .flatten(),
        restocked_at: epoch_col(row, "restocked_at_secs"),
        expires_at: epoch_col(row, "expires_at_secs"),
        created_at: epoch_col(row, "created_at_secs").unwrap_or_else(now_timestamp),
    }
}

async fn insert_redeem_event(
    tx: &mut Transaction<'_, Postgres>,
    code_id: Option<Uuid>,
    user_id: Option<Uuid>,
    event_type: &str,
    reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO redeem_code_events (redeem_code_id, user_id, event_type, reason) VALUES ($1, $2, $3, $4)",
    )
    .bind(code_id)
    .bind(user_id)
    .bind(event_type)
    .bind(reason)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_feedback_message(
    tx: &mut Transaction<'_, Postgres>,
    thread_id: Uuid,
    parent_message_id: Option<Uuid>,
    sender_user_id: Option<Uuid>,
    sender_type: SenderType,
    content: &str,
    is_internal: bool,
) -> Result<FeedbackMessage, sqlx::Error> {
    let row = sqlx::query(
        r#"
        INSERT INTO feedback_messages
            (thread_id, parent_message_id, sender_user_id, sender_type, content, attachments, is_internal)
        VALUES ($1, $2, $3, $4, $5, '[]'::jsonb, $6)
        RETURNING id::text, thread_id::text, parent_message_id::text, sender_user_id::text,
                  sender_type, content, attachments, is_internal,
                  EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
        "#,
    )
    .bind(thread_id)
    .bind(parent_message_id)
    .bind(sender_user_id)
    .bind(sender_type.as_str())
    .bind(content)
    .bind(is_internal)
    .fetch_one(&mut **tx)
    .await?;
    Ok(feedback_message_from_row(&row))
}

fn feedback_thread_sql(tail: &str) -> String {
    format!(
        r#"
        SELECT id::text, user_id AS user_id_raw, user_id::text, conversion_job_id::text, title,
               feedback_type, status, priority, admin_assignee::text,
               EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs,
               EXTRACT(EPOCH FROM updated_at)::bigint AS updated_at_secs
        FROM feedback_threads {tail}
        "#
    )
}

fn feedback_summary_sql(where_clause: &str) -> String {
    format!(
        r#"
        SELECT t.id::text, t.conversion_job_id::text, t.title, t.feedback_type, t.status, t.priority,
               COUNT(m.id)::bigint AS message_count,
               EXTRACT(EPOCH FROM MAX(m.created_at))::bigint AS latest_message_at_secs,
               EXTRACT(EPOCH FROM t.created_at)::bigint AS created_at_secs,
               EXTRACT(EPOCH FROM t.updated_at)::bigint AS updated_at_secs
        FROM feedback_threads t
        LEFT JOIN feedback_messages m ON m.thread_id = t.id
        {where_clause}
        GROUP BY t.id
        ORDER BY t.updated_at DESC
        "#
    )
}

fn feedback_thread_from_row(
    row: &PgRow,
    message_count: u32,
    latest_message_at: Option<String>,
) -> FeedbackThread {
    FeedbackThread {
        thread_id: row.get("id"),
        user_id: row.get("user_id"),
        conversion_job_id: row.get("conversion_job_id"),
        title: row.get("title"),
        feedback_type: row
            .get::<String, _>("feedback_type")
            .parse()
            .unwrap_or(FeedbackType::Issue),
        status: row
            .get::<String, _>("status")
            .parse()
            .unwrap_or(FeedbackStatus::Open),
        priority: row
            .get::<String, _>("priority")
            .parse()
            .unwrap_or(FeedbackPriority::Normal),
        admin_assignee: row.get("admin_assignee"),
        message_count,
        latest_message_at,
        created_at: epoch_col(row, "created_at_secs").unwrap_or_else(now_timestamp),
        updated_at: epoch_col(row, "updated_at_secs").unwrap_or_else(now_timestamp),
    }
}

fn feedback_summary_from_row(row: &PgRow) -> FeedbackThreadSummary {
    FeedbackThreadSummary {
        thread_id: row.get("id"),
        conversion_job_id: row.get("conversion_job_id"),
        title: row.get("title"),
        feedback_type: row.get("feedback_type"),
        status: row.get("status"),
        priority: row.get("priority"),
        message_count: row.get::<i64, _>("message_count").max(0) as u32,
        latest_message_at: epoch_col(row, "latest_message_at_secs"),
        created_at: epoch_col(row, "created_at_secs").unwrap_or_else(now_timestamp),
        updated_at: epoch_col(row, "updated_at_secs").unwrap_or_else(now_timestamp),
    }
}

fn feedback_message_from_row(row: &PgRow) -> FeedbackMessage {
    let attachments: Value = row.get("attachments");
    FeedbackMessage {
        message_id: row.get("id"),
        thread_id: row.get("thread_id"),
        parent_message_id: row.get("parent_message_id"),
        sender_user_id: row.get("sender_user_id"),
        sender_type: match row.get::<String, _>("sender_type").as_str() {
            "admin" => SenderType::Admin,
            "system" => SenderType::System,
            _ => SenderType::User,
        },
        content: row.get("content"),
        attachments: serde_json::from_value(attachments).unwrap_or_default(),
        is_internal: row.get("is_internal"),
        created_at: epoch_col(row, "created_at_secs").unwrap_or_else(now_timestamp),
    }
}
