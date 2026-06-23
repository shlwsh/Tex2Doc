//! PostgreSQL-backed commercial state.

use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

use crate::state::{
    ConversionJobRecord, ConversionReportRecord, ConversionStatus, EntitlementRecord,
    RechargeRecord, RedeemCodeBatchRecord, RedeemCodeRecord, RedeemFailure, UploadRecord,
    PREVIEW_CLOUD_CONVERSION_LIMIT,
};

const BUSINESS_SCHEMA: &str = include_str!("../../../docs-zh/money/001_docdb_business_schema.sql");
const FEEDBACK_SCHEMA: &str =
    include_str!("../../../docs-zh/money/003_feedback_and_session_storage.sql");

#[derive(Debug, Clone)]
pub struct DbStore {
    pool: PgPool,
}

impl DbStore {
    pub async fn connect_from_env() -> Result<Self, sqlx::Error> {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/docdb".to_string()
        });
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await?;
        let store = Self { pool };
        store.init_schema().await?;
        Ok(store)
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    async fn init_schema(&self) -> Result<(), sqlx::Error> {
        sqlx::raw_sql(BUSINESS_SCHEMA).execute(&self.pool).await?;
        sqlx::raw_sql(FEEDBACK_SCHEMA).execute(&self.pool).await?;
        sqlx::raw_sql(
            r#"
            CREATE TABLE IF NOT EXISTS commercial_entitlements (
                user_id UUID PRIMARY KEY REFERENCES app_users(id) ON DELETE CASCADE,
                count_balance BIGINT NOT NULL DEFAULT 0 CHECK (count_balance >= 0),
                valid_until TIMESTAMPTZ,
                source_order_id TEXT,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
            );
            CREATE TABLE IF NOT EXISTS app_access_tokens (
                token_hash TEXT PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
                expires_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            );
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_user(
        &self,
        email: &str,
        display_name: Option<&str>,
        password: &str,
    ) -> Result<AppUser, sqlx::Error> {
        let password_hash = crate::state::hash_text(password);
        let row = sqlx::query(
            r#"
            INSERT INTO app_users (email, password_hash, display_name, default_plan_id)
            VALUES ($1, $2, $3, 'preview')
            ON CONFLICT (email) DO UPDATE SET
                display_name = COALESCE(EXCLUDED.display_name, app_users.display_name),
                updated_at = now()
            RETURNING id::text, email, display_name, default_plan_id
            "#,
        )
        .bind(email)
        .bind(password_hash)
        .bind(display_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(AppUser::from_row(&row))
    }

    pub async fn user_by_id(&self, user_id: &str) -> Result<Option<AppUser>, sqlx::Error> {
        let user_id = parse_uuid(user_id)?;
        let row = sqlx::query(
            "SELECT id::text, email, display_name, default_plan_id FROM app_users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(AppUser::from_row))
    }

    pub async fn ensure_demo_admin(&self) -> Result<AppUser, sqlx::Error> {
        self.upsert_user("admin@example.com", Some("Demo Admin"), "demo-admin")
            .await
    }

    pub async fn cloud_conversions_used(&self, user_id: &str) -> Result<u64, sqlx::Error> {
        let user_id = parse_uuid(user_id)?;
        let row = sqlx::query(
            "SELECT COALESCE(SUM(quantity), 0)::bigint AS used FROM usage_events WHERE user_id = $1 AND event_type = 'cloud_conversion'",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        let used: i64 = row.get("used");
        Ok(used.max(0) as u64)
    }

    pub async fn entitlement(&self, user_id: &str) -> Result<EntitlementRecord, sqlx::Error> {
        let user_uuid = parse_uuid(user_id)?;
        let row = sqlx::query(
            r#"
            INSERT INTO commercial_entitlements (user_id)
            VALUES ($1)
            ON CONFLICT (user_id) DO UPDATE SET user_id = EXCLUDED.user_id
            RETURNING user_id::text, count_balance, EXTRACT(EPOCH FROM valid_until)::bigint AS valid_until_secs, source_order_id, EXTRACT(EPOCH FROM updated_at)::bigint AS updated_at_secs
            "#,
        )
        .bind(user_uuid)
        .fetch_one(&self.pool)
        .await?;
        Ok(EntitlementRecord {
            user_id: row.get("user_id"),
            count_balance: row.get::<i64, _>("count_balance").max(0) as u64,
            valid_until: row
                .try_get::<Option<i64>, _>("valid_until_secs")
                .ok()
                .flatten()
                .map(|value| value.to_string()),
            source_order_id: row.get("source_order_id"),
            updated_at: row
                .try_get::<Option<i64>, _>("updated_at_secs")
                .ok()
                .flatten()
                .map(|value| value.to_string())
                .unwrap_or_else(now_timestamp),
        })
    }

    pub async fn create_recharge(
        &self,
        user_id: String,
        recharge_type: String,
        package_id: String,
        quantity: u64,
        amount_cents: u64,
        status: &str,
        provider: &str,
        provider_trade_id: &str,
    ) -> Result<RechargeRecord, sqlx::Error> {
        let user_uuid = parse_uuid(&user_id)?;
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            r#"
            INSERT INTO recharges (user_id, recharge_type, package_id, quantity, amount_cents, currency, status, provider, provider_trade_id)
            VALUES ($1, $2, $3, $4, $5, 'CNY', $6, $7, $8)
            RETURNING id::text, user_id::text, recharge_type, package_id, quantity, amount_cents, currency, status, provider, provider_trade_id, EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
            "#,
        )
        .bind(user_uuid)
        .bind(&recharge_type)
        .bind(&package_id)
        .bind(quantity as i64)
        .bind(amount_cents as i64)
        .bind(status)
        .bind(provider)
        .bind(provider_trade_id)
        .fetch_one(&mut *tx)
        .await?;
        let recharge = recharge_from_row(&row);
        apply_entitlement_sql(&mut tx, user_uuid, &recharge).await?;
        tx.commit().await?;
        Ok(recharge)
    }

    pub async fn store_upload(
        &self,
        user_id: &str,
        file_name: String,
        bytes: Vec<u8>,
        object_key: String,
    ) -> Result<UploadRecord, sqlx::Error> {
        let user_uuid = parse_uuid(user_id)?;
        let sha = crate::state::hash_bytes(&bytes);
        let row = sqlx::query(
            r#"
            INSERT INTO uploads (user_id, file_name, object_key, bytes, sha256, status)
            VALUES ($1, $2, $3, $4, $5, 'stored')
            RETURNING id::text, file_name, object_key, bytes, EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
            "#,
        )
        .bind(user_uuid)
        .bind(&file_name)
        .bind(&object_key)
        .bind(bytes.len() as i64)
        .bind(sha)
        .fetch_one(&self.pool)
        .await?;
        Ok(UploadRecord {
            upload_id: row.get("id"),
            file_name,
            bytes,
            storage_path: Some(object_key.clone()),
            storage_key: Some(object_key),
            bytes_size: row.get::<i64, _>("bytes").max(0) as u64,
            created_at: row
                .try_get::<Option<i64>, _>("created_at_secs")
                .ok()
                .flatten()
                .map(|value| value.to_string())
                .unwrap_or_else(now_timestamp),
        })
    }

    pub async fn get_upload(
        &self,
        upload_id: &str,
        bytes: Option<Vec<u8>>,
    ) -> Result<Option<UploadRecord>, sqlx::Error> {
        let upload_uuid = parse_uuid(upload_id)?;
        let row = sqlx::query(
            "SELECT id::text, file_name, object_key, bytes, EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs FROM uploads WHERE id = $1",
        )
        .bind(upload_uuid)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| {
            let object_key: String = row.get("object_key");
            UploadRecord {
                upload_id: row.get("id"),
                file_name: row.get("file_name"),
                bytes: bytes.unwrap_or_default(),
                storage_path: Some(object_key.clone()),
                storage_key: Some(object_key),
                bytes_size: row.get::<i64, _>("bytes").max(0) as u64,
                created_at: row
                    .try_get::<Option<i64>, _>("created_at_secs")
                    .ok()
                    .flatten()
                    .map(|value| value.to_string())
                    .unwrap_or_else(now_timestamp),
            }
        }))
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
        let user_uuid = parse_uuid(&user_id)?;
        let upload_uuid = parse_uuid(&upload_id)?;
        let row = sqlx::query(
            r#"
            INSERT INTO conversion_jobs (user_id, upload_id, main_tex, profile, quality, engine, status)
            VALUES ($1, $2, $3, $4, $5, $6, 'queued')
            RETURNING id::text, user_id::text, upload_id::text, main_tex, profile, quality, engine, status,
                      result_docx_key, result_report_key, source_zip_key, result_log_key, storage_path,
                      zip_bytes, docx_bytes, log_bytes, error_code, error_message,
                      EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs,
                      EXTRACT(EPOCH FROM updated_at)::bigint AS updated_at_secs
            "#,
        )
        .bind(user_uuid)
        .bind(upload_uuid)
        .bind(&main_tex)
        .bind(&profile)
        .bind(&quality)
        .bind(&engine)
        .fetch_one(&self.pool)
        .await?;
        Ok(job_from_row(&row, None, None))
    }

    pub async fn get_job(
        &self,
        job_id: &str,
        docx: Option<Vec<u8>>,
    ) -> Result<Option<ConversionJobRecord>, sqlx::Error> {
        let job_uuid = parse_uuid(job_id)?;
        let row = sqlx::query(
            r#"
            SELECT id::text, user_id::text, upload_id::text, main_tex, profile, quality, engine, status,
                   result_docx_key, result_report_key, source_zip_key, result_log_key, storage_path,
                   zip_bytes, docx_bytes, log_bytes, error_code, error_message,
                   EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs,
                   EXTRACT(EPOCH FROM updated_at)::bigint AS updated_at_secs
            FROM conversion_jobs WHERE id = $1
            "#,
        )
        .bind(job_uuid)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| job_from_row(&row, docx, None)))
    }

    pub async fn list_jobs_by_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<ConversionJobRecord>, sqlx::Error> {
        let user_uuid = parse_uuid(user_id)?;
        let rows = sqlx::query(
            r#"
            SELECT id::text, user_id::text, upload_id::text, main_tex, profile, quality, engine, status,
                   result_docx_key, result_report_key, source_zip_key, result_log_key, storage_path,
                   zip_bytes, docx_bytes, log_bytes, error_code, error_message,
                   EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs,
                   EXTRACT(EPOCH FROM updated_at)::bigint AS updated_at_secs
            FROM conversion_jobs WHERE user_id = $1 ORDER BY created_at DESC
            "#,
        )
        .bind(user_uuid)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(|row| job_from_row(row, None, None)).collect())
    }

    pub async fn update_status(
        &self,
        job_id: &str,
        status: ConversionStatus,
    ) -> Result<(), sqlx::Error> {
        let job_uuid = parse_uuid(job_id)?;
        sqlx::query("UPDATE conversion_jobs SET status = $2, updated_at = now() WHERE id = $1")
            .bind(job_uuid)
            .bind(status.as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn complete_job(
        &self,
        job_id: &str,
        docx_key: Option<String>,
        docx_bytes: Option<u64>,
        log_key: Option<String>,
        log_bytes: Option<u64>,
        report: &ConversionReportRecord,
    ) -> Result<(), sqlx::Error> {
        let job_uuid = parse_uuid(job_id)?;
        sqlx::query(
            r#"
            UPDATE conversion_jobs
            SET status = 'completed',
                result_docx_key = $2,
                result_report_key = $3,
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
        .bind(job_uuid)
        .bind(docx_key)
        .bind(serde_json::to_string(report).unwrap_or_default())
        .bind(log_key)
        .bind(docx_bytes.map(|value| value as i64))
        .bind(log_bytes.map(|value| value as i64))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn fail_job(
        &self,
        job_id: &str,
        error_code: &str,
        error: &str,
        log_key: Option<String>,
        log_bytes: Option<u64>,
    ) -> Result<(), sqlx::Error> {
        let job_uuid = parse_uuid(job_id)?;
        sqlx::query(
            r#"
            UPDATE conversion_jobs
            SET status = 'failed',
                error_code = $2,
                error_message = $3,
                result_log_key = $4,
                log_bytes = $5,
                updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(job_uuid)
        .bind(error_code)
        .bind(error)
        .bind(log_key)
        .bind(log_bytes.map(|value| value as i64))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn try_consume_cloud_conversion(&self, user_id: &str) -> Result<u64, u64> {
        let user_uuid = parse_uuid(user_id).map_err(|_| 0)?;
        let mut tx = self.pool.begin().await.map_err(|_| 0)?;
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
        .map_err(|_| 0)?;
        let count_balance = entitlement.get::<i64, _>("count_balance");
        let valid_until_active = entitlement
            .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("valid_until")
            .ok()
            .flatten()
            .is_some_and(|value| value >= chrono::Utc::now());
        let used = self.cloud_conversions_used(user_id).await.map_err(|_| 0)?;
        if valid_until_active {
            tx.commit().await.map_err(|_| used)?;
            return Ok(used);
        }
        if count_balance > 0 {
            sqlx::query(
                "UPDATE commercial_entitlements SET count_balance = count_balance - 1, updated_at = now() WHERE user_id = $1",
            )
            .bind(user_uuid)
            .execute(&mut *tx)
            .await
            .map_err(|_| used)?;
            tx.commit().await.map_err(|_| used)?;
            return Ok(used);
        }
        if used >= PREVIEW_CLOUD_CONVERSION_LIMIT {
            tx.rollback().await.ok();
            return Err(used);
        }
        let period_id = ensure_usage_period(&mut tx, user_uuid).await.map_err(|_| used)?;
        sqlx::query(
            "INSERT INTO usage_events (user_id, usage_period_id, event_type, quantity) VALUES ($1, $2, 'cloud_conversion', 1)",
        )
        .bind(user_uuid)
        .bind(period_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| used)?;
        tx.commit().await.map_err(|_| used)?;
        Ok(used + 1)
    }

    pub async fn list_recharges(&self, user_id: &str) -> Result<Vec<RechargeRecord>, sqlx::Error> {
        let user_uuid = parse_uuid(user_id)?;
        let rows = sqlx::query(
            r#"
            SELECT id::text, user_id::text, recharge_type, package_id, quantity, amount_cents, currency, status, provider, provider_trade_id, EXTRACT(EPOCH FROM created_at)::bigint AS created_at_secs
            FROM recharges
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_uuid)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(recharge_from_row).collect())
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
        let package = crate::state::redeem_package(package_id).ok_or(RedeemFailure::InvalidCode)?;
        let created_by_uuid = parse_uuid(&created_by).ok();
        let expires_sql = parse_optional_epoch(&expires_at).map_err(|_| RedeemFailure::InvalidCode)?;

        let mut tx = self.pool.begin().await.map_err(|_| RedeemFailure::InvalidCode)?;
        let batch_no = format!("RC{}", Uuid::new_v4().simple().to_string()[..10].to_uppercase());
        let batch_row = sqlx::query(
            r#"
            INSERT INTO redeem_code_batches (batch_no, package_id, quantity, generated_count, exported_count, status, channel, note, expires_at, created_by)
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
        let created_at = batch_row
            .try_get::<Option<i64>, _>("created_at_secs")
            .ok()
            .flatten()
            .map(|value| value.to_string())
            .unwrap_or_else(now_timestamp);
        let batch_prefix = batch_no
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<String>();
        let mut codes = Vec::with_capacity(requested_count as usize);

        for _ in 0..requested_count {
            let code = crate::state::generate_redeem_code(&batch_prefix);
            let normalized = crate::state::normalize_redeem_code(&code)
                .ok_or(RedeemFailure::InvalidCode)?;
            let code_hash = crate::state::code_hash(&normalized);
            let nonce = crate::state::random_bytes(12);
            let ciphertext = crate::state::encrypt_code(&normalized, &nonce);
            sqlx::query(
                r#"
                INSERT INTO redeem_codes (batch_id, package_id, code_hash, code_ciphertext, code_nonce, code_preview, key_version, status, expires_at)
                VALUES ($1, $2, $3, $4, $5, $6, 'v1', 'unused', to_timestamp($7))
                "#,
            )
            .bind(parse_uuid(&batch_id).map_err(|_| RedeemFailure::InvalidCode)?)
            .bind(&package.id)
            .bind(code_hash)
            .bind(ciphertext)
            .bind(nonce)
            .bind(crate::state::code_preview(&code))
            .bind(expires_sql)
            .execute(&mut *tx)
            .await
            .map_err(|_| RedeemFailure::InvalidCode)?;
            codes.push(code);
        }

        sqlx::query(
            "INSERT INTO redeem_code_events (user_id, event_type, reason) VALUES ($1, 'generated', $2)",
        )
        .bind(created_by_uuid)
        .bind(format!("batch {batch_no}"))
        .execute(&mut *tx)
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

    pub async fn list_redeem_batches(
        &self,
    ) -> Result<Vec<RedeemCodeBatchRecord>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT b.id::text, b.batch_no, b.package_id, p.name AS package_name, p.package_type,
                   b.quantity, b.generated_count, b.exported_count, b.status, b.channel, b.note,
                   EXTRACT(EPOCH FROM b.expires_at)::bigint AS expires_at_secs,
                   COALESCE(b.created_by::text, '') AS created_by,
                   EXTRACT(EPOCH FROM b.created_at)::bigint AS created_at_secs
            FROM redeem_code_batches b
            JOIN redeem_packages p ON p.id = b.package_id
            ORDER BY b.created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(|row| redeem_batch_from_row(row, Vec::new())).collect())
    }

    pub async fn get_redeem_batch(
        &self,
        batch_id: &str,
        include_codes: bool,
    ) -> Result<Option<RedeemCodeBatchRecord>, sqlx::Error> {
        let batch_uuid = parse_uuid(batch_id)?;
        let row = sqlx::query(
            r#"
            SELECT b.id::text, b.batch_no, b.package_id, p.name AS package_name, p.package_type,
                   b.quantity, b.generated_count, b.exported_count, b.status, b.channel, b.note,
                   EXTRACT(EPOCH FROM b.expires_at)::bigint AS expires_at_secs,
                   COALESCE(b.created_by::text, '') AS created_by,
                   EXTRACT(EPOCH FROM b.created_at)::bigint AS created_at_secs
            FROM redeem_code_batches b
            JOIN redeem_packages p ON p.id = b.package_id
            WHERE b.id = $1
            "#,
        )
        .bind(batch_uuid)
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
        let batch_uuid = parse_uuid(batch_id)?;
        let rows = sqlx::query(
            "SELECT code_ciphertext, code_nonce FROM redeem_codes WHERE batch_id = $1 ORDER BY created_at ASC",
        )
        .bind(batch_uuid)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .filter_map(|row| {
                let ciphertext: Vec<u8> = row.get("code_ciphertext");
                let nonce: Vec<u8> = row.get("code_nonce");
                crate::state::decrypt_code(&ciphertext, &nonce).ok()
            })
            .map(crate::state::group_redeem_code)
            .collect())
    }

    pub async fn mark_redeem_batch_exported(&self, batch_id: &str) -> Result<(), sqlx::Error> {
        let batch_uuid = parse_uuid(batch_id)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE redeem_code_batches SET exported_count = generated_count, updated_at = now() WHERE id = $1",
        )
        .bind(batch_uuid)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO redeem_code_events (event_type, reason) VALUES ('exported', $1)",
        )
        .bind(batch_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AppUser {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub plan_id: String,
}

impl AppUser {
    fn from_row(row: &sqlx::postgres::PgRow) -> Self {
        Self {
            id: row.get("id"),
            email: row.get("email"),
            display_name: row.get("display_name"),
            plan_id: row.get("default_plan_id"),
        }
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

fn now_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn recharge_from_row(row: &sqlx::postgres::PgRow) -> RechargeRecord {
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
        created_at: row
            .try_get::<Option<i64>, _>("created_at_secs")
            .ok()
            .flatten()
            .map(|value| value.to_string())
            .unwrap_or_else(now_timestamp),
    }
}

async fn apply_entitlement_sql(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
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

fn redeem_batch_from_row(row: &sqlx::postgres::PgRow, codes: Vec<String>) -> RedeemCodeBatchRecord {
    let batch_no: String = row.get("batch_no");
    let batch_prefix = batch_no
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    RedeemCodeBatchRecord {
        batch_id: row.get("id"),
        batch_no,
        batch_prefix,
        package_id: row.get("package_id"),
        package_name: row.get("package_name"),
        recharge_type: row.get("package_type"),
        quantity: row.get::<i32, _>("quantity").max(0) as u64,
        generated_count: row.get::<i32, _>("generated_count").max(0) as u64,
        exported_count: row.get::<i32, _>("exported_count").max(0) as u64,
        status: row.get("status"),
        channel: row.get("channel"),
        note: row.get("note"),
        expires_at: row
            .try_get::<Option<i64>, _>("expires_at_secs")
            .ok()
            .flatten()
            .map(|value| value.to_string()),
        created_by: row.get("created_by"),
        created_at: row
            .try_get::<Option<i64>, _>("created_at_secs")
            .ok()
            .flatten()
            .map(|value| value.to_string())
            .unwrap_or_else(now_timestamp),
        codes,
    }
}

fn job_from_row(
    row: &sqlx::postgres::PgRow,
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
        status: match status_text.as_str() {
            "normalizing" => ConversionStatus::Normalizing,
            "detecting" => ConversionStatus::Detecting,
            "analyzing" => ConversionStatus::Analyzing,
            "compiling" => ConversionStatus::Compiling,
            "rendering" => ConversionStatus::Rendering,
            "verifying" => ConversionStatus::Verifying,
            "completed" => ConversionStatus::Completed,
            "failed" => ConversionStatus::Failed,
            "expired" => ConversionStatus::Expired,
            _ => ConversionStatus::Queued,
        },
        created_at: row
            .try_get::<Option<i64>, _>("created_at_secs")
            .ok()
            .flatten()
            .map(|value| value.to_string())
            .unwrap_or_else(now_timestamp),
        updated_at: row
            .try_get::<Option<i64>, _>("updated_at_secs")
            .ok()
            .flatten()
            .map(|value| value.to_string())
            .unwrap_or_else(now_timestamp),
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
            .map(|value| value.max(0) as u64),
        docx_bytes: row
            .try_get::<Option<i64>, _>("docx_bytes")
            .ok()
            .flatten()
            .map(|value| value.max(0) as u64),
        log_bytes: row
            .try_get::<Option<i64>, _>("log_bytes")
            .ok()
            .flatten()
            .map(|value| value.max(0) as u64),
    }
}

async fn ensure_usage_period(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
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
