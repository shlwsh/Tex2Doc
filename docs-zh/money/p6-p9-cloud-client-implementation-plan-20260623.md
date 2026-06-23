# Tex2Doc P6-P9 云端商业化与客户端对接细化方案

日期：2026-06-23
参考：`docs-zh/plan-0621.md`
输出目录：`docs-zh/money`

## 1. 当前结论

本轮将 P6-P9 从 preview mock 细化为可落地的工程闭环：

- P6：以本地 PostgreSQL `docdb` 为业务库，建立用户、refresh token、订阅、账单、用量、上传、转换任务、发布 manifest 表。
- P7：server 继续保留现有内存 worker preview 路径，但正式设计切换为 DB + 对象存储 + 队列 worker。
- P8：现有 nightly regression 保留，新增商业 API、用量扣减、账单状态、worker 状态的验收指标。
- P9：release manifest 从静态 JSON 过渡到 `release_manifests` 表，客户端按 channel/platform/arch 获取更新。
- 客户端：Slint 桌面端复用现有 `doc-commercial-api-client`；Flutter 增加 `CommercialApiClient`，先完成账号、用量、套餐查询的服务端对接入口。

## 2. PostgreSQL 初始化

本机 shell 当前未发现 `psql` 命令，但用户环境可使用 PostgreSQL 时执行：

```powershell
createdb -U postgres docdb
psql -U postgres -d docdb -f docs-zh/money/001_docdb_business_schema.sql
```

连接串建议：

```text
DATABASE_URL=postgres://postgres:postgres@localhost:5432/docdb
```

### 核心表

| 表 | 作用 |
|---|---|
| `app_users` | 用户主档，保存 email、password_hash、状态、默认套餐 |
| `auth_refresh_tokens` | refresh token hash、设备、过期和撤销信息 |
| `billing_plans` | 套餐目录，包含价格、额度、存储、功能 |
| `subscriptions` | 用户订阅周期和 provider 映射 |
| `invoices` | 账单和支付状态 |
| `usage_periods` | 月度额度窗口 |
| `usage_events` | 用量流水，云转换和存储都只追加流水 |
| `uploads` | 上传包元数据，二进制进入对象存储或本地 blob 目录 |
| `conversion_jobs` | 云端转换任务状态机 |
| `release_manifests` | P9 更新 manifest，按 channel/platform/arch/version 管理 |

## 3. P6 账号、订阅、用量 API

### API 契约

| API | 行为 | 数据表 |
|---|---|---|
| `POST /v1/auth/register` | 创建用户，Argon2id hash 密码，创建 preview subscription 和 usage_period | `app_users`, `subscriptions`, `usage_periods` |
| `POST /v1/auth/login` | 校验密码，签发 access JWT 和 refresh token | `app_users`, `auth_refresh_tokens` |
| `POST /v1/auth/refresh` | 校验 refresh token hash，轮换 token | `auth_refresh_tokens` |
| `GET /v1/me` | 返回当前用户和套餐 | `app_users`, `subscriptions` |
| `GET /v1/usage` | 汇总当期用量 | `usage_periods`, `usage_events` |
| `GET /v1/plans` | 返回套餐目录 | `billing_plans` |
| `POST /v1/billing/checkout` | 创建支付会话 | `subscriptions`, provider |
| `POST /v1/billing/portal` | 创建账单管理入口 | provider |

### 扣减策略

云转换不直接修改计数字段，而是在创建 conversion job 前做额度预占：

1. 查当前 active subscription 和 usage_period。
2. 汇总 `usage_events where event_type='cloud_conversion'`。
3. 若 `used >= limit`，返回 `402 quota_exceeded`。
4. 插入 `usage_events(quantity=1, source_id=job_id)`。
5. worker 最终失败时保留流水但标记 metadata，Beta 后可补“失败返还”策略。

## 4. P7 云端转换 worker

### 状态机

```text
queued -> normalizing -> detecting -> analyzing -> compiling -> rendering -> verifying -> completed
                                                        \-> failed
```

### 单体 preview 到生产演进

| 阶段 | 实现 |
|---|---|
| Preview | 当前 `ServerState` 内存 uploads/jobs/usage + `worker_service` |
| Beta | PostgreSQL 持久化 metadata，本地磁盘对象存储，tokio queue |
| GA | PostgreSQL + S3/MinIO + Redis queue，worker 独立进程 |

### Sandbox 要求

- 每个 job 独立工作目录。
- 禁用外部网络。
- 限制 wall time、CPU、内存、输出目录大小和子进程数。
- 输入 zip 解压前检查 zip-slip、总大小、文件数量。
- worker 输出 `result_docx_key`、`result_report_key`，server 只负责鉴权和下载代理。

## 5. P8 回归和质量指标

新增商业化回归矩阵：

| 维度 | Preview 门槛 | Beta 门槛 |
|---|---:|---:|
| 注册/登录/刷新成功率 | 100% contract test | 100% integration test |
| 用量扣减准确率 | 100% | 100% with concurrent tests |
| quota exceeded 拦截 | 100% | 100% |
| 云转换成功率 | 现有 28 fixtures 通过 | 每 profile 10+ realistic |
| DOCX openable | ZIP header | LibreOffice/Word smoke |
| 账单 webhook 幂等 | mock test | provider sandbox test |
| release manifest 校验 | sha256 格式 | sha256 + signature |

## 6. P9 自动升级和分发

`release_manifests` 表承载真实发布记录：

```text
GET /v1/releases/{channel}?platform=windows&arch=x64
```

返回：

```json
{
  "version": "0.1.0",
  "channel": "beta",
  "download_url": "https://releases.tex2doc.cn/desktop/windows/x64/Tex2Doc-0.1.0.msi",
  "sha256": "...64 hex...",
  "signature": "base64-ed25519-signature",
  "release_notes": "..."
}
```

客户端流程：

1. Slint 启动或点击 Check update。
2. `doc-commercial-api-client::release_manifest(channel)` 获取 manifest。
3. `updater::parse_manifest` + `verify_sha256`。
4. Beta 接入 Ed25519 公钥验签。
5. 按平台执行安装器，失败回滚或保留旧版本。

## 7. Slint 桌面端对接规划

现有状态：

- `cloud_account.rs` 已有 register/login/refresh/usage/plans/checkout/portal 阻塞桥。
- `cloud_convert.rs` 已有 upload/create/poll/download/report。
- `ui_bindings/account.rs` 和 `ui_bindings/billing.rs` 已接入 UI callback。

下一步：

1. 将 access token 保持内存态，refresh token 进入 `credential_store`。
2. 增加 `/v1/me` 启动恢复流程：有 refresh token 时自动 refresh，再拉 usage。
3. 云转换前强制 `GET /v1/usage`，额度不足时阻断。
4. P9 update page 使用 release manifest 的 platform/channel 查询。

## 8. Flutter 客户端对接规划

本轮新增 `flutter_app/lib/commercial_api.dart`：

- `register`
- `login`
- `usage`
- `plans`

并在 Flutter 首页加入商业 API 面板，默认指向：

```text
http://127.0.0.1:8080/v1/
```

Preview 用法：

1. 启动 server：`cargo run -p doc-server`
2. Flutter 中输入邮箱和密码。
3. 点击 Register 或 Login。
4. 点击 Usage 或 Plans 查看服务端返回。

后续扩展：

- 上传 zip：`POST /v1/uploads`
- 创建云转换：`POST /v1/conversions`
- 轮询任务：`GET /v1/conversions/{id}`
- 下载产物：`GET /v1/conversions/{id}/download/docx`

## 9. 验收清单

- `docdb` 可用，并成功执行 `001_docdb_business_schema.sql`。
- `billing_plans` 至少包含 `preview` 和 `pro`。
- server preview contract tests 仍通过。
- Flutter 可注册/登录并刷新 usage/plans。
- Slint 可注册/登录/查看 usage/plans/checkout/portal。
- 云转换任务能从上传到完成，失败有错误码。
- release manifest 的 sha256、signature、platform/channel/version 可追踪。
