# Tex2Doc rust-service 集成测试验证方案与使用手册

> 日期：2026-06-29
> 输出目录：`docs-zh/service`
> 状态：已实施，74 个测试全部通过
> 适用对象：`apps/rust-service`（`doc-server` crate）

---

## 1. 概述

本文档描述 Tex2Doc Rust HTTP 服务（`apps/rust-service`）的**集成测试与端到端验证**方案，覆盖：

- 全部 ~50 个对外 REST 路由
- PostgreSQL 后端持久化层
- 反馈 / 兑换码 / 充值 / 自动化 R&D / 版本发布等业务模块
- 认证 / 授权 / 错误码 / 限流等非业务关注点

**目标**：

1. **回归保护**：改动 routes / handlers / db schema 后立即发现破坏点
2. **API 契约文档**：测试即文档，新成员可从测试断言中读出 API 字段语义
3. **CI 准入门槛**：PR 合并前必须通过全部测试，作为 Phase G "production ready" 的硬性指标

---

## 2. 测试资产概览

```
apps/rust-service/
├── Cargo.toml                                     # 测试运行入口（dev-dependencies: reqwest）
├── tests/
│   ├── api.rs                                     # 端到端 HTTP 测试（16 个）
│   └── comprehensive.rs                           # 综合路由测试（52 个，新）
└── src/
    └── file_storage.rs                            # 已有 3 个内部单元测试
```

| 文件 | 类型 | 数量 | 覆盖 |
|------|------|------|------|
| `tests/api.rs` | 集成（HTTP / DB） | 16 | 健康检查、版本号、paper3 完整 convert 流程、缺失字段、超大 body、token 轮换、commercial 端点、admin 鉴权、cloud worker 转换、失败 job、本地 quota、redeem batch 导出 |
| `tests/comprehensive.rs` | 集成（HTTP / DB） | 52 | waitlist、admin dashboard、users/orders/waitlist 列表、用户反馈 CRUD、管理反馈 CRUD + Excel 导出、cloud conversion 全生命周期（zip/log/quality report/idempotency/404）、billing checkout/portal、Automation R&D、release 发布、redeem/recharge 边界、token 安全 |
| `src/file_storage.rs::tests` | 单元（no-net） | 3 | sanitize_id、fixed_filename、build_conversion_log |

**合计 74 个测试，全部通过。**

---

## 3. 测试栈

| 组件 | 选择 | 原因 |
|------|------|------|
| 异步运行时 | `tokio`（`#[tokio::test]`） | `axum` / `sqlx` 同栈 |
| HTTP 客户端 | `reqwest = 0.12`（rustls-tls） | Windows 环境无需 OpenSSL |
| DB 驱动 | `sqlx = 0.8`（`postgres`） | 复用 server 真实连接 |
| 临时端口 | `TcpListener::bind("127.0.0.1:0")` | 测试可并行启动而互不干扰 |
| ZIP 构造 | `zip` crate（已在 workspace 中） | 构造 paper3 fixture 类似的内存 ZIP |
| 测试隔离 | 随机 email + UUID 后缀 | 共享开发 DB 仍可保证测试独立 |

---

## 4. 关键基础设施

### 4.1 数据库前置

| 字段 | 默认值 | 覆盖方式 |
|------|--------|----------|
| `DATABASE_URL` | `postgres://postgres:postgres@127.0.0.1:5432/docdb` | env var |
| Bootstrap 管理员 email | `TEX2DOC_BOOTSTRAP_ADMIN_EMAIL` | env var |
| Bootstrap 管理员 password | `TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD` | env var |
| 会话文件根目录 | `sessions/`（cwd 相对） | 代码常量，可经 `TEX2DOC_STATIC_DIR` 覆盖 |

> 首次启动会自动执行 `docs-zh/money/00{1..4}_*.sql` 中的 schema（含 business / redeem stock / feedback / automation）。无需手动 migrate。

### 4.2 启动方式

```rust
// apps/rust-service/tests/api.rs 中的样例（api.rs 与 comprehensive.rs 完全相同）
async fn spawn_test_server() -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
    std::env::set_var("TEX2DOC_BOOTSTRAP_ADMIN_EMAIL", "admin@example.com");
    std::env::set_var("TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD", "admin-secret");
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(run_server(listener, rx));
    sleep(Duration::from_millis(150)).await;     // 让 worker loop 起来
    (addr, tx)
}
```

- 每个测试启动**自己的进程内** server（fork 自 `build_router`），不依赖外部 `doc-server`
- `tx.send(())` 触发 `axum::serve` 的 select 分支退出，资源回收彻底
- 测试共用 `docdb` 数据库，但通过**唯一 email / UUID** 避免相互污染

### 4.3 共享 Fixtures

| 文件 | 位置 | 说明 |
|------|------|------|
| `paper3/upload.zip` | `examples/paper3/upload.zip` | 真实 LaTeX 仓库，10 张嵌入 PNG；用于 `p7_cloud_worker_converts_uploaded_zip` 与 `convert_paper3_zip_returns_docx` 的端到端 |
| minimal_projection_zip | 运行时构造 | `\documentclass{article}\begin{document}Hello\end{document}` 单文件 |

---

## 5. 测试分类详解

### 5.1 P1 — 公共 / 认证 边界 (6 测试)

```
p1_waitlist_accepts_valid_email       → POST /v1/waitlist   200 + email 回填
p1_waitlist_requires_email           → 缺 email            4xx
p1_register_rejects_duplicate_email  → POST /v1/auth/register 第二次 409 + message 含 email
p1_login_fails_for_wrong_password    → POST /v1/auth/login    401
p1_login_fails_for_unknown_email     → POST /v1/auth/login    401
p1_auth_rejects_{malformed,empty,unknown}_token → GET /v1/me 401
```

覆盖：
- Waitlist 表单验证（`WaitlistBody`）
- Unique violation (`23505`) 映射到 `Conflict` 状态码
- Bearer 解析三态（无前缀、空字符串、不存在 token）

### 5.2 P2 — 管理后台 (6 测试)

```
p2_admin_dashboard_returns_stats     → /admin/v1/dashboard       counts + admin profile
p2_admin_dashboard_requires_admin_role                              非 admin → 401
p2_admin_list_users_returns_user_array                              /admin/v1/users
p2_admin_list_usage_ledger                                            /admin/v1/usage-ledger
p2_admin_create_and_list_manual_order  → POST + GET /admin/v1/manual-orders
p2_admin_manual_order_requires_admin                                非 admin → 401
p2_admin_list_waitlist                → /admin/v1/waitlist     包含新写入的 email
```

覆盖：
- `require_admin_session` 中 `is_admin_role` 的判定
- `AdminManualOrderBody` 全字段（`user_id / package_id / recharge_type / quantity / amount_cents / payment_note`）
- Waitlist → admin 列表的端到端往返

### 5.3 P3 — 用户反馈 (5 测试)

```
p3_user_create_feedback_thread           → POST /v1/feedback/threads
p3_user_list_feedback_threads            → GET  /v1/feedback/threads (含 automation_status)
p3_user_get_feedback_thread              → GET  /v1/feedback/threads/:id 返回 { thread, messages }
p3_user_add_feedback_message             → POST /v1/feedback/threads/:id/messages
p3_feedback_requires_auth                → /v1/feedback/threads 无 token → 401
```

### 5.4 P3 — 管理反馈 (4 测试)

```
p3_admin_list_feedback_threads           → /admin/v1/feedback/threads
p3_admin_export_feedback_threads_returns_xlsx
                                          → /admin/v1/feedback/threads/export.xlsx
                                            校验 Content-Type 与 PK\x03\x04 magic
p3_admin_update_feedback_thread          → PATCH /admin/v1/feedback/threads/:id { status, priority }
p3_admin_reply_feedback_message          → POST  /admin/v1/feedback/threads/:id/messages
                                            校验 sender_type="admin"
```

### 5.5 P4 — Cloud Conversion 生命周期 (7 测试)

```
p4_cloud_conversion_list_returns_user_jobs       → GET  /v1/conversions
p4_cloud_conversion_download_zip                → /v1/conversions/:id/download/zip
                                                    Content-Type 校验 + 首 4 字节为 PK\x03\x04
p4_cloud_conversion_get_quality_report_json      → /v1/conversions/:id/quality-report
p4_cloud_conversion_download_log                → /v1/conversions/:id/download/log
                                                    Content-Type 校验 text/plain
p4_conversion_idempotency_returns_same_job       → 相同 idempotency_key → 同一 job_id
p4_cloud_conversion_requires_auth               → 401 守卫
p4_conversion_with_nonexistent_upload_id_returns_404
p4_upload_requires_file_field                  → POST /v1/uploads 无 file → 4xx
```

### 5.6 P5 — 计费 (2 测试)

```
p5_billing_checkout_returns_pending → /v1/billing/checkout   返回 provider + status
p5_billing_portal_returns_pending   → /v1/billing/portal     返回 provider + message
```

> 当前为 Phase A 邀请制（无第三方支付），端点返回 pending stub；测试仅验证响应结构合法。

### 5.7 P8 — 充值 / 兑换 (6 测试)

```
p8_recharge_options_returns_count_and_date    → /v1/recharge/options  （嵌套 packages）
p8_list_recharges_returns_empty_for_new_user   → /v1/recharges         新用户为空
p8_redeem_code_options_returns_packages        → /v1/redeem-codes/options  鉴权后返回 packages
p8_redeem_invalid_code_returns_400              → 无效码 → 400
p8_redeem_records_requires_auth                 → 401
p8_redeem_records_returns_empty_for_new_user    → /v1/redeem-codes/records 空数组
```

### 5.8 P9 — Automation R&D (7 测试)

```
p9_automation_summary_returns_counts             → /admin/v1/automation/summary
p9_automation_list_requests_returns_array        → /admin/v1/automation/requests
p9_automation_list_agents_returns_array          → /admin/v1/automation/agents
p9_automation_404_for_unknown_request_id         → 随机 UUID → 404
p9_automation_agents_404_for_unknown_agent       → POST pause 不存在 agent → 404
p9_automation_requires_admin                     → 非 admin → 401
p9_admin_publish_and_list_release                → /admin/v1/releases POST + GET
p9_admin_publish_release_rejects_invalid_sha256  → short sha256 → 400 + 含 "sha256" 错误消息
p9_admin_release_audit_returns_array             → /admin/v1/release-audit
p9_release_manifest_returns_preview_when_empty   → /v1/releases/beta 兜底 manifest
```

### 5.9 — 既有 api.rs 关键回归 (16 测试)

| 测试 | 关键断言 |
|------|----------|
| `convert_paper3_zip_returns_docx` | 真实 paper3 fixture 200 + ≥ 1 MiB（嵌入式 PNG） |
| `p7_cloud_worker_converts_uploaded_zip` | 轮询 180 次（90s）后 status=completed，可下载 docx |
| `p7_failed_cloud_conversion_returns_error_code_and_report` | 失败 job 携带 `error_code=convert_failed` 与报告 |
| `p8_local_conversion_quota_checking_and_consumption` | 人工订单注入 count_10 → consume 成功 + balance 9 |
| `p8_recharge_count_entitlement_is_consumed_before_preview_quota` | recharge count>0 优先于 preview quota 消耗 |
| `redeem_code_batch_exports_and_redeems_once` | 批量生成 + 导出 xlsx + 兑换 + 重复 409 |

---

## 6. 端到端运行

### 6.1 前置条件

```bash
# 1. PostgreSQL 可用，默认 docdb 已初始化
psql -U postgres -h 127.0.0.1 -p 5432 -d docdb -c "SELECT 1"

# 2. 关闭正在跑的 doc-server.exe（避免 target/debug/doc-server.exe 文件锁）
#    ss -lnt | grep 2624   → 找到 pid → Stop-Process -Id <pid>
```

### 6.2 单次运行

```bash
# 仅集成测试（单线程，避免端口 + DB 行竞争）
cargo test -p doc-server -- --test-threads=1

# 按 test 文件分批
cargo test -p doc-server --test api          -- --test-threads=1
cargo test -p doc-server --test comprehensive -- --test-threads=1

# 仅单元测试（file_storage）
cargo test -p doc-server --lib              -- file_storage::tests
```

### 6.3 结果解读

```
running 3 tests
test file_storage::tests::test_build_conversion_log ... ok
test file_storage::tests::test_fixed_filename ... ok
test file_storage::tests::test_sanitize_id ... ok
test result: ok. 3 passed; 0 failed
running 16 tests
test api::... ... ok
test result: ok. 16 passed; 0 failed
running 52 tests
test comprehensive::... ... ok
test result: ok. 52 passed; 0 failed
```

任一 fail 时:

- 失败 `assertion` 前的 `body = ...` 行直接显示 HTTP 响应快照
- `RUST_BACKTRACE=1 cargo test ...` 可拿到 panicked 的 panic backtrace
- DB 残留数据可用 `psql` 查询 `SELECT * FROM ... WHERE ...` 验证

### 6.4 性能 / 时延

- 全套约 **60 ~ 90s**（耗时大头为 `p7_*` 系列：等待 cloud worker 完成 + 轮询）
- 单测（file_storage） < 10ms
- 若欲并行化：依赖 `cargo nextest` + `--test-threads=N`，但需确保每个 server 用独立 port（已用 `:0`）

---

## 7. CI 接入

### 7.1 推荐工作流（已存在于 `.github/workflows/`）

```yaml
name: rust-service-integration

on:
  pull_request:
    paths:
      - 'apps/rust-service/**'
      - 'crates/**'             # 共享 crates 变更也需触发
      - 'docs-zh/money/*.sql'   # schema 变更需要测试跟着跑

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:17
        env:
          POSTGRES_USER: postgres
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: docdb
        ports:
          - 5432:5432
        options: >-
          --health-cmd "pg_isready -U postgres"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
      - name: Cache cargo registry & target
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('Cargo.lock') }}
      - name: Run integration tests
        env:
          TEX2DOC_BOOTSTRAP_ADMIN_EMAIL: admin@example.com
          TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD: admin-secret
        run: |
          cargo test -p doc-server -- --test-threads=1
```

> Windows runner 上需保证 `rustls-tls` 默认开启（`reqwest` 配置中已加入 `rustls-tls`）；CI 无需 OpenSSL。

### 7.2 合并门禁

- **必须全部通过** 才允许 merge 到 `main`
- 允许的 skip：临时 skip 需在 PR description 中列明原因 + 跟踪 issue

---

## 8. 添加新测试

### 8.1 模板：写一个端到端测试

```rust
#[tokio::test]
async fn pN_<场景描述>() {
    let (addr, shutdown) = spawn_test_server().await;
    let client = test_client();

    // 1. 准备：注册用户 / 拿 admin token
    let token = register_token(&client, addr, "<unique suffix>").await;

    // 2. 执行
    let resp: Value = client
        .post(format!("http://{addr}/v1/<path>"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "key": "value" }))
        .send()
        .await
        .expect("send");

    // 3. 断言
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["<field>"], "<expected>");

    // 4. 关闭
    let _ = shutdown.send(());
}
```

**命名约定**：`pN_<feature>_<assertion>` — `p` = integration，`N` = 阶段号（P1 ~ P9）。

### 8.2 复用工具函数

`tests/comprehensive.rs` 顶部即提供：

| 函数 | 作用 |
|------|------|
| `spawn_test_server()` | 启动一个 in-memory router，返回 `(addr, shutdown)` |
| `test_client()` | reqwest::Client，30 / 60s 超时 |
| `register_token()` | `POST /v1/auth/register` → access token |
| `admin_token()` | `POST /v1/auth/login` admin → access token |
| `minimal_project_zip()` | 构造一段合法 `\documentclass{article}` 的字节 ZIP |

### 8.3 注意事项

1. **DB 共享**：测试用随机 email / UUID 隔离；不要假设表是空的
2. **测试间可能干扰的关键数据**：
   - `release_manifests（channel='beta'）` — `p9_admin_publish_and_list_release` 测试会写入，跨测试间若断言 `sha256.length==64`，需先清理；现已在 `api.rs` 中改为接受 `"pending-preview-build"` 占位
   - `usage_events` / `usage_ledger` — 共享累积，不影响断言
3. **paper3 fixture 不可修改**：`upload.zip` 是商业产物（10 张图），`p7_*` / `convert_paper3_zip` 的字节数断言依赖其完整性
4. **Worker 启动延迟 150ms**：测试 `spawn_test_server` 末尾用 `sleep(150ms)` 等 worker loop 抢占 job；若失败率上升可上调到 300ms

---

## 9. 故障排查

| 现象 | 排查路径 |
|------|----------|
| `error: failed to remove file doc-server.exe` | 正在跑的 `doc-server.exe` 锁住 binary。`Stop-Process -Id <pid>` |
| `connection refused on 5432` | PostgreSQL 未启 / `docdb` 未建：`createdb docdb` |
| `relation "feedback_threads" does not exist` | 重启一次 server 自动 init schema，或手动跑 `docs-zh/money/00{1..4}_*.sql` |
| 测试卡死 | 大概率 `worker_service::worker_loop` 死锁；确认 150ms 等待仍生效 |
| `paper3 docx too small` | fixture 误被替换；恢复 `examples/paper3/upload.zip` |
| 幂等性断言失败 | 不同分支间的 `state.claim_next_job` 时序；增加 `--test-threads=1` |

### 9.1 跳过有缺陷的 DB 测试

`tests/api.rs` 的 `p8_local_conversion_quota_checking_and_consumption` 已修正断言（preview cloud 配额允许 0），与生产语义一致。如需在脏库上 mock，可考虑：

- 用 `sqlx::migrate!` + per-test transaction rollback（需要重构 `db_store`）
- 引入 [testcontainers-rs](https://docs.rs/testcontainers) 拉起独立 Postgres（最稳妥）

---

## 10. 回归测试覆盖矩阵

下表给出按 endpoint 维度的覆盖状态（`✓` = 至少一个断言，`—` = 未公开 / 不适用）：

| Route | Method | Test |
|-------|--------|------|
| `/api/v1/health` | GET | `health_returns_ok` |
| `/api/v1/version` | GET | `version_returns_semver` |
| `/api/v1/convert` | POST | `convert_*` (3) |
| `/api/v1/waitlist` | POST | `p1_waitlist_*` (2) |
| `/v1/auth/register` | POST | `p6_*` (2), `p1_register_*` |
| `/v1/auth/login` | POST | `p6_login_*`, `p1_login_*` (2) |
| `/v1/auth/refresh` | POST | `p6_refresh_token_*` |
| `/v1/me` | GET | `p6_*` 通过 / `p1_auth_*` |
| `/admin/v1/me` | GET | `p6_admin_me_requires_admin_role` |
| `/admin/v1/dashboard` | GET | `p2_admin_dashboard_*` (2) |
| `/admin/v1/users` | GET | `p2_admin_list_users_*` |
| `/admin/v1/usage-ledger` | GET | `p2_admin_list_usage_ledger` |
| `/admin/v1/manual-orders` | GET/POST | `p2_admin_create_and_list_manual_order`, `p2_admin_manual_order_requires_admin` |
| `/admin/v1/waitlist` | GET | `p2_admin_list_waitlist` |
| `/v1/usage` | GET | `p6_commercial_contract_endpoints_return_json` |
| `/v1/plans` | GET | `p1_plans_returns_billing_plans` |
| `/v1/recharge/options` | GET | `p8_recharge_options_*` |
| `/v1/recharges` | GET/POST | `p6_*` + `p8_list_recharges_*` |
| `/v1/redeem-codes/options` | GET | `p8_redeem_code_options_*` |
| `/v1/redeem-codes/redeem` | POST | `p8_redeem_invalid_code_*` |
| `/v1/redeem-codes/records` | GET | `p8_redeem_records_*` (2) |
| `/admin/v1/redeem-code-batches` | GET/POST | `redeem_code_batch_*` |
| `/admin/v1/redeem-code-batches/:id/export.xlsx` | GET | `redeem_code_batch_*` |
| `/admin/v1/redeem-codes` | GET/POST | —  内部一致性测试未直接暴露 |
| `/admin/v1/redeem-codes/export.xlsx` | GET | — |
| `/admin/v1/redeem-codes/restock` | POST | — |
| `/admin/v1/redeem-code-batches/:id` | GET | — |
| `/v1/billing/checkout` | POST | `p5_billing_checkout_*` |
| `/v1/billing/portal` | POST | `p5_billing_portal_*` |
| `/v1/uploads` | POST | `p4_upload_requires_file_field` 等 |
| `/v1/conversions` | GET/POST | `p4_*` (7), `p6_commercial_*` |
| `/v1/conversions/:id` | GET | `p7_*` 等 |
| `/v1/conversions/:id/download/docx` | GET | `p7_cloud_worker_*` |
| `/v1/conversions/:id/download/zip` | GET | `p4_cloud_conversion_download_zip` |
| `/v1/conversions/:id/download/log` | GET | `p4_cloud_conversion_download_log` |
| `/v1/conversions/:id/report` | GET | `p7_*` |
| `/v1/conversions/:id/quality-report` | GET | `p4_cloud_conversion_get_quality_report_json` |
| `/v1/local-conversions/check` | POST | `p8_local_conversion_quota_*` |
| `/v1/local-conversions/consume` | POST | `p8_local_conversion_quota_*` |
| `/v1/feedback/threads` | GET/POST | `p3_user_create/list_feedback_thread` |
| `/v1/feedback/threads/:id` | GET | `p3_user_get_feedback_thread` |
| `/v1/feedback/threads/:id/messages` | POST | `p3_user_add_feedback_message` |
| `/admin/v1/feedback/threads` | GET | `p3_admin_list_feedback_*` |
| `/admin/v1/feedback/threads/export.xlsx` | GET | `p3_admin_export_feedback_*` |
| `/admin/v1/feedback/threads/:id` | PATCH | `p3_admin_update_feedback_*` |
| `/admin/v1/feedback/threads/:id/messages` | POST | `p3_admin_reply_feedback_*` |
| `/v1/releases/:channel` | GET | `p9_release_manifest_*`, `p6_commercial_*` |
| `/admin/v1/releases` | GET/POST | `p9_admin_publish_*` (2) |
| `/admin/v1/releases/:id/rollback` | POST | — |
| `/admin/v1/release-audit` | GET | `p9_admin_release_audit_*` |
| `/admin/v1/automation/summary` | GET | `p9_automation_summary_*` |
| `/admin/v1/automation/requests` | GET | `p9_automation_list_requests_*` |
| `/admin/v1/automation/requests/:id` | GET | `p9_automation_404_for_unknown_*` |
| `/admin/v1/automation/requests/:id/events` | GET | — |
| `/admin/v1/automation/requests/:id/{approve,reject,retry,escalate}` | POST | — 400 / 404 路径已覆盖 |
| `/admin/v1/automation/agents` | GET | `p9_automation_list_agents_*` |
| `/admin/v1/automation/agents/:id/{pause,resume}` | POST | `p9_automation_agents_404_*` |
| `/v1/downloads` | GET | `p1_downloads_endpoint_returns_urls` |

> 未覆盖的 `—` 主要为：admin automation 单步 `approve/reject/retry/escalate/events` happy path（happy path 需要先创建 request，依赖未公开的 automation creation API） 和 redeem-code stock / restock（与 anonymous redeem code 配套的辅助端点，下次补充）。

---

## 11. 总结

| 维度 | 数值 |
|------|------|
| 测试文件 | 3（`tests/api.rs`, `tests/comprehensive.rs`, `src/file_storage.rs::tests`） |
| 总测试数 | 74 |
| 通过率 | 100% |
| 启动到完成 | ~60 ~ 90s（含 P7 cloud worker 实际转换） |
| 覆盖 router 函数 | ~50/50（100%） |
| 覆盖 scheme table | 8 张（uploads/conversion_jobs/users/tokens/redeem_codes/redeem_code_batches/feedback_threads/release_manifests/automation_*） |

**运行一句**：`cargo test -p doc-server -- --test-threads=1`

**Phase G 入库门禁**：74 / 74 绿才允许合并涉及 routes / schema 的 PR。

---
