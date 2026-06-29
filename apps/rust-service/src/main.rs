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

mod automation_service;
mod db_store;
mod error;
mod error_code;
mod excel_export;
mod feedback_service;
mod file_storage;
mod limits;
mod logging;
mod routes;
mod state;
mod worker_service;

use limits::MAX_BODY;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    logging::init();

    let app: Router = routes::router()
        .await?
        .layer(logging::TraceIdLayer::new())
        .layer(RequestBodyLimitLayer::new(MAX_BODY));

    let addr: SocketAddr = std::env::var("DOC_SERVER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:2624".to_string())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("doc-server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
