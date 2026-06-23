//! P5/P6: Desktop adapter for Tex2Doc commercial account APIs.
//!
//! The Slint UI is synchronous, while `doc-commercial-api-client` is async.
//! This module owns the small blocking bridge used by UI callbacks.

use std::time::Duration;

use doc_commercial_api_client::{
    ApiClient, ApiError, AuthResponse, BillingPortalRequest, BillingSession, CheckoutRequest,
    ClientConfig, ConversionJob, LoginRequest, PlanSummary, RechargeRecord, RedeemCodeRecord,
    RedeemCodeRequest, RedeemCodeResult, RefreshRequest, RegisterRequest, UsageSummary,
    UserProfile,
};
use thiserror::Error;

#[derive(Debug)]
pub struct CloudAccountSession {
    pub access_token: String,
    pub refresh_token: String,
    pub display_name: Option<String>,
    pub email: String,
    pub plan_id: String,
    pub usage: UsageSummary,
}

#[derive(Debug, Error)]
pub enum CloudAccountError {
    #[error("invalid API base URL: {0}")]
    InvalidBaseUrl(String),
    #[error("runtime error: {0}")]
    Runtime(String),
    #[error("API error: {0}")]
    Api(#[from] ApiError),
}

pub type Result<T> = std::result::Result<T, CloudAccountError>;

pub fn login_and_fetch_usage_blocking(
    base_url: &str,
    email: &str,
    password: &str,
) -> Result<CloudAccountSession> {
    let base_url = parse_base_url(base_url)?;
    let email = email.trim().to_string();
    let password = password.to_string();
    let runtime = runtime()?;

    runtime.block_on(async move {
        let anonymous = ApiClient::new(ClientConfig {
            base_url: base_url.clone(),
            api_key: String::new(),
            timeout: Duration::from_secs(30),
        })?;
        let auth = anonymous.login(&LoginRequest { email, password }).await?;
        let authenticated = authenticated_client(base_url, &auth.access_token)?;
        let usage = authenticated.usage().await?;
        Ok(session_from_auth(auth, usage))
    })
}

pub fn register_and_fetch_usage_blocking(
    base_url: &str,
    email: &str,
    password: &str,
) -> Result<CloudAccountSession> {
    let base_url = parse_base_url(base_url)?;
    let email = email.trim().to_string();
    let password = password.to_string();
    let display_name = display_name_from_email(&email);
    let runtime = runtime()?;

    runtime.block_on(async move {
        let anonymous = ApiClient::new(ClientConfig {
            base_url: base_url.clone(),
            api_key: String::new(),
            timeout: Duration::from_secs(30),
        })?;
        let auth = anonymous
            .register(&RegisterRequest {
                email,
                password,
                display_name,
            })
            .await?;
        let authenticated = authenticated_client(base_url, &auth.access_token)?;
        let usage = authenticated.usage().await?;
        Ok(session_from_auth(auth, usage))
    })
}

pub fn refresh_and_fetch_usage_blocking(
    base_url: &str,
    refresh_token: Option<String>,
) -> Result<CloudAccountSession> {
    let refresh_token = refresh_token.ok_or_else(|| {
        CloudAccountError::Api(ApiError::Api {
            code: "missing_refresh_token".to_string(),
            message: "sign in before refreshing login".to_string(),
        })
    })?;
    let base_url = parse_base_url(base_url)?;
    let runtime = runtime()?;

    runtime.block_on(async move {
        let anonymous = ApiClient::new(ClientConfig {
            base_url: base_url.clone(),
            api_key: String::new(),
            timeout: Duration::from_secs(30),
        })?;
        let auth = anonymous.refresh(&RefreshRequest { refresh_token }).await?;
        let authenticated = authenticated_client(base_url, &auth.access_token)?;
        let user = authenticated.me().await?;
        let usage = authenticated.usage().await?;
        Ok(session_from_user(auth, user, usage))
    })
}

pub fn fetch_usage_blocking(base_url: &str, access_token: &str) -> Result<UsageSummary> {
    let base_url = parse_base_url(base_url)?;
    let access_token = access_token.to_string();
    let runtime = runtime()?;

    runtime.block_on(async move {
        let client = authenticated_client(base_url, &access_token)?;
        client.usage().await.map_err(CloudAccountError::from)
    })
}

pub fn fetch_plans_blocking(base_url: &str) -> Result<Vec<PlanSummary>> {
    let base_url = parse_base_url(base_url)?;
    let runtime = runtime()?;

    runtime.block_on(async move {
        let client = ApiClient::new(ClientConfig {
            base_url,
            api_key: String::new(),
            timeout: Duration::from_secs(30),
        })?;
        client.plans().await.map_err(CloudAccountError::from)
    })
}

pub fn create_checkout_blocking(
    base_url: &str,
    access_token: Option<String>,
    plan_id: &str,
) -> Result<BillingSession> {
    let access_token = access_token.ok_or_else(|| {
        CloudAccountError::Api(ApiError::Api {
            code: "missing_access_token".to_string(),
            message: "sign in before opening checkout".to_string(),
        })
    })?;
    let base_url = parse_base_url(base_url)?;
    let plan_id = plan_id.trim().to_string();
    let runtime = runtime()?;

    runtime.block_on(async move {
        let client = authenticated_client(base_url, &access_token)?;
        client
            .create_checkout(&CheckoutRequest {
                plan_id: if plan_id.is_empty() {
                    "pro".to_string()
                } else {
                    plan_id
                },
                success_url: "https://tex2doc.cn/billing/success".to_string(),
                cancel_url: "https://tex2doc.cn/billing/cancel".to_string(),
            })
            .await
            .map_err(CloudAccountError::from)
    })
}

pub fn create_billing_portal_blocking(
    base_url: &str,
    access_token: Option<String>,
) -> Result<BillingSession> {
    let access_token = access_token.ok_or_else(|| {
        CloudAccountError::Api(ApiError::Api {
            code: "missing_access_token".to_string(),
            message: "sign in before opening billing portal".to_string(),
        })
    })?;
    let base_url = parse_base_url(base_url)?;
    let runtime = runtime()?;

    runtime.block_on(async move {
        let client = authenticated_client(base_url, &access_token)?;
        client
            .create_billing_portal(&BillingPortalRequest {
                return_url: "https://tex2doc.cn/account".to_string(),
            })
            .await
            .map_err(CloudAccountError::from)
    })
}

pub fn redeem_code_blocking(
    base_url: &str,
    access_token: Option<String>,
    code: &str,
) -> Result<(RedeemCodeResult, UsageSummary)> {
    let access_token = access_token.ok_or_else(|| {
        CloudAccountError::Api(ApiError::Api {
            code: "missing_access_token".to_string(),
            message: "sign in before redeeming a code".to_string(),
        })
    })?;
    let base_url = parse_base_url(base_url)?;
    let code = code.trim().to_string();
    let runtime = runtime()?;

    runtime.block_on(async move {
        let client = authenticated_client(base_url, &access_token)?;
        let redeemed = client.redeem_code(&RedeemCodeRequest { code }).await?;
        let usage = client.usage().await?;
        Ok((redeemed, usage))
    })
}

pub fn fetch_recharge_table_blocking(
    base_url: &str,
    access_token: Option<String>,
) -> Result<Vec<RechargeTableRow>> {
    let access_token = access_token.ok_or_else(|| {
        CloudAccountError::Api(ApiError::Api {
            code: "missing_access_token".to_string(),
            message: "sign in before querying recharge records".to_string(),
        })
    })?;
    let base_url = parse_base_url(base_url)?;
    let runtime = runtime()?;

    runtime.block_on(async move {
        let client = authenticated_client(base_url, &access_token)?;
        let mut rows = Vec::new();
        for record in client.recharge_records().await? {
            rows.push(RechargeTableRow::from_recharge(record));
        }
        for record in client.redeem_code_records().await? {
            rows.push(RechargeTableRow::from_redeem(record));
        }
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(rows)
    })
}

pub fn fetch_conversion_table_blocking(
    base_url: &str,
    access_token: Option<String>,
) -> Result<Vec<ConversionTableRow>> {
    let access_token = access_token.ok_or_else(|| {
        CloudAccountError::Api(ApiError::Api {
            code: "missing_access_token".to_string(),
            message: "sign in before querying conversion records".to_string(),
        })
    })?;
    let base_url = parse_base_url(base_url)?;
    let runtime = runtime()?;

    runtime.block_on(async move {
        let client = authenticated_client(base_url, &access_token)?;
        let mut rows = client
            .conversions()
            .await?
            .into_iter()
            .map(ConversionTableRow::from_conversion)
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(rows)
    })
}

pub fn usage_line(usage: &UsageSummary) -> String {
    let remaining = usage
        .cloud_conversions_limit
        .saturating_sub(usage.cloud_conversions_used);
    format!(
        "Plan: {} | Cloud conversions: {}/{} | Remaining: {} | Count balance: {}",
        usage.plan_id,
        usage.cloud_conversions_used,
        usage.cloud_conversions_limit,
        remaining,
        usage.count_balance
    )
}

pub fn plans_line(plans: &[PlanSummary]) -> String {
    if plans.is_empty() {
        return "No plans returned.".to_string();
    }

    plans
        .iter()
        .map(|plan| {
            format!(
                "{}: {} {}.{:02}/mo, {} conversions",
                plan.id,
                plan.currency,
                plan.price_cents / 100,
                plan.price_cents % 100,
                plan.monthly_conversions
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn session_from_auth(auth: AuthResponse, usage: UsageSummary) -> CloudAccountSession {
    let user = auth.user;
    CloudAccountSession {
        access_token: auth.access_token,
        refresh_token: auth.refresh_token,
        display_name: user.display_name,
        email: user.email,
        plan_id: user.plan_id,
        usage,
    }
}

fn session_from_user(
    auth: AuthResponse,
    user: UserProfile,
    usage: UsageSummary,
) -> CloudAccountSession {
    CloudAccountSession {
        access_token: auth.access_token,
        refresh_token: auth.refresh_token,
        display_name: user.display_name,
        email: user.email,
        plan_id: user.plan_id,
        usage,
    }
}

fn display_name_from_email(email: &str) -> Option<String> {
    email
        .split('@')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn authenticated_client(base_url: url::Url, access_token: &str) -> Result<ApiClient> {
    ApiClient::new(ClientConfig {
        base_url,
        api_key: access_token.to_string(),
        timeout: Duration::from_secs(30),
    })
    .map_err(CloudAccountError::from)
}

#[derive(Debug, Clone)]
pub struct RechargeTableRow {
    pub id: String,
    pub kind: String,
    pub package: String,
    pub quantity: String,
    pub status: String,
    pub provider: String,
    pub created_at: String,
}

impl RechargeTableRow {
    fn from_recharge(record: RechargeRecord) -> Self {
        Self {
            id: record.recharge_id,
            kind: record.recharge_type,
            package: record.package_id,
            quantity: record.quantity.to_string(),
            status: record.status,
            provider: record.provider,
            created_at: record.created_at,
        }
    }

    fn from_redeem(record: RedeemCodeRecord) -> Self {
        Self {
            id: record.redeem_id,
            kind: "redeem-code".to_string(),
            package: format!("{} / {}", record.package_id, record.code_preview),
            quantity: record.quantity.to_string(),
            status: record.status,
            provider: record.batch_no,
            created_at: record.redeemed_at.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConversionTableRow {
    pub id: String,
    pub main_tex: String,
    pub profile: String,
    pub status: String,
    pub updated_at: String,
    pub error: String,
}

impl ConversionTableRow {
    fn from_conversion(record: ConversionJob) -> Self {
        Self {
            id: record.job_id,
            main_tex: record.main_tex.unwrap_or_default(),
            profile: record.profile.unwrap_or_default(),
            status: format!("{:?}", record.status),
            updated_at: record.updated_at,
            error: record
                .error_code
                .or(record.error)
                .unwrap_or_else(|| "-".to_string()),
        }
    }
}

fn parse_base_url(value: &str) -> Result<url::Url> {
    value
        .trim()
        .parse()
        .map_err(|e: url::ParseError| CloudAccountError::InvalidBaseUrl(e.to_string()))
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CloudAccountError::Runtime(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_line_includes_remaining_quota() {
        let usage = UsageSummary {
            plan_id: "preview".to_string(),
            cloud_conversions_used: 3,
            cloud_conversions_limit: 10,
            count_balance: 0,
            date_valid_until: None,
            entitlement_source_order_id: None,
            storage_bytes_used: 0,
            storage_bytes_limit: 1024,
            period_start: "2026-06-01T00:00:00Z".to_string(),
            period_end: "2026-07-01T00:00:00Z".to_string(),
        };

        assert_eq!(
            usage_line(&usage),
            "Plan: preview | Cloud conversions: 3/10 | Remaining: 7 | Count balance: 0"
        );
    }

    #[test]
    fn plans_line_formats_plan_summaries() {
        let plans = vec![PlanSummary {
            id: "pro".to_string(),
            name: "Pro".to_string(),
            price_cents: 2900,
            currency: "USD".to_string(),
            monthly_conversions: 1000,
            features: vec!["cloud".to_string()],
        }];

        assert_eq!(plans_line(&plans), "pro: USD 29.00/mo, 1000 conversions");
    }
}
