# PostgreSQL 持久化主链路规划设计与实现方案

生成日期：2026-06-24  
适用范围：Tex2Doc `doc-server` 商业化主链路  
目标数据库：PostgreSQL `docdb`  
核心要求：当前项目中所有业务相关的内存暂存内容必须落 PostgreSQL；服务端启动必须依赖数据库连接，连接失败直接失败退出，不允许回退到内存态。

## 1. 目标与结论

### 1.1 总目标

将 Tex2Doc 商业化服务端从 Preview 内存态彻底收口为 PostgreSQL 持久化主链路，使以下业务数据均可在服务重启、worker 重启、接口重试和客户端断线后恢复：

- 用户、访问令牌、刷新令牌、管理员身份。
- 套餐、订阅、充值、兑换码批次、兑换码明细、兑换事件。
- 用量周期、用量事件、权益余额、失败返还。
- 上传文件记录、转换任务、任务状态、源 ZIP、结果 DOCX、转换日志、转换报告。
- 反馈主题、反馈消息、管理端回复、反馈导出。
- 发布版本、升级策略、审计日志。

### 1.2 当前结论

当前代码已经开始从内存态迁移到 `DbStore + FileStorage`，并且 `crates/server/src/state.rs` 的 `ServerState` 已基本变为：

```text
ServerState
  -> DbStore
  -> FileStorage
  -> FeedbackStore
  -> mpsc::Sender<WorkerCommand>
```

这说明方向正确，但仍未达到“全部业务状态落库”的验收标准。需要继续收口的关键点包括：

1. `mpsc` 只能作为运行时唤醒信号，不能作为任务队列事实源；`conversion_jobs` 必须承担 DB queue。
2. `conversion_jobs.result_report_key` 当前复用为 report JSON 字符串，命名与用途不一致，应拆为 `result_report_key` 与 `report_json`。
3. `FileStorage::load(job_id, filename)` 使用当天日期拼路径，跨日下载历史文件会失败，应全部改为从数据库 object key 读取。
4. `admin_list_feedback_threads` 当前先全量查询再内存过滤分页，应改为 SQL 过滤、排序、分页。
5. `plans`、`recharge_options` 仍由路由硬编码返回，应逐步改为 DB 读取 `billing_plans` 与 `redeem_packages`。
6. Access token 已落 `app_access_tokens`，但 `auth_refresh_tokens` 尚未真正形成 hash、rotation、revocation 闭环。
7. usage 当前直接扣减，缺少“预占、确认、失败返还”的完整账本模型。

## 2. 持久化范围审计

| 业务域 | 当前风险/现状 | 目标归属 | 目标表/存储 |
|---|---|---|---|
| 用户 | `upsert_user` 已接 DB，但仍是 demo auth 风格 | PostgreSQL | `app_users` |
| access token | 已有 `app_access_tokens` 辅助表 | PostgreSQL | `app_access_tokens`，后续可替换为 JWT + session 表 |
| refresh token | schema 已有，接口仍需收口 | PostgreSQL | `auth_refresh_tokens` |
| 套餐 | DB 有 `billing_plans`，路由仍硬编码 | PostgreSQL | `billing_plans` |
| 订阅/发票 | schema 已有，当前商业闭环仍 mock | PostgreSQL | `subscriptions`、`invoices` |
| 用量 | `usage_events` 已使用，但缺预占/确认/返还 | PostgreSQL | `usage_periods`、`usage_events` 或新增 `usage_ledger` |
| 权益余额 | 当前用 `commercial_entitlements` 辅助表 | PostgreSQL | `commercial_entitlements`，建议补审计明细 |
| 充值 | 已接 `recharges` | PostgreSQL | `recharges` |
| 兑换码批次 | `DbStore.create_redeem_batch` 已落库 | PostgreSQL | `redeem_code_batches` |
| 兑换码明细 | 已落 `redeem_codes` | PostgreSQL | `redeem_codes` |
| 兑换事件 | 已有 `redeem_code_events` | PostgreSQL | `redeem_code_events` |
| 上传记录 | `uploads` 已落库，文件落 `sessions` | PostgreSQL + 文件存储 | `uploads.object_key` + `sessions/` |
| 上传 ZIP 字节 | 不应放内存长期保存 | 文件存储 | `sessions/.../source.zip`，DB 存 key/sha256/bytes |
| 转换任务 | `conversion_jobs` 已落库，但 worker 仍靠 mpsc | PostgreSQL | `conversion_jobs` 作为 DB queue |
| 结果 DOCX | 已落 `FileStorage`，DB 存 key | 文件存储 | `conversion_jobs.result_docx_key` |
| 转换日志 | 已落 `FileStorage`，DB 存 key | 文件存储 | `conversion_jobs.result_log_key` |
| 转换报告 | 当前 JSON 复用 `result_report_key` | PostgreSQL + 可选文件 | `conversion_jobs.report_json`，可选 `result_report_key` |
| 反馈主题 | `FeedbackStore` 已代理到 DB | PostgreSQL | `feedback_threads` |
| 反馈消息 | `FeedbackStore` 已代理到 DB | PostgreSQL | `feedback_messages` |
| 发布版本 | schema 已有，升级方案待实现 | PostgreSQL | `release_manifests`、后续策略/审计表 |
| worker 队列 | 当前 `tokio::sync::mpsc` | PostgreSQL | `conversion_jobs.status='queued'` + row lock |

## 3. 目标架构

### 3.1 统一原则

1. **DB 是业务事实源**  
   所有可被用户、财务、运营、客服、管理员或 worker 追溯的数据，必须以 PostgreSQL 为事实源。内存只允许保存连接池、配置、临时请求变量和短期唤醒信号。

2. **文件本体不进数据库**  
   ZIP、DOCX、日志等大文件保存在 `sessions/` 或未来对象存储，PostgreSQL 只保存 `object_key`、`sha256`、`bytes`、`status`、`expires_at`。

3. **队列状态落库**  
   worker 可以用 `mpsc` / notify 做加速唤醒，但任务排队、领取、重试、失败、恢复必须由 `conversion_jobs` 承担。

4. **所有扣费和权益变化可审计**  
   兑换、充值、扣减、失败返还不能只更新余额，必须有 ledger 或 usage event，支持事后核对。

5. **接口不吞 DB 错误**  
   不能用 `unwrap_or_default()` 隐藏数据库读写失败。查询失败应返回稳定错误码，后台 worker 失败应写入任务状态和日志。

### 3.2 目标调用链

```text
客户端
  -> Axum routes
  -> require_session / require_admin_session
  -> ServerState
  -> DbStore 读写 PostgreSQL
  -> FileStorage 读写 source.zip / result.docx / conversion.log
  -> conversion_jobs 作为任务队列事实源
  -> Worker 按 DB row lock 领取任务
  -> 转换引擎
  -> FileStorage 保存产物
  -> DbStore 更新完成/失败状态、usage ledger、report_json
```

## 4. 数据库设计补齐

### 4.1 保留并规范已有表

已有 `001_docdb_business_schema.sql` 和 `003_feedback_and_session_storage.sql` 应继续作为基线，保留以下核心表：

- `app_users`
- `auth_refresh_tokens`
- `billing_plans`
- `subscriptions`
- `invoices`
- `usage_periods`
- `usage_events`
- `redeem_packages`
- `recharges`
- `redeem_code_batches`
- `redeem_codes`
- `redeem_code_events`
- `uploads`
- `conversion_jobs`
- `feedback_threads`
- `feedback_messages`
- `release_manifests`

### 4.2 建议新增/调整表

#### 4.2.1 `app_access_tokens`

当前由 `DbStore::init_schema` 动态创建，应正式迁入 SQL migration。

```sql
CREATE TABLE IF NOT EXISTS app_access_tokens (
    token_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_app_access_tokens_user
    ON app_access_tokens(user_id, created_at DESC);
```

#### 4.2.2 `commercial_entitlements`

当前由 `DbStore::init_schema` 动态创建，也应正式迁入 SQL migration。

```sql
CREATE TABLE IF NOT EXISTS commercial_entitlements (
    user_id UUID PRIMARY KEY REFERENCES app_users(id) ON DELETE CASCADE,
    count_balance BIGINT NOT NULL DEFAULT 0 CHECK (count_balance >= 0),
    valid_until TIMESTAMPTZ,
    source_order_id TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

#### 4.2.3 `usage_ledger`

建议新增，解决“预占、确认、失败返还、人工调整”不可审计的问题。

```sql
CREATE TABLE IF NOT EXISTS usage_ledger (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    conversion_job_id UUID REFERENCES conversion_jobs(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL CHECK (event_type IN (
        'reserve', 'commit', 'refund', 'grant', 'adjust'
    )),
    quantity BIGINT NOT NULL,
    balance_after BIGINT,
    source TEXT NOT NULL DEFAULT 'system',
    reason TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_usage_ledger_user_created
    ON usage_ledger(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_usage_ledger_job
    ON usage_ledger(conversion_job_id);
```

#### 4.2.4 `conversion_jobs` 字段规范

当前 `result_report_key` 被用来保存 report JSON，应拆开：

```sql
ALTER TABLE conversion_jobs
    ADD COLUMN IF NOT EXISTS report_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS worker_id TEXT,
    ADD COLUMN IF NOT EXISTS locked_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS attempts INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS next_run_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN IF NOT EXISTS queued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN IF NOT EXISTS started_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS failed_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_conversion_jobs_queue
    ON conversion_jobs(status, next_run_at, created_at)
    WHERE status IN ('queued', 'failed');
```

字段语义：

| 字段 | 用途 |
|---|---|
| `source_zip_key` | 源 ZIP 文件 key |
| `result_docx_key` | 结果 DOCX 文件 key |
| `result_log_key` | 转换日志文件 key |
| `result_report_key` | 可选，报告文件 key |
| `report_json` | 结构化报告 JSON |
| `worker_id` | 当前领取任务的 worker 标识 |
| `locked_at` | worker 领取时间 |
| `attempts` | 尝试次数 |
| `next_run_at` | 下次可领取时间 |

#### 4.2.5 release 相关表

升级管理方案中已有扩展，建议纳入持久化主链路二期：

- `release_strategies`
- `release_rollout_events`
- `release_audit_log`
- `app_update_preferences`

## 5. 模块实现方案

### 5.1 `DbStore` 收口

目标：所有业务读写只通过 `DbStore` 进入 PostgreSQL。

必须补齐的方法：

| 方法 | 目标 |
|---|---|
| `list_plans()` | 替换 `routes.rs::plans()` 硬编码 |
| `list_redeem_packages()` | 替换 `recharge_options()` 与 `redeem_code_options()` 硬编码 |
| `issue_refresh_token()` | 写 `auth_refresh_tokens` hash |
| `rotate_refresh_token()` | refresh 时撤销旧 token，签发新 token |
| `revoke_token()` | logout / 管理操作可撤销 |
| `reserve_conversion_quota()` | 创建转换任务时预占权益 |
| `commit_conversion_quota()` | 转换成功确认扣减 |
| `refund_conversion_quota()` | 转换失败或取消返还 |
| `claim_next_job(worker_id)` | 用 DB row lock 领取 queued job |
| `mark_job_processing()` | 写 `locked_at`、`started_at`、`attempts` |
| `recover_stale_jobs()` | 将超时 processing 任务重新 queued 或 failed |
| `save_report_json()` | 保存结构化 report 到 `report_json` |
| `get_artifact_key(job_id, kind)` | 下载 ZIP/DOCX/log 统一从 DB key 读取 |

### 5.2 `ServerState` 简化

目标：`ServerState` 不保存任何业务集合，只保留服务依赖。

允许保留：

- `DbStore`
- `FileStorage`
- `WorkerNotifier` 或 `mpsc::Sender`，仅用于唤醒
- 静态配置，如 storage root、worker 配置

不允许保留：

- `HashMap` / `RwLock<Vec<业务记录>>`
- 业务记录缓存
- 仅存在内存中的 upload/job/recharge/redeem/feedback/usage 数据

建议接口形态：

```rust
pub struct ServerState {
    db: DbStore,
    file_storage: FileStorage,
    worker_notify: WorkerNotify,
}
```

`feedback_store()` 可以继续保留为 DB-backed facade，但不得包含内存 map。

### 5.3 上传与文件存储

当前 `store_upload` 会先把源 ZIP 写入 `FileStorage`，再写 `uploads`。目标流程应调整为：

1. 生成 `upload_id`。
2. 校验 ZIP。
3. 计算 `sha256`、`bytes`。
4. 写 `sessions/uploads/{YYYY}/{MM}/{DD}/{upload_id}/source.zip` 或统一 `sessions/{YYYY}/{MM}/{DD}/{upload_id}/source.zip`。
5. 写 `uploads`：`object_key`、`sha256`、`bytes`、`status='stored'`。
6. 返回 `upload_id`。

下载或转换读取时只能通过 `uploads.object_key` 加载文件，不再依赖当天日期或内存 bytes。

### 5.4 转换任务与 DB Queue

当前 worker 通过 `mpsc` 接收 `WorkerCommand { job_id }`。目标是：`mpsc` 只负责“快点醒来”，任务事实源是 `conversion_jobs`。

创建任务：

```text
POST /v1/conversions
  -> validate upload exists
  -> reserve_conversion_quota(user_id, job_id)
  -> INSERT conversion_jobs(status='queued', next_run_at=now())
  -> notify worker
  -> return job
```

领取任务：

```sql
WITH picked AS (
    SELECT id
    FROM conversion_jobs
    WHERE status = 'queued'
      AND next_run_at <= now()
    ORDER BY created_at ASC
    FOR UPDATE SKIP LOCKED
    LIMIT 1
)
UPDATE conversion_jobs j
SET status = 'normalizing',
    worker_id = $1,
    locked_at = now(),
    started_at = COALESCE(started_at, now()),
    attempts = attempts + 1,
    updated_at = now()
FROM picked
WHERE j.id = picked.id
RETURNING j.*;
```

worker loop：

```text
loop:
  job = db.claim_next_job(worker_id)
  if none:
    wait notify or sleep 1s
  else:
    process job
    success -> save files -> complete_job -> commit quota
    fail -> save log/report -> fail_job -> refund quota when needed
```

恢复策略：

- `normalizing/detecting/analyzing/compiling/rendering/verifying` 且 `locked_at < now() - interval '15 minutes'`：重置为 `queued`，`next_run_at=now()+backoff`。
- `attempts >= 3`：标记 `failed`，写 `worker_timeout` 或 `max_attempts_exceeded`。

### 5.5 转换产物与报告

成功：

1. `result.docx` 写入 `FileStorage`。
2. `conversion.log` 写入 `FileStorage`。
3. `report_json` 写入 `conversion_jobs.report_json`。
4. `result_docx_key`、`result_log_key`、`docx_bytes`、`log_bytes` 更新到 DB。
5. 状态更新为 `completed`，写 `completed_at`。

失败：

1. `conversion.log` 写入 `FileStorage`。
2. 最小失败 `report_json` 写入 DB。
3. `error_code`、`error_message`、`failed_at` 写 DB。
4. 状态更新为 `failed`。
5. 调用 `refund_conversion_quota()`。

下载：

- `download_conversion_docx` 从 `conversion_jobs.result_docx_key` 读取。
- `download_conversion_zip` 从 `conversion_jobs.source_zip_key` 或关联 `uploads.object_key` 读取。
- `download_conversion_log` 从 `conversion_jobs.result_log_key` 读取。
- 不再调用 `load_session_file(job_id, filename)` 拼当天路径。

### 5.6 用量与权益账本

当前 `try_consume_cloud_conversion` 会直接消耗次数或写 `usage_events`。建议改为三阶段：

| 阶段 | 时机 | 数据动作 |
|---|---|---|
| reserve | 创建 job 成功前 | 写 `usage_ledger(event_type='reserve')`，可选冻结余额或记录预占 |
| commit | 转换成功 | 写 `usage_ledger(event_type='commit')`，确认扣减 |
| refund | 转换失败/取消/worker 超时 | 写 `usage_ledger(event_type='refund')`，恢复余额或抵消预占 |

短期如果不新增冻结字段，可使用简化策略：

- 创建 job 时扣减 `commercial_entitlements.count_balance` 或写 preview `usage_events`。
- 转换失败时反向写 `usage_ledger(refund)` 并恢复 `count_balance`；preview quota 则写 `usage_events` 的负向表不适合当前 check，建议新增 `usage_ledger` 后由 ledger 聚合。

最终 `GET /v1/usage` 应从 DB 聚合：

- 订阅套餐额度：`billing_plans` / `subscriptions`。
- 当前周期已用：`usage_ledger` 或 `usage_events`。
- 兑换码余额：`commercial_entitlements`。
- 存储使用：`uploads.bytes + conversion_jobs.docx_bytes + log_bytes`。

### 5.7 Auth 与 Admin

当前 `auth_register` / `auth_login` 都走 `auth_response`，更像 demo upsert。目标：

| 接口 | 目标行为 |
|---|---|
| `POST /v1/auth/register` | 新建用户；email 唯一；password hash；签发 access/refresh |
| `POST /v1/auth/login` | 校验密码 hash；失败返回 401；签发 access/refresh |
| `POST /v1/auth/refresh` | 校验 refresh token hash；rotation；旧 token revoked |
| `GET /v1/me` | access token 查 DB，更新 last_used |
| Admin API | 管理员角色必须落 DB，不依赖 `demo-admin` 字符串 |

建议在 `app_users` 增加 `role` 或新增 `user_roles`：

```sql
ALTER TABLE app_users
    ADD COLUMN IF NOT EXISTS role TEXT NOT NULL DEFAULT 'user'
    CHECK (role IN ('user', 'admin'));
```

### 5.8 反馈模块

当前 `FeedbackStore` 已是 DB-backed facade，但仍需改进：

1. `admin_list_feedback_threads` 改为 SQL where / limit / offset，不要先全量查出再内存过滤。
2. `feedback_threads` 增加 `message_count`、`latest_message_at` 可选冗余字段，或继续通过 SQL 聚合，但分页必须在 DB 完成。
3. `attachments` 若支持上传文件，必须保存到文件存储，并在 DB 存 key，不允许只存临时 URL。
4. 管理端导出 Excel 的数据源必须来自 DB 查询结果。

### 5.9 发布与升级

二期纳入持久化主链路：

- `release_manifests` 返回真实 DB 数据，不再 mock。
- `release_strategies` 保存 optional/recommended/force/grayroll。
- `release_audit_log` 记录发布、回滚、策略变更。
- 客户端上报版本和设备信息到 `app_update_preferences`。

## 6. 实施阶段

### Phase 0：冻结内存态回退

目标：明确不再接受新的业务内存状态。

任务：

- 扫描 `crates/server/src`：`HashMap`、`RwLock`、`Mutex<Vec`、`in-memory`、`mpsc`、`Vec<.*Record`。
- 对每一处标注：允许的临时变量 / 必须迁移的业务状态。
- 将 `app_access_tokens`、`commercial_entitlements` 从 `DbStore::init_schema` 移入 SQL migration。
- 确认 `ServerState::new()` 连接 DB 失败即返回错误，`router()` 启动失败。

验收：

- 无业务 `HashMap`、`RwLock<Vec>`。
- 启动时 PostgreSQL 不可用则服务启动失败。

### Phase 1：服务端编译与 DB 主链路收口

目标：让 `doc-server` 可编译、可启动、可登录、可上传、可创建任务。

任务：

- 收口 `state.rs` 与 `DbStore` 接口。
- `plans` / `recharge_options` / `redeem_code_options` 改为 DB 查询。
- `auth_refresh` 接 `auth_refresh_tokens` rotation。
- 所有 `unwrap_or_default()` 隐藏 DB 错误的 API 改为返回 `ApiError`。
- 补 `DbStore` 单元/集成测试。

验收：

- `cargo check -p doc-server`
- 登录、刷新、用量、套餐、充值记录、兑换记录均从 DB 返回。

### Phase 2：上传、转换任务、文件、报告全持久化

目标：服务重启后上传、任务、产物、日志、报告可恢复。

任务：

- `FileStorage::load_key()` 成为下载和 worker 读取的唯一文件入口。
- `download_conversion_zip/log/docx` 改为 DB key 读取。
- `conversion_jobs` 增加 `report_json`、queue 字段。
- `complete_job` / `fail_job` 写 `report_json`。
- `result_report_key` 只作为文件 key，不再保存 JSON 字符串。

验收：

- 创建 conversion 后重启服务，仍能查询 job。
- 转换完成后重启服务，仍能下载 docx/log/report。
- 跨日期下载历史文件成功。

### Phase 3：DB Queue 替换内存队列事实源

目标：worker 崩溃或服务重启后 queued/processing 任务可恢复。

任务：

- 实现 `claim_next_job(worker_id)`。
- 实现 `recover_stale_jobs()`。
- `mpsc` 改名/抽象为 `WorkerNotify`，只做唤醒。
- worker 启动时先扫描 queued/stale jobs。
- attempts/backoff/max attempts 落库。

验收：

- 创建 job 后不依赖 mpsc 消息也能被 worker 扫描执行。
- worker 中断后任务可重新 queued 或 failed。
- 并发 worker 不会重复领取同一任务。

### Phase 4：用量账本与失败返还

目标：云转换权益、preview quota、兑换码余额均可审计。

任务：

- 新增 `usage_ledger`。
- 创建 job 时 reserve。
- 成功时 commit。
- 失败/取消时 refund。
- `GET /v1/usage` 从 ledger 和 entitlements 聚合。
- 兑换码核销和充值写 ledger 或至少写 entitlement audit。

验收：

- 成功转换扣一次。
- 失败转换返还。
- 重试不重复扣费。
- 管理端可按用户/job 追溯扣减原因。

### Phase 5：反馈与管理查询 SQL 化

目标：反馈与兑换记录支持生产级分页过滤导出。

任务：

- `admin_list_feedback_threads` SQL 过滤分页。
- `admin_list_redeem_batches` 支持分页、渠道、状态、批次号过滤。
- `redeem_code_events` 记录导出、失败兑换、重复兑换。
- Excel 导出从 DB 查询，不依赖已有内存对象。

验收：

- 管理端查询 10 万条反馈/兑换码时不会全量加载。
- 导出前后 `exported_count` 和 `redeem_code_events` 正确。

### Phase 6：发布升级持久化

目标：版本检查、灰度、回滚、审计全部落库。

任务：

- `GET /v1/releases/:channel` 查 DB。
- 增加 admin release CRUD。
- 增加 release strategy、rollout、audit 表。
- Slint/Flutter 版本检查使用真实 manifest。

验收：

- 发布新版本后客户端能查到。
- 回滚后客户端不再收到坏版本。
- 审计日志可追溯发布操作。

## 7. API 行为调整清单

| API | 当前重点 | 调整目标 |
|---|---|---|
| `/v1/auth/register` | demo upsert | 注册语义明确，重复 email 返回已存在或登录 |
| `/v1/auth/login` | demo upsert | 校验密码，失败 401 |
| `/v1/auth/refresh` | token 简化 | refresh token hash + rotation + revocation |
| `/v1/plans` | 硬编码 | DB `billing_plans` |
| `/v1/recharge/options` | 硬编码 | DB `redeem_packages` + pricing |
| `/v1/uploads` | DB + file 初步完成 | 保证只存 key，不依赖内存 bytes |
| `/v1/conversions` | DB job + mpsc | DB queue + quota reserve |
| `/v1/conversions/:id` | DB 查询 | 返回 storage_info、report summary、quota state |
| `/v1/conversions/:id/download/*` | 部分按 job_id 拼路径 | 全部按 DB object key |
| `/v1/conversions/:id/report` | report JSON 字符串 | `report_json` |
| `/v1/usage` | 部分聚合 | ledger + entitlement + storage 聚合 |
| `/admin/v1/feedback/threads` | 内存过滤分页风险 | SQL 过滤分页 |
| `/v1/releases/:channel` | mock/硬编码风险 | DB manifest + strategy |

## 8. 测试与验收标准

### 8.1 数据库集成测试

必须覆盖：

- DB 不可用时 `router()` 返回错误，服务不启动。
- 注册、登录、refresh、token revoke。
- 套餐、兑换包从 DB 返回。
- 创建兑换码批次、导出、核销、重复核销失败。
- 充值写 `recharges`，权益写 `commercial_entitlements`。
- 上传写 `uploads` 和文件系统，重启后可读。
- 创建 conversion 写 `conversion_jobs(status=queued)`。
- worker 执行成功后写 DOCX/log/report，重启后可下载。
- worker 失败后写 failed、error_code、log、report，并返还额度。
- 反馈创建、追加、管理回复、导出。
- release manifest 查询、发布、回滚。

### 8.2 服务重启恢复测试

建议增加 E2E：

```text
1. 启动 doc-server + PostgreSQL
2. 注册登录
3. 上传 zip
4. 创建 conversion
5. 在 queued 或 processing 时停止服务
6. 重启服务
7. worker 重新领取任务或标记失败
8. 查询 conversion 状态
9. 下载 source.zip / result.docx / conversion.log / report
```

### 8.3 内存状态扫描门禁

每次提交前运行：

```powershell
Select-String -Path "crates\server\src\*.rs" `
  -Pattern "HashMap|RwLock|Mutex<Vec|in-memory|Vec<.*Record|mpsc" `
  -CaseSensitive:$false
```

判定规则：

- `mpsc` 允许存在，但只能作为 worker notify，不得作为任务事实源。
- `Vec<Record>` 允许作为 DB 查询返回值，不允许作为长期状态字段。
- `HashMap/RwLock/Mutex` 只允许用于 schema 初始化锁、配置缓存或短期非业务缓存。

### 8.4 推荐命令

```powershell
cargo check -p doc-server
cargo test -p doc-server --test api -- --nocapture
cargo test -p doc-commercial-api-client
cargo test -p doc-desktop-slint
flutter test --no-pub
flutter analyze
```

如引入真实 PostgreSQL 集成测试，建议增加：

```powershell
$env:DATABASE_URL="postgres://postgres:postgres@localhost:5432/docdb_test"
cargo test -p doc-server --test db_persistence -- --nocapture
```

## 9. 风险与处理

| 风险 | 表现 | 处理 |
|---|---|---|
| 当前迁移中间态导致编译失败 | `state.rs` / `db_store.rs` / `routes.rs` 接口不一致 | 先跑 `cargo check -p doc-server`，以编译错误为任务清单收口 |
| DB schema 与代码重复初始化冲突 | SQL 文件与 `DbStore::init_schema` 各自建表 | 把新增辅助表迁入 SQL，`init_schema` 只执行 migration |
| 文件 key 与日期路径不一致 | 跨日下载失败 | 所有读取通过 DB object key，不用 `session_dir(job_id)` 反推 |
| worker 重复消费 | 多 worker 同时处理同一 job | `FOR UPDATE SKIP LOCKED` + status/worker_id/locked_at |
| 失败重复扣费 | job 重试或失败未返还 | usage ledger 加幂等键：`conversion_job_id + event_type` |
| 查询全量加载 | 管理端反馈/兑换记录数据大后变慢 | SQL where/limit/offset，必要时加索引 |
| token 安全不足 | demo token 长期有效 | hash、expires_at、revoked_at、rotation、last_used_at |

## 10. 推荐立即执行顺序

1. **先修编译**：以 `cargo check -p doc-server` 为第一门槛，收口 `ServerState`、`DbStore`、`routes`、`worker_service`。
2. **迁移辅助表到 SQL**：`app_access_tokens`、`commercial_entitlements`、`usage_ledger`、`conversion_jobs.report_json/queue字段`。
3. **改文件读取**：所有 ZIP/DOCX/log/report 下载从 DB key 走 `FileStorage::load_key()`。
4. **落 DB queue**：`mpsc` 降级为 notify，worker 通过 `claim_next_job` 领取任务。
5. **补 usage ledger**：创建任务 reserve，成功 commit，失败 refund。
6. **SQL 化管理查询**：反馈、兑换码批次、兑换记录全部分页过滤落 SQL。
7. **补重启恢复 E2E**：证明服务重启后任务、文件、权益均不丢。

## 11. 最终验收定义

当以下条件全部满足，才可认为“PostgreSQL 持久化主链路完成”：

- 服务端无任何业务数据只存在内存。
- PostgreSQL 不可用时服务启动失败。
- 上传、任务、产物、日志、报告、反馈、兑换码、充值、用量、token 均可在服务重启后恢复。
- worker 不依赖内存队列保存任务，支持 DB 领取、重试、超时恢复。
- 所有扣费、返还、充值、兑换有可审计记录。
- 所有文件读取通过 DB object key，不依赖当天日期推导路径。
- 管理端列表和导出支持 SQL 过滤分页，不全量加载到内存。
- `cargo check -p doc-server`、服务端 DB 集成测试、重启恢复 E2E 全部通过。

完成后，Tex2Doc 的商业化后端才能从 Preview 态进入邀请制 Beta 所需的“可恢复、可审计、可支持”状态。
