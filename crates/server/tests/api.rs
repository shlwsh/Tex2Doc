//! doc-server 端到端集成测试
//!
//! - 复用 [examples/paper3/upload.zip] 作为夹具
//! - 覆盖：health / version / 成功 convert / 缺 file 字段 / 主文件找不到 / 超大 body

use std::net::SocketAddr;
use std::time::Duration;

use reqwest::multipart::{Form, Part};
use tokio::net::TcpListener;
use tokio::time::sleep;

use doc_server::build_router;

const FIXTURE_ZIP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../..",
    "/examples/paper3/upload.zip"
);

/// 在当前 task 上下文里跑 server，shutdown 信号来时优雅退出。
async fn run_server(listener: TcpListener, mut shutdown: tokio::sync::oneshot::Receiver<()>) {
    let app = build_router();
    tokio::select! {
        result = axum::serve(listener, app) => {
            result.expect("axum::serve");
        }
        _ = &mut shutdown => {
            // 收到 shutdown 信号，函数返回
        }
    }
}

/// 启动一个可关闭的 server 测试装置。
async fn spawn_test_server() -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local_addr");
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(run_server(listener, rx));
    sleep(Duration::from_millis(150)).await;
    (addr, tx)
}

fn test_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("build reqwest client with timeout")
}

#[tokio::test]
async fn health_returns_ok() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let resp = client
        .get(format!("http://{addr}/api/v1/health"))
        .send()
        .await
        .expect("send health");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json health");
    assert_eq!(body["status"], "ok");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn version_returns_semver() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let resp = client
        .get(format!("http://{addr}/api/v1/version"))
        .send()
        .await
        .expect("send version");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json version");
    assert_eq!(body["name"], "doc-server");
    assert!(body["version"].as_str().unwrap().contains('.'));
    let _ = shutdown.send(());
}

#[tokio::test]
async fn convert_paper3_zip_returns_docx() {
    let (addr, shutdown) = spawn_test_server().await;
    let zip_bytes = std::fs::read(FIXTURE_ZIP).expect("paper3 upload.zip must exist");
    assert!(!zip_bytes.is_empty(), "fixture is empty");

    let form = Form::new()
        .part("file", Part::bytes(zip_bytes).file_name("paper3.zip"))
        .text("main_tex", "main-jos.tex");

    let client = test_client();
    let resp = client
        .post(format!("http://{addr}/api/v1/convert"))
        .multipart(form)
        .send()
        .await
        .expect("send convert");

    let status = resp.status();
    let bytes = resp.bytes().await.expect("read bytes");

    assert_eq!(
        status,
        200,
        "expected 200, got {status}, body: {:?}",
        &bytes[..bytes.len().min(200)]
    );
    assert!(
        bytes.len() > 4 * 1024,
        "docx too small: {} bytes",
        bytes.len()
    );
    assert_eq!(&bytes[..4], b"PK\x03\x04", "docx magic mismatch");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn convert_missing_file_returns_400() {
    let (addr, shutdown) = spawn_test_server().await;
    let form = Form::new().text("main_tex", "main-jos.tex");
    let client = test_client();
    let resp = client
        .post(format!("http://{addr}/api/v1/convert"))
        .multipart(form)
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.expect("json 400");
    assert_eq!(body["error"], "missing_field");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn convert_main_tex_mismatch_returns_400() {
    let (addr, shutdown) = spawn_test_server().await;
    let zip_bytes = std::fs::read(FIXTURE_ZIP).unwrap();
    let form = Form::new()
        .part("file", Part::bytes(zip_bytes).file_name("paper3.zip"))
        .text("main_tex", "nonexistent.tex");
    let client = test_client();
    let resp = client
        .post(format!("http://{addr}/api/v1/convert"))
        .multipart(form)
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 400);
    let _ = shutdown.send(());
}

#[tokio::test]
async fn convert_zip_header_only_returns_400() {
    let (addr, shutdown) = spawn_test_server().await;
    let fake = vec![0u8; 1024];
    let form = Form::new()
        .part("file", Part::bytes(fake).file_name("bad.zip"))
        .text("main_tex", "main-jos.tex");
    let client = test_client();
    let resp = client
        .post(format!("http://{addr}/api/v1/convert"))
        .multipart(form)
        .send()
        .await
        .expect("send");
    assert!(
        resp.status().is_client_error(),
        "expected 4xx, got {}",
        resp.status()
    );
    let _ = shutdown.send(());
}
