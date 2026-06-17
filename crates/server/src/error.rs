//! 服务端错误模型 + HTTP 状态码映射。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("IO 错误：{0}")]
    Io(String),

    #[error("multipart 字段缺失：{0}")]
    MissingField(&'static str),

    #[error("核心转换失败：{0}")]
    Core(#[from] doc_core::CoreError),
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            ServerError::Io(_) => (StatusCode::BAD_REQUEST, "io"),
            ServerError::MissingField(_) => (StatusCode::BAD_REQUEST, "missing_field"),
            ServerError::Core(doc_core::CoreError::Parse(_))
            | ServerError::Core(doc_core::CoreError::Io(_)) => (StatusCode::BAD_REQUEST, "parse"),
            ServerError::Core(doc_core::CoreError::Unsupported(_)) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "unsupported")
            }
            ServerError::Core(doc_core::CoreError::Serialize(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal")
            }
        };
        let body = ErrorBody {
            error: code,
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

/// 给 `routes.rs` 当作 `Result<T, ApiError>` 用。
pub type ApiError = ServerError;
