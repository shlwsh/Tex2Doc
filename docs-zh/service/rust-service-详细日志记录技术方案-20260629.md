# Tex2Doc rust-service 详细日志记录技术方案

> 日期：2026-06-29
> 输出目录：`docs-zh/service`
> 适用对象：`apps/rust-service`（`doc-server` crate）
> 状态：技术方案

---

## 1. 背景与目标

`apps/rust-service` 当前已经使用 `tracing`、`tracing-subscriber` 和 `tower_http::trace::TraceLayer`，并在转换 worker 中有少量 `info` / `warn` / `error` 日志。现状能够看到服务启动、worker 状态和少数失败信息，但还不能满足生产排障、调用追溯和审计要求。

本方案目标是在保持现有 Rust 技术栈的基础上，补齐以下能力：

1. API 调用记录：记录接口路径、方法、调用方、TraceID、耗时、状态码、错误码。
2. 出入参记录：按配置记录请求参数、响应摘要、文件上传摘要，支持敏感字段脱敏。
3. 核心转换方法记录：覆盖上传校验、任务入队、worker claim、转换执行、质量报告构建、结果落盘、失败退款等关键步骤。
4. 数据库操作记录：记录 SQL 脚本/语句、绑定参数摘要、影响行数、返回记录摘要、耗时和错误。
5. 分级管理配置：支持全局日志级别和按模块日志级别配置。
6. 文件大小可配置：日志写入文件，达到配置大小后自动滚动创建新文件。
7. TraceID 追溯：每一次 API 调用生成或继承 TraceID，并贯穿 API、DB、worker、转换产物日志。

---

## 2. 当前代码切入点

| 模块 | 当前职责 | 日志改造切入点 |
|------|----------|----------------|
| `apps/rust-service/src/main.rs` | 初始化 `tracing_subscriber::fmt()`、挂载 `TraceLayer` | 替换为统一 `logging` 初始化，添加文件层、JSON 层、TraceID 层 |
| `apps/rust-service/src/routes.rs` | 所有 HTTP 路由、鉴权、上传、转换、反馈、admin、自动化接口 | API 出入参、鉴权结果、错误响应、handler 级业务字段 |
| `apps/rust-service/src/state.rs` | `ServerState`，封装 DB、队列、文件存储、业务状态方法 | job/upload/user 维度的业务日志，TraceID 传递到任务和产物 |
| `apps/rust-service/src/worker_service.rs` | worker loop、任务 claim、转换执行、失败处理 | worker span、转换阶段 span、`spawn_blocking` span 传递 |
| `apps/rust-service/src/db_store.rs` | PostgreSQL schema 初始化和业务 SQL | SQL 审计封装、语句名、参数摘要、耗时、影响行数、记录摘要 |
| `apps/rust-service/src/file_storage.rs` | 会话文件与 `conversion.log` 产物 | 产物日志增加 TraceID、转换阶段、文件路径摘要 |
| `apps/rust-service/src/automation_service.rs` | 自动化 R&D 数据库服务 | 复用 DB 审计封装，记录自动化请求状态流转 |

---

## 3. 总体架构

新增 `apps/rust-service/src/logging.rs`，负责日志配置、TraceID、脱敏、文件滚动 writer 和通用日志辅助方法。

建议结构：

```text
apps/rust-service/src/
├── logging.rs                  # 新增：日志初始化、TraceID、脱敏、文件滚动
├── main.rs                     # 调用 logging::init()
├── routes.rs                   # API middleware + handler 业务日志
├── state.rs                    # job/upload/user 业务事件日志
├── worker_service.rs           # 转换链路 span
├── db_store.rs                 # DB 审计辅助方法
└── file_storage.rs             # conversion.log 增加 trace_id
```

日志输出分为两类：

| 日志类型 | 文件 | 用途 |
|----------|------|------|
| 运行日志 | `logs/rust-service/service.log` | 服务启动、worker、转换、错误、普通业务事件 |
| API 审计日志 | `logs/rust-service/api.log` | HTTP 调用、出入参摘要、状态码、耗时 |
| DB 审计日志 | `logs/rust-service/db.log` | SQL、参数摘要、影响行数、返回记录摘要 |
| 安全日志 | `logs/rust-service/security.log` | 鉴权失败、权限不足、非法上传、zip slip 等安全事件 |
| 转换任务产物日志 | `sessions/.../{job_id}/conversion.log` | 单个 job 可下载的转换过程记录 |

所有日志建议统一使用 JSON Lines，便于后续导入 Loki、ELK、ClickHouse 或云日志平台。

---

## 4. 配置项设计

优先使用环境变量，不引入额外配置文件；后续可扩展为 `config.toml`。

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `RUST_LOG` | `info` | 保持现有兼容，作为 tracing EnvFilter |
| `TEX2DOC_LOG_LEVEL` | `info` | 全局日志级别，优先级低于 `RUST_LOG` |
| `TEX2DOC_API_LOG_LEVEL` | `info` | API 审计日志级别 |
| `TEX2DOC_DB_LOG_LEVEL` | `debug` | DB SQL 审计日志级别 |
| `TEX2DOC_CONVERSION_LOG_LEVEL` | `debug` | 转换链路日志级别 |
| `TEX2DOC_SECURITY_LOG_LEVEL` | `warn` | 安全日志级别 |
| `TEX2DOC_LOG_DIR` | `logs/rust-service` | 日志文件目录，启动时自动创建 |
| `TEX2DOC_LOG_FORMAT` | `json` | `json` 或 `compact` |
| `TEX2DOC_LOG_TO_STDOUT` | `true` | 是否保留控制台输出 |
| `TEX2DOC_LOG_MAX_FILE_SIZE_MB` | `128` | 单个日志文件最大大小 |
| `TEX2DOC_LOG_MAX_FILES` | `30` | 每类日志最多保留文件数 |
| `TEX2DOC_TRACE_HEADER` | `X-Trace-Id` | 对外 TraceID header 名称 |
| `TEX2DOC_LOG_BODY_MODE` | `metadata` | `off`、`metadata`、`safe-json`、`full` |
| `TEX2DOC_LOG_BODY_MAX_BYTES` | `8192` | 单条出入参 body 最大记录字节数 |
| `TEX2DOC_LOG_SQL_MODE` | `summary` | `off`、`summary`、`full` |
| `TEX2DOC_LOG_SQL_PARAMS` | `true` | 是否记录绑定参数摘要 |
| `TEX2DOC_LOG_SQL_RESULT_MODE` | `summary` | `count`、`summary`、`full` |
| `TEX2DOC_LOG_REDACT_PII` | `true` | 是否脱敏邮箱、token、密码等敏感信息 |

示例：

```powershell
$env:RUST_LOG="doc_server=debug,tower_http=info,sqlx=warn"
$env:TEX2DOC_LOG_DIR="E:\logs\tex2doc\rust-service"
$env:TEX2DOC_LOG_MAX_FILE_SIZE_MB="256"
$env:TEX2DOC_LOG_MAX_FILES="60"
$env:TEX2DOC_LOG_BODY_MODE="safe-json"
$env:TEX2DOC_LOG_SQL_MODE="summary"
```

---

## 5. TraceID 设计

### 5.1 TraceID 生成与透传

规则：

1. 请求头包含 `X-Trace-Id` 时复用该值。
2. 请求头包含 W3C `traceparent` 时提取其中的 trace id，并同时写入 `X-Trace-Id` 响应头。
3. 两者都不存在时生成新的 UUID v4 或 32 位 hex trace id。
4. 每个响应都返回 `X-Trace-Id`。
5. TraceID 写入 `Request::extensions()`，handler、DB、worker 可读取。

建议新增：

```rust
pub struct TraceContext {
    pub trace_id: String,
    pub parent_span_id: Option<String>,
}
```

### 5.2 异步任务 TraceID 延续

`create_conversion` 创建 job 时需要把 TraceID 写入数据库，建议在 `conversion_jobs` 增加字段：

```sql
ALTER TABLE conversion_jobs
    ADD COLUMN IF NOT EXISTS trace_id TEXT;
```

`ConversionJobRecord` 增加 `trace_id: Option<String>`。worker 从 DB claim job 后读取 `trace_id`，创建 worker span：

```rust
let span = tracing::info_span!(
    "conversion.job",
    trace_id = %trace_id,
    job_id = %job_id,
    worker_id = %worker_id
);
```

`spawn_blocking` 内部执行转换时要显式进入 span，否则阻塞线程里的日志可能丢失上下文：

```rust
let span = tracing::Span::current();
let result = tokio::task::spawn_blocking(move || {
    span.in_scope(|| execute_conversion(input))
}).await;
```

### 5.3 conversion.log 增加 TraceID

`FileStorage::build_conversion_log` 增加 `trace_id` 参数，并在文件头记录：

```text
TraceID:   1f4f0bb0-6c44-4dc1-a7a2-1dbd938d87ef
Job:       {job_id}
User:      {user_id}
Upload:    {upload_id}
```

这样用户下载 `/v1/conversions/:id/download/log` 后，也能拿 TraceID 反查服务运行日志。

---

## 6. API 调用与出入参日志

### 6.1 HTTP 中间件

在 `router_with_state` 或 `main.rs` 的 Router layer 中增加自定义 middleware：

```text
trace_id_middleware
  -> 解析或生成 TraceID
  -> 记录 request_start
  -> 执行 next
  -> 记录 request_end
  -> 响应头写入 X-Trace-Id
```

记录字段：

| 字段 | 说明 |
|------|------|
| `event` | `api.request.start` / `api.request.end` |
| `trace_id` | 全链路追踪 ID |
| `method` | HTTP method |
| `path` | 原始路径，优先使用 matched path |
| `query` | query 摘要，敏感字段脱敏 |
| `remote_addr` | 调用方 IP，优先解析 `X-Forwarded-For` |
| `user_agent` | User-Agent 摘要 |
| `content_type` | 请求 Content-Type |
| `request_bytes` | 请求体大小 |
| `status` | HTTP 状态码 |
| `error_code` | `ApiError` 映射后的错误码 |
| `duration_ms` | 请求耗时 |
| `response_bytes` | 响应体大小或估算值 |

### 6.2 Handler 级业务日志

中间件不应强行完整读取所有 body。对于重要 handler，应在业务入口显式记录安全摘要：

| handler | 建议记录 |
|---------|----------|
| `auth_register` | email hash、display_name 是否存在，不记录 password |
| `auth_login` | email hash、登录成功/失败，不记录 password/token |
| `upload_project` | user_id、upload_id、zip_bytes、sha256、file_count、uncompressed_bytes |
| `create_conversion` | user_id、upload_id、job_id、main_tex、profile、quality、engine、idempotency_key hash |
| `list_conversions` | user_id、返回记录数 |
| `get_conversion` | user_id、job_id、status、docx_ready、report_ready |
| `download_conversion_docx` | user_id、job_id、docx_bytes、storage_key hash |
| `download_conversion_log` | user_id、job_id、log_bytes、storage_key hash |
| admin 接口 | admin_id、role、操作对象、结果 |
| feedback 接口 | thread_id、user_id/admin_id、动作、状态流转 |
| automation 接口 | request_id、agent_id、动作、from_status、to_status |

### 6.3 出入参脱敏规则

默认不记录原始 body，只记录摘要。启用 `TEX2DOC_LOG_BODY_MODE=safe-json` 后，仅记录允许字段。

必须脱敏字段：

```text
password
access_token
refresh_token
Authorization
token
code
plaintext_code
code_ciphertext
code_nonce
payment_note
email
```

文件上传类接口不记录文件内容，只记录：

```json
{
  "file_name": "project.zip",
  "bytes": 1048576,
  "sha256": "sha256:abcd...",
  "zip_file_count": 42,
  "zip_uncompressed_bytes": 3145728
}
```

---

## 7. 核心转换方法日志

转换链路应按阶段建立 span，并记录阶段开始、结束、耗时和结果。

### 7.1 worker 主链路

覆盖 `worker_service.rs`：

| 方法 | 事件 | 字段 |
|------|------|------|
| `worker_loop` | `worker.poll` | worker_id、queued/picked、recovered_count |
| `claim_next_job` 调用点 | `worker.job.claim` | worker_id、job_id、duration_ms |
| `process_job` | `conversion.job.start/end` | trace_id、job_id、upload_id、user_id、engine、profile、quality、status |
| `validate_zip` | `conversion.zip.validate` | zip_bytes、file_count、uncompressed_bytes、result |
| `execute_conversion` | `conversion.execute` | engine、fallback、duration_ms、result |
| `execute_semantic` | `conversion.semantic` | backend、compatibility_score、quality_score、diagnostic_count |
| `execute_legacy` | `conversion.legacy` | docx_bytes、warning_count |
| `build_quality_run` | `conversion.quality_run` | dimension_scores 是否存在、quality_status |
| `complete_job` | `conversion.job.complete` | docx_bytes、log_bytes、result_docx_key hash、result_log_key hash |
| `fail_job` | `conversion.job.fail` | error_code、retryable、should_refund、refund_result |

### 7.2 状态流转日志

`ServerState::update_status` 每次状态变更记录：

```json
{
  "event": "conversion.status.change",
  "trace_id": "...",
  "job_id": "...",
  "from_status": "normalizing",
  "to_status": "rendering",
  "duration_since_start_ms": 5320
}
```

当前 DB `update_status` 不返回旧状态。若要记录 `from_status`，有两种方案：

1. 简化方案：只记录 `to_status` 和 job_id。
2. 完整方案：DB 更新前查询当前状态，或使用 `UPDATE ... RETURNING old/new` 的 CTE。

建议 Phase 1 采用简化方案，Phase 2 补齐完整状态流转。

---

## 8. 数据库 SQL 与记录日志

### 8.1 记录原则

DB 日志必须满足排障和审计，同时避免泄漏敏感数据。

默认记录：

1. SQL 名称：如 `conversion.create_job`、`auth.user_for_token`。
2. SQL 类型：`SELECT` / `INSERT` / `UPDATE` / `DELETE` / `DDL`。
3. SQL 摘要：压缩空白后的 SQL，或 SQL hash。
4. 参数摘要：字段名、类型、长度、hash，敏感值脱敏。
5. 耗时：`duration_ms`。
6. 影响行数：`rows_affected`。
7. 返回记录摘要：主键、业务 ID、状态、数量，不记录大字段。
8. 错误：数据库错误码、约束名、message 摘要。

### 8.2 SQL 封装方式

`DbStore` 目前直接使用 `sqlx::query(...).bind(...).fetch_* / execute(...)`。建议新增轻量审计辅助函数，先覆盖关键路径，再逐步扩展。

建议接口形态：

```rust
async fn audit_execute<'q>(
    &self,
    op: &'static str,
    sql: &'static str,
    params: AuditParams,
    query: sqlx::query::Query<'q, Postgres, PgArguments>,
) -> Result<PgQueryResult, sqlx::Error>
```

实际落地时可根据 `sqlx` 类型限制调整为宏：

```rust
db_audit_execute!(
    self.pool,
    op = "conversion.update_status",
    sql = "UPDATE conversion_jobs SET status = $2, updated_at = now() WHERE id = $1",
    params = { job_id: job_id, status: status.as_str() },
    query = sqlx::query("UPDATE ...")
        .bind(parse_uuid(job_id)?)
        .bind(status.as_str())
);
```

### 8.3 schema 初始化脚本日志

`DbStore::init_schema` 当前执行：

```rust
sqlx::raw_sql(BUSINESS_SCHEMA).execute(&self.pool).await?;
sqlx::raw_sql(REDEEM_STOCK_SCHEMA).execute(&self.pool).await?;
sqlx::raw_sql(FEEDBACK_SCHEMA).execute(&self.pool).await?;
sqlx::raw_sql(AUTOMATION_SCHEMA).execute(&self.pool).await?;
```

建议记录：

| 字段 | 示例 |
|------|------|
| `event` | `db.schema.execute` |
| `script` | `001_docdb_business_schema.sql` |
| `script_sha256` | `sha256:...` |
| `sql_bytes` | 12345 |
| `duration_ms` | 87 |
| `result` | `ok` / `error` |

默认不在每次启动日志中打印完整 schema；只有 `TEX2DOC_LOG_SQL_MODE=full` 时打印完整脚本文本。

### 8.4 关键 SQL 操作分级

| 级别 | 操作 | 示例 |
|------|------|------|
| `info` | 关键业务写操作 | 注册、登录 token 更新、创建上传、创建转换任务、扣减额度、完成/失败任务、反馈回复、admin 操作 |
| `debug` | 普通查询 | list plans、list conversions、get report、download 前查 job |
| `warn` | 业务异常但请求可控 | 查无记录、唯一约束冲突、额度不足、重复幂等请求 |
| `error` | DB 错误 | 连接失败、SQL 执行失败、事务提交失败、schema 初始化失败 |

### 8.5 返回记录摘要

对 `SELECT` 结果默认只记录数量和关键字段：

```json
{
  "event": "db.query.end",
  "trace_id": "...",
  "db_op": "conversion.get_job",
  "rows": 1,
  "record_summary": {
    "job_id": "e2d...",
    "user_id_hash": "sha256:...",
    "status": "completed",
    "docx_ready": true
  }
}
```

对 `INSERT` / `UPDATE` 默认记录：

```json
{
  "event": "db.execute.end",
  "trace_id": "...",
  "db_op": "conversion.complete_job",
  "rows_affected": 1,
  "record_summary": {
    "job_id": "...",
    "status": "completed",
    "docx_bytes": 32488,
    "log_bytes": 1204
  }
}
```

---

## 9. 文件滚动方案

### 9.1 推荐实现

`tracing-appender` 原生更偏向按时间滚动。由于本需求明确要求“日志文件大小可配置”，建议在本项目内实现一个小型 `SizeRotatingFileWriter`，避免引入不可控依赖。

核心行为：

1. 启动时创建 `TEX2DOC_LOG_DIR`。
2. 当前文件写入前检查大小。
3. 大于 `TEX2DOC_LOG_MAX_FILE_SIZE_MB` 时关闭当前文件。
4. 将 `service.log` 轮转为 `service.20260629-153000.1.log` 或 `service.log.1`。
5. 删除超过 `TEX2DOC_LOG_MAX_FILES` 的旧文件。
6. 新建 `service.log` 继续写入。
7. 写入通过 `tracing_appender::non_blocking` 或独立通道避免阻塞请求线程。

建议文件命名：

```text
service.log
service.20260629-153000.1.log
service.20260629-160512.2.log
api.log
api.20260629-153000.1.log
db.log
db.20260629-153000.1.log
```

### 9.2 异常处理

| 场景 | 策略 |
|------|------|
| 日志目录无法创建 | 服务启动失败，并在 stderr 输出明确错误 |
| 写文件失败 | 降级到 stdout/stderr，并打 `logging.file_write_failed` |
| 滚动删除失败 | 不影响服务请求，记录 `warn` |
| 单条日志超过文件上限 | 允许写入当前文件，然后立即滚动，避免截断 JSON |

---

## 10. 日志样例

### 10.1 API 调用开始

```json
{"ts":"2026-06-29T15:20:11.120+08:00","level":"INFO","target":"api","event":"api.request.start","trace_id":"1f4f0bb0-6c44-4dc1-a7a2-1dbd938d87ef","method":"POST","path":"/api/v1/conversions","user_agent":"Tex2Doc/1.0","request_bytes":184}
```

### 10.2 API 调用结束

```json
{"ts":"2026-06-29T15:20:11.238+08:00","level":"INFO","target":"api","event":"api.request.end","trace_id":"1f4f0bb0-6c44-4dc1-a7a2-1dbd938d87ef","method":"POST","path":"/api/v1/conversions","status":200,"duration_ms":118,"response_bytes":512}
```

### 10.3 转换任务

```json
{"ts":"2026-06-29T15:20:12.001+08:00","level":"INFO","target":"conversion","event":"conversion.job.start","trace_id":"1f4f0bb0-6c44-4dc1-a7a2-1dbd938d87ef","job_id":"9aa0...","upload_id":"2c8d...","engine":"semantic-engine","profile":"auto","quality":"standard"}
```

### 10.4 SQL 执行

```json
{"ts":"2026-06-29T15:20:12.118+08:00","level":"DEBUG","target":"db","event":"db.execute.end","trace_id":"1f4f0bb0-6c44-4dc1-a7a2-1dbd938d87ef","db_op":"conversion.update_status","sql_kind":"UPDATE","sql_hash":"sha256:4bd2...","params":{"job_id":"9aa0...","status":"rendering"},"rows_affected":1,"duration_ms":8}
```

### 10.5 错误日志

```json
{"ts":"2026-06-29T15:20:13.002+08:00","level":"ERROR","target":"conversion","event":"conversion.job.fail","trace_id":"1f4f0bb0-6c44-4dc1-a7a2-1dbd938d87ef","job_id":"9aa0...","error_code":"convert_failed","retryable":true,"message":"semantic-engine fallback failed"}
```

---

## 11. 安全与合规

### 11.1 禁止默认记录的内容

1. 原始上传 ZIP 内容。
2. 生成的 DOCX 内容。
3. 原始 LaTeX 文本内容，除非在本地调试环境显式开启。
4. 密码、access token、refresh token、Authorization header。
5. 明文兑换码、兑换码密文、nonce。
6. 支付备注和其他可能包含个人信息的长文本。

### 11.2 脱敏策略

| 类型 | 策略 |
|------|------|
| email | 保留域名或 hash，如 `sha256:...` |
| token | 只保留前缀和 hash，如 `access-*** sha256:...` |
| UUID | 可直接记录，若涉及用户 ID 可 hash |
| 文件路径 | 记录相对 key 或 hash，不记录服务器绝对路径 |
| SQL 参数 | 按字段名规则脱敏 |
| 长文本 | 截断到 `TEX2DOC_LOG_BODY_MAX_BYTES` |

---

## 12. 实施阶段

### Phase 1：日志基础设施

1. 新增 `logging.rs`。
2. `main.rs` 改为 `logging::init()`。
3. 实现 JSON/compact 格式、stdout/file 双输出。
4. 实现按大小滚动日志文件。
5. 增加基础单元测试：配置解析、脱敏、文件滚动。

### Phase 2：TraceID 和 API 审计

1. 新增 TraceID middleware。
2. 所有响应写入 `X-Trace-Id`。
3. 记录 API start/end。
4. 在重点 handler 记录出入参摘要。
5. `ApiError::into_response` 增加错误码日志上下文。

### Phase 3：转换链路日志

1. `create_conversion` 写入 job trace_id。
2. `conversion_jobs` 增加 `trace_id` 字段。
3. worker claim 后恢复 trace_id。
4. `process_job`、`execute_conversion`、`execute_semantic`、`execute_legacy` 建立 span。
5. `conversion.log` 增加 TraceID 和阶段明细。

### Phase 4：DB SQL 审计

1. 先覆盖 schema 初始化、auth、upload、conversion、quota、feedback、automation 关键写操作。
2. 再覆盖关键查询路径。
3. 对 `DbStore` 和 `AutomationService` 建立统一审计 helper。
4. 增加 SQL 参数和记录摘要脱敏。

### Phase 5：验收与运维接入

1. 集成测试验证 TraceID 响应头。
2. 集成测试验证日志文件创建与滚动。
3. 手工压测验证日志写入不会显著影响接口延迟。
4. 给运维提供日志目录、保留策略、检索示例。

---

## 13. 测试与验收标准

| 验收项 | 标准 |
|--------|------|
| TraceID | 每个 API 响应都有 `X-Trace-Id`，日志中可按 TraceID 查到 API、DB、worker 事件 |
| API 日志 | 请求开始/结束均记录，包含 path、method、status、duration_ms |
| 出入参 | 上传只记录文件摘要，JSON body 按配置记录且敏感字段脱敏 |
| 转换日志 | 一个 cloud conversion 可查到 create、enqueue、claim、execute、complete/fail 全链路 |
| DB 日志 | 关键 SQL 有 op、sql_hash/SQL、params 摘要、rows、duration_ms |
| 文件大小 | 设置很小的 `TEX2DOC_LOG_MAX_FILE_SIZE_MB` 后能自动生成新日志文件 |
| 分级配置 | 调整 `TEX2DOC_DB_LOG_LEVEL` 可打开/关闭 DB 详细日志 |
| 安全 | password/token/兑换码/文件内容不出现在默认日志中 |
| 兼容 | 现有 74 个 rust-service 测试通过 |

---

## 14. 风险与注意事项

1. 日志体积风险：DB full SQL、full body 模式会快速放大日志量，生产默认必须使用 `summary`。
2. 隐私风险：出入参和 SQL 参数必须先脱敏再写日志。
3. 性能风险：文件写入必须异步或 non-blocking，避免阻塞 API 请求。
4. span 丢失风险：`spawn_blocking` 内部需要显式传递 span。
5. worker 追溯风险：只靠内存队列传 TraceID 不够，必须持久化到 `conversion_jobs.trace_id`。
6. DB 记录摘要风险：对大查询不要记录完整返回记录，只记录数量和关键字段。

---

## 15. 推荐最小变更清单

```text
apps/rust-service/Cargo.toml
  - 增加 tracing-subscriber json/time/local-time 相关 feature
  - 可选增加 tracing-appender，用于 non-blocking writer

apps/rust-service/src/main.rs
  - 使用 logging::init()
  - Router 增加 trace_id middleware

apps/rust-service/src/logging.rs
  - 新增日志配置、TraceID、脱敏、文件滚动 writer、审计 helper

apps/rust-service/src/routes.rs
  - handler 级出入参摘要日志
  - create_conversion 写入 TraceID

apps/rust-service/src/state.rs
  - ConversionJobRecord 增加 trace_id
  - complete_job/fail_job/build_conversion_log 传递 trace_id

apps/rust-service/src/worker_service.rs
  - conversion span 和 spawn_blocking span 传递

apps/rust-service/src/db_store.rs
  - conversion_jobs 增加 trace_id 读写
  - schema/auth/upload/conversion/quota/feedback/automation SQL 审计

apps/rust-service/src/file_storage.rs
  - conversion.log 增加 TraceID、阶段和结果摘要

docs-zh/money/001_docdb_business_schema.sql
  - conversion_jobs ADD COLUMN trace_id TEXT
```

---

## 16. 后续扩展

1. 对接 OpenTelemetry：TraceID 与 `traceparent` 完全兼容，后续可输出 OTLP。
2. 对接日志平台：JSON Lines 可直接采集到 Loki/ELK/ClickHouse。
3. 增加 admin 审计查询：将高价值审计事件异步写入 DB 表，供后台查询。
4. 增加采样策略：高频查询可按 TraceID 或错误状态采样，降低日志成本。
5. 增加告警规则：按 `conversion.job.fail`、`db.error`、`security.auth_failed` 建立告警。

