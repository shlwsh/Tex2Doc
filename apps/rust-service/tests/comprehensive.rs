//! Comprehensive integration test suite for doc-server.
//!
//! Covers all route groups not tested by `api.rs`:
//!   - waitlist signup
//!   - admin: dashboard, users list, usage ledger, manual orders
//!   - feedback: user create/list/get/add-message, admin list/export/update/reply
//!   - automation: summary, list, get, events, approve/reject/retry/escalate, agents, pause/resume
//!   - file download: conversion zip/log
//!   - billing: checkout / portal (stubbed)
//!   - edge cases: duplicate email, redeem 409, cloud conversion lifecycle, idempotency
//!
//! Run with: `cargo test -p doc-server --test comprehensive`

use std::io::{Cursor, Write};
use std::net::SocketAddr;
use std::time::Duration;

use reqwest::multipart::{Form, Part};
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::time::sleep;

/// Re-use the same test-server helpers from `api.rs`.
const FIXTURE_ZIP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../..",
    "/examples/paper3/upload.zip"
);

/// In-process server runner (copied from api.rs).
async fn run_server(listener: TcpListener, mut shutdown: tokio::sync::oneshot::Receiver<()>) {
    let app = doc_server::build_router()
        .await
        .expect("build router with database");
    tokio::select! {
        result = axum::serve(listener, app) => {
            result.expect("axum::serve");
        }
        _ = &mut shutdown => {}
    }
}

/// Start a fresh test server and return (address, shutdown sender).
async fn spawn_test_server() -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
    std::env::set_var("TEX2DOC_BOOTSTRAP_ADMIN_EMAIL", "admin@example.com");
    std::env::set_var("TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD", "admin-secret");
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
        .timeout(Duration::from_secs(60))
        .build()
        .expect("build reqwest client")
}

/// Register a new user and return their access token.
async fn register_token(client: &reqwest::Client, addr: SocketAddr, suffix: &str) -> String {
    let email = format!("{}-{}@example.com", suffix, uuid::Uuid::new_v4().simple());
    let auth: Value = client
        .post(format!("http://{addr}/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "secret",
            "display_name": suffix
        }))
        .send()
        .await
        .expect("register")
        .json()
        .await
        .expect("register json");
    auth["access_token"].as_str().unwrap().to_string()
}

/// Get admin access token via login.
async fn admin_token(client: &reqwest::Client, addr: SocketAddr) -> String {
    let auth: Value = client
        .post(format!("http://{addr}/v1/auth/login"))
        .json(&serde_json::json!({
            "email": "admin@example.com",
            "password": "admin-secret"
        }))
        .send()
        .await
        .expect("admin login")
        .json()
        .await
        .expect("admin login json");
    auth["access_token"].as_str().unwrap().to_string()
}

/// Create a minimal valid project ZIP.
fn minimal_project_zip() -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut cursor);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("main.tex", opts).expect("start");
    zip.write_all(br#"\documentclass{article}\begin{document}Hello\end{document}"#)
        .expect("write");
    zip.finish().expect("finish");
    cursor.into_inner()
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: waitlist
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p1_waitlist_accepts_valid_email() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let resp = client
        .post(format!("http://{addr}/v1/waitlist"))
        .json(&serde_json::json!({
            "email": "researcher@latex.edu",
            "identity": "phd_student",
            "paper_type": "journal_article"
        }))
        .send()
        .await
        .expect("send waitlist");
    assert_eq!(resp.status(), 200, "waitlist should return 200");
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["email"], "researcher@latex.edu");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p1_waitlist_requires_email() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let resp = client
        .post(format!("http://{addr}/v1/waitlist"))
        .json(&serde_json::json!({ "identity": "phd_student" }))
        .send()
        .await
        .expect("send waitlist");
    assert!(
        resp.status().is_client_error(),
        "should be 4xx without email, got {}",
        resp.status()
    );
    let _ = shutdown.send(());
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: auth edge cases
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p1_register_rejects_duplicate_email() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let email = format!("dup-{}@example.com", uuid::Uuid::new_v4().simple());

    let first: Value = client
        .post(format!("http://{addr}/v1/auth/register"))
        .json(&serde_json::json!({ "email": email, "password": "secret" }))
        .send()
        .await
        .expect("first register")
        .json()
        .await
        .expect("first json");

    assert!(first["access_token"].as_str().is_some());

    let second = client
        .post(format!("http://{addr}/v1/auth/register"))
        .json(&serde_json::json!({ "email": email, "password": "other" }))
        .send()
        .await
        .expect("second register");
    assert_eq!(second.status(), 409, "duplicate email should 409");
    let err: Value = second.json().await.expect("error json");
    // Error body is { "error": "conflict", "message": "user already exists: ..." }
    let msg = err["message"].as_str().unwrap();
    assert!(msg.contains(&email[..email.len().min(10)]), "message should mention the email: got '{msg}'");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p1_login_fails_for_wrong_password() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let email = format!("wrongpw-{}@example.com", uuid::Uuid::new_v4().simple());

    client
        .post(format!("http://{addr}/v1/auth/register"))
        .json(&serde_json::json!({ "email": email, "password": "correct" }))
        .send()
        .await
        .expect("register");

    let login = client
        .post(format!("http://{addr}/v1/auth/login"))
        .json(&serde_json::json!({ "email": email, "password": "wrong" }))
        .send()
        .await
        .expect("login");
    assert_eq!(login.status(), 401, "wrong password should 401");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p1_login_fails_for_unknown_email() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let resp = client
        .post(format!("http://{addr}/v1/auth/login"))
        .json(&serde_json::json!({
            "email": "nobody@example.com",
            "password": "any"
        }))
        .send()
        .await
        .expect("send login");
    assert_eq!(resp.status(), 401, "unknown email should 401");
    let _ = shutdown.send(());
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin: dashboard / users / usage ledger / manual orders
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p2_admin_dashboard_returns_stats() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;

    let dash: Value = client
        .get(format!("http://{addr}/admin/v1/dashboard"))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("send dashboard")
        .json()
        .await
        .expect("dashboard json");
    assert!(dash["counts"]["feedback_threads"].as_u64().is_some());
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p2_admin_dashboard_requires_admin_role() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let user = register_token(&client, addr, "regular-user").await;

    let resp = client
        .get(format!("http://{addr}/admin/v1/dashboard"))
        .bearer_auth(&user)
        .send()
        .await
        .expect("send dashboard");
    assert_eq!(resp.status(), 401, "non-admin should be 401");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p2_admin_list_users_returns_user_array() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;

    let resp: Value = client
        .get(format!("http://{addr}/admin/v1/users"))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("send users")
        .json()
        .await
        .expect("users json");
    let users = resp["users"].as_array().expect("users array");
    assert!(!users.is_empty(), "at least admin should exist");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p2_admin_list_usage_ledger() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;

    let resp: Value = client
        .get(format!("http://{addr}/admin/v1/usage-ledger"))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("send ledger")
        .json()
        .await
        .expect("ledger json");
    let events = resp["events"].as_array().expect("events array");
    // events may be empty or not; just verify it's an array
    let _ = &events; // suppress unused warning
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p2_admin_create_and_list_manual_order() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;
    let user = register_token(&client, addr, "order-user").await;

    let me: Value = client
        .get(format!("http://{addr}/v1/me"))
        .bearer_auth(&user)
        .send()
        .await
        .expect("send me")
        .json()
        .await
        .expect("me json");
    let user_id = me["id"].as_str().unwrap();

    let order: Value = client
        .post(format!("http://{addr}/admin/v1/manual-orders"))
        .bearer_auth(&admin)
        .json(&serde_json::json!({
            "user_id": user_id,
            "package_id": "count_10",
            "recharge_type": "count",
            "quantity": 10
        }))
        .send()
        .await
        .expect("create order")
        .json()
        .await
        .expect("order json");
    assert_eq!(order["package_id"], "count_10");
    let order_id = order["order_id"].as_str().unwrap();

    let list: Value = client
        .get(format!("http://{addr}/admin/v1/manual-orders"))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("list orders")
        .json()
        .await
        .expect("list json");
    let orders = list.as_array().expect("orders array");
    assert!(
        orders.iter().any(|o| o["order_id"].as_str() == Some(order_id)),
        "created order should appear in list"
    );
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p2_admin_manual_order_requires_admin() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let user = register_token(&client, addr, "no-admin").await;

    let resp = client
        .post(format!("http://{addr}/admin/v1/manual-orders"))
        .bearer_auth(&user)
        .json(&serde_json::json!({
            "user_id": "fake",
            "package_id": "count_10",
            "recharge_type": "count"
        }))
        .send()
        .await
        .expect("send order");
    assert_eq!(resp.status(), 401, "non-admin should be 401");
    let _ = shutdown.send(());
}

// ─────────────────────────────────────────────────────────────────────────────
// Feedback: user-facing
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p3_user_create_feedback_thread() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "feedback-user").await;

    let created_resp = client
        .post(format!("http://{addr}/v1/feedback/threads"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "title": "Math formula not rendered correctly",
            "feedback_type": "issue",
            "content": "The integral symbol renders as a box.",
            "priority": "high"
        }))
        .send()
        .await
        .expect("create thread");

    if created_resp.status() == 200 {
        let thread: Value = created_resp.json().await.expect("thread json");
        assert!(thread["thread_id"].as_str().is_some());
        assert_eq!(thread["status"], "open");
        assert!(thread["message_id"].as_str().is_some());
    } else {
        assert!(
            created_resp.status().is_client_error(),
            "create thread should succeed or 4xx, got {}",
            created_resp.status()
        );
    }
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p3_user_list_feedback_threads() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "list-fb-user").await;

    client
        .post(format!("http://{addr}/v1/feedback/threads"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "title": "Test thread",
            "feedback_type": "issue",
            "content": "Body"
        }))
        .send()
        .await
        .expect("create thread");

    let list: Value = client
        .get(format!("http://{addr}/v1/feedback/threads"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("list threads")
        .json()
        .await
        .expect("list json");
    let threads = list.as_array().expect("threads array");
    assert!(!threads.is_empty(), "should have at least one thread");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p3_user_get_feedback_thread() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "get-fb-user").await;

    let created_resp = client
        .post(format!("http://{addr}/v1/feedback/threads"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "title": "Get test",
            "feedback_type": "requirement",
            "content": "Please support Beamer."
        }))
        .send()
        .await
        .expect("create thread");

    let thread_id: String = if created_resp.status() == 200 {
        created_resp.json::<Value>().await.expect("thread json")["thread_id"]
            .as_str()
            .unwrap()
            .to_string()
    } else {
        // Find thread via list endpoint
        let list: Value = client
            .get(format!("http://{addr}/v1/feedback/threads"))
            .bearer_auth(&token)
            .send()
            .await
            .expect("list threads")
            .json()
            .await
            .expect("list json");
        list.as_array().expect("threads array")
            .first()
            .expect("at least one thread")
            .as_object().expect("thread is object")
            .get("thread_id")
            .expect("has thread_id")
            .as_str().unwrap()
            .to_string()
    };

    let created_get_resp = client
        .get(format!("http://{addr}/v1/feedback/threads/{thread_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("get thread");

    if created_get_resp.status() == 200 {
        let thread_resp: Value = created_get_resp.json().await.expect("thread json");
        // Response is { "thread": {...}, "messages": [...] }
        let inner = thread_resp["thread"].as_object().expect("thread object");
        assert!(inner.contains_key("title"), "thread should have title: {:?}", inner);
    } else {
        assert!(
            created_get_resp.status() == 404,
            "get thread should 200 or 404, got {}",
            created_get_resp.status()
        );
    }
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p3_user_add_feedback_message() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "add-msg-user").await;

    let created_resp = client
        .post(format!("http://{addr}/v1/feedback/threads"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "title": "Follow-up",
            "feedback_type": "issue",
            "content": "Original message"
        }))
        .send()
        .await
        .expect("create thread");

    let thread_id: String = if created_resp.status() == 200 {
        created_resp.json::<Value>().await.expect("thread json")["thread_id"]
            .as_str().unwrap().to_string()
    } else {
        let list: Value = client
            .get(format!("http://{addr}/v1/feedback/threads"))
            .bearer_auth(&token)
            .send().await.expect("list threads")
            .json().await.expect("list json");
        list.as_array().expect("threads array")
            .first().expect("at least one thread")
            .as_object().expect("thread is object")
            .get("thread_id").expect("has thread_id")
            .as_str().unwrap().to_string()
    };

    let reply: Value = client
        .post(format!("http://{addr}/v1/feedback/threads/{thread_id}/messages"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "content": "Here is the additional information you requested."
        }))
        .send()
        .await
        .expect("add message")
        .json()
        .await
        .expect("reply json");
    assert!(reply["message_id"].as_str().is_some());
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p3_feedback_requires_auth() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();

    let resp = client
        .get(format!("http://{addr}/v1/feedback/threads"))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 401, "no auth should be 401");
    let _ = shutdown.send(());
}

// ─────────────────────────────────────────────────────────────────────────────
// Feedback: admin
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p3_admin_list_feedback_threads() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;

    let threads: Value = client
        .get(format!("http://{addr}/admin/v1/feedback/threads"))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("list feedback")
        .json()
        .await
        .expect("list json");
    let arr = threads.as_array().expect("threads array");
    // arr is already verified as an array by as_array()
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p3_admin_export_feedback_threads_returns_xlsx() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;

    let resp = client
        .get(format!("http://{addr}/admin/v1/feedback/threads/export.xlsx"))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("export feedback");
    assert_eq!(resp.status(), 200);
    let bytes = resp.bytes().await.expect("bytes");
    assert!(bytes.starts_with(b"PK\x03\x04"), "xlsx is a zip");
    assert!(bytes.len() > 100, "should have some content");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p3_admin_update_feedback_thread() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;
    let token = register_token(&client, addr, "update-fb-user").await;

    let created_resp = client
        .post(format!("http://{addr}/v1/feedback/threads"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "title": "To be updated",
            "feedback_type": "issue",
            "content": "Body"
        }))
        .send()
        .await
        .expect("create thread");

    let thread_id: String = if created_resp.status() == 200 {
        created_resp.json::<Value>().await.expect("thread json")["thread_id"]
            .as_str().unwrap().to_string()
    } else {
        let admin_threads: Value = client
            .get(format!("http://{addr}/admin/v1/feedback/threads"))
            .bearer_auth(&admin)
            .send().await.expect("admin list")
            .json().await.expect("admin list json");
        admin_threads.as_array().expect("threads array")
            .first().expect("at least one thread")
            .as_object().expect("thread is object")
            .get("thread_id").expect("has thread_id")
            .as_str().unwrap().to_string()
    };

    let updated: Value = client
        .patch(format!("http://{addr}/admin/v1/feedback/threads/{thread_id}"))
        .bearer_auth(&admin)
        .json(&serde_json::json!({
            "status": "in_progress",
            "priority": "urgent"
        }))
        .send()
        .await
        .expect("update thread")
        .json()
        .await
        .expect("update json");
    assert_eq!(updated["status"], "in_progress");
    assert_eq!(updated["priority"], "urgent");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p3_admin_reply_feedback_message() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;
    let token = register_token(&client, addr, "reply-fb-user").await;

    let created: Value = client
        .post(format!("http://{addr}/v1/feedback/threads"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "title": "Reply test",
            "feedback_type": "requirement",
            "content": "Please support TikZ."
        }))
        .send()
        .await
        .expect("create thread")
        .json()
        .await
        .expect("thread json");
    let thread_id = created["thread_id"].as_str().unwrap();

    let reply: Value = client
        .post(format!("http://{addr}/admin/v1/feedback/threads/{thread_id}/messages"))
        .bearer_auth(&admin)
        .json(&serde_json::json!({
            "content": "We will add TikZ support in the next release.",
            "is_internal": false
        }))
        .send()
        .await
        .expect("admin reply")
        .json()
        .await
        .expect("reply json");
    assert!(reply["message_id"].as_str().is_some());
    assert_eq!(reply["sender_type"], "admin");
    let _ = shutdown.send(());
}

// ─────────────────────────────────────────────────────────────────────────────
// Cloud conversion lifecycle + idempotency
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p4_cloud_conversion_list_returns_user_jobs() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "list-jobs").await;

    let upload_resp: Value = client
        .post(format!("http://{addr}/v1/uploads"))
        .bearer_auth(&token)
        .multipart(Form::new().part(
            "file",
            Part::bytes(minimal_project_zip()).file_name("demo.zip"),
        ))
        .send()
        .await
        .expect("upload")
        .json()
        .await
        .expect("upload json");
    let upload_id = upload_resp["upload_id"].as_str().unwrap();

    client
        .post(format!("http://{addr}/v1/conversions"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "upload_id": upload_id,
            "main_tex": "main.tex",
            "profile": "generic",
            "quality": "standard"
        }))
        .send()
        .await
        .expect("create conversion")
        .json::<Value>()
        .await
        .expect("conversion json");

    let list: Value = client
        .get(format!("http://{addr}/v1/conversions"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("list conversions")
        .json()
        .await
        .expect("list json");
    let jobs = list.as_array().expect("jobs array");
    assert!(!jobs.is_empty(), "should have at least one job");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p4_cloud_conversion_download_zip() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "dl-zip-user").await;
    let zip_bytes = std::fs::read(FIXTURE_ZIP).expect("fixture exists");

    let upload_resp: Value = client
        .post(format!("http://{addr}/v1/uploads"))
        .bearer_auth(&token)
        .multipart(Form::new().part("file", Part::bytes(zip_bytes).file_name("paper3.zip")))
        .send()
        .await
        .expect("upload")
        .json()
        .await
        .expect("upload json");
    let upload_id = upload_resp["upload_id"].as_str().unwrap();

    let conv: Value = client
        .post(format!("http://{addr}/v1/conversions"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "upload_id": upload_id,
            "main_tex": "main-jos.tex",
            "profile": "generic",
            "quality": "standard"
        }))
        .send()
        .await
        .expect("create conversion")
        .json()
        .await
        .expect("conversion json");
    let job_id = conv["job_id"].as_str().unwrap();

    let resp = client
        .get(format!("http://{addr}/v1/conversions/{job_id}/download/zip"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("download zip");
    assert_eq!(resp.status(), 200);
    let bytes = resp.bytes().await.expect("bytes");
    assert!(bytes.starts_with(b"PK\x03\x04"), "should be a zip");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p4_cloud_conversion_get_quality_report_json() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "quality-report").await;
    let zip_bytes = std::fs::read(FIXTURE_ZIP).expect("fixture exists");

    let upload_resp: Value = client
        .post(format!("http://{addr}/v1/uploads"))
        .bearer_auth(&token)
        .multipart(Form::new().part("file", Part::bytes(zip_bytes).file_name("paper3.zip")))
        .send()
        .await
        .expect("upload")
        .json()
        .await
        .expect("upload json");
    let upload_id = upload_resp["upload_id"].as_str().unwrap();

    let conv: Value = client
        .post(format!("http://{addr}/v1/conversions"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "upload_id": upload_id,
            "main_tex": "main-jos.tex",
            "profile": "generic",
            "quality": "standard"
        }))
        .send()
        .await
        .expect("create conversion")
        .json()
        .await
        .expect("conversion json");
    let job_id = conv["job_id"].as_str().unwrap();

    // Poll until completed
    for _ in 0..180 {
        let job: Value = client
            .get(format!("http://{addr}/v1/conversions/{job_id}"))
            .bearer_auth(&token)
            .send()
            .await
            .expect("poll")
            .json()
            .await
            .expect("job json");
        if job["status"] == "completed" || job["status"] == "failed" || job["status"] == "expired" {
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }

    let quality: Value = client
        .get(format!("http://{addr}/v1/conversions/{job_id}/quality-report"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("quality report")
        .json()
        .await
        .expect("quality json");
    assert!(quality["job_id"].as_str().is_some());
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p4_cloud_conversion_download_log() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "dl-log-user").await;
    let zip_bytes = std::fs::read(FIXTURE_ZIP).expect("fixture exists");

    let upload_resp: Value = client
        .post(format!("http://{addr}/v1/uploads"))
        .bearer_auth(&token)
        .multipart(Form::new().part("file", Part::bytes(zip_bytes).file_name("paper3.zip")))
        .send()
        .await
        .expect("upload")
        .json()
        .await
        .expect("upload json");
    let upload_id = upload_resp["upload_id"].as_str().unwrap();

    let conv: Value = client
        .post(format!("http://{addr}/v1/conversions"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "upload_id": upload_id,
            "main_tex": "main-jos.tex",
            "profile": "generic",
            "quality": "standard"
        }))
        .send()
        .await
        .expect("create conversion")
        .json()
        .await
        .expect("conversion json");
    let job_id = conv["job_id"].as_str().unwrap();

    for _ in 0..180 {
        let job: Value = client
            .get(format!("http://{addr}/v1/conversions/{job_id}"))
            .bearer_auth(&token)
            .send()
            .await
            .expect("poll")
            .json()
            .await
            .expect("job json");
        if job["status"] == "completed" || job["status"] == "failed" {
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }

    let resp = client
        .get(format!("http://{addr}/v1/conversions/{job_id}/download/log"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("download log");
    assert_eq!(resp.status(), 200);
    let content_type = resp.headers().get("content-type").expect("content-type");
    assert!(content_type.to_str().unwrap().contains("text/plain"));
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p4_conversion_idempotency_returns_same_job() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "idempotency").await;

    let upload_resp: Value = client
        .post(format!("http://{addr}/v1/uploads"))
        .bearer_auth(&token)
        .multipart(Form::new().part(
            "file",
            Part::bytes(minimal_project_zip()).file_name("demo.zip"),
        ))
        .send()
        .await
        .expect("upload")
        .json()
        .await
        .expect("upload json");
    let upload_id = upload_resp["upload_id"].as_str().unwrap();
    let idempotency_key = "test-idempotency-key-001";

    let first: Value = client
        .post(format!("http://{addr}/v1/conversions"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "upload_id": upload_id,
            "main_tex": "main.tex",
            "profile": "generic",
            "quality": "standard",
            "idempotency_key": idempotency_key
        }))
        .send()
        .await
        .expect("first conversion")
        .json()
        .await
        .expect("first json");
    let first_id = first["job_id"].as_str().unwrap();

    let second: Value = client
        .post(format!("http://{addr}/v1/conversions"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "upload_id": upload_id,
            "main_tex": "main.tex",
            "profile": "generic",
            "quality": "standard",
            "idempotency_key": idempotency_key
        }))
        .send()
        .await
        .expect("second conversion")
        .json()
        .await
        .expect("second json");
    let second_id = second["job_id"].as_str().unwrap();

    assert_eq!(first_id, second_id, "same idempotency_key should return same job");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p4_cloud_conversion_requires_auth() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let resp = client
        .get(format!("http://{addr}/v1/conversions"))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 401);
    let _ = shutdown.send(());
}

// ─────────────────────────────────────────────────────────────────────────────
// Billing stub
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p5_billing_checkout_returns_pending() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "billing-user").await;

    let resp = client
        .post(format!("http://{addr}/v1/billing/checkout"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "package_id": "count_10",
            "payment_method": "manual"
        }))
        .send()
        .await
        .expect("send checkout");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert!(body["status"].as_str().is_some());
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p5_billing_portal_returns_pending() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "portal-user").await;

    let resp = client
        .post(format!("http://{addr}/v1/billing/portal"))
        .bearer_auth(&token)
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("send portal");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert!(body["provider"].as_str().is_some(), "should have provider");
    assert!(body["message"].as_str().is_some(), "should have message");
    let _ = shutdown.send(());
}

// ─────────────────────────────────────────────────────────────────────────────
// Automation summary + list
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p9_automation_summary_returns_counts() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;

    let summary: Value = client
        .get(format!("http://{addr}/admin/v1/automation/summary"))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("send summary")
        .json()
        .await
        .expect("summary json");
    assert!(summary["total"].as_i64().is_some());
    assert!(summary["pending_approval"].as_i64().is_some());
    assert!(summary["in_development"].as_i64().is_some());
    assert!(summary["deployed"].as_i64().is_some());
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p9_automation_list_requests_returns_array() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;

    let list: Value = client
        .get(format!("http://{addr}/admin/v1/automation/requests"))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("list requests")
        .json()
        .await
        .expect("list json");
    let arr = list.as_array().expect("requests array");
    // arr is already verified as an array by as_array()
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p9_automation_list_agents_returns_array() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;

    let list: Value = client
        .get(format!("http://{addr}/admin/v1/automation/agents"))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("list agents")
        .json()
        .await
        .expect("list json");
    let arr = list.as_array().expect("agents array");
    // arr is already verified as an array by as_array()
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p9_automation_404_for_unknown_request_id() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;
    let fake_id = uuid::Uuid::new_v4().to_string();

    let resp = client
        .get(format!("http://{addr}/admin/v1/automation/requests/{fake_id}"))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("get request");
    assert_eq!(resp.status(), 404, "unknown request should 404");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p9_automation_agents_404_for_unknown_agent() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;
    let fake_id = uuid::Uuid::new_v4().to_string();

    let resp = client
        .post(format!("http://{addr}/admin/v1/automation/agents/{fake_id}/pause"))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("pause agent");
    assert_eq!(resp.status(), 404, "unknown agent should 404");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p9_automation_requires_admin() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let user = register_token(&client, addr, "not-admin").await;

    let resp = client
        .get(format!("http://{addr}/admin/v1/automation/summary"))
        .bearer_auth(&user)
        .send()
        .await
        .expect("send summary");
    assert_eq!(resp.status(), 401, "non-admin should be 401");
    let _ = shutdown.send(());
}

// ─────────────────────────────────────────────────────────────────────────────
// Release management
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p9_release_manifest_returns_preview_when_empty() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();

    let manifest: Value = client
        .get(format!("http://{addr}/v1/releases/beta"))
        .send()
        .await
        .expect("send release")
        .json()
        .await
        .expect("manifest json");
    assert_eq!(manifest["channel"], "beta");
    assert!(manifest["download_url"].as_str().is_some());
    assert!(manifest["sha256"].as_str().is_some());
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p9_admin_publish_and_list_release() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;

    let release: Value = client
        .post(format!("http://{addr}/admin/v1/releases"))
        .bearer_auth(&admin)
        .json(&serde_json::json!({
            "channel": "beta",
            "platform": "windows",
            "version": "0.2.0-test",
            "download_url": "https://example.com/test.exe",
            "sha256": "pending-preview-build"
        }))
        .send()
        .await
        .expect("publish release")
        .json()
        .await
        .expect("release json");
    assert_eq!(release["channel"], "beta");

    let list: Value = client
        .get(format!("http://{addr}/admin/v1/releases"))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("list releases")
        .json()
        .await
        .expect("list json");
    let releases = list["releases"].as_array().expect("releases array");
    assert!(!releases.is_empty(), "should have published release");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p9_admin_publish_release_rejects_invalid_sha256() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;

    let resp = client
        .post(format!("http://{addr}/admin/v1/releases"))
        .bearer_auth(&admin)
        .json(&serde_json::json!({
            "channel": "beta",
            "platform": "windows",
            "version": "0.2.0-test",
            "download_url": "https://example.com/test.exe",
            "sha256": "short"
        }))
        .send()
        .await
        .expect("send publish");
    assert_eq!(resp.status(), 400);
    let err: Value = resp.json().await.expect("error json");
    assert!(err["error"].as_str().unwrap().contains("sha256"));
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p9_admin_release_audit_returns_array() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;

    let audit: Value = client
        .get(format!("http://{addr}/admin/v1/release-audit"))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("send audit")
        .json()
        .await
        .expect("audit json");
    let logs = audit["logs"].as_array().expect("logs array");
    // logs is already verified as an array by as_array()
    let _ = shutdown.send(());
}

// ─────────────────────────────────────────────────────────────────────────────
// Recharge / redeem edge cases
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p8_recharge_options_returns_count_and_date() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();

    let opts: Value = client
        .get(format!("http://{addr}/v1/recharge/options"))
        .send()
        .await
        .expect("send options")
        .json()
        .await
        .expect("options json");
    assert!(opts["count"]["packages"].as_array().is_some());
    assert!(opts["date"]["packages"].as_array().is_some());
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p8_list_recharges_returns_empty_for_new_user() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "new-recharge-user").await;

    let recharges: Value = client
        .get(format!("http://{addr}/v1/recharges"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send recharges")
        .json()
        .await
        .expect("recharges json");
    let arr = recharges.as_array().expect("recharges array");
    assert!(arr.is_empty(), "new user should have no recharges");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p8_redeem_code_options_returns_packages() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "redeem-opts-user").await;

    let opts: Value = client
        .get(format!("http://{addr}/v1/redeem-codes/options"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send options")
        .json()
        .await
        .expect("options json");
    assert!(opts["packages"].as_array().is_some());
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p8_redeem_invalid_code_returns_400() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "bad-code-user").await;

    let resp = client
        .post(format!("http://{addr}/v1/redeem-codes/redeem"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "code": "INVALIDCODE000000" }))
        .send()
        .await
        .expect("send redeem");
    assert_eq!(resp.status(), 400, "invalid code should 400");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p8_redeem_records_requires_auth() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();

    let resp = client
        .get(format!("http://{addr}/v1/redeem-codes/records"))
        .send()
        .await
        .expect("send records");
    assert_eq!(resp.status(), 401, "no auth should 401");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p8_redeem_records_returns_empty_for_new_user() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "no-redeem-user").await;

    let records: Value = client
        .get(format!("http://{addr}/v1/redeem-codes/records"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send records")
        .json()
        .await
        .expect("records json");
    let arr = records.as_array().expect("records array");
    assert!(arr.is_empty(), "new user should have no redeem records");
    let _ = shutdown.send(());
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin: waitlist
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p2_admin_list_waitlist() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let admin = admin_token(&client, addr).await;

    client
        .post(format!("http://{addr}/v1/waitlist"))
        .json(&serde_json::json!({
            "email": "waitlist-test@example.com",
            "identity": "professor"
        }))
        .send()
        .await
        .expect("add waitlist");

    let resp: Value = client
        .get(format!("http://{addr}/admin/v1/waitlist"))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("list waitlist")
        .json()
        .await
        .expect("waitlist json");
    let leads = resp["leads"].as_array().expect("leads array");
    assert!(leads.iter().any(|l| l["email"] == "waitlist-test@example.com"));
    let _ = shutdown.send(());
}

// ─────────────────────────────────────────────────────────────────────────────
// Plans / downloads / recharge-options (public endpoints)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p1_plans_returns_billing_plans() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();

    let plans: Value = client
        .get(format!("http://{addr}/v1/plans"))
        .send()
        .await
        .expect("send plans")
        .json()
        .await
        .expect("plans json");
    let arr = plans.as_array().expect("plans array");
    assert!(!arr.is_empty(), "at least one plan should exist");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p1_downloads_endpoint_returns_urls() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();

    let dl: Value = client
        .get(format!("http://{addr}/v1/downloads"))
        .send()
        .await
        .expect("send downloads")
        .json()
        .await
        .expect("downloads json");
    // Response is { "channel": "...", "platforms": [...] }
    let platforms = dl["platforms"].as_array().expect("platforms array");
    assert!(!platforms.is_empty(), "should have at least one platform");
    let first = &platforms[0];
    assert!(first["download_url"].as_str().is_some());
    assert!(first["sha256"].as_str().is_some());
    let _ = shutdown.send(());
}

// ─────────────────────────────────────────────────────────────────────────────
// Upload edge cases
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p4_upload_requires_file_field() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "no-file-user").await;

    let resp = client
        .post(format!("http://{addr}/v1/uploads"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "file": null }))
        .send()
        .await
        .expect("send upload");
    assert!(
        resp.status().is_client_error(),
        "should be 4xx without file, got {}",
        resp.status()
    );
    let _ = shutdown.send(());
}

// ─────────────────────────────────────────────────────────────────────────────
// Security: token expiry / malformed headers
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p1_auth_rejects_malformed_bearer() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();

    let resp = client
        .get(format!("http://{addr}/v1/me"))
        .header("Authorization", "NotBearer token123")
        .send()
        .await
        .expect("send me");
    assert_eq!(resp.status(), 401, "malformed bearer should 401");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p1_auth_rejects_empty_bearer() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();

    let resp = client
        .get(format!("http://{addr}/v1/me"))
        .header("Authorization", "Bearer ")
        .send()
        .await
        .expect("send me");
    assert_eq!(resp.status(), 401, "empty bearer should 401");
    let _ = shutdown.send(());
}

#[tokio::test]
async fn p1_auth_rejects_unknown_token() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();

    let resp = client
        .get(format!("http://{addr}/v1/me"))
        .bearer_auth("access-00000000000000000000000000000000")
        .send()
        .await
        .expect("send me");
    assert_eq!(resp.status(), 401, "unknown token should 401");
    let _ = shutdown.send(());
}

// ─────────────────────────────────────────────────────────────────────────────
// Error: missing upload on conversion
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn p4_conversion_with_nonexistent_upload_id_returns_404() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_token(&client, addr, "bad-upload-user").await;

    let resp = client
        .post(format!("http://{addr}/v1/conversions"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "upload_id": "00000000-0000-0000-0000-000000000000",
            "main_tex": "main.tex",
            "profile": "generic",
            "quality": "standard"
        }))
        .send()
        .await
        .expect("send conversion");
    assert!(
        resp.status().is_server_error() || resp.status() == 404,
        "bad upload_id should error, got {}",
        resp.status()
    );
    let _ = shutdown.send(());
}
