# Tex2Doc Slint 与 Flutter 商业化开发目标及技术方案
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



**日期**：2026-06-23  
**输出目录**：`docs-zh/dev`  
**范围**：Slint 桌面客户端、Flutter 多端客户端/Web/PWA 入口、Rust `doc-server` 商业 API 与云端转换服务。  
**目标**：结合当前实现现状，规划下一步开发目标和技术方案，使客户端与服务端能力达到受控 PoC、邀请制 Beta 和后续商业化推广的要求。

> 说明：当前工程中真正的服务端是 Rust `crates/server`。Flutter 位于 `flutter_app`，更适合作为 Web/PWA、多端演示和轻量获客入口。本文将“Slint 客户端”和“Flutter 多端入口”分别规划，并把它们依赖的 Rust 服务端能力一并纳入。

## 一、当前现状

### 1.1 Slint 桌面客户端现状

Slint 桌面端已经接近商业化 Preview 客户端形态，当前能力包括：

| 模块 | 当前状态 | 相关实现 |
|---|---|---|
| 本地转换 | 已接入项目路径、profile、quality、输出 DOCX/report | `crates/desktop-slint/src/local_convert.rs`, `commands.rs`, `main.rs` |
| 云端转换 | 已有目录/zip 打包、上传、创建 conversion、轮询、下载 DOCX/report | `cloud_convert.rs` |
| 账号系统 | 已有 register、login、refresh、usage、plans | `cloud_account.rs` |
| 账单入口 | 已有 checkout、billing portal 阻塞桥和浏览器打开逻辑 | `cloud_account.rs`, `main.rs` |
| 凭据存储 | refresh token 不写入 settings，尝试走 macOS `security`、Linux `secret-tool`、Windows PowerShell DPAPI 文件 | `credential_store.rs` |
| 任务历史 | recent jobs 可持久化，记录输出路径和 report 路径 | `job.rs`, `job_history.rs` |
| 诊断包 | 可导出状态、recent jobs、版本和更新信息 | `diagnostics.rs` |
| 更新检查 | 可请求 release manifest、校验版本和 sha256 状态 | `desktop_update.rs`, `updater.rs` |
| UI 产品化 | 已有多 Tab、i18n、主题、多语言和商业色调 | `main.slint`, `i18n.rs`, `theme.rs` |

主要缺口：

- 云端转换前未形成强制的额度预检、失败返还和可恢复轮询。
- 任务进度仍偏粗，无法完整展示 server worker 的阶段、排队位置、失败错误码和修复建议。
- token 安全存储还处于 preview adapter，需要三平台验证。
- 桌面端缺少商业发布打包、签名、公证、安装、升级执行和回滚。
- 诊断包缺少完整 compile report、server job id、错误码、日志摘要和用户授权说明。
- GUI 手工/自动验收矩阵尚未成为发布门禁。

### 1.2 Flutter 多端入口现状

Flutter 当前已经从早期 demo 页面升级为具备设计系统的多端产品壳：

| 模块 | 当前状态 | 相关实现 |
|---|---|---|
| 产品壳 | `Sidebar + TopBar + Workspace`，支持桌面/平板/移动响应式 | `flutter_app/lib/workspace_app.dart` |
| 主题 | 支持 default/blue/green/purple/orange/dark | `ui/app_theme.dart`, `ui/app_tokens.dart` |
| 多语言 | 接入 `flutter_localizations` 与项目 i18n delegate | `ui/app_i18n.dart` |
| 偏好设置 | 主题和语言可持久化 | `ui/app_preferences*.dart` |
| 本地/浏览器转换 | 支持选择 zip、调用 bridge、下载 DOCX | `workspace_app.dart`, `bridge*.dart`, `wasm_bridge.dart`, `native_bridge.dart` |
| 商业 API | 已有 `CommercialApiClient`，支持 register/login/usage/plans | `commercial_api.dart` |
| 状态组件 | 已有 Loading、Empty、Error、StatusPill 等 | `ui/app_components.dart` |

主要缺口：

- Flutter 商业 API 仅覆盖账号、用量、套餐，没有云端 upload/conversion/download/report。
- 账号状态只在页面内存里，缺少 session 恢复、refresh token、退出登录和安全存储策略。
- ConvertPanel 当前主要走本地 bridge/wasm 转换，不是商业云端转换闭环。
- 缺少 waitlist、试用引导、套餐页、下载桌面端 CTA、隐私说明和错误上报。
- Flutter 若作为 Web/PWA 入口，需要处理浏览器 CORS、文件大小限制、上传进度、断点失败和移动端布局。

### 1.3 Rust 服务端现状

`crates/server` 当前是商业 API 和云端转换的 preview server：

| 能力 | 当前状态 | 相关实现 |
|---|---|---|
| API 路由 | `/v1` 与 `/api/v1` 双路径兼容 | `routes.rs` |
| Auth | register/login/refresh 返回 demo token | `routes.rs` |
| 访问控制 | Bearer token preview 校验 | `require_session` |
| Usage | in-memory 用量计数，preview 限额 100 次 | `state.rs` |
| Plans/Billing | 返回 preview/pro 套餐，checkout/portal 为 mock URL | `routes.rs` |
| Upload | multipart 上传 zip，存入内存 | `routes.rs`, `state.rs` |
| Conversion | 创建 job，进入 mpsc queue | `routes.rs`, `worker_service.rs` |
| Worker | 调用 SemanticTexEngine，失败可 fallback legacy rule | `worker_service.rs` |
| Report | 返回 conversion report JSON | `state.rs`, `worker_service.rs` |
| Release | 返回 preview release manifest | `routes.rs` |

主要缺口：

- 用户、refresh token、订阅、用量、上传、任务、产物仍未落库。
- demo token 未替换为 JWT，密码未 Argon2id hash，refresh token 未 hash/轮换/撤销。
- upload/job/docx/report 全部在内存中，服务重启即丢。
- worker 缺少持久化队列、对象存储、任务恢复、取消、重试、timeout、dead letter。
- 云端 TeX 执行缺少 sandbox、zip guard、资源限制和日志脱敏。
- billing 缺真实 provider、webhook 签名、幂等、订阅状态同步和失败返还。

## 二、商业化推广前的产品定位

### 2.1 Slint 的定位

Slint 桌面端应作为首个商业化主客户端：

```text
Tex2Doc Pro Desktop
  本地转换
  云端高质量转换
  账号/套餐/用量
  质量报告
  诊断包
  自动更新
```

商业推广时，Slint 客户端负责承接真实用户转换流程。它必须优先达到“可安装、可登录、可转换、可诊断、可升级”的水平。

### 2.2 Flutter 的定位

Flutter 不建议在下一阶段承担主商业客户端，而应作为：

```text
Web/PWA 获客入口
  在线轻量试用
  账号注册/登录
  用量和套餐查看
  小文件云端转换体验
  引导下载 Pro Desktop
```

如果资源有限，Flutter 的第一优先级不是补齐桌面全部能力，而是成为 landing page 后面的互动试用入口，把用户转化到 waitlist 或 Slint Desktop。

### 2.3 Rust 服务端的定位

Rust 服务端是商业化的生产底座：

```text
Commercial API + Worker Control Plane
  auth/session
  plans/billing/usage ledger
  upload/conversion/artifact
  release manifest
  worker queue
  sandbox conversion
  observability/support
```

商业化推广是否能扩大，取决于服务端能否从 preview mock 迁移到可持久化、可审计、可隔离、可恢复的 Beta 架构。

## 三、下一步开发目标

### 3.1 14 天目标：受控 PoC 可交付

目标：让 5-10 个合作用户能在人工支持下完成真实论文试用。

| 方向 | 目标 | 验收 |
|---|---|---|
| Slint | 完成云端转换用户路径收口 | 登录、查看额度、选择项目、云端转换、下载 DOCX/report、导出诊断包 |
| Flutter | 完成商业 API 面板稳定化 | register/login/usage/plans 可联调 preview server，错误可读 |
| Server | 完成 preview API 稳定化 | 合同测试覆盖 auth/usage/plans/upload/conversion/report |
| 安全 | 增加 zip guard 第一版 | 过大 zip、空 zip、zip-slip、超文件数稳定拒绝 |
| 质量 | 输出 conversion stats | paper3、7 profile fixtures、DOCX ZIP/XML/openability 可追踪 |
| 推广 | 形成 demo + waitlist | 3 个 demo 包、试用手册、用户反馈表 |

### 3.2 30 天目标：邀请制 Beta 准备

目标：从 preview mock 进入生产底座初版。

| 方向 | 目标 | 验收 |
|---|---|---|
| Slint | Beta 客户端基础体验 | 三平台至少完成手工启动和转换验收，Windows 优先打包 |
| Flutter | 云端转换轻量入口 | upload/create/poll/download/report 可用，限制小文件 |
| Server | 生产 auth/store 起步 | PostgreSQL、JWT、refresh token hash、usage ledger |
| Worker | 持久化任务和本地对象存储 | 服务重启后 job/upload/artifact 不丢 |
| Billing | 支付沙箱 | checkout/webhook 幂等流程跑通 |
| Support | 诊断闭环 | 可按 job_id 定位用户、输入、阶段、错误、report |

### 3.3 60-90 天目标：付费 Beta 准入

目标：具备小规模收费和稳定支持能力。

| 方向 | 目标 | 验收 |
|---|---|---|
| Slint | 签名安装包和自动更新 | Windows/macOS/Linux 至少两个平台稳定发布 |
| Flutter | Web/PWA 获客闭环 | 试用、登录、套餐、云端小文件转换、下载桌面端 CTA |
| Server | 生产计费和用量 | usage ledger 可审计，失败返还，支付状态同步 |
| Worker | sandbox worker | 禁网络、timeout、CPU/memory/disk/process 限制 |
| Quality | Beta 样本门禁 | 每 profile 10+ realistic，DOCX openability >= 98% |
| Ops | 监控告警 | queue depth、fail rate、duration、billing webhook failure 可观测 |

## 四、Slint 客户端技术方案

### 4.1 用户路径收口

目标路径：

```text
启动
  -> 自动恢复 session
  -> 拉取 /v1/me 和 /v1/usage
  -> 选择本地或云端转换
  -> 选择项目目录或 zip
  -> 自动识别 main tex
  -> 选择 profile/quality
  -> 云端转换前额度预检
  -> 上传、排队、转换、验证、下载
  -> 展示 report 摘要
  -> 写入 recent jobs
  -> 可打开 DOCX/report 或导出诊断包
```

实施要点：

- 启动恢复不只提示“stored session found”，应自动调用 refresh，再调用 `/v1/me` 和 `/v1/usage`。
- 云端转换前必须检查 token、usage、project/main_tex、输出路径和 API base URL。
- 额度不足时直接跳转 Billing 页或提示 checkout。
- `poll_until_ready` 应暴露阶段进度，不只是内部轮询。
- recent jobs 中保存 server `job_id`，便于恢复轮询和售后定位。

### 4.2 云端转换状态模型

把服务端状态映射到 UI：

| Server 状态 | UI 阶段 | 文案 |
|---|---|---|
| `queued` | 排队 | Waiting for worker |
| `normalizing` | 输入整理 | Preparing project |
| `detecting` | 主文件/Profile 识别 | Detecting document |
| `analyzing` | 兼容性分析 | Analyzing TeX project |
| `compiling` | 转换 | Converting to DOCX |
| `rendering` | 渲染产物 | Rendering output |
| `verifying` | 验证 | Verifying DOCX |
| `completed` | 完成 | Ready |
| `failed` | 失败 | Failed with actionable reason |
| `expired` | 过期 | Artifacts expired |

技术实现：

- `cloud_convert::poll_until_ready` 增加 progress callback 或返回阶段事件。
- UI 中新增阶段状态属性：`cloud_job_id`、`cloud_job_status`、`cloud_progress_label`、`cloud_error_code`。
- 错误展示优先使用服务端 `error_code`，没有时 fallback 到 error string。

### 4.3 账号与凭据

短期：

- access token 仅保存在内存。
- refresh token 进入 `credential_store`。
- settings 只保存 `api_base_url`、`last_login_email`、locale、theme、release channel。

Beta：

- Windows 优先从 PowerShell DPAPI 文件升级到更明确的 Credential Manager 或 DPAPI adapter。
- macOS `security` 命令和 Linux `secret-tool` 增加可用性检测和用户提示。
- refresh token 失败时清理本地 session，避免反复失败。
- logout 调服务端 `/v1/auth/logout` 后再删除本地 refresh token。

### 4.4 Billing 与套餐

实施要点：

- Billing 页展示当前套餐、周期、云转换额度、剩余额度、下次重置时间。
- checkout/portal 使用系统浏览器，客户端不内嵌支付页。
- checkout 成功后提供“Refresh subscription”按钮，重新拉 `/v1/me` 和 `/v1/usage`。
- 额度不足时从 Convert 页跳转或提示 Billing 操作。

### 4.5 诊断包与售后

PoC 诊断包应包含：

```text
diagnostics.json
status.txt
recent_jobs.txt
compile_report.json
cloud_job.json
app_settings.redacted.json
update_status.txt
```

注意：

- 默认不打包用户源码。
- 如需源码样本，必须有单独授权提示。
- token、邮箱可脱敏，job_id 保留。

### 4.6 打包与发布

优先顺序：

1. Windows x64 installer。
2. macOS DMG/pkg。
3. Linux AppImage/deb。

技术要求：

- artifact 生成 sha256。
- release manifest 按 channel/platform/arch 查询。
- manifest 加 Ed25519/minisign 签名。
- updater 下载后先校验 sha256，再校验 signature。
- 安装失败保留旧版本和手工下载链接。

## 五、Flutter 多端入口技术方案

### 5.1 角色边界

Flutter 下一阶段不复制 Slint 的完整桌面工作台，而是补齐商业获客和轻量试用能力：

```text
访问 Web/PWA
  -> 注册/登录
  -> 查看套餐和额度
  -> 上传小 zip 试转换
  -> 下载 DOCX/report
  -> 失败时展示诊断
  -> 引导下载 Slint Pro Desktop
```

### 5.2 CommercialApiClient 扩展

在 `flutter_app/lib/commercial_api.dart` 增加：

```text
refresh(refreshToken)
me(accessToken)
checkout(accessToken, planId)
portal(accessToken)
uploadProjectZip(accessToken, bytes, fileName)
createConversion(accessToken, uploadId, mainTex, profile, quality)
getConversion(accessToken, jobId)
downloadConversionDocx(accessToken, jobId)
getConversionReport(accessToken, jobId)
releaseManifest(channel, platform, arch)
```

数据模型：

```text
ConversionJob
UploadResult
ConversionReport
BillingSession
ReleaseManifest
ApiErrorBody
```

### 5.3 Session 管理

Web/PWA：

- access token 内存保存。
- refresh token 可放 `localStorage`，但必须明确标记为 preview 策略。
- Beta 前建议使用 httpOnly secure cookie 或短期 refresh token 策略。

桌面/移动 Flutter：

- 使用平台安全存储插件前，需要评估引入依赖和跨平台构建成本。
- 如果 Flutter 暂定位为 Web/PWA，可以先不做移动安全存储。

### 5.4 云端转换 UI

Flutter 新增 `CloudConvertPanel`：

- zip 选择。
- main tex 输入，默认 `main.tex`，可自动建议候选。
- profile 下拉：auto、jos-paper、chinese-academic、tacl、cvpr、nature、springer、generic。
- quality 下拉：preview、standard、strict。
- 上传进度。
- job 状态轮询。
- report 摘要。
- DOCX 下载。

限制策略：

- PoC 阶段限制 zip <= 20 MB。
- 浏览器端只允许单个 zip 上传，不做目录打包。
- 大项目提示使用 Desktop。

### 5.5 商业推广组件

Flutter 作为推广入口，需要新增：

- Pricing panel。
- Waitlist / invite code panel。
- Demo samples panel。
- Desktop download CTA。
- Privacy/data retention notice。
- Support contact / diagnostic upload entrance。

所有文案继续走 `AppStrings`，不要回到硬编码。

## 六、Rust 服务端配套技术方案

### 6.1 API 合同

必须稳定以下 API：

```text
POST /v1/auth/register
POST /v1/auth/login
POST /v1/auth/refresh
POST /v1/auth/logout
GET  /v1/me
GET  /v1/usage
GET  /v1/plans
GET  /v1/billing/subscription
POST /v1/billing/checkout
POST /v1/billing/portal
POST /v1/billing/webhook
POST /v1/uploads
POST /v1/conversions
GET  /v1/conversions/:id
GET  /v1/conversions/:id/download/docx
GET  /v1/conversions/:id/report
GET  /v1/conversions/:id/logs
GET  /v1/conversions/:id/diagnostic-bundle
GET  /v1/releases/:channel
```

兼容策略：

- 保留 `/api/v1` 到 Beta 结束。
- 文档中主推 `/v1`。
- 客户端 SDK 统一使用 `/v1`。

### 6.2 生产 Auth

替换 demo token：

- password 使用 Argon2id。
- access token 使用 JWT，15-30 分钟有效。
- refresh token 随机高熵，只存 hash。
- refresh token 支持轮换、撤销、设备标识。
- `/v1/auth/logout` 撤销当前 refresh token。
- auth middleware 从 routes 中抽出。

验收：

```text
无 token 返回 401。
过期 access token 返回 401。
refresh token 可轮换。
logout 后 refresh token 不可继续使用。
服务重启后 session 仍可刷新。
```

### 6.3 PostgreSQL Store

优先使用 `docs-zh/money/001_docdb_business_schema.sql` 作为初始 schema，迁移成 server 可执行 migration。

关键表：

- `app_users`
- `auth_refresh_tokens`
- `billing_plans`
- `subscriptions`
- `usage_periods`
- `usage_events`
- `uploads`
- `conversion_jobs`
- `release_manifests`

建议新增领域模块：

| 模块 | 职责 |
|---|---|
| `commercial_domain` | DTO/领域模型和状态枚举 |
| `commercial_store` | PostgreSQL repository 和 transaction |
| `commercial_auth` | password/JWT/refresh middleware |
| `commercial_billing` | provider trait、webhook、幂等 |
| `commercial_worker` | queue consumer、sandbox runner |

### 6.4 Usage Ledger

转换创建时：

```text
1. 检查 active subscription。
2. 检查 usage period。
3. 创建 quota reservation。
4. 写 usage_events: reserved。
5. 创建 conversion_jobs。
6. 入队。
```

转换结束：

```text
success:
  reserved -> consumed

failed/cancelled/timeout:
  reserved -> refunded
```

验收：

- 并发创建不超卖。
- 失败任务返还额度。
- 所有额度变化可通过 usage_events 重放。

### 6.5 Upload 与 Worker

Preview 到 Beta 迁移：

| 阶段 | Upload | Queue | Artifact |
|---|---|---|---|
| 当前 | 内存 bytes | mpsc | 内存 docx/report |
| PoC | 内存 + zip guard | mpsc + timeout | 内存 + report 错误码 |
| Beta | PostgreSQL metadata + local blob dir | Postgres queue | local artifact store |
| GA | PostgreSQL + S3/MinIO | Redis/NATS/Postgres queue | S3/MinIO + retention |

最小 sandbox：

- zip slip 检查。
- 总大小、文件数、单文件大小限制。
- 每 job 独立 workspace。
- 禁 shell escape。
- no network。
- wall-clock timeout。
- 输出目录大小限制。
- 日志脱敏。

### 6.6 Release Manifest

`/v1/releases/{channel}` 支持：

```text
channel
platform
arch
current_version
```

返回：

```json
{
  "version": "1.26.6.12",
  "channel": "beta",
  "platform": "windows",
  "arch": "x64",
  "download_url": "...",
  "sha256": "...",
  "signature": "...",
  "release_notes": "..."
}
```

服务端从 `release_manifests` 表读取，不再返回静态 preview manifest。

## 七、联调与测试方案

### 7.1 API 合同测试

Server 必须覆盖：

```text
auth register/login/refresh/logout
me/usage/plans
billing checkout/portal
upload zip
create conversion
poll conversion
download docx
get report
quota exceeded
release manifest
```

命令建议：

```powershell
cargo test -p doc-server
```

### 7.2 Slint 验收

最低验收矩阵：

| 场景 | Windows | macOS | Linux |
|---|---|---|---|
| 启动和设置恢复 | 必测 | 必测 | 必测 |
| 注册/登录/刷新/退出 | 必测 | 必测 | 必测 |
| 本地转换 | 必测 | 必测 | 必测 |
| 云端转换 | 必测 | 必测 | 必测 |
| checkout/portal | 必测 | 可延后 | 可延后 |
| 诊断包 | 必测 | 必测 | 必测 |
| 更新检查 | 必测 | 必测 | 必测 |
| token 安全存储 | 必测 | 必测 | 必测 |

命令建议：

```powershell
cargo test -p doc-desktop-slint
cargo check -p doc-desktop-slint
```

### 7.3 Flutter 验收

最低验收：

```powershell
cd flutter_app
flutter test
flutter build web --no-source-maps --no-tree-shake-icons
```

Web 手工验收：

- 主题切换。
- 语言切换。
- register/login。
- usage/plans。
- 上传 zip。
- 云端转换。
- 下载 DOCX/report。
- 额度不足提示。

### 7.4 端到端联调

推荐本地联调顺序：

```powershell
cargo run -p doc-server
cargo run -p doc-desktop-slint
cd flutter_app; flutter run -d chrome
```

联调样本：

- paper3 zip。
- 7 profile minimal fixture。
- 1 个缺图片样本。
- 1 个 unsupported package 样本。
- 1 个超大小 zip。
- 1 个 zip-slip 恶意样本。

## 八、阶段排期

### Week 1：PoC 客户端收口

| 任务 | 产出 |
|---|---|
| Slint 启动自动 refresh + usage | 重启后恢复登录 |
| Slint 云端转换前 usage 预检 | 额度不足阻断 |
| Slint recent jobs 保存 server job id | 支持售后定位 |
| Flutter API 错误模型优化 | 错误可读、可展示 |
| Server zip guard 第一版 | 防 zip-slip 和大文件 |
| API 合同测试补齐 | auth/upload/conversion/release |

### Week 2：Flutter 云端入口和诊断

| 任务 | 产出 |
|---|---|
| Flutter 扩展 upload/conversion/report API | `CommercialApiClient` 完整云转方法 |
| Flutter 新增 CloudConvertPanel | Web/PWA 小文件云转体验 |
| Slint 诊断包增加 cloud job/report | 售后可定位 |
| Server report 增加 error_code | 客户端可解释失败 |
| conversion stats 输出 | 发布前质量门禁 |

### Week 3-4：生产底座起步

| 任务 | 产出 |
|---|---|
| PostgreSQL migration | docdb 可初始化 |
| JWT + refresh token hash | 替换 demo token |
| usage ledger | 额度可审计 |
| local object storage | upload/artifact 不丢 |
| Postgres queue 第一版 | worker 可恢复 |
| Stripe test mode | checkout/webhook 沙箱 |

### Week 5-8：Beta 化

| 任务 | 产出 |
|---|---|
| Slint Windows installer | 可发给 Beta 用户 |
| release manifest DB 化 | 更新可追踪 |
| manifest 签名 | 更新安全 |
| sandbox worker 第一版 | 云端自助上传安全门槛 |
| Flutter PWA 发布预演 | 获客入口可访问 |
| quality dashboard 初版 | profile 维度质量趋势 |

## 九、商业化准入标准

### 9.1 受控 PoC 准入

- Slint 完成登录、用量、本地转换、云端转换、下载、report、诊断包。
- Flutter 完成 register/login/usage/plans，云端转换可作为 preview。
- Server preview API 合同测试通过。
- paper3 和 7 profile fixtures 通过。
- 每个失败都有错误信息和 report。
- 有 waitlist、试用手册和支持通道。

### 9.2 邀请制 Beta 准入

- Slint Windows 安装包可交付。
- Flutter Web/PWA 可作为轻量试用入口。
- Server 使用 PostgreSQL 保存用户、session、usage、upload、conversion。
- JWT + refresh token hash 可用。
- usage ledger 可审计。
- Worker 有基础 sandbox 和 timeout。
- DOCX openability >= 98%。

### 9.3 付费 Beta 准入

- 支付 provider test/live 切换完成。
- webhook 幂等完成。
- 失败任务返还额度。
- Slint 至少 Windows/macOS 可签名安装。
- release manifest 签名和 sha256 校验完成。
- 支持人员可按 job_id 在 10 分钟内定位失败。

## 十、立即任务清单

| 优先级 | 任务 | 模块 | 截止建议 |
|---|---|---|---|
| P0 | Slint 启动自动 refresh + `/v1/me` + usage | Slint/Server | 2 天 |
| P0 | Slint 云端转换前额度预检 | Slint | 2 天 |
| P0 | server zip guard 和大小/文件数限制 | Server | 3 天 |
| P0 | conversion report 增加 error_code | Server/Slint/Flutter | 3 天 |
| P0 | Flutter 扩展 upload/conversion/report API | Flutter | 4 天 |
| P0 | Slint 诊断包加入 cloud job/report | Slint | 5 天 |
| P1 | Flutter CloudConvertPanel | Flutter | 1 周 |
| P1 | PostgreSQL migration 接入 | Server | 1 周 |
| P1 | JWT + refresh token hash 设计与实现 | Server | 2 周 |
| P1 | Windows installer 预研与产物 | Slint/Release | 2 周 |

## 十一、结论

下一步应以 Slint 作为商业化主客户端，以 Flutter 作为 Web/PWA 获客和轻量试用入口，以 Rust `doc-server` 作为生产化商业底座。三者的优先级不应平均分配：

```text
第一优先级：Slint + Server 跑通受控 PoC 和邀请制 Beta。
第二优先级：Flutter 补齐云端小文件试用和获客转化。
第三优先级：三平台发布、支付、sandbox、监控和质量 dashboard 进入付费 Beta。
```

只要完成 P0 任务，Tex2Doc 就可以用受控 PoC 方式开始商业化推广；完成 P1/P2 任务后，才适合扩大到邀请制 Beta 和付费 Beta。
