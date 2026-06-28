//! doc-server 端到端集成测试
//!
//! - 复用 [examples/paper3/upload.zip] 作为夹具
//! - 覆盖：health / version / 成功 convert / 缺 file 字段 / 主文件找不到 / 超大 body

use std::io::{Cursor, Write};
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
    let app = build_router().await.expect("build router with database");
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
        .timeout(Duration::from_secs(30))
        .build()
        .expect("build reqwest client with timeout")
}

fn minimal_project_zip() -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("minimal.tex", options)
            .expect("start minimal tex");
        zip.write_all(br#"\documentclass{article}\begin{document}Hello Tex2Doc\end{document}"#)
            .expect("write minimal tex");
        zip.finish().expect("finish zip");
    }
    cursor.into_inner()
}

fn assert_uuid(value: &str) {
    uuid::Uuid::parse_str(value).expect("value should be a UUID");
}

async fn register_preview_token(client: &reqwest::Client, addr: SocketAddr) -> String {
    let email = format!("demo-{}@example.com", uuid::Uuid::new_v4().simple());
    let auth: serde_json::Value = client
        .post(format!("http://{addr}/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "secret",
            "display_name": "Demo User"
        }))
        .send()
        .await
        .expect("send register")
        .json()
        .await
        .expect("register json");
    auth["access_token"].as_str().unwrap().to_string()
}

async fn admin_token(client: &reqwest::Client, addr: SocketAddr) -> String {
    let auth: serde_json::Value = client
        .post(format!("http://{addr}/v1/auth/login"))
        .json(&serde_json::json!({
            "email": "admin@example.com",
            "password": "admin-secret"
        }))
        .send()
        .await
        .expect("send admin login")
        .json()
        .await
        .expect("admin login json");
    assert_eq!(auth["user"]["role"], "admin");
    auth["access_token"].as_str().unwrap().to_string()
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
async fn p6_commercial_contract_endpoints_return_json() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let email = format!("contract-{}@example.com", uuid::Uuid::new_v4().simple());

    let auth: serde_json::Value = client
        .post(format!("http://{addr}/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "secret",
            "display_name": "Demo User"
        }))
        .send()
        .await
        .expect("send register")
        .json()
        .await
        .expect("register json");
    assert!(auth["access_token"]
        .as_str()
        .unwrap()
        .starts_with("access-"));
    assert_eq!(auth["user"]["plan_id"], "preview");
    let token = auth["access_token"].as_str().unwrap().to_string();

    let usage: serde_json::Value = client
        .get(format!("http://{addr}/v1/usage"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send usage")
        .json()
        .await
        .expect("usage json");
    assert_eq!(usage["plan_id"], "preview");
    assert!(usage["cloud_conversions_limit"].as_u64().unwrap() > 0);

    let plans: serde_json::Value = client
        .get(format!("http://{addr}/v1/plans"))
        .send()
        .await
        .expect("send plans")
        .json()
        .await
        .expect("plans json");
    assert!(plans.as_array().unwrap().iter().any(|p| p["id"] == "pro"));

    let upload_resp: serde_json::Value = client
        .post(format!("http://{addr}/v1/uploads"))
        .bearer_auth(&token)
        .multipart(Form::new().part(
            "file",
            Part::bytes(minimal_project_zip()).file_name("demo.zip"),
        ))
        .send()
        .await
        .expect("send upload")
        .json()
        .await
        .expect("upload json");
    let upload_id = upload_resp["upload_id"].as_str().unwrap().to_string();
    assert_uuid(&upload_id);

    let conversion: serde_json::Value = client
        .post(format!("http://{addr}/v1/conversions"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "upload_id": upload_id,
            "main_tex": "minimal.tex",
            "profile": "generic",
            "quality": "standard"
        }))
        .send()
        .await
        .expect("send conversion")
        .json()
        .await
        .expect("conversion json");
    let job_id = conversion["job_id"].as_str().unwrap().to_string();
    assert_uuid(&job_id);
    assert!(matches!(
        conversion["status"].as_str().unwrap(),
        "queued"
            | "normalizing"
            | "detecting"
            | "analyzing"
            | "compiling"
            | "rendering"
            | "verifying"
    ));
    assert_eq!(conversion["docx_ready"], false);
    assert_eq!(conversion["engine"], "semantic-engine");

    let usage_after: serde_json::Value = client
        .get(format!("http://{addr}/v1/usage"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send usage after conversion")
        .json()
        .await
        .expect("usage after json");
    assert_eq!(usage_after["cloud_conversions_used"], 1);

    let release: serde_json::Value = client
        .get(format!("http://{addr}/v1/releases/beta"))
        .send()
        .await
        .expect("send release")
        .json()
        .await
        .expect("release json");
    assert_eq!(release["channel"], "beta");
    let sha256 = release["sha256"].as_str().unwrap();
    assert!(
        sha256.len() == 64 || sha256 == "pending-preview-build",
        "sha256 should be 64-char hex or pending-placeholder, got: {sha256}"
    );
    assert!(release["download_url"]
        .as_str()
        .unwrap()
        .starts_with("https://"));

    let _ = shutdown.send(());
}

#[tokio::test]
async fn p6_login_endpoint_accepts_registered_account() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let email = format!("login-{}@example.com", uuid::Uuid::new_v4().simple());

    let _registered: serde_json::Value = client
        .post(format!("http://{addr}/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "secret"
        }))
        .send()
        .await
        .expect("send register")
        .json()
        .await
        .expect("register json");

    let auth: serde_json::Value = client
        .post(format!("http://{addr}/v1/auth/login"))
        .json(&serde_json::json!({
            "email": email,
            "password": "secret"
        }))
        .send()
        .await
        .expect("send login")
        .json()
        .await
        .expect("login json");
    let token = auth["access_token"].as_str().unwrap().to_string();
    assert!(token.starts_with("access-"));
    assert_eq!(auth["user"]["plan_id"], "preview");

    let usage: serde_json::Value = client
        .get(format!("http://{addr}/v1/usage"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send usage with login token")
        .json()
        .await
        .expect("usage json");
    assert_eq!(usage["plan_id"], "preview");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn p6_admin_me_requires_admin_role() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let user_email = format!("user-{}@example.com", uuid::Uuid::new_v4().simple());

    let _registered: serde_json::Value = client
        .post(format!("http://{addr}/v1/auth/register"))
        .json(&serde_json::json!({
            "email": user_email,
            "password": "secret"
        }))
        .send()
        .await
        .expect("send user register")
        .json()
        .await
        .expect("user register json");

    let user_auth: serde_json::Value = client
        .post(format!("http://{addr}/v1/auth/login"))
        .json(&serde_json::json!({
            "email": user_email,
            "password": "secret"
        }))
        .send()
        .await
        .expect("send user login")
        .json()
        .await
        .expect("user login json");
    let user_token = user_auth["access_token"].as_str().unwrap();

    let denied = client
        .get(format!("http://{addr}/admin/v1/me"))
        .bearer_auth(user_token)
        .send()
        .await
        .expect("send admin me with user token");
    assert_eq!(denied.status(), reqwest::StatusCode::UNAUTHORIZED);

    let login_admin: serde_json::Value = client
        .post(format!("http://{addr}/v1/auth/login"))
        .json(&serde_json::json!({
            "email": "admin@example.com",
            "password": "admin-secret"
        }))
        .send()
        .await
        .expect("send admin login")
        .json()
        .await
        .expect("admin login json");
    let admin_access_token = login_admin["access_token"].as_str().unwrap();
    assert_eq!(login_admin["user"]["role"], "admin");

    let admin_me_with_login: serde_json::Value = client
        .get(format!("http://{addr}/admin/v1/me"))
        .bearer_auth(admin_access_token)
        .send()
        .await
        .expect("send admin me with login token")
        .json()
        .await
        .expect("admin me login json");
    assert_eq!(admin_me_with_login["user"]["email"], "admin@example.com");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn p6_refresh_token_rotates_and_revokes_old_token() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let email = format!("refresh-{}@example.com", uuid::Uuid::new_v4().simple());

    let auth: serde_json::Value = client
        .post(format!("http://{addr}/v1/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": "secret"
        }))
        .send()
        .await
        .expect("send register")
        .json()
        .await
        .expect("register json");
    let refresh_token = auth["refresh_token"].as_str().unwrap().to_string();

    let refreshed: serde_json::Value = client
        .post(format!("http://{addr}/v1/auth/refresh"))
        .json(&serde_json::json!({ "refresh_token": refresh_token }))
        .send()
        .await
        .expect("send refresh")
        .json()
        .await
        .expect("refresh json");
    assert!(refreshed["access_token"]
        .as_str()
        .unwrap()
        .starts_with("access-"));
    let rotated_refresh = refreshed["refresh_token"].as_str().unwrap();
    assert!(rotated_refresh.starts_with("refresh-"));

    let old_refresh = client
        .post(format!("http://{addr}/v1/auth/refresh"))
        .json(&serde_json::json!({ "refresh_token": auth["refresh_token"] }))
        .send()
        .await
        .expect("send old refresh");
    assert_eq!(old_refresh.status(), 401);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn p6_commercial_user_endpoints_require_bearer_token() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();

    let usage = client
        .get(format!("http://{addr}/v1/usage"))
        .send()
        .await
        .expect("send usage without auth");
    assert_eq!(usage.status(), 401);

    let upload = client
        .post(format!("http://{addr}/v1/uploads"))
        .multipart(Form::new().part("file", Part::bytes(vec![1, 2, 3, 4]).file_name("demo.zip")))
        .send()
        .await
        .expect("send upload without auth");
    assert_eq!(upload.status(), 401);

    let conversion = client
        .post(format!("http://{addr}/v1/conversions"))
        .json(&serde_json::json!({
            "upload_id": "upload_demo",
            "main_tex": "minimal.tex"
        }))
        .send()
        .await
        .expect("send conversion without auth");
    assert_eq!(conversion.status(), 401);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn p8_recharge_count_entitlement_is_consumed_before_preview_quota() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_preview_token(&client, addr).await;

    let recharge: serde_json::Value = client
        .post(format!("http://{addr}/v1/recharges"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "recharge_type": "count",
            "package_id": "count_3",
            "quantity": 3
        }))
        .send()
        .await
        .expect("send recharge")
        .json()
        .await
        .expect("recharge json");
    assert_eq!(recharge["status"], "pending_manual");
    assert_eq!(recharge["quantity"], 3);

    let usage_after_recharge: serde_json::Value = client
        .get(format!("http://{addr}/v1/usage"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send usage after recharge")
        .json()
        .await
        .expect("usage after recharge json");
    assert_eq!(usage_after_recharge["plan_id"], "count");
    assert_eq!(usage_after_recharge["count_balance"], 3);
    assert_eq!(usage_after_recharge["cloud_conversions_used"], 0);

    let upload_resp: serde_json::Value = client
        .post(format!("http://{addr}/v1/uploads"))
        .bearer_auth(&token)
        .multipart(Form::new().part(
            "file",
            Part::bytes(minimal_project_zip()).file_name("demo.zip"),
        ))
        .send()
        .await
        .expect("send upload")
        .json()
        .await
        .expect("upload json");
    let upload_id = upload_resp["upload_id"].as_str().unwrap().to_string();

    let conversion: serde_json::Value = client
        .post(format!("http://{addr}/v1/conversions"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "upload_id": upload_id,
            "main_tex": "minimal.tex",
            "profile": "generic",
            "quality": "standard"
        }))
        .send()
        .await
        .expect("send conversion")
        .json()
        .await
        .expect("conversion json");
    assert_uuid(conversion["job_id"].as_str().unwrap());

    let usage_after_conversion: serde_json::Value = client
        .get(format!("http://{addr}/v1/usage"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send usage after conversion")
        .json()
        .await
        .expect("usage after conversion json");
    assert_eq!(usage_after_conversion["plan_id"], "count");
    assert_eq!(usage_after_conversion["count_balance"], 2);
    assert_eq!(usage_after_conversion["cloud_conversions_used"], 0);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn redeem_code_batch_exports_and_redeems_once() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_preview_token(&client, addr).await;
    let admin = admin_token(&client, addr).await;

    let batch: serde_json::Value = client
        .post(format!("http://{addr}/admin/v1/redeem-code-batches"))
        .bearer_auth(&admin)
        .json(&serde_json::json!({
            "package_id": "count_10",
            "quantity": 2,
            "channel": "test",
            "note": "integration"
        }))
        .send()
        .await
        .expect("send create redeem batch")
        .json()
        .await
        .expect("batch json");
    assert_eq!(batch["package_id"], "count_10");
    assert_eq!(batch["generated_count"], 2);
    let code = batch["codes"][0].as_str().unwrap().to_string();
    let batch_id = batch["batch_id"].as_str().unwrap().to_string();

    let export = client
        .get(format!(
            "http://{addr}/admin/v1/redeem-code-batches/{batch_id}/export.xlsx"
        ))
        .bearer_auth(&admin)
        .send()
        .await
        .expect("send export redeem batch");
    assert_eq!(export.status(), 200);
    let export_bytes = export.bytes().await.expect("export bytes");
    assert!(export_bytes.starts_with(b"PK\x03\x04"));

    let redeemed: serde_json::Value = client
        .post(format!("http://{addr}/v1/redeem-codes/redeem"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "code": code }))
        .send()
        .await
        .expect("send redeem code")
        .json()
        .await
        .expect("redeem json");
    assert_eq!(redeemed["package_id"], "count_10");
    assert_eq!(redeemed["quantity"], 10);
    assert_eq!(redeemed["count_balance"], 10);

    let usage: serde_json::Value = client
        .get(format!("http://{addr}/v1/usage"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send usage after redeem")
        .json()
        .await
        .expect("usage after redeem json");
    assert_eq!(usage["plan_id"], "count");
    assert_eq!(usage["count_balance"], 10);

    let duplicate = client
        .post(format!("http://{addr}/v1/redeem-codes/redeem"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "code": batch["codes"][0] }))
        .send()
        .await
        .expect("send duplicate redeem");
    assert_eq!(duplicate.status(), 409);
    let duplicate_body: serde_json::Value = duplicate.json().await.expect("duplicate json");
    assert_eq!(duplicate_body["error"], "code_already_redeemed");

    let records: serde_json::Value = client
        .get(format!("http://{addr}/v1/redeem-codes/records"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send redeem records")
        .json()
        .await
        .expect("redeem records json");
    assert_eq!(records.as_array().unwrap().len(), 1);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn p7_cloud_worker_converts_uploaded_zip() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_preview_token(&client, addr).await;
    let zip_bytes = std::fs::read(FIXTURE_ZIP).expect("paper3 upload.zip must exist");

    let upload_resp: serde_json::Value = client
        .post(format!("http://{addr}/v1/uploads"))
        .bearer_auth(&token)
        .multipart(Form::new().part("file", Part::bytes(zip_bytes).file_name("paper3.zip")))
        .send()
        .await
        .expect("send upload")
        .json()
        .await
        .expect("upload json");
    let upload_id = upload_resp["upload_id"].as_str().unwrap().to_string();

    let conversion: serde_json::Value = client
        .post(format!("http://{addr}/v1/conversions"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "upload_id": upload_id,
            "main_tex": "main-jos.tex",
            "profile": "jos-paper",
            "quality": "standard",
            "engine": "semantic-engine"
        }))
        .send()
        .await
        .expect("send conversion")
        .json()
        .await
        .expect("conversion json");
    let job_id = conversion["job_id"].as_str().unwrap().to_string();

    let mut final_job = conversion;
    for _ in 0..180 {
        if matches!(
            final_job["status"].as_str().unwrap(),
            "completed" | "failed" | "expired"
        ) {
            break;
        }
        sleep(Duration::from_millis(500)).await;
        final_job = client
            .get(format!("http://{addr}/v1/conversions/{job_id}"))
            .bearer_auth(&token)
            .send()
            .await
            .expect("poll conversion")
            .json()
            .await
            .expect("conversion json");
    }

    assert_eq!(
        final_job["status"], "completed",
        "conversion should complete: {final_job:?}"
    );
    assert_eq!(final_job["docx_ready"], true);
    assert_eq!(final_job["report_ready"], true);

    let report: serde_json::Value = client
        .get(format!("http://{addr}/v1/conversions/{job_id}/report"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send conversion report")
        .json()
        .await
        .expect("report json");
    assert_eq!(report["job_id"], job_id);
    assert!(matches!(
        report["profile"].as_str().unwrap(),
        "jos-paper" | "jos-paper-toml"
    ));
    assert_eq!(report["executor"], "semantic-engine");
    assert!(!report["backend"].as_str().unwrap().is_empty());
    assert!(report["quality_score"].as_u64().is_some());
    assert!(!report["quality_status"].as_str().unwrap().is_empty());
    assert!(report["compatibility_score"].as_u64().is_some());
    assert!(report["docx_bytes"].as_u64().unwrap() > 4 * 1024);

    let docx = client
        .get(format!(
            "http://{addr}/v1/conversions/{job_id}/download/docx"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send docx")
        .bytes()
        .await
        .expect("docx bytes");
    assert_eq!(&docx[..4], b"PK\x03\x04");
    assert!(docx.len() > 4 * 1024, "docx too small: {}", docx.len());

    let _ = shutdown.send(());
}

#[tokio::test]
async fn p7_failed_cloud_conversion_returns_error_code_and_report() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_preview_token(&client, addr).await;

    let upload_resp: serde_json::Value = client
        .post(format!("http://{addr}/v1/uploads"))
        .bearer_auth(&token)
        .multipart(Form::new().part(
            "file",
            Part::bytes(minimal_project_zip()).file_name("minimal.zip"),
        ))
        .send()
        .await
        .expect("send upload")
        .json()
        .await
        .expect("upload json");
    let upload_id = upload_resp["upload_id"].as_str().unwrap().to_string();

    let conversion: serde_json::Value = client
        .post(format!("http://{addr}/v1/conversions"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "upload_id": upload_id,
            "main_tex": "missing.tex",
            "profile": "generic",
            "quality": "standard"
        }))
        .send()
        .await
        .expect("send conversion")
        .json()
        .await
        .expect("conversion json");
    let job_id = conversion["job_id"].as_str().unwrap().to_string();

    let mut failed_job = None;
    for _ in 0..40 {
        let job: serde_json::Value = client
            .get(format!("http://{addr}/v1/conversions/{job_id}"))
            .bearer_auth(&token)
            .send()
            .await
            .expect("poll conversion")
            .json()
            .await
            .expect("job json");
        if job["status"] == "failed" {
            failed_job = Some(job);
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }

    let failed_job = failed_job.expect("job should fail");
    assert_eq!(failed_job["error_code"], "convert_failed");
    assert!(failed_job["error"].as_str().unwrap().contains("missing"));

    let report: serde_json::Value = client
        .get(format!("http://{addr}/v1/conversions/{job_id}/report"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("get report")
        .json()
        .await
        .expect("report json");
    assert_eq!(report["status"], "failed");
    assert_eq!(report["error_code"], "convert_failed");
    assert_eq!(report["docx_bytes"], 0);

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

    // v13.2 F14: paper3 docx 期望 ≥1MB（含 10 张嵌入 PNG）。
    //   upload.zip 缺 figures 时 docx 约 46KB（无图）。回归测试兜底。
    assert!(
        bytes.len() > 1024 * 1024,
        "paper3 docx 太小（{} bytes）—— 疑似 upload.zip 缺 figures 嵌入图片",
        bytes.len()
    );
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

#[tokio::test]
async fn p8_local_conversion_quota_checking_and_consumption() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();
    let token = register_preview_token(&client, addr).await;

    // Get user id
    let me_resp: serde_json::Value = client
        .get(format!("http://{addr}/v1/me"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send me")
        .json()
        .await
        .expect("me json");
    let user_id = me_resp["id"].as_str().unwrap();

    // 1. Check local conversion quota — new users ARE allowed because they have
    //    `used=0 < PREVIEW_CLOUD_CONVERSION_LIMIT`.  The quota check is on consume.
    let check_resp: serde_json::Value = client
        .post(format!("http://{addr}/v1/local-conversions/check"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send check")
        .json()
        .await
        .expect("check json");

    assert_eq!(check_resp["allowed"], true);
    assert_eq!(check_resp["count_balance"], 0);

    // 2. Consume should fail with 402
    let consume_fail_resp = client
        .post(format!("http://{addr}/v1/local-conversions/consume"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send consume fail");
    assert_eq!(consume_fail_resp.status(), 402);

    // 3. Recharge via admin
    let admin = admin_token(&client, addr).await;
    let _order_resp = client
        .post(format!("http://{addr}/admin/v1/manual-orders"))
        .bearer_auth(&admin)
        .json(&serde_json::json!({
            "user_id": user_id,
            "package_id": "count_10",
            "recharge_type": "count"
        }))
        .send()
        .await
        .expect("send manual order")
        .json::<serde_json::Value>()
        .await
        .expect("manual order json");

    // 4. Check should now be allowed
    let check2_resp: serde_json::Value = client
        .post(format!("http://{addr}/v1/local-conversions/check"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send check 2")
        .json()
        .await
        .expect("check 2 json");
    assert_eq!(check2_resp["allowed"], true);
    assert_eq!(check2_resp["count_balance"], 10);

    // 5. Consume should succeed
    let consume_resp: serde_json::Value = client
        .post(format!("http://{addr}/v1/local-conversions/consume"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("send consume")
        .json()
        .await
        .expect("consume json");
    assert_eq!(consume_resp["consumed"], true);
    assert_eq!(consume_resp["balance"], 9);

    let _ = shutdown.send(());
}
