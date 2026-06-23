# Semantic TeX Engine 商业化差距评估与 GA 实施方案

**文档版本**：20260622-012427  
**评估日期**：2026-06-22  
**基准报告**：`docs-zh/semantic-tex-engine-progress-report-20260621-180000.md`  
**结合进展**：

- `docs-zh/plan-0621.md`
- `docs-zh/semantic-tex-engine-commercialization-readiness-implementation-plan-20260622-005850.md`
- `docs-zh/semantic-tex-engine-p5-desktop-cloud-account-progress-20260622-011315.md`
- `docs-zh/semantic-tex-engine-p5-desktop-cloud-convert-progress-20260622-011837.md`
- 当前工作区 P5 桌面 Cloud Convert recent jobs 接入、P6/P7 preview server、P8 nightly regression、P9 updater skeleton

**目标**：结合当前真实开发进展，评估项目距离商业化发布还缺哪些工作，并给出可执行、可验收的技术实施方案。

---

## 一、总体结论

当前项目已经具备“受控 PoC / 技术 Preview”的基础，但距离公开收费商业化仍有明显工程缺口。

核心判断：

```text
Semantic TeX -> DOCX 转换核心已经有商业潜力；
商业产品闭环、生产级云端、支付计费、安全隔离、发布运维和质量承诺尚未完成。
```

建议当前发布定位：

| 发布级别 | 当前就绪度 | 结论 |
|---|---:|---|
| 内部 Preview | 90% | 可用于团队内部演示、paper3 回归、7 profile fixture 验证 |
| 受控 PoC | 75% | 可给少量合作用户试用，但需要人工支持与失败样本回收 |
| 邀请制 Beta | 60% | 需要补齐真实账号、持久化、sandbox、GUI 验收和错误诊断 |
| 付费 Beta | 42% | 需要支付计费、用量账本、token 安全存储、安装包和签名升级 |
| 正式 GA | 30% | 需要 SLA、监控告警、合规、三平台签名发布和大样本质量基准 |
| Enterprise | 15% | 需要私有化部署、SSO、审计、租户隔离和模板定制平台 |

因此不建议当前直接公开自助收费发布。更稳妥的商业化路径是：

```text
内部 Preview
  -> 受控 PoC
  -> 邀请制 Beta
  -> 付费 Beta
  -> Pro Desktop + Cloud GA
  -> Team / Enterprise
```

---

## 二、当前开发进展基线

### 2.1 转换核心

当前已经具备：

| 能力 | 状态 | 商业意义 |
|---|---|---|
| V1 Rust rule-based 引擎 | 保持独立 | 可作为 fallback 与效果对照 |
| V2 Semantic TeX Engine | 已独立实现 | 商业化主路径 |
| RuleBased / XeLaTeX Hook / LuaTeX Node 三路径 | 已建立 | 覆盖无 runtime、中文 CTeX、长期 LuaTeX 语义采集 |
| 7 类 Journal Profile | 已实现 | 首期目标市场明确 |
| ActiveProfile / ProfileRef | 已推进 | 支撑自动检测、profile-aware 转换 |
| ProfileStyleMap | 已接 DOCX 渲染 | 支撑期刊样式差异化 |
| CompatibilityAnalyzer / RuleEngine | 已接入方向 | 支撑模板兼容性诊断和宏语义泛化 |
| QualityGate V2 | 已有 | 支撑自动验收和商业质量报告 |
| semantic CLI | 已产品化推进 | 支撑专业用户、CI、桌面端和云端 Worker |
| paper3 三路径脚本 | 已有 | 支撑演示、验收和竞品对比 |

必须继续保持的架构边界：

```text
旧 Rust 规则引擎和新语义引擎作为两条独立实现路径存在。
商业云端可以通过 engine/backend 参数选择执行路径，
但不能把二者耦合成不可验证的一条混合实现。
```

### 2.2 P5 桌面端

当前 Slint 桌面端已从骨架推进到商业闭环 MVP 雏形：

- 本地转换已接入 `SemanticTexEngine::compile_dir_to_docx()`。
- 支持 profile、quality、输出路径、报告摘要、任务历史。
- 新增 Account 区域：API base URL、email、password、Login、Usage。
- 登录后可调用商业 API 并保存 access token、refresh token、用户名称和剩余额度到 `AppState`。
- 新增 Cloud Convert：
  - 目录打包为 zip。
  - 直接上传 zip。
  - 创建云端转换。
  - 轮询任务。
  - 下载 DOCX。
  - 保存 report JSON。
- Cloud Convert 已接入 recent jobs 初版。
- 当前验证：
  - `cargo test -p doc-desktop-slint cloud_convert -- --nocapture` 通过。
  - `cargo check -p doc-desktop-slint` 通过。

仍未商业化的原因：

- 没有系统文件/目录选择器和拖拽。
- 没有注册、退出、刷新登录、忘记密码。
- token 未接系统 keychain。
- billing checkout/portal UI 未接入。
- 云端任务进度仍较粗。
- 未完成 GUI 真实操作验收。
- 未完成 Windows/macOS/Linux 安装包、签名、公证和升级执行。

### 2.3 P6 商业 API

当前 API 已覆盖 preview 合约：

- auth/register、auth/login、auth/refresh。
- me、usage、plans。
- billing checkout/portal。
- uploads。
- conversions create/get/download/report。
- releases manifest。
- `/v1` 与 `/api/v1` 双路径兼容。
- 用户端点、上传、转换、下载和账单端点有 Bearer token 门禁。
- preview 用量有基础额度扣减，额度不足返回 402。

仍未商业化的原因：

- token 是 demo token，不是 JWT。
- 没有密码哈希、邮箱验证、refresh token 轮换与撤销。
- 没有 PostgreSQL 用户、订阅、用量事件表。
- 没有真实支付 provider、webhook、幂等处理。
- 用量是内存累计值，服务重启即丢失。
- 没有租户隔离、RBAC、审计日志。

### 2.4 P7 云端 Worker

当前 Worker 已能完成 preview 级云端转换：

```text
upload zip
  -> create conversion
  -> mpsc queue
  -> worker
  -> SemanticTexEngine::compile_zip_to_docx
  -> fallback legacy rule engine
  -> store docx/report in memory
  -> download endpoints
```

仍未商业化的原因：

- uploads/jobs/docx/report 全部在内存中。
- 无对象存储。
- 无持久化队列。
- 无任务恢复、取消、重试、超时清理。
- 无 sandbox。
- 无 CPU、内存、磁盘、进程数、运行时间限制。
- 无横向扩展和 worker 调度。

### 2.5 P8 回归体系

当前已具备：

- 7 个 profile，每个 minimal + 3 realistic fixture。
- nightly regression 脚本。
- DOCX ZIP 结构检查。
- XML well-formed 检查。
- 可选 LibreOffice headless 打开验证。

仍未商业化的原因：

- realistic fixture 数量不足。
- Word/LibreOffice 实际打开验证不是强制门禁。
- 缺公式、表格、图片、引用、样式覆盖率指标。
- 缺失败样本库、失败分类、质量趋势 dashboard。
- 缺真实用户模板的大样本统计。

### 2.6 P9 自动升级与分发

当前已具备：

- release manifest 解析。
- 版本比较。
- SHA256 校验。
- signature status 占位。
- 服务端 release manifest preview 响应。

仍未商业化的原因：

- 无真实 artifact 下载。
- 无 Ed25519/minisign 或平台签名验签。
- 无 MSI、DMG、AppImage/deb/rpm。
- 无 Windows/macOS 代码签名与公证。
- updater 未接 UI 和安装器执行。

---

## 三、商业化发布必须补齐的能力

### 3.1 产品闭环

商业用户完整链路应为：

```text
注册/登录
  -> 查看套餐和额度
  -> 选择本地或云端转换
  -> 选择 TeX 项目目录或 zip
  -> 自动识别 main tex / profile
  -> 上传并创建任务
  -> 查看进度
  -> 下载 DOCX/report
  -> 查看质量诊断
  -> 用量扣减或失败返还
  -> 订阅升级/管理
  -> 客户端自动升级
```

当前只完成了该链路的 MVP 子集。商业化前至少需要打通：

- Desktop 注册/登录/退出。
- Secure token storage。
- Cloud Convert 进度与失败诊断。
- Billing checkout/portal。
- 用量展示与额度不足处理。
- 任务历史可复用、可打开、可导出诊断包。

### 3.2 生产级云端架构

必须从内存态 preview 迁移到生产架构：

```text
PostgreSQL
  users / sessions / subscriptions / usage_events / uploads / conversions / artifacts / audit_logs

Object Storage
  uploaded_zip / generated_docx / compile_report / compile_log / diagnostic_bundle

Queue
  conversion_queue / retry / dead_letter / priority / delayed_cleanup

Sandbox Worker
  per-job isolated workspace / no network / timeout / cgroup / seccomp / cleanup
```

### 3.3 安全与隔离

TeX 项目是高风险输入，云端商业化必须满足：

- 解压防 zip slip。
- 上传大小、文件数量、单文件大小限制。
- 禁用 shell escape。
- 默认禁止网络访问。
- 每 job 独立临时目录。
- 运行用户降权。
- 限制 CPU、内存、磁盘、进程数、运行时间。
- TeXLive/LuaTeX/XeLaTeX runtime 镜像固定版本。
- 日志脱敏。
- 任务完成后清理临时目录。

### 3.4 质量承诺

商业用户买的是可编辑、可打开、可诊断的 DOCX，不只是生成文件。GA 前必须建立 profile 级质量指标：

| 指标 | Beta 门槛 | GA 门槛 |
|---|---:|---:|
| DOCX 实际打开率 | >= 98% | >= 99% |
| 无崩溃转换率 | >= 98% | >= 99.5% |
| Profile 自动识别准确率 | >= 95% | >= 97% |
| 缺失资源可诊断率 | >= 95% | >= 99% |
| 未解析引用阻断率 | 可报告 | 可报告并定位 |
| 公式 OMML fallback 可见率 | 100% 记录 | 100% 记录并分类 |
| P95 云端转换耗时 | < 120s | < 60s |

### 3.5 计费与用量

正式收费必须实现：

- Argon2id 密码哈希。
- JWT access token。
- Refresh token 轮换、撤销、设备维度管理。
- 邮箱验证和找回密码。
- Plan / Entitlement / Subscription。
- Usage event ledger。
- 额度预占、成功确认、失败返还。
- Stripe 或等价支付 provider。
- Webhook 签名验证。
- Webhook 幂等处理。
- 套餐升级/降级、取消、续订状态同步。

### 3.6 分发、升级和运维

GA 必须完成：

- Windows MSI/MSIX。
- macOS DMG/pkg + notarization。
- Linux AppImage/deb/rpm。
- Release artifact SHA256。
- Manifest 签名。
- 客户端升级 UI。
- 回滚策略。
- 版本兼容策略。
- 监控告警、错误聚合、任务追踪。
- 隐私政策、服务条款、数据删除策略。

---

## 四、目标商业化技术架构

### 4.1 总体架构

```text
Desktop Slint App
  |-- Local Convert: SemanticTexEngine / legacy rule path
  |-- Cloud Convert: Commercial API Client
  |-- Account / Usage / Billing
  |-- Update / Diagnostics

Commercial API
  |-- Auth / Sessions
  |-- Plans / Billing / Webhooks
  |-- Usage Ledger
  |-- Uploads / Artifacts
  |-- Conversion Job API
  |-- Release Manifest

Worker Control Plane
  |-- Queue
  |-- Scheduler
  |-- Retry / Timeout / Cleanup

Sandbox Worker
  |-- unzip guard
  |-- SemanticTexEngine
  |-- XeLaTeX/LuaLaTeX runtime
  |-- DOCX verifier
  |-- report/log bundle

Storage
  |-- PostgreSQL
  |-- Object Storage
  |-- Redis/NATS/Postgres queue

Observability
  |-- metrics
  |-- logs
  |-- traces
  |-- quality dashboard
```

### 4.2 推荐 Rust crate 拆分

| Crate | 目标 |
|---|---|
| `doc-commercial-domain` | 用户、套餐、订阅、用量、转换任务领域模型 |
| `doc-commercial-store` | PostgreSQL repository、migration、transaction |
| `doc-commercial-auth` | 密码哈希、JWT、refresh token、auth middleware |
| `doc-commercial-billing` | 支付 provider trait、Stripe adapter、webhook |
| `doc-commercial-worker` | queue consumer、sandbox runner、artifact writer |
| `doc-commercial-observability` | metrics/log/tracing event model |
| `doc-desktop-slint` | 桌面 UI，继续依赖 `doc-commercial-api-client` |

现有 `crates/server` 可继续作为 preview server，但 GA 应逐步抽出上述生产模块，避免 `routes.rs` 继续承载全部业务逻辑。

### 4.3 数据模型草案

```text
users
  id
  email
  password_hash
  display_name
  email_verified_at
  created_at
  updated_at

refresh_tokens
  id
  user_id
  token_hash
  device_name
  expires_at
  revoked_at
  created_at

plans
  id
  name
  monthly_conversions
  max_upload_bytes
  max_retention_days
  price_cents
  currency

subscriptions
  id
  user_id
  plan_id
  provider
  provider_customer_id
  provider_subscription_id
  status
  current_period_start
  current_period_end

usage_events
  id
  user_id
  conversion_id
  event_type
  units
  idempotency_key
  created_at

uploads
  id
  user_id
  object_key
  file_name
  bytes
  sha256
  status
  created_at
  expires_at

conversions
  id
  user_id
  upload_id
  main_tex
  profile
  engine
  backend
  quality
  status
  error_code
  error_message
  created_at
  started_at
  finished_at

artifacts
  id
  conversion_id
  kind
  object_key
  bytes
  sha256
  created_at

audit_logs
  id
  user_id
  action
  resource_type
  resource_id
  metadata_json
  created_at
```

### 4.4 API 合约升级

保留现有 preview endpoint，但 GA 前补齐：

```text
POST /v1/auth/register
POST /v1/auth/login
POST /v1/auth/refresh
POST /v1/auth/logout
POST /v1/auth/password/forgot
POST /v1/auth/password/reset
GET  /v1/me

GET  /v1/plans
GET  /v1/usage
GET  /v1/billing/subscription
POST /v1/billing/checkout
POST /v1/billing/portal
POST /v1/billing/webhook

POST /v1/uploads
GET  /v1/uploads/:id
DELETE /v1/uploads/:id

POST /v1/conversions
GET  /v1/conversions
GET  /v1/conversions/:id
POST /v1/conversions/:id/cancel
GET  /v1/conversions/:id/download/docx
GET  /v1/conversions/:id/report
GET  /v1/conversions/:id/logs
GET  /v1/conversions/:id/diagnostic-bundle

GET  /v1/releases/:channel
```

---

## 五、实施路线图

### P10: Preview 收口与 PoC 准入

周期：1-2 周

目标：把当前 preview 变成可交给合作用户的受控 PoC。

任务：

- 完成 Slint GUI 真实操作验收。
- 添加文件/目录选择器。
- 添加项目拖拽入口。
- Cloud Convert recent jobs 显示 job id、状态、输出路径和错误。
- 增加本地 server + desktop 的端到端手工验收脚本。
- 增加诊断包导出：输入 profile、report、错误、日志摘要。
- nightly regression 强制生成 `conversion_stats.md`。
- 对 P5-P9 所有当前改动运行 `gitnexus_detect_changes()` 并梳理影响范围。

验收：

```text
桌面端可在 Linux 上完成：
登录 -> 用量 -> 选择 paper3 -> 云端转换 -> 下载 DOCX/report -> recent jobs 可见。

脚本可完成：
server API test
desktop cloud_convert unit test
nightly regression
commercial verify
```

### P11: 账号、订阅、用量生产化

周期：2-3 周

目标：替换 demo token 和内存用量，形成真实商业账户系统。

任务：

- 新增 PostgreSQL migration。
- 实现 Argon2id 密码哈希。
- 实现 JWT access token。
- 实现 refresh token hash 存储、轮换、撤销。
- 实现 auth middleware。
- 实现 usage event ledger。
- 实现 quota reservation：
  - 创建 conversion 时预占额度。
  - 成功时确认。
  - 失败时返还。
- 接入 billing provider trait。
- Stripe test mode adapter。
- Webhook 签名和幂等。

验收：

```text
服务重启后用户、订阅、用量和任务不丢失。
重复 webhook 不重复发放额度。
额度不足稳定返回 402。
失败任务可返还预占额度。
```

### P12: 云端 Worker 生产化与 Sandbox

周期：3-4 周

目标：把内存态 worker 替换为可生产运行的隔离转换平台。

任务：

- 对象存储接入：S3/MinIO。
- 任务表与 artifact 表落库。
- 队列接入：Redis/NATS/Postgres queue 三选一。
- worker 支持重试、取消、超时、过期清理。
- zip slip 防护。
- sandbox runner：
  - rootless container 或 namespace。
  - no network。
  - cgroup CPU/memory。
  - disk quota。
  - process limit。
  - wall-clock timeout。
- 固定 TeX runtime image 版本。
- 编译日志脱敏和 artifact 化。

验收：

```text
恶意 zip 不能写出 workspace。
超时任务会失败并清理临时目录。
worker 崩溃后 queued/running 任务可恢复或标记失败。
并发 N 个任务不会互相污染输出。
```

### P13: 桌面商业客户端产品化

周期：2-3 周

目标：把 Slint MVP 提升为可给 Beta 用户使用的跨平台客户端。

任务：

- 文件/目录 picker。
- 拖拽 zip/project。
- 登录、注册、退出、刷新 token。
- token 接入系统 keychain：
  - macOS Keychain。
  - Windows Credential Manager。
  - Linux Secret Service，失败时降级提示。
- 用量、套餐、额度不足提示。
- billing checkout/portal 打开外部浏览器。
- 云端任务进度条和阶段状态。
- 失败诊断可复制。
- 自动升级 UI。
- 隐私提示和数据保留说明。

验收：

```text
Windows/macOS/Linux 至少各完成一次：
安装 -> 登录 -> 本地转换 -> 云端转换 -> 下载 -> 查看报告 -> 退出登录。
```

### P14: 质量基准与发布流水线

周期：3-4 周

目标：建立商业可承诺的质量和发布门禁。

任务：

- 每 profile 10+ realistic fixture 进入 Beta 基准。
- 每 profile 30+ realistic fixture 进入 GA 基准。
- Word/LibreOffice 实际打开验证作为 Beta/GA 门禁。
- 公式、表格、图片、引用、样式 coverage 统计。
- 失败样本库和失败分类：
  - unsupported package
  - unknown macro
  - runtime missing
  - asset missing
  - docx invalid
  - quality failed
- 发布流水线：
  - cargo test。
  - nightly regression。
  - installer build。
  - artifact sha256。
  - signature。
  - release manifest。

验收：

```text
Beta 门槛：
7 profiles x 10 samples，DOCX 实际打开率 >= 98%，无崩溃转换率 >= 98%。

GA 门槛：
7 profiles x 30 samples，DOCX 实际打开率 >= 99%，无崩溃转换率 >= 99.5%。
```

### P15: GA 运维与合规

周期：4-6 周

目标：完成公开收费 GA 所需的运营能力。

任务：

- OpenTelemetry tracing。
- Prometheus metrics。
- structured logs。
- Sentry/类似错误聚合。
- conversion quality dashboard。
- status page。
- SLA 指标和告警。
- 隐私政策。
- 服务条款。
- 数据删除 API 和流程。
- 文件保留周期策略。
- 管理后台最小版：
  - 用户查询。
  - conversion 查询。
  - artifact 删除。
  - 手工额度调整。
  - 失败样本标注。

验收：

```text
线上出现转换失败时，能够在 5 分钟内定位用户、任务、worker、日志、report 和 artifact。
用户可请求删除上传文件和生成文件。
运营人员可手工处理退款/额度补偿。
```

---

## 六、优先级清单

### 立即做

| 优先级 | 工作项 | 原因 |
|---|---|---|
| P0 | 完成桌面端真实 GUI 云端转换验收 | 当前已有代码，但缺用户视角验证 |
| P0 | 引入 PostgreSQL schema 草案和 migration | 账号/任务/用量生产化的前提 |
| P0 | 设计 sandbox runner 最小原型 | TeX 云端执行是最大安全风险 |
| P0 | 建立 failure taxonomy | 商业质量报告需要可解释失败 |
| P0 | 扩展 nightly regression 指标 | 不能只看 DOCX 是否生成 |

### Beta 前必须做

| 工作项 | 验收标准 |
|---|---|
| 真实 auth | JWT + refresh token + password hash |
| 真实 usage ledger | 服务重启后用量不丢，失败可返还 |
| object storage | 上传和产物不再存内存 |
| queue | worker 可恢复、可重试、可取消 |
| sandbox | 恶意输入不能逃逸 workspace |
| desktop keychain | token 不以明文保存在配置文件 |
| GUI 验收矩阵 | 三平台至少完成安装和转换流程 |

### GA 前必须做

| 工作项 | 验收标准 |
|---|---|
| 支付生产模式 | webhook 幂等、订阅状态同步 |
| 三平台签名安装包 | Windows/macOS/Linux 可安装和升级 |
| 质量基准 | 每 profile 30+ 样本，DOCX 打开率 >= 99% |
| 监控告警 | 转换失败、队列堆积、worker 异常有告警 |
| 合规文档 | 隐私政策、服务条款、数据删除策略 |
| 客服支持流程 | 失败任务可定位、可补偿、可复现 |

---

## 七、风险评估

| 风险 | 等级 | 影响 | 缓解 |
|---|---|---|---|
| TeX 云端执行逃逸 | 高 | 安全事故 | sandbox、no network、cgroup、seccomp、最小权限 |
| 真实模板转换质量不足 | 高 | 退款和口碑风险 | 扩充真实样本、失败分类、质量 dashboard |
| 中文 CTeX 字体差异 | 中高 | 输出效果不稳定 | runtime image 固定字体集，允许用户上传字体或选择字体策略 |
| Word/LibreOffice 差异 | 中高 | DOCX 看似合法但用户打不开 | Word-open/LibreOffice-open 双门禁 |
| 计费用量不一致 | 中高 | 资损和纠纷 | usage ledger、idempotency key、失败返还 |
| 三平台安装复杂 | 中 | 发布延迟 | 提前选定 cargo-dist/cargo-bundle/自建 pipeline |
| LLM fallback 成本不可控 | 中 | 成本失控 | 默认关闭，按套餐/开关启用，缓存规则输出 |

---

## 八、商业发布建议

### 8.1 受控 PoC

可在完成 P10 后启动，范围建议：

- 5-10 个真实用户。
- 只支持 7 个已实现 profile。
- 文件大小和转换次数限额。
- 明确声明复杂 TikZ/minted/自定义 class 可能降级。
- 失败样本必须授权回收用于改进。

### 8.2 邀请制 Beta

可在完成 P11-P13 后启动，范围建议：

- 50-100 个用户。
- 启用真实账号和 usage ledger。
- 支持支付测试模式或人工开通 Pro。
- 提供桌面客户端安装包。
- 每周发布质量报告。

### 8.3 付费 Beta

可在完成 P12-P14 后启动，范围建议：

- 小规模收费。
- 明确 SLA 不承诺或低承诺。
- 提供失败返还额度策略。
- 提供人工支持通道。

### 8.4 GA

可在完成 P15 且质量指标达标后启动：

- 公开自助注册、支付和下载。
- 三平台签名安装包。
- 数据保留和删除策略上线。
- 监控、告警、客服、退款和事故响应机制上线。

---

## 九、结论

当前项目距离商业化最大的差距不在“能不能转换出 DOCX”，而在以下五件事：

1. 生产级云端执行环境。
2. 真实账号、订阅、用量和支付。
3. 桌面端完整商业用户体验。
4. 可承诺的质量基准和失败诊断体系。
5. 三平台签名发布、升级、监控和合规。

建议下一阶段不要继续扩大功能面，而是优先收口：

```text
P10 Preview 收口
  -> P11 账号/用量生产化
  -> P12 Worker sandbox
  -> P13 桌面商业客户端
  -> P14 质量与发布门禁
  -> P15 GA 运维合规
```

只有完成上述路线后，Tex2Doc / Semantic TeX Engine 才适合从“技术预览”进入可收费、可支持、可持续迭代的商业产品状态。
