# PostgreSQL 持久化主链路开发实现报告

生成日期：2026-06-24  
适用范围：Tex2Doc `doc-server` 商业化主链路  
依据文档：`docs-zh/home/PostgreSQL持久化主链路规划设计与实现方案-20260624.md`

## 1. 实现结论

已基于规划方案完成 PostgreSQL 持久化主链路的核心开发收口：用户认证、访问令牌、刷新令牌、套餐与兑换选项、用量预占与失败返还、转换任务队列、转换报告、下载定位、发布版本读取等关键业务状态均已从硬编码或运行时暂存转向 PostgreSQL 事实源。

当前 `tokio::sync::mpsc` 仅保留为 worker 唤醒信号，不再承担任务队列事实源；转换任务的排队、领取、锁定、恢复、完成和失败状态由 `conversion_jobs` 持久化管理。

## 2. 已完成开发项

### 2.1 数据库 Schema

已扩展以下 SQL：

- `docs-zh/money/001_docdb_business_schema.sql`
- `docs-zh/money/003_feedback_and_session_storage.sql`

主要新增和增强：

- `app_users.role`：支持管理员身份持久化。
- `app_access_tokens`：访问令牌落库，支持过期、吊销、最后使用时间。
- `auth_refresh_tokens`：刷新令牌 rotation 与吊销链路接入。
- `commercial_entitlements`：商业权益余额持久化。
- `usage_ledger`：用量账本，支持 reserve/refund。
- `conversion_jobs.report_json`：转换报告 JSON 结构化落库。
- `conversion_jobs.worker_id`、`locked_at`、`attempts`、`next_run_at`、`queued_at`、`started_at`、`failed_at`：支持数据库任务队列。
- `idx_conversion_jobs_queue`：支持 queued 任务领取查询。

### 2.2 认证与令牌持久化

已完成：

- Demo 管理员用户写入 `role='admin'`。
- access token 写入 `app_access_tokens`，接口鉴权从数据库读取并更新 `last_used_at`。
- refresh token 写入 `auth_refresh_tokens`。
- `/v1/auth/refresh` 使用数据库刷新令牌，并在成功刷新后吊销旧 refresh token，实现基础 rotation。

### 2.3 商业配置 DB 化

已完成：

- `/v1/plans` 从 `billing_plans` 读取。
- `/v1/recharge/options` 从 `redeem_packages` 读取。
- `/v1/redeem-codes/options` 从 `redeem_packages` 读取。
- 发布版本 `/v1/release/manifest` 优先从 `release_manifests` 读取，缺省时回退到 preview manifest。

### 2.4 用量预占与失败返还

已完成：

- 创建转换任务后先写入 `conversion_jobs`，再执行 `reserve_cloud_conversion(user_id, job_id)`。
- 预占成功后写入 `usage_ledger(action='reserve')`。
- quota 不足时将任务持久化为失败状态并返回 `quota_exhausted`。
- 转换失败时通过 `refund_cloud_conversion_for_job(job_id, reason)` 进行幂等返还。
- preview quota 统计优先基于 `usage_ledger` 的 reserve/refund 净值，兼容旧 `usage_events`。

### 2.5 转换任务 DB Queue

已完成：

- worker 由 mpsc 直接消费改为定时/通知唤醒后从 PostgreSQL 领取任务。
- 使用 `FOR UPDATE SKIP LOCKED` 领取 `conversion_jobs.status='queued'` 的任务。
- 领取任务时写入 `worker_id`、`locked_at`、`attempts`、`started_at`。
- 定时恢复 stale running 任务，超过锁定时间后重新进入 queued。
- `mpsc` 保留为轻量通知机制，不保存业务状态。

### 2.6 转换文件与报告读取

已完成：

- 下载 ZIP 和转换日志时，改为读取数据库中的 `source_zip_key`、`result_log_key`。
- 移除基于 `job_id + 当天日期 + filename` 反推路径的加载方式，避免跨日下载失败。
- 转换完成/失败报告写入 `conversion_jobs.report_json`。
- 读取历史任务时优先解析 `report_json`，兼容旧 `result_report_key` 中保存 JSON 字符串的历史数据。

### 2.7 API 测试

已新增：

- `p6_refresh_token_rotates_and_revokes_old_token`

覆盖 refresh token 成功轮换、旧 refresh token 再次使用返回 401 的关键行为。

## 3. 验证结果

已执行并通过：

```text
cargo check -p doc-server
cargo test -p doc-server --test api -- --nocapture --test-threads=1
cargo test -p doc-commercial-api-client
cargo test -p doc-desktop-slint
```

其中 `doc-server` API 集成测试共 14 项通过。API 测试本地使用共享 PostgreSQL 测试库，建议继续以 `--test-threads=1` 作为稳定验证方式。

同时已扫描服务端源码中的内存暂存关键词，未发现仍以 `HashMap`、`RwLock`、`Mutex<Vec>` 等结构保存商业主链路状态；仅保留 worker mpsc 通知。

## 4. GitNexus 影响分析

开发前已按项目要求对关键符号执行影响分析，涉及：

- `ServerState`
- `DbStore`
- `process_job`
- `router_with_state`
- `FileStorage.load`
- `create_conversion`
- `issue_token`
- `auth_refresh`
- `release_manifest`
- `fail_job_with_code`

开发后已执行 `detect_changes(scope=all)`。结果显示影响范围为 critical，原因是本次变更触达认证、用量、转换任务、worker、商业配置等主链路核心模块，属于预期内的高影响改造。

## 5. 变更文件

核心实现文件：

- `crates/server/src/db_store.rs`
- `crates/server/src/state.rs`
- `crates/server/src/routes.rs`
- `crates/server/src/worker_service.rs`
- `crates/server/src/file_storage.rs`
- `crates/server/tests/api.rs`
- `docs-zh/money/001_docdb_business_schema.sql`
- `docs-zh/money/003_feedback_and_session_storage.sql`

本轮未处理、也未回滚已有的外部变更：

- `AGENTS.md`
- `CLAUDE.md`

## 6. 剩余边界与后续建议

本轮已完成 PostgreSQL 持久化主链路的工程收口，但以下内容建议作为后续商业化增强继续推进：

1. 支付提供商接入：当前充值、发票、订阅表已具备承载基础，仍需接入真实支付 webhook 和对账流程。
2. 发布策略管理端：当前 release manifest 已支持数据库读取，后续可补管理端 CRUD、灰度策略和审计。
3. 兑换码与权益审计看板：已有账本和事件表，可进一步做运营查询、导出和异常检测。
4. 多 worker 压测：DB queue 已具备并发领取基础，建议补充多实例 worker 压测和长任务恢复测试。
5. 令牌安全强化：当前实现为数据库持久化闭环，后续建议引入 token hash 存储、设备标识、IP/UA 审计和管理员吊销入口。

## 7. 验收建议

建议按以下步骤验收：

1. 初始化或迁移 PostgreSQL schema。
2. 启动 `doc-server`，确认数据库连接失败时服务直接失败退出。
3. 登录获取 access/refresh token，验证 refresh rotation。
4. 创建转换任务，确认 `conversion_jobs` 写入 queued/running/completed 或 failed 状态。
5. 人为制造转换失败，确认 `usage_ledger` 出现 refund，权益或 preview quota 被返还。
6. 重启服务后查询历史任务、下载 ZIP/日志，确认依赖数据库 object key 而非当天路径推导。
7. 多 worker 并发启动，确认同一 queued 任务不会被重复领取。
