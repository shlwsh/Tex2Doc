//! HTTP 路由。

use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, get_service, patch, post};
use axum::{extract::Request, Json, Router};
use mime::Mime;
use serde::Deserialize;
use serde_json::json;
use std::io::Cursor;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use doc_core::{convert_zip, ConvertOptions};

use crate::automation_service::{
    AutomationService, RequestFilters,
};
use crate::db_store::{AppUser, ManualOrderRecord};
use crate::error::ApiError;
use crate::error_code::ConversionErrorCode;
use crate::excel_export::write_xml_part;
use crate::feedback_service::{
    AddMessageRequest, AdminReplyRequest, AdminUpdateThreadRequest, CreateThreadRequest,
    FeedbackError, ThreadFilters,
};
use crate::limits::{
    MAX_BODY, MAX_UPLOAD_FILE_BYTES, MAX_UPLOAD_FILE_COUNT, MAX_UPLOAD_UNCOMPRESSED_BYTES,
    MAX_UPLOAD_ZIP_BYTES,
};
use crate::state::{
    ConversionJobRecord, ConversionStatus, RechargeRecord, RedeemCodeBatchRecord, RedeemCodeRecord,
    RedeemCodeResult, RedeemFailure, ServerState, PREVIEW_CLOUD_CONVERSION_LIMIT,
};
use crate::worker_service;

/// 默认主 tex 路径（与 paper3 e2e 一致）。
const DEFAULT_MAIN_TEX: &str = "main-jos.tex";

/// 组装对外 router。
pub async fn router() -> Result<Router, sqlx::Error> {
    Ok(router_with_state(
        worker_service::spawn_worker_state().await?,
    ))
}

/// 组装带状态的 router，供测试或外部嵌入复用。
pub fn router_with_state(state: ServerState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/version", get(version))
        .route("/v1/downloads", get(downloads))
        .route("/api/v1/downloads", get(downloads))
        .route("/v1/waitlist", post(create_waitlist_lead))
        .route("/api/v1/waitlist", post(create_waitlist_lead))
        .route("/api/v1/convert", post(convert))
        .route("/v1/auth/register", post(auth_register))
        .route("/api/v1/auth/register", post(auth_register))
        .route("/v1/auth/login", post(auth_login))
        .route("/api/v1/auth/login", post(auth_login))
        .route("/v1/auth/refresh", post(auth_refresh))
        .route("/api/v1/auth/refresh", post(auth_refresh))
        .route("/v1/me", get(me))
        .route("/api/v1/me", get(me))
        .route("/admin/v1/me", get(admin_me))
        .route("/admin/v1/dashboard", get(admin_dashboard))
        .route("/admin/v1/users", get(admin_list_users))
        .route("/admin/v1/usage-ledger", get(admin_list_usage_ledger))
        .route(
            "/admin/v1/manual-orders",
            get(admin_list_manual_orders).post(admin_create_manual_order),
        )
        .route("/admin/v1/waitlist", get(admin_list_waitlist))
        .route("/v1/usage", get(usage))
        .route("/api/v1/usage", get(usage))
        .route("/v1/plans", get(plans))
        .route("/api/v1/plans", get(plans))
        .route("/v1/recharge/options", get(recharge_options))
        .route("/api/v1/recharge/options", get(recharge_options))
        .route("/v1/recharges", get(list_recharges).post(create_recharge))
        .route(
            "/api/v1/recharges",
            get(list_recharges).post(create_recharge),
        )
        .route("/v1/redeem-codes/options", get(redeem_code_options))
        .route("/api/v1/redeem-codes/options", get(redeem_code_options))
        .route("/v1/redeem-codes/redeem", post(redeem_code))
        .route("/api/v1/redeem-codes/redeem", post(redeem_code))
        .route("/v1/redeem-codes/records", get(list_redeem_records))
        .route("/api/v1/redeem-codes/records", get(list_redeem_records))
        .route(
            "/admin/v1/redeem-code-batches",
            get(admin_list_redeem_batches).post(admin_create_redeem_batch),
        )
        .route(
            "/admin/v1/redeem-code-batches/:id/export.xlsx",
            get(admin_export_redeem_batch),
        )
        // Admin: redeem codes list (上货管理)
        .route(
            "/admin/v1/redeem-codes",
            get(admin_list_redeem_codes).post(admin_bulk_stock_redeem_codes),
        )
        .route(
            "/admin/v1/redeem-codes/export.xlsx",
            get(admin_export_redeem_codes),
        )
        .route(
            "/admin/v1/redeem-codes/restock",
            post(admin_restock_redeem_codes),
        )
        .route(
            "/admin/v1/redeem-code-batches/:id",
            get(admin_get_redeem_batch),
        )
        .route("/v1/billing/checkout", post(billing_checkout))
        .route("/api/v1/billing/checkout", post(billing_checkout))
        .route("/v1/billing/portal", post(billing_portal))
        .route("/api/v1/billing/portal", post(billing_portal))
        .route("/v1/uploads", post(upload_project))
        .route("/api/v1/uploads", post(upload_project))
        .route(
            "/v1/conversions",
            get(list_conversions).post(create_conversion),
        )
        .route("/v1/local-conversions/check", post(check_local_conversion))
        .route("/api/v1/local-conversions/check", post(check_local_conversion))
        .route("/v1/local-conversions/consume", post(consume_local_conversion))
        .route("/api/v1/local-conversions/consume", post(consume_local_conversion))
        .route(
            "/api/v1/conversions",
            get(list_conversions).post(create_conversion),
        )
        .route("/v1/conversions/:id", get(get_conversion))
        .route("/api/v1/conversions/:id", get(get_conversion))
        .route(
            "/v1/conversions/:id/download/docx",
            get(download_conversion_docx),
        )
        .route(
            "/api/v1/conversions/:id/download/docx",
            get(download_conversion_docx),
        )
        .route("/v1/conversions/:id/report", get(get_conversion_report))
        .route("/api/v1/conversions/:id/report", get(get_conversion_report))
        // QualityRun 多维质量报告（对应技术方案第 1.5 节）
        .route("/v1/conversions/:id/quality-report", get(get_quality_report_json))
        .route(
            "/api/v1/conversions/:id/quality-report",
            get(get_quality_report_json),
        )
        // Conversion file download (enhanced)
        .route(
            "/v1/conversions/:id/download/zip",
            get(download_conversion_zip),
        )
        .route(
            "/v1/conversions/:id/download/log",
            get(download_conversion_log),
        )
        // Feedback routes (user)
        .route(
            "/v1/feedback/threads",
            get(list_feedback_threads).post(create_feedback_thread),
        )
        .route("/v1/feedback/threads/:id", get(get_feedback_thread))
        .route(
            "/v1/feedback/threads/:id/messages",
            post(add_feedback_message),
        )
        // Admin feedback routes
        .route(
            "/admin/v1/feedback/threads",
            get(admin_list_feedback_threads),
        )
        .route(
            "/admin/v1/feedback/threads/export.xlsx",
            get(admin_export_feedback_threads),
        )
        .route(
            "/admin/v1/feedback/threads/:id",
            patch(admin_update_feedback_thread),
        )
        .route(
            "/admin/v1/feedback/threads/:id/messages",
            post(admin_reply_feedback_message),
        )
        .route("/v1/releases/:channel", get(release_manifest))
        .route("/api/v1/releases/:channel", get(release_manifest))
        .route(
            "/admin/v1/releases",
            get(admin_list_releases).post(admin_publish_release),
        )
        .route(
            "/admin/v1/releases/:id/rollback",
            post(admin_rollback_release),
        )
        .route("/admin/v1/release-audit", get(admin_release_audit))
        // Automation R&D routes
        .route("/admin/v1/automation/summary", get(automation_summary))
        .route("/admin/v1/automation/requests", get(automation_list_requests))
        .route(
            "/admin/v1/automation/requests/:id",
            get(automation_get_request),
        )
        .route(
            "/admin/v1/automation/requests/:id/events",
            get(automation_get_events),
        )
        .route(
            "/admin/v1/automation/requests/:id/approve",
            post(automation_approve),
        )
        .route(
            "/admin/v1/automation/requests/:id/reject",
            post(automation_reject),
        )
        .route(
            "/admin/v1/automation/requests/:id/retry",
            post(automation_retry),
        )
        .route(
            "/admin/v1/automation/requests/:id/escalate",
            post(automation_escalate),
        )
        .route("/admin/v1/automation/agents", get(automation_list_agents))
        .route(
            "/admin/v1/automation/agents/:id/pause",
            post(automation_pause_agent),
        )
        .route(
            "/admin/v1/automation/agents/:id/resume",
            post(automation_resume_agent),
        )
        .merge(static_router())
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(tower_http::limit::RequestBodyLimitLayer::new(MAX_BODY))
}

fn static_router() -> Router<ServerState> {
    let static_root = std::env::var("TEX2DOC_STATIC_DIR")
        .unwrap_or_else(|_| "apps/rust-service/static".to_string());
    let home_dir = format!("{static_root}/home");
    let user_dir = format!("{static_root}/user");
    let admin_dir = format!("{static_root}/admin");

    Router::new()
        .route_service(
            "/",
            get_service(ServeFile::new(format!("{home_dir}/index.html"))),
        )
        .nest_service(
            "/app",
            ServeDir::new(&user_dir)
                .not_found_service(ServeFile::new(format!("{user_dir}/index.html"))),
        )
        .nest_service(
            "/admin",
            ServeDir::new(&admin_dir)
                .not_found_service(ServeFile::new(format!("{admin_dir}/index.html"))),
        )
        .nest_service("/assets", ServeDir::new(format!("{static_root}/assets")))
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

async fn version() -> Json<serde_json::Value> {
    Json(json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn downloads() -> Json<serde_json::Value> {
    Json(json!({
        "channel": "preview",
        "platforms": [
            {
                "platform": "windows",
                "arch": "x64",
                "version": env!("CARGO_PKG_VERSION"),
                "download_url": "https://releases.tex2doc.cn/desktop/preview/Tex2Doc-preview-windows-x64.exe",
                "sha256": "pending-preview-build",
                "signature": "",
                "release_notes": "邀请制 Beta 阶段预览安装包。正式签名安装包由 release 管理接口发布后替换。",
                "known_limits": [
                    "当前为 Preview 交付包",
                    "建议配合示例项目和诊断包使用",
                    "如遇转换失败请通过反馈模块提交 job id 和诊断信息"
                ]
            }
        ]
    }))
}

async fn create_waitlist_lead(
    State(state): State<ServerState>,
    Json(payload): Json<WaitlistBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !payload.email.contains('@') {
        return Err(ApiError::BadRequest {
            code: "invalid_email",
            message: "waitlist email is invalid".to_string(),
        });
    }
    let lead = state
        .create_waitlist_lead(
            payload.email,
            payload.identity,
            payload.paper_type,
            payload.current_tool,
            payload.pain_point,
            payload.paid_intent,
        )
        .await
        .map_err(db_error)?;
    Ok(Json(lead))
}

#[derive(Debug, Deserialize)]
struct AuthRequest {
    email: Option<String>,
    display_name: Option<String>,
    password: Option<String>,
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WaitlistBody {
    email: String,
    identity: Option<String>,
    paper_type: Option<String>,
    current_tool: Option<String>,
    pain_point: Option<String>,
    paid_intent: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CheckoutBody {
    plan_id: Option<String>,
    success_url: Option<String>,
    cancel_url: Option<String>,
    return_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConversionBody {
    upload_id: Option<String>,
    main_tex: Option<String>,
    profile: Option<String>,
    quality: Option<String>,
    engine: Option<String>,
    backend: Option<String>,
    /// Idempotency key for request deduplication
    idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RechargeBody {
    recharge_type: Option<String>,
    package_id: Option<String>,
    quantity: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AdminManualOrderBody {
    user_id: String,
    recharge_type: Option<String>,
    package_id: String,
    quantity: Option<u64>,
    amount_cents: Option<u64>,
    payment_note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PublishReleaseBody {
    channel: String,
    platform: String,
    arch: Option<String>,
    version: String,
    download_url: String,
    sha256: String,
    signature: Option<String>,
    file_size_bytes: Option<u64>,
    release_title: Option<String>,
    release_notes: Option<String>,
    strategy_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RollbackReleaseBody {
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RedeemCodeBody {
    code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdminRedeemBatchBody {
    package_id: Option<String>,
    quantity: Option<u64>,
    channel: Option<String>,
    note: Option<String>,
    expires_at: Option<String>,
    /// When true, anonymous callers (no Authorization header) can redeem a
    /// code from this batch and have an account provisioned automatically.
    /// Defaults to false to preserve existing behavior.
    auto_provision: Option<bool>,
}

/// Request body for the admin redeem-code list endpoint.
#[derive(Debug, Deserialize)]
struct AdminRedeemCodeListQuery {
    stock_status: Option<String>,
    batch_id: Option<String>,
    package_id: Option<String>,
    search: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
}

/// Request body for bulk stocking redeem codes.
#[derive(Debug, Deserialize)]
struct AdminRedeemCodeStockBody {
    code_ids: Vec<String>,
}

/// Request body for restocking (resetting) redeem codes by plaintext codes.
#[derive(Debug, Deserialize)]
struct AdminRedeemCodeRestockBody {
    /// Plain-text redeem codes, one per line (may include dashes / whitespace).
    codes: String,
}

async fn auth_register(
    State(state): State<ServerState>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let email = require_non_empty(payload.email, "email")?;
    let password = require_non_empty(payload.password, "password")?;
    let user = state
        .register_user(&email, payload.display_name.as_deref(), &password)
        .await
        .map_err(|error| {
            if is_unique_violation(&error) {
                ApiError::Conflict(format!("user already exists: {email}"))
            } else {
                db_error(error)
            }
        })?;
    issue_auth_response(&state, &user).await
}

async fn auth_login(
    State(state): State<ServerState>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let email = require_non_empty(payload.email, "email")?;
    let password = require_non_empty(payload.password, "password")?;
    let user = state
        .login_user(&email, &password)
        .await
        .map_err(db_error)?
        .ok_or_else(|| ApiError::Unauthorized("invalid email or password".to_string()))?;
    issue_auth_response(&state, &user).await
}

async fn auth_refresh(
    State(state): State<ServerState>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let suffix = require_non_empty(payload.refresh_token, "refresh_token")?;
    let user = state
        .user_for_refresh_token(&suffix)
        .await
        .map_err(db_error)?
        .ok_or_else(|| ApiError::Unauthorized("invalid refresh token".to_string()))?;
    let access_token = state
        .issue_token(&user.id, "access")
        .await
        .map_err(db_error)?;
    let refresh_token = state
        .issue_token(&user.id, "refresh")
        .await
        .map_err(db_error)?;
    Ok(Json(json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "user": app_user_json(&user),
    })))
}

async fn me(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    Ok(Json(app_user_json(&session)))
}

async fn admin_me(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = require_admin_session(&state, &headers).await?;
    Ok(Json(json!({
        "user": app_user_json(&admin),
        "permissions": admin_permissions(&admin.role),
    })))
}

async fn admin_dashboard(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = require_admin_session(&state, &headers).await?;
    let plans = state.list_billing_plans().await.map_err(db_error)?;
    let redeem_batches = state.list_redeem_batches().await;
    let feedback_threads = state
        .feedback_store()
        .admin_list(&ThreadFilters::default())
        .await;
    let open_feedback = feedback_threads
        .iter()
        .filter(|thread| thread.status == "open" || thread.status == "in_progress")
        .count();

    Ok(Json(json!({
        "admin": app_user_json(&admin),
        "counts": {
            "billing_plans": plans.len(),
            "redeem_batches": redeem_batches.len(),
            "feedback_threads": feedback_threads.len(),
            "open_feedback": open_feedback,
        },
        "release_channels": ["stable", "beta"],
        "modules": [
            "dashboard",
            "redeem_codes",
            "feedback",
            "releases",
            "audit",
            "billing",
            "conversions"
        ],
        "generated_at": chrono::Utc::now().to_rfc3339(),
    })))
}

async fn admin_list_users(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _admin = require_admin_session(&state, &headers).await?;
    let users = state.list_users().await.map_err(db_error)?;
    Ok(Json(json!({ "users": users })))
}

async fn admin_list_usage_ledger(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _admin = require_admin_session(&state, &headers).await?;
    let events = state.list_usage_ledger().await.map_err(db_error)?;
    Ok(Json(json!({ "events": events })))
}

async fn admin_create_manual_order(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<AdminManualOrderBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = require_admin_session(&state, &headers).await?;
    let recharge_type = payload.recharge_type.unwrap_or_else(|| "count".to_string());
    let (normalized_type, quantity, default_amount_cents) =
        compute_recharge_amount(&recharge_type, &payload.package_id, payload.quantity)?;
    let order = state
        .create_manual_order(
            payload.user_id,
            normalized_type,
            payload.package_id,
            quantity,
            payload.amount_cents.unwrap_or(default_amount_cents),
            admin.id,
            payload.payment_note,
        )
        .await
        .map_err(ApiError::Io)?;
    Ok(Json(manual_order_json(&order)))
}

async fn admin_list_manual_orders(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _admin = require_admin_session(&state, &headers).await?;
    let orders = state.list_manual_orders().await.map_err(db_error)?;
    Ok(Json(json!(orders
        .iter()
        .map(manual_order_json)
        .collect::<Vec<_>>())))
}

async fn admin_list_waitlist(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _admin = require_admin_session(&state, &headers).await?;
    let leads = state.list_waitlist_leads().await.map_err(db_error)?;
    Ok(Json(json!({ "leads": leads })))
}

async fn usage(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let used = state.cloud_conversions_used(&session.id).await;
    let entitlement = state.entitlement(&session.id).await;
    let date_valid = entitlement
        .valid_until
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|valid_until| {
            valid_until >= crate::state::now_timestamp().parse().unwrap_or_default()
        });
    Ok(Json(json!({
        "plan_id": if date_valid {
            "date"
        } else if entitlement.count_balance > 0 {
            "count"
        } else {
            "preview"
        },
        "cloud_conversions_used": used,
        "cloud_conversions_limit": PREVIEW_CLOUD_CONVERSION_LIMIT,
        "count_balance": entitlement.count_balance,
        "date_valid_until": entitlement.valid_until,
        "entitlement_source_order_id": entitlement.source_order_id,
        "storage_bytes_used": 0,
        "storage_bytes_limit": 1_073_741_824_u64,
        "period_start": "2026-06-01T00:00:00Z",
        "period_end": "2026-07-01T00:00:00Z",
    })))
}

async fn plans(State(state): State<ServerState>) -> Result<Json<serde_json::Value>, ApiError> {
    let plans = state.list_billing_plans().await.map_err(db_error)?;
    Ok(Json(json!(plans
        .iter()
        .map(|plan| json!({
            "id": plan.id,
            "name": plan.name,
            "price_cents": plan.price_cents,
            "currency": plan.currency,
            "monthly_conversions": plan.monthly_conversions,
            "storage_bytes": plan.storage_bytes,
            "features": plan.features,
        }))
        .collect::<Vec<_>>())))
}

async fn recharge_options(
    State(state): State<ServerState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let packages = state.list_redeem_packages().await.map_err(db_error)?;
    let count_packages = packages
        .iter()
        .filter(|package| package.package_type == "count")
        .map(|package| {
            json!({
                "id": package.id,
                "name": package.name,
                "quantity": package.quantity,
                "amount_cents": package.quantity * 100,
            })
        })
        .collect::<Vec<_>>();
    let date_packages = packages
        .iter()
        .filter(|package| package.package_type == "date")
        .map(|package| {
            json!({
                "id": package.id,
                "name": package.name,
                "days": package.quantity,
                "amount_cents": package.quantity * 100,
            })
        })
        .collect::<Vec<_>>();
    Ok(Json(json!({
        "currency": "CNY",
        "provider": "manual-order",
        "phase": "A",
        "support_text": "邀请制 Beta 阶段通过兑换码或人工订单开通权益；Stripe/支付沙箱暂未启用。",
        "count": {
            "unit_price_cents": 100,
            "minimum_quantity": 3,
            "packages": count_packages
        },
        "date": {
            "packages": date_packages
        }
    })))
}

async fn redeem_code_options(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _session = require_session(&state, &headers).await?;
    let packages = state.list_redeem_packages().await.map_err(db_error)?;
    Ok(Json(json!({
        "enabled": true,
        "provider": "redeem-code",
        "code_format_hint": "T2D-XXXX-XXXX-XXXX-XX",
        "support_text": "请输入购买或活动获得的兑换码",
        "packages": packages.iter().map(|package| json!({
            "id": package.id,
            "name": package.name,
            "recharge_type": package.package_type,
            "quantity": package.quantity,
        })).collect::<Vec<_>>(),
    })))
}

async fn redeem_code(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<RedeemCodeBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let code = payload.code.unwrap_or_default();

    // Try the existing-session path first if a bearer token is supplied.
    // This keeps already-signed-in users on their existing account, with no
    // chance of accidentally provisioning a new one.
    if let Some(token) = bearer_token_from_headers(&headers) {
        if let Ok(Some(user)) = state.user_for_token(&token).await {
            let result = state
                .redeem_code(user.id.clone(), code.clone())
                .await
                .map_err(redeem_failure_to_error)?;
            return Ok(Json(redeem_result_json(&result)));
        }
        // Invalid/expired token: fall through to anonymous path so callers
        // aren't penalized for a stale credential. The anonymous path will
        // refuse with REDEEM_REQUIRES_LOGIN if the batch disallows it.
    }

    // Anonymous path — auto-provisions an account when the batch allows it.
    let result = state
        .redeem_code_anonymous(code)
        .await
        .map_err(|failure| match failure {
            // Map auto_provision=false onto a stable client-friendly code so
            // older clients can prompt the user to sign in first.
            RedeemFailure::InvalidCode => ApiError::Coded {
                status: StatusCode::UNAUTHORIZED,
                code: "redeem_requires_login",
                message: "this redeem code requires sign-in; please log in and try again"
                    .to_string(),
            },
            other => redeem_failure_to_error(other),
        })?;
    Ok(Json(redeem_result_json(&result)))
}

/// Extract the bearer token from the `Authorization` header, if present.
fn bearer_token_from_headers(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

async fn list_redeem_records(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let records = state.list_redeem_records(&session.id).await;
    Ok(Json(json!(records
        .iter()
        .map(redeem_record_json)
        .collect::<Vec<_>>())))
}

async fn admin_create_redeem_batch(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<AdminRedeemBatchBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = require_admin_session(&state, &headers).await?;
    let package_id = payload.package_id.unwrap_or_else(|| "count_10".to_string());
    let quantity = payload.quantity.unwrap_or(100);
    let batch = state
        .create_redeem_batch(
            &package_id,
            quantity,
            payload.channel,
            payload.note,
            payload.expires_at,
            admin.id,
            payload.auto_provision.unwrap_or(false),
        )
        .await
        .map_err(redeem_failure_to_error)?;
    Ok(Json(redeem_batch_json(&batch, true)))
}

async fn admin_list_redeem_batches(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _admin = require_admin_session(&state, &headers).await?;
    let batches = state.list_redeem_batches().await;
    Ok(Json(json!(batches
        .iter()
        .map(|batch| redeem_batch_json(batch, false))
        .collect::<Vec<_>>())))
}

async fn admin_get_redeem_batch(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _admin = require_admin_session(&state, &headers).await?;
    let batch = state
        .get_redeem_batch_detail(&id)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("redeem batch {id}")))?;
    Ok(Json(redeem_batch_json(&batch, true)))
}

async fn admin_export_redeem_batch(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let _admin = require_admin_session(&state, &headers).await?;
    let batch = state
        .get_redeem_batch(&id)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("redeem batch {id}")))?;
    let body = build_redeem_codes_xlsx(&batch)?;
    state
        .mark_redeem_batch_exported(&id)
        .await
        .map_err(ApiError::Io)?;
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        )
        .header(
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"redeem-codes-{}.xlsx\"",
                batch.batch_no
            ),
        )
        .header(header::CONTENT_LENGTH, body.len())
        .body(axum::body::Body::from(body))
        .map_err(|e| ApiError::Io(e.to_string()))
}

// ─── Admin: redeem-codes (上货列表) ─────────────────────────────────────────

fn redeem_code_record_json(record: &RedeemCodeRecord) -> serde_json::Value {
    json!({
        "code_id": record.code_id,
        "batch_id": record.batch_id,
        "batch_no": record.batch_no,
        "code_preview": record.code_preview,
        "package_id": record.package_id,
        "package_name": record.package_name,
        "recharge_type": record.recharge_type,
        "quantity": record.quantity,
        "status": record.status,
        "stock_status": record.stock_status,
        "stocked_by": record.stocked_by,
        "stocked_at": record.stocked_at,
        "redeemed_by": record.redeemed_by,
        "redeemed_recharge_id": record.redeemed_recharge_id,
        "redeemed_at": record.redeemed_at,
        "restocked_by": record.restocked_by,
        "restocked_at": record.restocked_at,
        "expires_at": record.expires_at,
        "created_at": record.created_at,
    })
}

async fn admin_list_redeem_codes(
    State(state): State<ServerState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<AdminRedeemCodeListQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _admin = require_admin_session(&state, &headers).await?;
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * page_size;

    let records = state
        .admin_list_redeem_codes(
            params.stock_status.as_deref(),
            params.batch_id.as_deref(),
            params.package_id.as_deref(),
            params.search.as_deref(),
            Some(page_size),
            Some(offset),
        )
        .await;

    let total = state
        .admin_count_redeem_codes(
            params.stock_status.as_deref(),
            params.batch_id.as_deref(),
            params.package_id.as_deref(),
            params.search.as_deref(),
        )
        .await;

    Ok(Json(json!({
        "records": records.iter().map(redeem_code_record_json).collect::<Vec<_>>(),
        "total": total,
        "page": page,
        "page_size": page_size,
    })))
}

async fn admin_bulk_stock_redeem_codes(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<AdminRedeemCodeStockBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = require_admin_session(&state, &headers).await?;
    let affected = state
        .admin_stock_redeem_codes(&admin.id, &payload.code_ids)
        .await
        .map_err(redeem_failure_to_error)?;
    Ok(Json(json!({ "affected": affected })))
}

async fn admin_restock_redeem_codes(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<AdminRedeemCodeRestockBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = require_admin_session(&state, &headers).await?;
    let codes: Vec<String> = payload
        .codes
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    let affected = state
        .admin_restock_redeem_codes(&admin.id, &codes)
        .await
        .map_err(redeem_failure_to_error)?;
    Ok(Json(json!({ "affected": affected })))
}

async fn admin_export_redeem_codes(
    State(state): State<ServerState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<AdminRedeemCodeListQuery>,
) -> Result<Response, ApiError> {
    let _admin = require_admin_session(&state, &headers).await?;
    let records = state
        .admin_list_redeem_codes(
            params.stock_status.as_deref(),
            params.batch_id.as_deref(),
            params.package_id.as_deref(),
            params.search.as_deref(),
            Some(10000),
            Some(0),
        )
        .await;
    let body = build_redeem_codes_list_xlsx(&records);
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        )
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"redeem-codes-list.xlsx\"",
        )
        .header(header::CONTENT_LENGTH, body.len())
        .body(axum::body::Body::from(body))
        .map_err(|e| ApiError::Io(e.to_string()))
}

fn build_redeem_codes_list_xlsx(records: &[RedeemCodeRecord]) -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        write_xml_part(&mut zip, opts, "[Content_Types].xml", content_types_xml());
        write_xml_part(&mut zip, opts, "_rels/.rels", rels_xml());
        write_xml_part(&mut zip, opts, "xl/workbook.xml", workbook_xml_codes_list());
        write_xml_part(
            &mut zip,
            opts,
            "xl/_rels/workbook.xml.rels",
            workbook_rels_xml_codes_list(),
        );
        write_xml_part(
            &mut zip,
            opts,
            "xl/worksheets/sheet1.xml",
            redeem_codes_list_sheet_xml(records),
        );
        zip.finish().expect("zip finish should not fail");
    }
    cursor.into_inner()
}

fn workbook_xml_codes_list() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets><sheet name="兑换码列表" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#
}

fn workbook_rels_xml_codes_list() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#
}

fn redeem_codes_list_sheet_xml(records: &[RedeemCodeRecord]) -> String {
    let headers = [
        "批次号",
        "兑换码预览",
        "套餐 ID",
        "套餐名称",
        "转换次数",
        "上货状态",
        "上货时间",
        "使用时间",
        "重置时间",
        "过期时间",
        "创建时间",
    ];

    let mut sheet = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    sheet.push_str(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    );
    sheet.push_str("<sheetData>");
    let mut header_row = String::from(r#"<row r="1">"#);
    for (col, h) in headers.iter().enumerate() {
        let col_letter = (b'A' + col as u8) as char;
        header_row.push_str(&format!(
            r#"<c r="{}{}" t="inlineStr"><is><t>{}</t></is></c>"#,
            col_letter,
            1,
            xml_escape(h)
        ));
    }
    header_row.push_str("</row>");
    sheet.push_str(&header_row);

    for (idx, r) in records.iter().enumerate() {
        let row_num = (idx + 2) as u32;
        let mut row = format!(r#"<row r="{row_num}">"#);
        let stock_label = match r.stock_status.as_str() {
            "new" => "new（新建）",
            "stocked" => "stocked（已上货）",
            "redeemed" => "redeemed（已使用）",
            "restocked" => "restocked（已恢复）",
            _ => r.stock_status.as_str(),
        };
        let vals = [
            r.batch_no.as_str(),
            r.code_preview.as_str(),
            r.package_id.as_str(),
            r.package_name.as_str(),
            &r.quantity.to_string(),
            stock_label,
            r.stocked_at.as_deref().unwrap_or("-"),
            r.redeemed_at.as_deref().unwrap_or("-"),
            r.restocked_at.as_deref().unwrap_or("-"),
            r.expires_at.as_deref().unwrap_or("-"),
            r.created_at.as_str(),
        ];
        for (col, val) in vals.iter().enumerate() {
            let col_letter = (b'A' + col as u8) as char;
            row.push_str(&format!(
                r#"<c r="{}{}" t="inlineStr"><is><t>{}</t></is></c>"#,
                col_letter,
                row_num,
                xml_escape(val)
            ));
        }
        row.push_str("</row>");
        sheet.push_str(&row);
    }
    sheet.push_str("</sheetData></worksheet>");
    sheet
}

async fn create_recharge(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<RechargeBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let recharge_type = payload.recharge_type.unwrap_or_else(|| "count".to_string());
    let package_id = payload.package_id.unwrap_or_else(|| "count_3".to_string());
    let (normalized_type, quantity, amount_cents) =
        compute_recharge_amount(&recharge_type, &package_id, payload.quantity)?;
    let record = state
        .create_recharge(
            session.id,
            normalized_type,
            package_id,
            quantity,
            amount_cents,
        )
        .await
        .map_err(ApiError::Io)?;
    Ok(Json(recharge_json(&record)))
}

async fn list_recharges(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let records = state.list_recharges(&session.id).await;
    Ok(Json(json!(records
        .iter()
        .map(recharge_json)
        .collect::<Vec<_>>())))
}

async fn billing_checkout(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<CheckoutBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let plan_id = payload.plan_id.unwrap_or_else(|| "pro".to_string());
    let success_url = payload
        .success_url
        .unwrap_or_else(|| "https://tex2doc.cn/success".to_string());
    let cancel_url = payload
        .cancel_url
        .unwrap_or_else(|| "https://tex2doc.cn/cancel".to_string());
    Ok(Json(json!({
        "provider": "manual-order",
        "phase": "A",
        "status": "manual_required",
        "plan_id": plan_id,
        "user_id": session.id,
        "success_url": success_url,
        "cancel_url": cancel_url,
        "message": "当前 Beta 阶段请联系运营创建人工订单，或使用兑换码开通权益。",
    })))
}

async fn billing_portal(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<CheckoutBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _session = require_session(&state, &headers).await?;
    let return_url = payload
        .return_url
        .unwrap_or_else(|| "https://tex2doc.cn/account".to_string());
    Ok(Json(json!({
        "provider": "manual-order",
        "phase": "A",
        "return_url": return_url,
        "message": "当前仅支持兑换码和人工订单，第三方支付门户暂未启用。",
    })))
}

async fn upload_project(
    State(state): State<ServerState>,
    request: Request,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (parts, body) = request.into_parts();
    let session = require_session(&state, &parts.headers).await?;
    let full_body = axum::body::to_bytes(body, MAX_BODY)
        .await
        .map_err(|e| ApiError::Io(format!("body read error: {e}")))?;
    let file_part =
        extract_multipart_field(&full_body, "file")?.ok_or(ApiError::MissingField("file"))?;
    if file_part.is_empty() {
        return Err(ApiError::MissingField("file"));
    }
    validate_project_zip(&file_part)?;
    let record = state
        .store_upload(&session.id, "project.zip".to_string(), file_part)
        .await
        .map_err(ApiError::Io)?;
    Ok(Json(json!({
        "upload_id": record.upload_id,
        "status": "stored",
        "bytes": record.bytes_size,
        "file_name": record.file_name,
        "created_at": record.created_at,
    })))
}

async fn create_conversion(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<ConversionBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let upload_id = payload
        .upload_id
        .unwrap_or_else(|| "upload_demo".to_string());
    if state.get_upload(&upload_id).await.is_none() {
        return Err(ApiError::NotFound(format!("upload {upload_id}")));
    }
    let profile = payload
        .profile
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "auto".to_string());
    let quality = payload
        .quality
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "standard".to_string());
    let engine = payload
        .engine
        .or(payload.backend)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "semantic-engine".to_string());
    let main_tex = payload
        .main_tex
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MAIN_TEX.to_string());

    // Check for existing job with same idempotency key (if provided)
    if let Some(ref key) = payload.idempotency_key {
        if let Some(existing) = state.find_job_by_idempotency_key(key).await {
            // Return existing job if it's not failed
            if existing.status != ConversionStatus::Failed {
                return Ok(Json(serde_json::json!({
                    "job_id": existing.job_id,
                    "status": existing.status,
                    "idempotent": true,
                    "message": "Returning existing job with same idempotency key"
                })));
            }
        }
    }

    let job = state
        .create_job(
            session.id.clone(),
            upload_id,
            main_tex,
            profile,
            quality,
            engine,
            payload.idempotency_key,
        )
        .await
        .map_err(ApiError::Io)?;
    if let Err(used) = state
        .reserve_cloud_conversion(&session.id, &job.job_id)
        .await
    {
        state
            .fail_job(
                &job.job_id,
                ConversionErrorCode::QuotaExhausted,
                format!(
                    "cloud conversion entitlement exhausted: preview_used={used}, preview_limit={PREVIEW_CLOUD_CONVERSION_LIMIT}"
                ),
            )
            .await;
        return Err(ApiError::PaymentRequired(format!(
            "cloud conversion entitlement exhausted: preview_used={used}, preview_limit={PREVIEW_CLOUD_CONVERSION_LIMIT}"
        )));
    }
    state
        .enqueue_job(job.job_id.clone())
        .await
        .map_err(ApiError::Io)?;
    Ok(Json(job_json(&job)))
}

async fn check_local_conversion(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let entitlement = state.entitlement(&session.id).await;
    let valid_until_active = entitlement
        .valid_until
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|valid_until| {
            valid_until >= crate::state::now_timestamp().parse().unwrap_or_default()
        });
    let used = state.cloud_conversions_used(&session.id).await;
    let allowed = valid_until_active || entitlement.count_balance > 0 || used < PREVIEW_CLOUD_CONVERSION_LIMIT;
    Ok(Json(json!({
        "allowed": allowed,
        "valid_until_active": valid_until_active,
        "count_balance": entitlement.count_balance,
        "used": used,
        "limit": PREVIEW_CLOUD_CONVERSION_LIMIT,
    })))
}

async fn consume_local_conversion(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    match state.consume_local_conversion(&session.id).await {
        Ok(_used) => {
            let entitlement = state.entitlement(&session.id).await;
            Ok(Json(json!({
                "consumed": true,
                "balance": entitlement.count_balance,
            })))
        }
        Err(_used) => Err(ApiError::PaymentRequired(
            "local conversion entitlement exhausted".to_string(),
        )),
    }
}

async fn list_conversions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let jobs = state.list_jobs_by_user(&session.id).await;
    Ok(Json(json!(jobs.iter().map(job_json).collect::<Vec<_>>())))
}

async fn get_conversion(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let job = state
        .get_job(&id)
        .await
        .filter(|job| job.user_id == session.id)
        .ok_or_else(|| ApiError::NotFound(format!("conversion {id}")))?;
    Ok(Json(job_json(&job)))
}

async fn download_conversion_docx(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let session = require_session(&state, &headers).await?;
    let job = state
        .get_job(&id)
        .await
        .filter(|job| job.user_id == session.id)
        .ok_or_else(|| ApiError::NotFound(format!("conversion {id}")))?;
    let key = job
        .result_docx_key
        .as_deref()
        .ok_or_else(|| ApiError::Conflict(format!("conversion {id} docx is not ready")))?;
    let body = state
        .load_storage_key(key)
        .ok_or_else(|| ApiError::NotFound(format!("conversion {id} docx file")))?;
    let mime: Mime = "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        .parse()
        .expect("static mime is valid");
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{id}.docx\""),
        )
        .header(header::CONTENT_LENGTH, body.len())
        .body(axum::body::Body::from(body))
        .map_err(|e| ApiError::Io(e.to_string()))
}

async fn get_conversion_report(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let job = state
        .get_job(&id)
        .await
        .filter(|job| job.user_id == session.id)
        .ok_or_else(|| ApiError::NotFound(format!("conversion {id}")))?;
    let report = job
        .report
        .ok_or_else(|| ApiError::Conflict(format!("conversion {id} report is not ready")))?;
    Ok(Json(
        serde_json::to_value(report).map_err(|e| ApiError::Io(e.to_string()))?,
    ))
}

/// 返回完整的 QualityRun JSON（多维评分报告）。
///
/// 对应技术方案第 1.5 节"服务报告 API 增强"：
/// - GET /api/v1/conversions/:id/quality-report
async fn get_quality_report_json(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let job = state
        .get_job(&id)
        .await
        .filter(|job| job.user_id == session.id)
        .ok_or_else(|| ApiError::NotFound(format!("conversion {id}")))?;

    let report = job
        .report
        .as_ref()
        .ok_or_else(|| ApiError::Conflict(format!("conversion {id} report is not ready")))?;

    // 优先返回完整的 QualityRun JSON
    if let Some(quality_run_json) = &report.quality_run_json {
        let parsed: serde_json::Value =
            serde_json::from_str(quality_run_json).map_err(|e| ApiError::Io(e.to_string()))?;
        return Ok(Json(parsed));
    }

    // 降级：返回包含 dimension_scores 的结构化报告
    let response = serde_json::json!({
        "job_id": &report.job_id,
        "status": report.status.as_str(),
        "quality_score": report.quality_score,
        "profile": &report.profile,
        "executor": &report.executor,
        "backend": &report.backend,
        "quality_status": &report.quality_status,
        "dimension_scores": report.dimension_scores,
        "warnings": &report.warnings,
        "message": &report.message,
    });
    Ok(Json(response))
}

async fn release_manifest(
    State(state): State<ServerState>,
    Path(channel): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if let Some(manifest) = state
        .latest_release_manifest(&channel)
        .await
        .map_err(db_error)?
    {
        return Ok(Json(manifest));
    }
    Ok(Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "channel": channel,
        "platform": "windows",
        "arch": "x64",
        "download_url": "https://releases.tex2doc.cn/desktop/latest",
        "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "signature": "pending-p9-signature",
        "signature_algorithm": "sha256",
        "file_size_bytes": 0,
        "release_title": "Tex2Doc Preview",
        "release_notes": "P9 preview manifest. Installers and real signatures are pending platform release builds.",
        "strategy": {
            "type": "optional",
            "block_if_outdated": false,
            "rollout_percentage": 100,
            "prompt_title": "Tex2Doc Preview",
            "prompt_message": "预览版更新信息，正式安装包发布后会由后台 manifest 替换。",
            "prompt_dismissable": true
        }
    })))
}

async fn admin_list_releases(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _admin = require_admin_session(&state, &headers).await?;
    let releases = state.list_release_manifests().await.map_err(db_error)?;
    Ok(Json(json!({ "releases": releases })))
}

async fn admin_publish_release(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<PublishReleaseBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = require_admin_session(&state, &headers).await?;
    if payload.sha256.len() != 64 && payload.sha256 != "pending-preview-build" {
        return Err(ApiError::BadRequest {
            code: "invalid_sha256",
            message: "sha256 must be a 64-character hex digest".to_string(),
        });
    }
    let release = state
        .publish_release(
            &admin.email,
            payload.channel,
            payload.platform,
            payload.arch.unwrap_or_else(|| "x64".to_string()),
            payload.version,
            payload.download_url,
            payload.sha256,
            payload.signature,
            payload.file_size_bytes.unwrap_or_default(),
            payload.release_title,
            payload.release_notes,
            payload.strategy_type,
        )
        .await
        .map_err(db_error)?;
    Ok(Json(release))
}

async fn admin_rollback_release(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<RollbackReleaseBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = require_admin_session(&state, &headers).await?;
    let release = state
        .rollback_release(&admin.email, &id, payload.reason)
        .await
        .map_err(db_error)?;
    Ok(Json(release))
}

async fn admin_release_audit(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _admin = require_admin_session(&state, &headers).await?;
    let logs = state.list_release_audit().await.map_err(db_error)?;
    Ok(Json(json!({ "logs": logs })))
}

async fn issue_auth_response(
    state: &ServerState,
    user: &AppUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let access_token = state
        .issue_token(&user.id, "access")
        .await
        .map_err(db_error)?;
    let refresh_token = state
        .issue_token(&user.id, "refresh")
        .await
        .map_err(db_error)?;
    Ok(Json(json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "user": app_user_json(user),
    })))
}

fn require_non_empty(value: Option<String>, field: &'static str) -> Result<String, ApiError> {
    let value = value.unwrap_or_default();
    if value.trim().is_empty() {
        return Err(ApiError::BadRequest {
            code: "missing_field",
            message: format!("{field} is required"),
        });
    }
    Ok(value.trim().to_string())
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(|db_error| db_error.code().as_deref() == Some("23505"))
}

async fn require_session(state: &ServerState, headers: &HeaderMap) -> Result<AppUser, ApiError> {
    let value = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| ApiError::Unauthorized("missing bearer token".to_string()))?
        .to_str()
        .map_err(|_| ApiError::Unauthorized("invalid authorization header".to_string()))?;
    let token = value
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::Unauthorized("expected Bearer token".to_string()))?
        .trim();
    if token.is_empty() {
        return Err(ApiError::Unauthorized("empty bearer token".to_string()));
    }
    state
        .user_for_token(token)
        .await
        .map_err(db_error)?
        .ok_or_else(|| ApiError::Unauthorized("invalid bearer token".to_string()))
}

async fn require_admin_session(
    state: &ServerState,
    headers: &HeaderMap,
) -> Result<AppUser, ApiError> {
    let value = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| ApiError::Unauthorized("missing admin bearer token".to_string()))?
        .to_str()
        .map_err(|_| ApiError::Unauthorized("invalid authorization header".to_string()))?;
    let token = value
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::Unauthorized("expected Bearer token".to_string()))?
        .trim();
    let user = state
        .user_for_token(token)
        .await
        .map_err(db_error)?
        .ok_or_else(|| ApiError::Unauthorized("invalid admin bearer token".to_string()))?;
    if is_admin_role(&user.role) {
        Ok(user)
    } else {
        Err(ApiError::Unauthorized("admin role required".to_string()))
    }
}

fn app_user_json(user: &AppUser) -> serde_json::Value {
    json!({
        "id": user.id,
        "email": user.email,
        "display_name": user.display_name,
        "plan_id": user.plan_id,
        "role": user.role,
        "status": user.status,
    })
}

fn is_admin_role(role: &str) -> bool {
    matches!(role, "admin" | "operator" | "support")
}

fn admin_permissions(role: &str) -> Vec<&'static str> {
    match role {
        "admin" => vec![
            "dashboard",
            "users",
            "billing",
            "redeem",
            "feedback",
            "releases",
            "audit",
        ],
        "operator" => vec!["dashboard", "billing", "redeem", "feedback", "releases"],
        "support" => vec!["dashboard", "feedback"],
        _ => Vec::new(),
    }
}

fn db_error(error: sqlx::Error) -> ApiError {
    ApiError::Io(format!("database error: {error}"))
}

fn compute_recharge_amount(
    recharge_type: &str,
    package_id: &str,
    quantity: Option<u64>,
) -> Result<(String, u64, u64), ApiError> {
    match recharge_type {
        "count" => {
            let resolved_quantity = quantity.unwrap_or(match package_id {
                "count_10" => 10,
                "count_30" => 30,
                _ => 3,
            });
            if resolved_quantity < 3 {
                return Err(ApiError::BadRequest {
                    code: "invalid_recharge_quantity",
                    message: "count recharge requires at least 3 conversions".to_string(),
                });
            }
            Ok((
                "count".to_string(),
                resolved_quantity,
                resolved_quantity * 100,
            ))
        }
        "date" => {
            let (days, amount_cents) = match package_id {
                "day" => (1, 500),
                "week" => (7, 1400),
                "month" => (30, 3000),
                "year" => (365, 12000),
                _ => {
                    return Err(ApiError::BadRequest {
                        code: "invalid_recharge_package",
                        message: format!("unsupported date package: {package_id}"),
                    });
                }
            };
            Ok(("date".to_string(), days, amount_cents))
        }
        other => Err(ApiError::BadRequest {
            code: "invalid_recharge_type",
            message: format!("unsupported recharge type: {other}"),
        }),
    }
}

fn recharge_json(record: &RechargeRecord) -> serde_json::Value {
    json!({
        "recharge_id": record.recharge_id,
        "user_id": record.user_id,
        "recharge_type": record.recharge_type,
        "package_id": record.package_id,
        "quantity": record.quantity,
        "amount_cents": record.amount_cents,
        "currency": record.currency,
        "status": record.status,
        "provider": record.provider,
        "provider_trade_id": record.provider_trade_id,
        "created_at": record.created_at,
    })
}

fn manual_order_json(record: &ManualOrderRecord) -> serde_json::Value {
    json!({
        "order_id": record.order_id,
        "user_id": record.user_id,
        "recharge_id": record.recharge_id,
        "recharge_type": record.recharge_type,
        "package_id": record.package_id,
        "quantity": record.quantity,
        "amount_cents": record.amount_cents,
        "currency": record.currency,
        "status": record.status,
        "operator_id": record.operator_id,
        "payment_note": record.payment_note,
        "created_at": record.created_at,
    })
}

fn redeem_failure_to_error(error: RedeemFailure) -> ApiError {
    match error {
        RedeemFailure::InvalidCode => ApiError::Coded {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_code",
            message: "兑换码无效".to_string(),
        },
        RedeemFailure::AlreadyRedeemed => ApiError::Coded {
            status: StatusCode::CONFLICT,
            code: "code_already_redeemed",
            message: "兑换码已被使用".to_string(),
        },
        RedeemFailure::Voided => ApiError::Coded {
            status: StatusCode::CONFLICT,
            code: "code_voided",
            message: "兑换码已作废".to_string(),
        },
        RedeemFailure::Expired => ApiError::Coded {
            status: StatusCode::GONE,
            code: "code_expired",
            message: "兑换码已过期".to_string(),
        },
    }
}

fn redeem_batch_json(batch: &RedeemCodeBatchRecord, include_codes: bool) -> serde_json::Value {
    json!({
        "batch_id": batch.batch_id,
        "batch_no": batch.batch_no,
        "package_id": batch.package_id,
        "package_name": batch.package_name,
        "recharge_type": batch.recharge_type,
        "quantity": batch.quantity,
        "generated_count": batch.generated_count,
        "exported_count": batch.exported_count,
        "status": batch.status,
        "channel": batch.channel,
        "note": batch.note,
        "expires_at": batch.expires_at,
        "created_at": batch.created_at,
        "codes": if include_codes {
            json!(batch.codes)
        } else {
            json!([])
        },
    })
}

fn redeem_result_json(result: &RedeemCodeResult) -> serde_json::Value {
    let mut value = json!({
        "redeem_id": result.redeem_id,
        "recharge_id": result.recharge_id,
        "package_id": result.package_id,
        "package_name": result.package_name,
        "recharge_type": result.recharge_type,
        "quantity": result.quantity,
        "count_balance": result.count_balance,
        "date_valid_until": result.date_valid_until,
        "redeemed_at": result.redeemed_at,
        "is_new_account": result.is_new_account,
    });
    if let Some(token) = &result.access_token {
        value["access_token"] = json!(token);
    }
    if let Some(token) = &result.refresh_token {
        value["refresh_token"] = json!(token);
    }
    if let Some(user) = &result.user {
        value["user"] = app_user_json(user);
    }
    value
}

fn redeem_record_json(record: &RedeemCodeRecord) -> serde_json::Value {
    json!({
        "redeem_id": record.code_id,
        "batch_id": record.batch_id,
        "batch_no": record.batch_no,
        "code_preview": record.code_preview,
        "package_id": record.package_id,
        "package_name": record.package_name,
        "recharge_type": record.recharge_type,
        "quantity": record.quantity,
        "status": record.status,
        "redeemed_recharge_id": record.redeemed_recharge_id,
        "redeemed_at": record.redeemed_at,
    })
}

fn build_redeem_codes_xlsx(batch: &RedeemCodeBatchRecord) -> Result<Vec<u8>, ApiError> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("[Content_Types].xml", options)
            .map_err(|e| ApiError::Io(e.to_string()))?;
        std::io::Write::write_all(&mut zip, content_types_xml().as_bytes())
            .map_err(|e| ApiError::Io(e.to_string()))?;
        zip.start_file("_rels/.rels", options)
            .map_err(|e| ApiError::Io(e.to_string()))?;
        std::io::Write::write_all(&mut zip, rels_xml().as_bytes())
            .map_err(|e| ApiError::Io(e.to_string()))?;
        zip.start_file("xl/workbook.xml", options)
            .map_err(|e| ApiError::Io(e.to_string()))?;
        std::io::Write::write_all(&mut zip, workbook_xml().as_bytes())
            .map_err(|e| ApiError::Io(e.to_string()))?;
        zip.start_file("xl/_rels/workbook.xml.rels", options)
            .map_err(|e| ApiError::Io(e.to_string()))?;
        std::io::Write::write_all(&mut zip, workbook_rels_xml().as_bytes())
            .map_err(|e| ApiError::Io(e.to_string()))?;
        zip.start_file("xl/worksheets/sheet1.xml", options)
            .map_err(|e| ApiError::Io(e.to_string()))?;
        std::io::Write::write_all(&mut zip, redeem_sheet_xml(batch).as_bytes())
            .map_err(|e| ApiError::Io(e.to_string()))?;
        zip.finish().map_err(|e| ApiError::Io(e.to_string()))?;
    }
    Ok(cursor.into_inner())
}

fn content_types_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#
}

fn rels_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#
}

fn workbook_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets><sheet name="redeem_codes" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#
}

fn workbook_rels_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#
}

fn redeem_sheet_xml(batch: &RedeemCodeBatchRecord) -> String {
    let headers = [
        "批次号",
        "兑换码",
        "套餐 ID",
        "套餐名称",
        "转换次数",
        "过期时间",
        "状态",
        "备注",
    ];
    let mut rows = String::new();
    rows.push_str(&xlsx_row(1, &headers));
    for (idx, code) in batch.codes.iter().enumerate() {
        let note = batch.note.as_deref().unwrap_or_default();
        let expires = batch.expires_at.as_deref().unwrap_or_default();
        let quantity = batch.quantity.to_string();
        let row = [
            batch.batch_no.as_str(),
            code.as_str(),
            batch.package_id.as_str(),
            batch.package_name.as_str(),
            quantity.as_str(),
            expires,
            "unused",
            note,
        ];
        rows.push_str(&xlsx_row((idx + 2) as u32, &row));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheetData>{rows}</sheetData>
</worksheet>"#
    )
}

fn xlsx_row(row_idx: u32, values: &[&str]) -> String {
    let mut row = format!(r#"<row r="{row_idx}">"#);
    for (idx, value) in values.iter().enumerate() {
        let col = ((b'A' + idx as u8) as char).to_string();
        row.push_str(&format!(
            r#"<c r="{col}{row_idx}" t="inlineStr"><is><t>{}</t></is></c>"#,
            xml_escape(value)
        ));
    }
    row.push_str("</row>");
    row
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn job_json(job: &ConversionJobRecord) -> serde_json::Value {
    json!({
        "job_id": job.job_id,
        "user_id": job.user_id,
        "upload_id": job.upload_id,
        "main_tex": job.main_tex,
        "profile": job.profile,
        "quality": job.quality,
        "engine": job.engine,
        "status": job.status.as_str(),
        "created_at": job.created_at,
        "updated_at": job.updated_at,
        "docx_ready": job.result_docx_key.is_some(),
        "report_ready": job.report.is_some(),
        "error_code": job.error_code,
        "error": job.error,
        "storage": {
            "source_zip_key": job.source_zip_key,
            "result_docx_key": job.result_docx_key,
            "result_log_key": job.result_log_key,
            "zip_bytes": job.zip_bytes,
            "docx_bytes": job.docx_bytes,
            "log_bytes": job.log_bytes,
        }
    })
}

fn validate_project_zip(bytes: &[u8]) -> Result<(), ApiError> {
    if bytes.len() > MAX_UPLOAD_ZIP_BYTES {
        return Err(ApiError::BadRequest {
            code: "upload_too_large",
            message: format!(
                "uploaded zip is too large: {} bytes, limit={}",
                bytes.len(),
                MAX_UPLOAD_ZIP_BYTES
            ),
        });
    }

    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ApiError::BadRequest {
        code: "invalid_zip",
        message: format!("invalid zip archive: {e}"),
    })?;

    if archive.is_empty() {
        return Err(ApiError::BadRequest {
            code: "empty_zip",
            message: "uploaded zip is empty".to_string(),
        });
    }
    if archive.len() > MAX_UPLOAD_FILE_COUNT {
        return Err(ApiError::BadRequest {
            code: "too_many_files",
            message: format!(
                "uploaded zip contains too many entries: {}, limit={}",
                archive.len(),
                MAX_UPLOAD_FILE_COUNT
            ),
        });
    }

    let mut total_uncompressed = 0_u64;
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(|e| ApiError::BadRequest {
            code: "invalid_zip_entry",
            message: format!("invalid zip entry #{index}: {e}"),
        })?;
        let name = file.name();
        if name.contains('\\') || file.enclosed_name().is_none() {
            return Err(ApiError::BadRequest {
                code: "zip_slip",
                message: format!("unsafe zip entry path: {name}"),
            });
        }
        if file.size() > MAX_UPLOAD_FILE_BYTES {
            return Err(ApiError::BadRequest {
                code: "file_too_large",
                message: format!(
                    "zip entry is too large: {name}, bytes={}, limit={}",
                    file.size(),
                    MAX_UPLOAD_FILE_BYTES
                ),
            });
        }
        total_uncompressed = total_uncompressed.saturating_add(file.size());
        if total_uncompressed > MAX_UPLOAD_UNCOMPRESSED_BYTES {
            return Err(ApiError::BadRequest {
                code: "uncompressed_too_large",
                message: format!(
                    "zip uncompressed size is too large: {total_uncompressed}, limit={MAX_UPLOAD_UNCOMPRESSED_BYTES}"
                ),
            });
        }
    }
    Ok(())
}

/// 在 body 中找第一个 boundary 行。
/// 形如 `--------------------------rMDEL0ec3WnYUgVuELFD3i`（包含自带的 dash 前缀）。
fn find_first_boundary(body: &[u8]) -> Option<String> {
    let s = String::from_utf8_lossy(body);
    for line in s.lines() {
        let t = line.trim();
        if t.len() > 2 {
            // 跳过 closing `<boundary>--`
            if t.ends_with("--") {
                continue;
            }
            // 必须包含至少一个非 dash 字符（即真正的 boundary 值）
            if t.chars().any(|c| c != '-') {
                return Some(t.to_string());
            }
        }
    }
    None
}

/// 从 body 中提取 name="<field>" 段的内容（字节）。
fn extract_multipart_field(body: &[u8], field_name: &str) -> Result<Option<Vec<u8>>, ApiError> {
    let boundary = find_first_boundary(body)
        .ok_or_else(|| ApiError::Io("missing multipart boundary".to_string()))?;

    let start_marker = boundary;
    let end_marker = format!("{start_marker}--");

    let mut search_pos = 0;
    while let Some(p) = memchr::memmem::find(&body[search_pos..], start_marker.as_bytes()) {
        let part_start = search_pos + p;
        let mut after = &body[part_start + start_marker.len()..];

        // 跳过该 part 的 boundary 行尾的 CRLF
        if after.starts_with(b"\r\n") {
            after = &after[2..];
        } else if after.starts_with(b"\n") {
            after = &after[1..];
        } else {
            break;
        }

        // 找 header 与 content 的分界
        let (header_end, content_offset) = if let Some(i) = memchr::memmem::find(after, b"\r\n\r\n")
        {
            (i, i + 4)
        } else if let Some(i) = memchr::memmem::find(after, b"\n\n") {
            (i, i + 2)
        } else {
            break;
        };

        let headers = String::from_utf8_lossy(&after[..header_end]).to_lowercase();
        let search_for = format!("name=\"{}\"", field_name);

        if headers.contains(&search_for) {
            let content = &after[content_offset..];
            // 找下一个 boundary 来确定 content 结束
            let content_end =
                if let Some(p) = memchr::memmem::find(content, start_marker.as_bytes()) {
                    p
                } else if let Some(p) = memchr::memmem::find(content, end_marker.as_bytes()) {
                    p
                } else {
                    content.len()
                };

            // 去掉末尾 CRLF
            let mut end = content_end;
            while end > 0 && (content[end - 1] == b'\r' || content[end - 1] == b'\n') {
                end -= 1;
            }
            return Ok(Some(content[..end].to_vec()));
        }

        // 不是目标字段：跳到下一个 part
        match memchr::memmem::find(
            &body[part_start + start_marker.len()..],
            start_marker.as_bytes(),
        ) {
            Some(q) => search_pos = part_start + start_marker.len() + q,
            None => break,
        }
    }

    Ok(None)
}

#[axum::debug_handler]
async fn convert(request: Request) -> Result<Response, ApiError> {
    let (_parts, body) = request.into_parts();

    let full_body = axum::body::to_bytes(body, MAX_BODY)
        .await
        .map_err(|e| ApiError::Io(format!("body read error: {e}")))?;

    let file_part =
        extract_multipart_field(&full_body, "file")?.ok_or(ApiError::MissingField("file"))?;

    if file_part.is_empty() {
        return Err(ApiError::MissingField("file"));
    }
    validate_project_zip(&file_part)?;

    let main_tex = extract_multipart_field(&full_body, "main_tex")?
        .and_then(|v| String::from_utf8(v).ok())
        .unwrap_or_else(|| DEFAULT_MAIN_TEX.to_string());

    let options = ConvertOptions::default();
    let result = convert_zip(&file_part, &main_tex, &options)?;

    if result.docx.len() < 4 * 1024 {
        return Err(ApiError::Io(format!(
            "docx 字节数异常：{}（应 ≥ 4 KiB）",
            result.docx.len()
        )));
    }

    if &result.docx[..4] != b"PK\x03\x04" {
        return Err(ApiError::Io("docx 头部非 PK\\x03\\x04".to_string()));
    }

    let mime: Mime = "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        .parse()
        .expect("static mime is valid");

    let docx_len = result.docx.len();

    let resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}.docx\"", sanitize(&main_tex)),
        )
        .header(header::CONTENT_LENGTH, docx_len)
        .body(axum::body::Body::from(result.docx))
        .map_err(|e| ApiError::Io(e.to_string()))?;
    Ok(resp.into_response())
}

// ─────────────────────────────────────────────────────────────────────────────
// Conversion file download handlers (enhanced session storage)
// ─────────────────────────────────────────────────────────────────────────────

/// Download the original ZIP for a conversion job.
async fn download_conversion_zip(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
) -> Result<Response, ApiError> {
    let session = require_session(&state, &headers).await?;
    let job = state
        .get_job(&job_id)
        .await
        .filter(|j| j.user_id == session.id)
        .ok_or_else(|| ApiError::NotFound("conversion job not found".to_string()))?;

    let key = job.source_zip_key.as_deref().ok_or_else(|| {
        ApiError::NotFound("ZIP file key not found for this conversion".to_string())
    })?;
    let bytes = state
        .load_storage_key(key)
        .ok_or_else(|| ApiError::NotFound("ZIP file not found for this conversion".to_string()))?;

    let resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}.zip\"", sanitize(&job.main_tex)),
        )
        .header(header::CONTENT_LENGTH, bytes.len())
        .body(axum::body::Body::from(bytes))
        .map_err(|e| ApiError::Io(e.to_string()))?;
    Ok(resp.into_response())
}

/// Download the conversion log for a conversion job.
async fn download_conversion_log(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
) -> Result<Response, ApiError> {
    let session = require_session(&state, &headers).await?;
    let job = state
        .get_job(&job_id)
        .await
        .filter(|j| j.user_id == session.id)
        .ok_or_else(|| ApiError::NotFound("conversion job not found".to_string()))?;

    let key = job.result_log_key.as_deref().ok_or_else(|| {
        ApiError::NotFound("conversion log key not found for this job".to_string())
    })?;
    let bytes = state
        .load_storage_key(key)
        .ok_or_else(|| ApiError::NotFound("conversion log not found for this job".to_string()))?;

    let resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"conversion-{}.log\"",
                sanitize(&job.main_tex)
            ),
        )
        .header(header::CONTENT_LENGTH, bytes.len())
        .body(axum::body::Body::from(bytes))
        .map_err(|e| ApiError::Io(e.to_string()))?;
    Ok(resp.into_response())
}

// ─────────────────────────────────────────────────────────────────────────────
// Feedback handlers (user-facing)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateThreadJson {
    conversion_job_id: Option<String>,
    title: String,
    feedback_type: String,
    content: String,
    priority: Option<String>,
}

async fn create_feedback_thread(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<CreateThreadJson>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let request = CreateThreadRequest {
        conversion_job_id: payload.conversion_job_id,
        title: payload.title,
        feedback_type: payload.feedback_type,
        content: payload.content,
        priority: payload.priority,
    };
    let store = state.feedback_store();
    let result = store.create_thread(session.id, request).await;
    match result {
        Ok((thread, msg)) => Ok(Json(serde_json::json!({
            "thread_id": thread.thread_id,
            "status": thread.status.as_str(),
            "created_at": thread.created_at,
            "message_id": msg.message_id,
        }))),
        Err(e) => Err(feedback_error_to_api_error(e)),
    }
}

async fn list_feedback_threads(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let store = state.feedback_store();
    let threads = store.list_user_threads(&session.id).await;

    // Get automation status for each thread (if any)
    let automation_service = AutomationService::new(state.pool().clone());
    let mut summaries = Vec::new();
    for t in threads {
        let auto_status = get_automation_status_for_feedback(
            &automation_service,
            &t.thread_id,
        ).await;
        summaries.push(thread_summary_from_summary(&t, auto_status));
    }
    Ok(Json(serde_json::json!(summaries)))
}

async fn get_automation_status_for_feedback(
    service: &AutomationService,
    feedback_id: &str,
) -> Option<(&'static str, &'static str)> {
    // Query automation_requests table for this feedback thread
    let pool = &service.pool;
    let result = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT status, id FROM automation_requests
        WHERE feedback_thread_id = $1
        ORDER BY created_at DESC LIMIT 1
        "#,
    )
    .bind(feedback_id)
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some((status, id))) => {
            let status_str: &'static str = Box::leak(status.into_boxed_str());
            let id_str: &'static str = Box::leak(id.into_boxed_str());
            Some((status_str, id_str))
        }
        _ => None,
    }
}

async fn get_feedback_thread(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(thread_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let store = state.feedback_store();
    let result = store.get_thread_for_user(&session.id, &thread_id).await;
    match result {
        Ok((t, messages)) => {
            let msgs: Vec<_> = messages.into_iter().map(|m| message_json(&m)).collect();
            Ok(Json(serde_json::json!({
                "thread": thread_json(&t),
                "messages": msgs,
            })))
        }
        Err(FeedbackError::NotFound) => {
            Err(ApiError::NotFound("feedback thread not found".to_string()))
        }
        Err(FeedbackError::Forbidden) | Err(FeedbackError::Unauthorized) => {
            Err(ApiError::Unauthorized("not authorized".to_string()))
        }
        Err(e) => Err(feedback_error_to_api_error(e)),
    }
}

#[derive(Debug, Deserialize)]
struct AddMessageJson {
    content: String,
    parent_message_id: Option<String>,
}

async fn add_feedback_message(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(thread_id): Path<String>,
    Json(payload): Json<AddMessageJson>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&state, &headers).await?;
    let request = AddMessageRequest {
        content: payload.content,
        parent_message_id: payload.parent_message_id,
    };
    let store = state.feedback_store();
    let result = store.add_message(session.id, &thread_id, request).await;
    match result {
        Ok(msg) => Ok(Json(serde_json::json!(message_json(&msg)))),
        Err(e) => Err(feedback_error_to_api_error(e)),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Feedback handlers (admin-facing)
// ─────────────────────────────────────────────────────────────────────────────

async fn admin_list_feedback_threads(
    State(state): State<ServerState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<ThreadFilters>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _session = require_admin_session(&state, &headers).await?;
    let store = state.feedback_store();
    let threads = store.admin_list(&params).await;

    // Get automation status for each thread
    let automation_service = AutomationService::new(state.pool().clone());
    let mut summaries = Vec::new();
    for t in threads {
        let auto_status = get_automation_status_for_feedback(
            &automation_service,
            &t.thread_id,
        ).await;
        summaries.push(thread_summary_from_summary(&t, auto_status));
    }
    Ok(Json(serde_json::json!(summaries)))
}

async fn admin_export_feedback_threads(
    State(state): State<ServerState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<ThreadFilters>,
) -> Result<Response, ApiError> {
    let _session = require_admin_session(&state, &headers).await?;
    let store = state.feedback_store();
    let threads = store.admin_list(&params).await;
    let bytes = state.build_feedback_export(threads);
    let resp = Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        )
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"feedback-export.xlsx\"",
        )
        .header(header::CONTENT_LENGTH, bytes.len())
        .body(axum::body::Body::from(bytes))
        .map_err(|e| ApiError::Io(e.to_string()))?;
    Ok(resp.into_response())
}

#[derive(Debug, Deserialize)]
struct AdminUpdateJson {
    status: Option<String>,
    priority: Option<String>,
    admin_assignee: Option<String>,
}

async fn admin_update_feedback_thread(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(thread_id): Path<String>,
    Json(payload): Json<AdminUpdateJson>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_admin_session(&state, &headers).await?;
    let request = AdminUpdateThreadRequest {
        status: payload.status,
        priority: payload.priority,
        admin_assignee: payload.admin_assignee.or(Some(session.id)),
    };
    let store = state.feedback_store();
    let result = store.admin_update(&thread_id, request).await;
    match result {
        Ok(thread) => Ok(Json(serde_json::json!(thread_json(&thread)))),
        Err(e) => Err(feedback_error_to_api_error(e)),
    }
}

#[derive(Debug, Deserialize)]
struct AdminReplyJson {
    content: String,
    is_internal: Option<bool>,
}

async fn admin_reply_feedback_message(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(thread_id): Path<String>,
    Json(payload): Json<AdminReplyJson>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_admin_session(&state, &headers).await?;
    let request = AdminReplyRequest {
        content: payload.content,
        is_internal: payload.is_internal,
    };
    let store = state.feedback_store();
    let result = store.admin_reply(session.id, &thread_id, request).await;
    match result {
        Ok(msg) => Ok(Json(serde_json::json!(message_json(&msg)))),
        Err(e) => Err(feedback_error_to_api_error(e)),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper functions for feedback JSON serialization
// ─────────────────────────────────────────────────────────────────────────────

fn thread_json(t: &crate::feedback_service::FeedbackThread) -> serde_json::Value {
    serde_json::json!({
        "thread_id": t.thread_id,
        "user_id": t.user_id,
        "conversion_job_id": t.conversion_job_id,
        "title": t.title,
        "feedback_type": t.feedback_type.as_str(),
        "status": t.status.as_str(),
        "priority": t.priority.as_str(),
        "admin_assignee": t.admin_assignee,
        "created_at": t.created_at,
        "updated_at": t.updated_at,
    })
}

fn thread_summary_from_summary(
    t: &crate::feedback_service::FeedbackThreadSummary,
    automation_status: Option<(&str, &str)>,
) -> serde_json::Value {
    let (automation_status_str, automation_request_id) = automation_status
        .map(|(s, id)| (s.to_string(), Some(id.to_string())))
        .unwrap_or_else(|| ("none".to_string(), None));

    serde_json::json!({
        "thread_id": t.thread_id,
        "conversion_job_id": t.conversion_job_id,
        "title": t.title,
        "feedback_type": t.feedback_type,
        "status": t.status,
        "priority": t.priority,
        "message_count": t.message_count,
        "latest_message_at": t.latest_message_at,
        "created_at": t.created_at,
        "updated_at": t.updated_at,
        "automation_status": automation_status_str,
        "automation_request_id": automation_request_id,
    })
}

fn message_json(m: &crate::feedback_service::FeedbackMessage) -> serde_json::Value {
    serde_json::json!({
        "message_id": m.message_id,
        "thread_id": m.thread_id,
        "parent_message_id": m.parent_message_id,
        "sender_user_id": m.sender_user_id,
        "sender_type": m.sender_type.as_str(),
        "content": m.content,
        "created_at": m.created_at,
    })
}

fn feedback_error_to_api_error(e: crate::feedback_service::FeedbackError) -> ApiError {
    match e {
        FeedbackError::NotFound => ApiError::NotFound("feedback thread not found".to_string()),
        FeedbackError::Forbidden => {
            ApiError::Unauthorized("not authorized to access this feedback".to_string())
        }
        FeedbackError::Unauthorized => ApiError::Unauthorized("not authorized".to_string()),
        FeedbackError::Validation(msg) => ApiError::BadRequest {
            code: "validation",
            message: msg,
        },
    }
}

fn sanitize(name: &str) -> String {
    name.rsplit('/')
        .next()
        .unwrap_or(name)
        .trim_end_matches(".tex")
        .replace(['\\', ' '], "_")
}

// ─────────────────────────────────────────────────────────────────────────────
// Automation R&D handlers (admin-facing)
// ─────────────────────────────────────────────────────────────────────────────

async fn automation_summary(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _session = require_admin_session(&state, &headers).await?;
    let store = AutomationService::new(state.pool().clone());
    let summary = store.get_summary().await?;
    Ok(Json(serde_json::json!(summary)))
}

async fn automation_list_requests(
    State(state): State<ServerState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<RequestFilters>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _session = require_admin_session(&state, &headers).await?;
    let store = AutomationService::new(state.pool().clone());
    let requests = store.list_requests(&params).await?;
    Ok(Json(serde_json::json!(requests)))
}

async fn automation_get_request(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _session = require_admin_session(&state, &headers).await?;
    let store = AutomationService::new(state.pool().clone());
    let request = store.get_request(&id).await?;
    match request {
        Some(r) => Ok(Json(serde_json::json!(r))),
        None => Err(ApiError::NotFound("request not found".to_string())),
    }
}

async fn automation_get_events(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _session = require_admin_session(&state, &headers).await?;
    let store = AutomationService::new(state.pool().clone());
    let events = store.get_events(&id).await?;
    Ok(Json(serde_json::json!(events)))
}

#[derive(Debug, Deserialize)]
struct AutomationActionJson {
    reason: Option<String>,
    assignee: Option<String>,
}

async fn automation_approve(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_admin_session(&state, &headers).await?;
    let store = AutomationService::new(state.pool().clone());
    let request = store.approve(&id, &session.id).await?;
    Ok(Json(serde_json::json!(request)))
}

async fn automation_reject(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<AutomationActionJson>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_admin_session(&state, &headers).await?;
    let reason = payload.reason.unwrap_or_else(|| "No reason provided".to_string());
    let store = AutomationService::new(state.pool().clone());
    let request = store.reject(&id, &session.id, &reason).await?;
    Ok(Json(serde_json::json!(request)))
}

async fn automation_retry(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_admin_session(&state, &headers).await?;
    let store = AutomationService::new(state.pool().clone());
    let request = store.retry(&id, &session.id).await?;
    Ok(Json(serde_json::json!(request)))
}

async fn automation_escalate(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<AutomationActionJson>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_admin_session(&state, &headers).await?;
    let assignee = payload.assignee.unwrap_or_else(|| "human".to_string());
    let store = AutomationService::new(state.pool().clone());
    let request = store.escalate(&id, &session.id, &assignee).await?;
    Ok(Json(serde_json::json!(request)))
}

async fn automation_list_agents(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _session = require_admin_session(&state, &headers).await?;
    let store = AutomationService::new(state.pool().clone());
    let agents = store.list_agents().await?;
    Ok(Json(serde_json::json!(agents)))
}

async fn automation_pause_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _session = require_admin_session(&state, &headers).await?;
    let store = AutomationService::new(state.pool().clone());
    let agent = store.pause_agent(&id).await?;
    Ok(Json(serde_json::json!(agent)))
}

async fn automation_resume_agent(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _session = require_admin_session(&state, &headers).await?;
    let store = AutomationService::new(state.pool().clone());
    let agent = store.resume_agent(&id).await?;
    Ok(Json(serde_json::json!(agent)))
}
