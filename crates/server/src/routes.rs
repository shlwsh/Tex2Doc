//! HTTP 路由。

use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{extract::Request, Json, Router};
use mime::Mime;
use serde::Deserialize;
use serde_json::json;
use std::io::Cursor;
use tower_http::cors::{Any, CorsLayer};

use doc_core::{convert_zip, ConvertOptions};

use crate::error::ApiError;
use crate::limits::{
    MAX_BODY, MAX_UPLOAD_FILE_BYTES, MAX_UPLOAD_FILE_COUNT, MAX_UPLOAD_UNCOMPRESSED_BYTES,
    MAX_UPLOAD_ZIP_BYTES,
};
use crate::state::{
    ConversionJobRecord, RechargeRecord, ServerState, PREVIEW_CLOUD_CONVERSION_LIMIT,
};
use crate::worker_service;

/// 默认主 tex 路径（与 paper3 e2e 一致）。
const DEFAULT_MAIN_TEX: &str = "main-jos.tex";

/// 组装对外 router。
pub fn router() -> Router {
    router_with_state(worker_service::spawn_worker_state())
}

/// 组装带状态的 router，供测试或外部嵌入复用。
pub fn router_with_state(state: ServerState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/version", get(version))
        .route("/api/v1/convert", post(convert))
        .route("/v1/auth/register", post(auth_register))
        .route("/api/v1/auth/register", post(auth_register))
        .route("/v1/auth/login", post(auth_login))
        .route("/api/v1/auth/login", post(auth_login))
        .route("/v1/auth/refresh", post(auth_refresh))
        .route("/api/v1/auth/refresh", post(auth_refresh))
        .route("/v1/me", get(me))
        .route("/api/v1/me", get(me))
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
        .route("/v1/releases/:channel", get(release_manifest))
        .route("/api/v1/releases/:channel", get(release_manifest))
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(tower_http::limit::RequestBodyLimitLayer::new(MAX_BODY))
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

#[derive(Debug, Deserialize)]
struct AuthRequest {
    email: Option<String>,
    display_name: Option<String>,
    refresh_token: Option<String>,
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
}

#[derive(Debug, Deserialize)]
struct RechargeBody {
    recharge_type: Option<String>,
    package_id: Option<String>,
    quantity: Option<u64>,
}

async fn auth_register(Json(payload): Json<AuthRequest>) -> Json<serde_json::Value> {
    auth_response(
        payload
            .email
            .unwrap_or_else(|| "demo@example.com".to_string()),
        payload.display_name,
    )
}

async fn auth_login(Json(payload): Json<AuthRequest>) -> Json<serde_json::Value> {
    auth_response(
        payload
            .email
            .unwrap_or_else(|| "demo@example.com".to_string()),
        payload.display_name,
    )
}

async fn auth_refresh(Json(payload): Json<AuthRequest>) -> Json<serde_json::Value> {
    let suffix = payload.refresh_token.unwrap_or_else(|| "demo".to_string());
    Json(json!({
        "access_token": format!("demo-access-{suffix}"),
        "refresh_token": format!("demo-refresh-{suffix}"),
        "user": demo_user("demo@example.com", None),
    }))
}

async fn me(headers: HeaderMap) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&headers)?;
    Ok(Json(demo_user(
        &session.email,
        Some("Demo User".to_string()),
    )))
}

async fn usage(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&headers)?;
    let used = state.cloud_conversions_used(&session.user_id).await;
    Ok(Json(json!({
        "plan_id": "preview",
        "cloud_conversions_used": used,
        "cloud_conversions_limit": PREVIEW_CLOUD_CONVERSION_LIMIT,
        "storage_bytes_used": 0,
        "storage_bytes_limit": 1_073_741_824_u64,
        "period_start": "2026-06-01T00:00:00Z",
        "period_end": "2026-07-01T00:00:00Z",
    })))
}

async fn plans() -> Json<serde_json::Value> {
    Json(json!([
        {
            "id": "preview",
            "name": "Preview",
            "price_cents": 0,
            "currency": "USD",
            "monthly_conversions": 100,
            "features": ["local-convert", "cloud-preview", "quality-report"]
        },
        {
            "id": "pro",
            "name": "Pro",
            "price_cents": 2900,
            "currency": "USD",
            "monthly_conversions": 1000,
            "features": ["priority-worker", "journal-profiles", "desktop-sync"]
        }
    ]))
}

async fn recharge_options() -> Json<serde_json::Value> {
    Json(json!({
        "currency": "CNY",
        "provider": "mock-pay",
        "count": {
            "unit_price_cents": 100,
            "minimum_quantity": 3,
            "packages": [
                {"id": "count_3", "name": "3 次", "quantity": 3, "amount_cents": 300},
                {"id": "count_10", "name": "10 次", "quantity": 10, "amount_cents": 1000},
                {"id": "count_30", "name": "30 次", "quantity": 30, "amount_cents": 3000}
            ]
        },
        "date": {
            "packages": [
                {"id": "day", "name": "日卡", "days": 1, "amount_cents": 500},
                {"id": "week", "name": "周卡", "days": 7, "amount_cents": 1400},
                {"id": "month", "name": "月卡", "days": 30, "amount_cents": 3000},
                {"id": "year", "name": "年卡", "days": 365, "amount_cents": 12000}
            ]
        }
    }))
}

async fn create_recharge(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<RechargeBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&headers)?;
    let recharge_type = payload.recharge_type.unwrap_or_else(|| "count".to_string());
    let package_id = payload.package_id.unwrap_or_else(|| "count_3".to_string());
    let (normalized_type, quantity, amount_cents) =
        compute_recharge_amount(&recharge_type, &package_id, payload.quantity)?;
    let record = state
        .create_recharge(
            session.user_id,
            normalized_type,
            package_id,
            quantity,
            amount_cents,
        )
        .await;
    Ok(Json(recharge_json(&record)))
}

async fn list_recharges(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&headers)?;
    let records = state.list_recharges(&session.user_id).await;
    Ok(Json(json!(records
        .iter()
        .map(recharge_json)
        .collect::<Vec<_>>())))
}

async fn billing_checkout(
    headers: HeaderMap,
    Json(payload): Json<CheckoutBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _session = require_session(&headers)?;
    let plan_id = payload.plan_id.unwrap_or_else(|| "pro".to_string());
    let success_url = payload
        .success_url
        .unwrap_or_else(|| "https://tex2doc.cn/success".to_string());
    let cancel_url = payload
        .cancel_url
        .unwrap_or_else(|| "https://tex2doc.cn/cancel".to_string());
    Ok(Json(json!({
        "url": format!("https://billing.tex2doc.cn/checkout?plan={plan_id}&success={success_url}&cancel={cancel_url}"),
        "expires_at": "2026-06-21T23:59:59Z",
    })))
}

async fn billing_portal(
    headers: HeaderMap,
    Json(payload): Json<CheckoutBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _session = require_session(&headers)?;
    let return_url = payload
        .return_url
        .unwrap_or_else(|| "https://tex2doc.cn/account".to_string());
    Ok(Json(json!({
        "url": format!("https://billing.tex2doc.cn/portal?return={return_url}"),
        "expires_at": "2026-06-21T23:59:59Z",
    })))
}

async fn upload_project(
    State(state): State<ServerState>,
    request: Request,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (parts, body) = request.into_parts();
    let _session = require_session(&parts.headers)?;
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
        .store_upload("project.zip".to_string(), file_part)
        .await;
    Ok(Json(json!({
        "upload_id": record.upload_id,
        "status": "stored",
        "bytes": record.bytes.len() as u64,
        "file_name": record.file_name,
        "created_at": record.created_at,
    })))
}

async fn create_conversion(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(payload): Json<ConversionBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&headers)?;
    let upload_id = payload
        .upload_id
        .unwrap_or_else(|| "upload_demo".to_string());
    if state.get_upload(&upload_id).await.is_none() {
        return Err(ApiError::NotFound(format!("upload {upload_id}")));
    }
    let profile = payload.profile.unwrap_or_else(|| "auto".to_string());
    let quality = payload.quality.unwrap_or_else(|| "standard".to_string());
    let engine = payload
        .engine
        .or(payload.backend)
        .unwrap_or_else(|| "semantic-engine".to_string());
    let main_tex = payload
        .main_tex
        .unwrap_or_else(|| DEFAULT_MAIN_TEX.to_string());
    state
        .try_consume_cloud_conversion(&session.user_id)
        .await
        .map_err(|used| {
            ApiError::PaymentRequired(format!(
                "preview cloud conversion quota exceeded: used={used}, limit={PREVIEW_CLOUD_CONVERSION_LIMIT}"
            ))
        })?;
    let job = state
        .create_job(
            session.user_id.clone(),
            upload_id,
            main_tex,
            profile,
            quality,
            engine,
        )
        .await;
    state
        .enqueue_job(job.job_id.clone())
        .await
        .map_err(ApiError::Io)?;
    Ok(Json(job_json(&job)))
}

async fn list_conversions(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let session = require_session(&headers)?;
    let jobs = state.list_jobs_by_user(&session.user_id).await;
    Ok(Json(json!(jobs.iter().map(job_json).collect::<Vec<_>>())))
}

async fn get_conversion(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _session = require_session(&headers)?;
    let job = state
        .get_job(&id)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("conversion {id}")))?;
    Ok(Json(job_json(&job)))
}

async fn download_conversion_docx(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let _session = require_session(&headers)?;
    let job = state
        .get_job(&id)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("conversion {id}")))?;
    let body = job
        .docx
        .ok_or_else(|| ApiError::Conflict(format!("conversion {id} docx is not ready")))?;
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
    let _session = require_session(&headers)?;
    let job = state
        .get_job(&id)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("conversion {id}")))?;
    let report = job
        .report
        .ok_or_else(|| ApiError::Conflict(format!("conversion {id} report is not ready")))?;
    Ok(Json(
        serde_json::to_value(report).map_err(|e| ApiError::Io(e.to_string()))?,
    ))
}

async fn release_manifest(Path(channel): Path<String>) -> Json<serde_json::Value> {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "channel": channel,
        "download_url": "https://releases.tex2doc.cn/desktop/latest",
        "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "signature": "pending-p9-signature",
        "release_notes": "P9 preview manifest. Installers and real signatures are pending platform release builds.",
    }))
}

fn auth_response(email: String, display_name: Option<String>) -> Json<serde_json::Value> {
    Json(json!({
        "access_token": format!("demo-access-{}", email.replace('@', "_")),
        "refresh_token": format!("demo-refresh-{}", email.replace('@', "_")),
        "user": demo_user(&email, display_name),
    }))
}

#[derive(Debug, Clone)]
struct DemoSession {
    user_id: String,
    email: String,
}

fn require_session(headers: &HeaderMap) -> Result<DemoSession, ApiError> {
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
    if !(token.starts_with("demo-access-") || token.starts_with("demo-refresh-")) {
        return Err(ApiError::Unauthorized(
            "unsupported bearer token for preview server".to_string(),
        ));
    }
    let suffix = token
        .strip_prefix("demo-access-")
        .or_else(|| token.strip_prefix("demo-refresh-"))
        .unwrap_or("demo");
    Ok(DemoSession {
        user_id: suffix.to_string(),
        email: "demo@example.com".to_string(),
    })
}

fn demo_user(email: &str, display_name: Option<String>) -> serde_json::Value {
    json!({
        "id": "user_demo",
        "email": email,
        "display_name": display_name,
        "plan_id": "preview",
    })
}

fn compute_recharge_amount(
    recharge_type: &str,
    package_id: &str,
    quantity: Option<u64>,
) -> Result<(String, u64, u64), ApiError> {
    match recharge_type {
        "count" => {
            let resolved_quantity = quantity.unwrap_or_else(|| match package_id {
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
        "docx_ready": job.docx.is_some(),
        "report_ready": job.report.is_some(),
        "error_code": job.error_code,
        "error": job.error,
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

fn sanitize(name: &str) -> String {
    name.rsplit('/')
        .next()
        .unwrap_or(name)
        .trim_end_matches(".tex")
        .replace(['\\', ' '], "_")
}
