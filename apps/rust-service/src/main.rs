//! Doc-engine HTTP 服务端（MVP）。
//!
//! 路由：
//! - `GET  /api/v1/health`  健康检查
//! - `GET  /api/v1/version` 版本号
//! - `POST /api/v1/convert` multipart 上传 `.zip` + 表单字段 `main_tex`，返回 `.docx` 字节流
//!
//! 限制：单请求体 ≤ 50 MiB（`tower_http::limit::RequestBodyLimitLayer`）。

use std::net::SocketAddr;

use axum::Router;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

mod db_store;
mod error;
mod excel_export;
mod feedback_service;
mod file_storage;
mod limits;
mod routes;
mod state;
mod worker_service;

use limits::MAX_BODY;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let app: Router = routes::router()
        .await?
        .layer(TraceLayer::new_for_http())
        .layer(RequestBodyLimitLayer::new(MAX_BODY));

    let addr: SocketAddr = std::env::var("DOC_SERVER_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("doc-server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
