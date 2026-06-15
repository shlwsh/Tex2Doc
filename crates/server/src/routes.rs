//! HTTP 路由。

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{extract::Request, Json, Router};
use mime::Mime;
use serde_json::json;

use doc_core::{convert_zip, ConvertOptions};

use crate::error::ApiError;
use crate::limits::MAX_BODY;

/// 默认主 tex 路径（与 paper3 e2e 一致）。
const DEFAULT_MAIN_TEX: &str = "main-jos.tex";

/// 组装对外 router。
pub fn router() -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/version", get(version))
        .route("/api/v1/convert", post(convert))
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
