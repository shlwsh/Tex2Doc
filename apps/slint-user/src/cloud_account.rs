//! P5/P6: Desktop adapter for Tex2Doc commercial account APIs.
//!
//! The Slint UI is synchronous, while `doc-commercial-api-client` is async.
//! This module owns the small blocking bridge used by UI callbacks.

use std::time::Duration;

use doc_commercial_api_client::{
    ApiClient, ApiError, AuthResponse, BillingPortalRequest, BillingSession, CheckoutRequest,
    ClientConfig, ConversionJob, CreateFeedbackRequest, FeedbackThread, LoginRequest, PlanSummary,
    RechargeRecord, RedeemCodeRecord, RedeemCodeRequest, RedeemCodeResult, RefreshRequest,
    RegisterRequest, UsageSummary, UserProfile,
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

pub fn fetch_feedback_threads_blocking(
    base_url: &str,
    access_token: Option<String>,
) -> Result<Vec<FeedbackTableRow>> {
    let access_token = access_token.ok_or_else(|| {
        CloudAccountError::Api(ApiError::Api {
            code: "missing_access_token".to_string(),
            message: "sign in before querying feedback".to_string(),
        })
    })?;
    let base_url = parse_base_url(base_url)?;
    let runtime = runtime()?;

    runtime.block_on(async move {
        let client = authenticated_client(base_url, &access_token)?;
        let mut rows = client
            .feedback_threads()
            .await?
            .into_iter()
            .map(FeedbackTableRow::from_thread)
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| b.latest_message_at.cmp(&a.latest_message_at));
        Ok(rows)
    })
}

pub fn create_feedback_thread_blocking(
    base_url: &str,
    access_token: Option<String>,
    feedback_type: &str,
    title: &str,
    content: &str,
    conversion_job_id: &str,
) -> Result<Vec<FeedbackTableRow>> {
    let access_token = access_token.ok_or_else(|| {
        CloudAccountError::Api(ApiError::Api {
            code: "missing_access_token".to_string(),
            message: "sign in before submitting feedback".to_string(),
        })
    })?;
    let base_url = parse_base_url(base_url)?;
    let request = CreateFeedbackRequest {
        title: title.trim().to_string(),
        feedback_type: normalize_feedback_type(feedback_type).to_string(),
        content: content.trim().to_string(),
        conversion_job_id: optional_trimmed(conversion_job_id),
        priority: Some("normal".to_string()),
    };
    let runtime = runtime()?;

    runtime.block_on(async move {
        let client = authenticated_client(base_url, &access_token)?;
        client.create_feedback_thread(&request).await?;
        let mut rows = client
            .feedback_threads()
            .await?
            .into_iter()
            .map(FeedbackTableRow::from_thread)
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| b.latest_message_at.cmp(&a.latest_message_at));
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

fn normalize_feedback_type(value: &str) -> &str {
    match value.trim() {
        "feature" | "requirement" => "requirement",
        _ => "issue",
    }
}

fn optional_trimmed(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
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
    pub has_zip: bool,
    pub has_docx: bool,
    pub has_log: bool,
    pub docx_size: String,
    pub zip_size: String,
    pub log_size: String,
}

impl ConversionTableRow {
    fn from_conversion(record: ConversionJob) -> Self {
        let storage = record.storage_info.as_ref();
        let has_docx = storage.map(|s| s.has_docx()).unwrap_or(false);
        let has_zip = storage.map(|s| s.has_zip()).unwrap_or(false);
        let has_log = storage.map(|s| s.has_log()).unwrap_or(false);
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
            has_docx,
            has_zip,
            has_log,
            docx_size: storage
                .and_then(|s| s.docx_size())
                .map(format_size)
                .unwrap_or_default(),
            zip_size: storage
                .and_then(|s| s.zip_size())
                .map(format_size)
                .unwrap_or_default(),
            log_size: storage
                .and_then(|s| s.log_size())
                .map(format_size)
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FeedbackTableRow {
    pub thread_id: String,
    pub title: String,
    pub feedback_type: String,
    pub status: String,
    pub priority: String,
    pub message_count: i32,
    pub latest_message_at: String,
    pub created_at: String,
    pub conversion_job_id: String,
    // 自动化研发相关字段
    pub automation_status: String,
    pub automation_request_id: String,
}

impl FeedbackTableRow {
    fn from_thread(thread: FeedbackThread) -> Self {
        let latest_message_at = thread
            .latest_message_at
            .clone()
            .or(thread.updated_at.clone())
            .unwrap_or_else(|| thread.created_at.clone());

        let automation_status = thread
            .automation_status
            .unwrap_or_else(|| "none".to_string());
        let automation_request_id = thread.automation_request_id.unwrap_or_default();

        Self {
            thread_id: thread.thread_id,
            title: thread.title,
            feedback_type: thread.feedback_type,
            status: thread.status,
            priority: thread.priority,
            message_count: thread.message_count.unwrap_or(0) as i32,
            latest_message_at,
            created_at: thread.created_at,
            conversion_job_id: thread.conversion_job_id.unwrap_or_else(|| "-".to_string()),
            automation_status,
            automation_request_id,
        }
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / 1024.0 / 1024.0)
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
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

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

    #[test]
    fn slint_cloud_account_blocking_calls_work_against_demo_server() {
        let base_url = spawn_demo_server();

        let session =
            login_and_fetch_usage_blocking(&base_url, " demo@example.com ", "password").unwrap();
        assert_eq!(session.email, "demo@example.com");
        assert_eq!(session.plan_id, "preview");
        assert_eq!(session.usage.count_balance, 8);
        assert_eq!(session.usage.period_start, "");

        let refreshed =
            refresh_and_fetch_usage_blocking(&base_url, Some(session.refresh_token.clone()))
                .unwrap();
        assert_eq!(refreshed.email, "demo@example.com");

        let registered =
            register_and_fetch_usage_blocking(&base_url, "new-demo@example.com", "password")
                .unwrap();
        assert_eq!(registered.display_name.as_deref(), Some("Demo User"));

        let usage = fetch_usage_blocking(&base_url, &session.access_token).unwrap();
        assert_eq!(usage.cloud_conversions_limit, 100);

        let plans = fetch_plans_blocking(&base_url).unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].id, "pro");

        let checkout =
            create_checkout_blocking(&base_url, Some(session.access_token.clone()), "pro").unwrap();
        assert_eq!(checkout.url, "https://billing.example/checkout");

        let portal =
            create_billing_portal_blocking(&base_url, Some(session.access_token.clone())).unwrap();
        assert_eq!(portal.url, "https://billing.example/portal");

        let (redeemed, usage_after_redeem) = redeem_code_blocking(
            &base_url,
            Some(session.access_token.clone()),
            "T2D-DEMO-001",
        )
        .unwrap();
        assert_eq!(redeemed.package_id, "count_100");
        assert_eq!(usage_after_redeem.count_balance, 8);

        let recharge_rows =
            fetch_recharge_table_blocking(&base_url, Some(session.access_token.clone())).unwrap();
        assert_eq!(recharge_rows.len(), 2);
        assert_eq!(recharge_rows[0].id, "redeem_1");

        let conversion_rows =
            fetch_conversion_table_blocking(&base_url, Some(session.access_token)).unwrap();
        assert_eq!(conversion_rows.len(), 1);
        assert_eq!(conversion_rows[0].status, "Completed");
    }

    fn spawn_demo_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        thread::spawn(move || {
            for stream in listener.incoming().take(32) {
                let Ok(stream) = stream else {
                    continue;
                };
                handle_demo_request(stream);
            }
        });

        format!("http://{addr}/v1/")
    }

    fn handle_demo_request(mut stream: TcpStream) {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        let mut buffer = [0_u8; 8192];
        let n = stream.read(&mut buffer).unwrap_or(0);
        let request = String::from_utf8_lossy(&buffer[..n]);
        let request_line = request.lines().next().unwrap_or_default();

        let (status, body) = demo_response(request_line);
        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
    }

    fn demo_response(request_line: &str) -> (&'static str, &'static str) {
        match request_line {
            line if line.starts_with("POST /v1/auth/login ") => ("200 OK", AUTH_BODY),
            line if line.starts_with("POST /v1/auth/register ") => ("200 OK", AUTH_BODY),
            line if line.starts_with("POST /v1/auth/refresh ") => ("200 OK", AUTH_BODY),
            line if line.starts_with("GET /v1/me ") => ("200 OK", USER_BODY),
            line if line.starts_with("GET /v1/usage ") => ("200 OK", USAGE_BODY),
            line if line.starts_with("GET /v1/plans ") => ("200 OK", PLANS_BODY),
            line if line.starts_with("POST /v1/billing/checkout ") => ("200 OK", CHECKOUT_BODY),
            line if line.starts_with("POST /v1/billing/portal ") => ("200 OK", PORTAL_BODY),
            line if line.starts_with("POST /v1/redeem-codes/redeem ") => ("200 OK", REDEEM_BODY),
            line if line.starts_with("GET /v1/recharges ") => ("200 OK", RECHARGES_BODY),
            line if line.starts_with("GET /v1/redeem-codes/records ") => {
                ("200 OK", REDEEM_RECORDS_BODY)
            }
            line if line.starts_with("GET /v1/conversions ") => ("200 OK", CONVERSIONS_BODY),
            _ => ("404 Not Found", r#"{"error":"not found"}"#),
        }
    }

    const AUTH_BODY: &str = r#"{
        "access_token":"demo-access-token",
        "refresh_token":"demo-refresh-token",
        "user":{
            "id":"user_demo",
            "email":"demo@example.com",
            "display_name":"Demo User",
            "plan_id":"preview"
        }
    }"#;

    const USER_BODY: &str = r#"{
        "id":"user_demo",
        "email":"demo@example.com",
        "display_name":"Demo User",
        "plan_id":"preview"
    }"#;

    const USAGE_BODY: &str = r#"{
        "plan_id":"preview",
        "cloud_conversions_used":2,
        "cloud_conversions_limit":100,
        "count_balance":8,
        "storage_bytes_used":0,
        "storage_bytes_limit":1073741824
    }"#;

    const PLANS_BODY: &str = r#"[{
        "id":"pro",
        "name":"Pro",
        "price_cents":2900,
        "currency":"USD",
        "monthly_conversions":1000,
        "features":["cloud"]
    }]"#;

    const CHECKOUT_BODY: &str = r#"{
        "url":"https://billing.example/checkout",
        "expires_at":"2026-06-24T12:00:00Z"
    }"#;

    const PORTAL_BODY: &str = r#"{
        "url":"https://billing.example/portal",
        "expires_at":"2026-06-24T12:00:00Z"
    }"#;

    const REDEEM_BODY: &str = r#"{
        "redeem_id":"redeem_1",
        "recharge_id":"recharge_1",
        "package_id":"count_100",
        "package_name":"100 Count",
        "recharge_type":"count",
        "quantity":100,
        "count_balance":108,
        "date_valid_until":null,
        "redeemed_at":"2026-06-24T12:00:00Z"
    }"#;

    const RECHARGES_BODY: &str = r#"[{
        "recharge_id":"recharge_1",
        "recharge_type":"count",
        "package_id":"count_100",
        "quantity":100,
        "amount_cents":0,
        "currency":"USD",
        "status":"completed",
        "provider":"redeem-code",
        "provider_trade_id":"redeem_1",
        "created_at":"2026-06-24T12:00:00Z"
    }]"#;

    const REDEEM_RECORDS_BODY: &str = r#"[{
        "redeem_id":"redeem_1",
        "batch_id":"batch_1",
        "batch_no":"BATCH-001",
        "code_preview":"T2D-****-001",
        "package_id":"count_100",
        "package_name":"100 Count",
        "recharge_type":"count",
        "quantity":100,
        "status":"redeemed",
        "redeemed_recharge_id":"recharge_1",
        "redeemed_at":"2026-06-24T12:01:00Z"
    }]"#;

    const CONVERSIONS_BODY: &str = r#"[{
        "job_id":"job_1",
        "upload_id":"upload_1",
        "main_tex":"main.tex",
        "profile":"standard",
        "quality":"balanced",
        "engine":"tectonic",
        "status":"completed",
        "created_at":"2026-06-24T11:00:00Z",
        "updated_at":"2026-06-24T12:00:00Z",
        "docx_ready":true,
        "report_ready":true,
        "error_code":null,
        "error":null
    }]"#;
}
