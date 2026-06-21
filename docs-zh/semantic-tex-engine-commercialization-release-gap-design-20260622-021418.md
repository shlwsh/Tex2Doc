# Semantic TeX Engine 商业化发布差距评估与实施设计方案

**文档版本**：20260622-021418
**评估日期**：2026-06-22
**基准报告**：`docs-zh/semantic-tex-engine-progress-report-20260621-180000.md`
**评估对象**：Tex2Doc / Semantic TeX Engine 当前工程实现、桌面端、商业 API、云端 Worker、质量回归与发布体系
**目标**：结合当前真实开发进展，判断距离商业化发布还缺哪些工作，并提出可落地、可验收的设计实施方案。

---

## 一、总体判断

当前项目已经从“语义转换技术验证”推进到“可演示的商业化 Preview 原型”。核心转换引擎、Journal Profile、三路径转换、CLI、质量报告、Slint 桌面端、商业 API client、preview server、云端转换 worker、recent jobs、诊断包和更新检查均已有实现或雏形。

但它还没有达到公开收费商业化发布标准。当前最适合的发布级别是：

```text
内部 Preview：基本具备
受控 PoC：接近具备
邀请制 Beta：仍需补齐生产基础设施
付费 Beta：尚未具备
正式 GA：尚未具备
Enterprise：尚未具备
```

商业化阻断项不在“是否能生成 DOCX”，而在以下能力尚未生产化：

1. 生产级账号、订阅、支付与用量账本。
2. 云端转换的持久化、队列化、隔离执行和资源限制。
3. 大样本质量基准、真实 Word 打开验证和 profile 质量承诺。
4. 三平台客户端安装包、签名、公证、自动升级和回滚。
5. 安全、合规、监控、诊断、支持和故障闭环。

建议商业化路线不要直接进入公开 SaaS，而是采用：

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

当前已经形成商业化的核心技术资产：

| 能力 | 当前状态 | 商业意义 |
|---|---|---|
| V1 Rust rule-based DOCX 引擎 | 保持独立 | 可作为 fallback、对照组和低依赖离线路径 |
| V2 Semantic TeX Engine | 已独立实现 | 商业主路径 |
| RuleBased / XeLaTeX Hook / LuaTeX Node 三后端 | 已建立 | 覆盖无 runtime、中文 CTeX、长期 LuaTeX 语义采集 |
| JournalDetector | 已支持首期 profile | 降低用户配置成本 |
| 7 类 Journal Profile | 已实现 | 覆盖首期商业目标模板 |
| ProfileStyleMap | 已接 DOCX 渲染 | 支持不同期刊样式映射 |
| CompatibilityAnalyzer / RuleEngine | 已接入方向 | 支撑模板兼容性诊断和宏泛化 |
| QualityGate | 已接编译报告 | 支撑自动验收和商业质量报告 |
| semantic CLI | 已新增 detect/analyze/convert/verify 入口 | 支撑专业用户、CI 和云端 worker |
| paper3 三路径输出 | 已验证 | 支撑演示、对比和回归 |

仍需补齐：

- LuaTeX Node 采集还不是完整生产级语义采集器，需加强段落、字体、表格、公式、引用、浮动体和布局信息抽取。
- XeLaTeX Hook 与 LuaTeX Node 的事件协议需要版本化和稳定 schema。
- XDV/LayoutGraph 尚未成为高保真版式恢复的主力输入。
- `semantic-verify` 需要从占位/初级检查升级为真实 DOCX 质量验证器。
- 对真实客户模板的大样本转换成功率和质量指标尚未建立。

### 2.2 桌面端

Slint 桌面端已从骨架推进到 Preview 可演示闭环：

```text
选择项目/zip
  -> 本地转换
  -> 云端转换
  -> 登录/注册/刷新/退出
  -> 用量与套餐展示
  -> checkout / portal 入口
  -> recent jobs 持久化
  -> 诊断包导出
  -> 更新检查
```

当前已具备的商业化雏形：

- 本地转换调用 `SemanticTexEngine::compile_dir_to_docx()`。
- 云端转换调用 `doc-commercial-api-client`，支持 upload、create conversion、poll、download docx、save report。
- access token / refresh token 已从普通 settings 中剥离，具备安全存储 preview adapter。
- recent jobs 可跨重启保留。
- 诊断包可导出 `diagnostics.json`、`status.txt`、`recent_jobs.txt`。
- 更新检查可请求 release manifest 并显示 current/latest、sha256、签名状态和 release notes。

仍需补齐：

- Windows/macOS/Linux 真实 GUI 操作验收。
- 拖拽目录/zip、main tex 自动识别、冲突选择 UI。
- job 详情页、report 打开、失败原因分类、重新下载、恢复轮询、取消任务。
- token 在 macOS Keychain、Windows Credential Manager、Linux Secret Service 上的 Beta 级验证。
- 安装包、代码签名、公证、updater 安装执行和回滚。
- 崩溃报告、问题上报、完整诊断包用户授权流程。

### 2.3 商业 API 与 Server

当前 preview server 已覆盖基础商业合约：

- auth/register、auth/login、auth/refresh。
- me、usage、plans。
- billing checkout/portal。
- uploads。
- conversions create/get/download/report。
- releases manifest。
- `/v1` 与 `/api/v1` 双路径兼容。
- Bearer token 门禁。
- preview 用量扣减，额度不足返回 402。

仍未生产化：

- demo token 不是 JWT。
- 密码未做生产级 Argon2id hash。
- refresh token 未持久化、未轮换、未撤销。
- 用户、套餐、订阅、用量、任务、上传、产物都未落库。
- billing URL 和套餐仍为 preview/mock。
- 缺 webhook 签名验证、幂等处理、支付状态同步。
- 缺租户隔离、RBAC、审计日志、管理员后台。

### 2.4 云端 Worker

当前 worker 已形成 preview 链路：

```text
upload zip
  -> create conversion
  -> in-memory queue
  -> worker
  -> SemanticTexEngine::compile_zip_to_docx
  -> legacy fallback
  -> in-memory docx/report
  -> download/report endpoints
```

仍未生产化：

- upload、job、docx、report 全部为内存态。
- 没有对象存储。
- 没有持久化队列。
- 没有 retry、cancel、timeout、dead letter、任务恢复。
- 没有 sandbox。
- 没有 CPU、内存、磁盘、进程数和 wall-clock 限制。
- 没有固定 TeX runtime 镜像和版本追踪。
- 没有日志脱敏、artifact retention 和清理策略。

### 2.5 回归与质量体系

当前已有：

- 7 个 profile 的 minimal + realistic fixture。
- paper3 三路径回归。
- nightly regression 脚本。
- DOCX ZIP 结构检查。
- XML well-formed 检查。
- 可选 LibreOffice headless 打开验证。

仍需补齐：

- 真实客户模板样本库。
- failure fixture 和 golden fixture。
- Word 实际打开验证门禁。
- 公式、表格、图片、引用、样式、字体、中文断行覆盖率指标。
- profile 维度质量趋势 dashboard。
- 转换失败分类和用户可读修复建议。

### 2.6 自动升级与发布

当前已具备：

- release manifest 解析。
- version 比较。
- SHA256 校验。
- signature status 占位。
- server release manifest preview endpoint。
- desktop Check Update UI。

仍需补齐：

- 真实 artifact 下载。
- Ed25519/minisign/sigstore 或平台签名验签。
- Windows MSI/MSIX。
- macOS DMG/pkg、codesign、notarization。
- Linux AppImage/deb/rpm。
- updater 安装执行、失败回滚和版本兼容策略。

---

## 三、商业化就绪度评估

| 发布阶段 | 当前就绪度 | 判断 |
|---|---:|---|
| 内部 Preview | 94% | 可以继续内部演示、paper3 回归和开发联调 |
| 受控 PoC | 82% | 可给少量合作用户试用，但需要人工支持和样本回收 |
| 邀请制 Beta | 68% | 需要生产 auth 雏形、持久化任务、GUI 验收和 sandbox |
| 付费 Beta | 50% | 需要支付、用量账本、安装包、签名升级和支持闭环 |
| 正式 GA | 35% | 需要 SLA、监控告警、质量基准、三平台发布和合规 |
| Enterprise | 18% | 需要私有化、SSO、审计、租户隔离和模板定制平台 |

短期可销售的产品形态不是“全自助 SaaS”，而应是：

```text
面向合作机构/期刊/课题组的受控 PoC：
  - 桌面端 + 云端转换
  - 限量账号
  - 样本回收
  - 人工支持
  - 明确质量边界
```

---

## 四、商业化发布目标架构

### 4.1 总体架构

```text
Slint Desktop Client
  |-- Local Convert: SemanticTexEngine / Legacy Rule Engine
  |-- Cloud Convert: commercial-api-client
  |-- Account / Usage / Billing
  |-- Recent Jobs / Diagnostics / Update

Commercial API Server
  |-- Auth / Sessions / Devices
  |-- Plans / Subscriptions / Billing / Webhooks
  |-- Usage Ledger / Quota Reservation
  |-- Uploads / Conversion Jobs / Artifacts
  |-- Release Manifest

Storage Layer
  |-- PostgreSQL
  |-- Object Storage: uploaded zip / generated docx / report / logs
  |-- Queue: conversion jobs / retry / dead letter

Sandbox Worker
  |-- unzip guard
  |-- fixed TeX runtime image
  |-- SemanticTexEngine
  |-- legacy fallback
  |-- semantic-verify
  |-- report/log/diagnostic bundle

Observability & Support
  |-- metrics
  |-- structured logs
  |-- traces
  |-- quality dashboard
  |-- support ticket diagnostics
```

### 4.2 推荐模块拆分

当前 preview server 可继续保留，但生产化建议逐步拆出以下 crate：

| Crate | 职责 |
|---|---|
| `doc-commercial-domain` | 用户、套餐、订阅、用量、上传、任务、产物领域模型 |
| `doc-commercial-store` | PostgreSQL migrations、repository、transaction |
| `doc-commercial-auth` | Argon2id、JWT、refresh token、auth middleware |
| `doc-commercial-billing` | Billing provider trait、Stripe adapter、webhook、幂等 |
| `doc-commercial-worker` | Queue consumer、sandbox runner、artifact writer |
| `doc-commercial-observability` | metrics、logs、trace、quality event |
| `doc-commercial-admin` | 内部管理、样本回收、任务排障、用户支持 |
| `doc-desktop-slint` | 继续作为跨平台客户端入口 |

---

## 五、关键技术设计

### 5.1 账号与 Token 设计

目标：替换 demo token，形成可收费、可撤销、可审计的账号体系。

设计：

```text
access token:
  - JWT
  - 15-30 分钟有效
  - 包含 user_id / session_id / plan / scopes

refresh token:
  - 随机高熵 token
  - 只存 hash
  - 支持轮换
  - 支持设备级撤销

password:
  - Argon2id hash
  - 邮箱唯一
  - 支持邮箱验证和找回密码
```

核心表：

```text
users
refresh_tokens
devices
audit_logs
```

验收标准：

- 服务重启后登录状态、刷新状态不丢失。
- logout 后 refresh token 不可继续使用。
- 同一 refresh token 重放会触发撤销或拒绝。
- 未授权请求稳定返回 401，额度不足稳定返回 402。

### 5.2 订阅、支付与用量账本

目标：从 preview 用量扣减升级为可对账的商业账本。

设计：

```text
plans
subscriptions
usage_events
quota_reservations
billing_events
```

任务创建时：

```text
1. 检查 subscription / plan。
2. 创建 quota reservation。
3. 写 usage event: reserved。
4. 入队 conversion job。
```

任务结束时：

```text
success:
  - confirm reservation
  - usage event: consumed

failed/cancelled/timeout:
  - release reservation
  - usage event: refunded
```

支付 webhook：

- 验证签名。
- 使用 idempotency key 防重复处理。
- 写 billing event。
- 更新 subscription。
- 发放或调整权益。

验收标准：

- 重复 webhook 不重复发放额度。
- 失败任务返还额度。
- 并发创建任务不会超卖额度。
- 所有额度变化可通过 usage_events 重放审计。

### 5.3 云端转换生产架构

目标：把内存 worker 替换为可恢复、可扩展、可隔离的生产 worker。

Job 状态机：

```text
created
  -> queued
  -> running
  -> succeeded
  -> failed
  -> cancelled
  -> expired
```

异常分支：

```text
queued/running -> retrying -> queued
running -> timed_out -> failed
running -> worker_lost -> queued 或 failed
succeeded/failed -> artifact_expired
```

数据流：

```text
Desktop
  -> POST /uploads
  -> Object Storage upload
  -> POST /conversions
  -> PostgreSQL conversion row
  -> Queue
  -> Sandbox Worker
  -> Object Storage artifacts
  -> GET /conversions/:id/report
  -> GET /conversions/:id/download/docx
```

验收标准：

- 服务重启后任务、上传和产物不丢失。
- worker 崩溃后任务可恢复或明确失败。
- 并发任务输出不会互相污染。
- 所有 artifact 有 sha256、bytes、created_at、expires_at。

### 5.4 Sandbox 安全设计

TeX 输入不可信，云端商业化必须隔离执行。

最低安全策略：

```text
zip guard:
  - 禁止 zip slip
  - 限制文件数量
  - 限制总大小
  - 限制单文件大小
  - 拒绝绝对路径和 .. 路径

runtime:
  - 禁用 shell escape
  - 默认无网络
  - 独立 workspace
  - 降权用户执行
  - CPU/memory/disk/process/wall-clock 限制
  - 固定 TeXLive/XeLaTeX/LuaLaTeX 版本
  - 输出日志脱敏
```

建议实现：

1. Beta 阶段：rootless container + cgroup + no network。
2. GA 阶段：worker image 固定版本，按 release 记录 runtime hash。
3. Enterprise 阶段：可私有化部署，支持客户自有字体包和模板包。

验收标准：

- 恶意 zip 不能写出 workspace。
- shell escape 测试用例不能执行系统命令。
- 超时任务会失败并清理临时目录。
- 大文件/大量文件上传被稳定拒绝。

### 5.5 质量验证与商业指标

目标：把“可转换”升级为“有质量边界、可承诺、可回归”。

`semantic-verify` 应实现：

- DOCX ZIP 结构检查。
- `word/document.xml`、`styles.xml`、`rels` XML well-formed。
- Word/LibreOffice openability。
- styleId 覆盖检查。
- media/rels 一致性检查。
- bookmark/hyperlink/REF 字段检查。
- OMML 数量、fallback 数量和 fallback 原因。
- unresolved reference 定位。
- profile detection confidence。
- report JSON schema 验证。

商业指标：

| 指标 | PoC 门槛 | Beta 门槛 | GA 门槛 |
|---|---:|---:|---:|
| DOCX 实际打开率 | >= 95% | >= 98% | >= 99% |
| 无崩溃转换率 | >= 95% | >= 98% | >= 99.5% |
| Profile 自动识别准确率 | >= 90% | >= 95% | >= 97% |
| main tex 自动识别准确率 | >= 90% | >= 95% | >= 98% |
| 缺失资源可诊断率 | >= 85% | >= 95% | >= 99% |
| 未解析引用定位率 | >= 80% | >= 90% | >= 98% |
| P95 云端转换耗时 | < 180s | < 120s | < 60s |

样本库目标：

| Profile | PoC 样本数 | Beta 样本数 | GA 样本数 |
|---|---:|---:|---:|
| jos-paper | 10 | 20 | 50 |
| chinese-academic | 15 | 30 | 80 |
| tacl | 8 | 15 | 40 |
| cvpr | 8 | 15 | 40 |
| nature | 5 | 10 | 25 |
| springer | 10 | 20 | 50 |
| generic/arXiv | 25 | 50 | 150 |

### 5.6 桌面端商业产品设计

目标：让非开发者能完成转换并理解结果。

必须补齐的 UI/UX：

- project/zip 拖拽。
- main tex 自动识别和多候选选择。
- local/cloud convert 分流说明。
- profile/backend/quality 自动选择与高级选项。
- 进度状态：uploading、queued、running、rendering、verifying、downloading。
- 失败诊断：缺字体、缺包、缺图片、编译失败、额度不足、网络失败、服务端超时。
- recent jobs 详情页：打开 DOCX、打开 report、打开日志、导出诊断包、重新转换。
- 账号页：当前套餐、剩余额度、到期时间、checkout、portal。
- 更新页：检查、下载、校验、安装、重启。

三平台发布：

| 平台 | 格式 | 必需能力 |
|---|---|---|
| Windows | MSI/MSIX | code signing、installer、auto update、credential manager |
| macOS | DMG/pkg | codesign、notarization、keychain、update |
| Linux | AppImage/deb/rpm | desktop entry、Secret Service、update 或手动下载 |

Slint 可行性判断：

- Slint 与 Rust workspace 集成良好，适合作为轻量跨平台桌面客户端。
- 当前 UI 已能支撑商业 MVP 的主要入口。
- 商业发布前需要完成许可、打包、平台控件体验、无障碍、字体渲染、输入法和高 DPI 验收。
- 若后续需要复杂 WebView、内嵌支付、富文本报告和大型表格管理，可保留 Slint 主壳，局部引入系统浏览器或 WebView。

### 5.7 支持、诊断与运维

目标：商业用户失败时可以被支持团队快速定位。

诊断包分层：

```text
basic diagnostics:
  - app version
  - platform
  - profile/backend/quality
  - conversion id
  - report summary
  - recent jobs
  - error code/message

full diagnostics:
  - basic diagnostics
  - compile report JSON
  - sanitized logs
  - DOCX verifier result
  - optional source snapshot after explicit user consent
```

错误码体系：

```text
AUTH_*
BILLING_*
UPLOAD_*
ZIP_*
TEX_RUNTIME_*
CONVERT_*
VERIFY_*
QUOTA_*
NETWORK_*
UPDATE_*
```

运维指标：

- conversion_created_total。
- conversion_success_total。
- conversion_failed_total by error_code/profile/backend。
- conversion_duration_seconds。
- docx_openability_rate。
- worker_queue_depth。
- worker_timeout_total。
- quota_reservation_leaks。
- billing_webhook_failures。
- desktop_update_check_failures。

---

## 六、实施路线图

### P10：PoC 收口

周期：1-2 周
目标：把当前 Preview 变成可交给合作用户的受控 PoC。

实施内容：

- 完成 Linux/Windows/macOS 至少一轮真实 GUI 操作验收。
- 完成 drag/drop project/zip。
- 完成 main tex 自动识别与多候选选择。
- recent jobs 补齐 job id、report path、打开输出/报告入口。
- 诊断包加入 compile report、quality gate、错误码。
- `semantic-verify` 最小实现：DOCX ZIP、XML、styles、rels、media、LibreOffice openability。
- nightly regression 输出 profile 维度 `conversion_stats.md`。
- paper3 三路径脚本纳入 PoC 验收。

验收标准：

```text
桌面端可完成：
登录 -> 用量 -> 选择 paper3 -> 云端转换 -> 下载 DOCX/report -> recent jobs 可见 -> 导出诊断包。

脚本可完成：
paper3 三路径输出
server API integration test
desktop cloud_convert tests
nightly regression
semantic-verify
commercial_verify
```

### P11：账号、订阅、用量生产化

周期：2-3 周
目标：替换 demo token 和内存用量，形成真实商业账户系统。

实施内容：

- 新增 PostgreSQL migration。
- 实现 `doc-commercial-auth`。
- 实现 `doc-commercial-store`。
- 实现 JWT access token、refresh token hash、轮换和撤销。
- 实现 usage ledger 与 quota reservation。
- 接入 billing provider trait。
- 接入 Stripe test mode 或等价支付 provider。
- 实现 webhook 签名验证和幂等。

验收标准：

```text
服务重启后用户、session、套餐、用量不丢失。
重复 webhook 不重复发放额度。
额度不足稳定返回 402。
失败任务返还额度。
logout 后 refresh token 不能继续刷新。
```

### P12：云端 Worker 生产化与 Sandbox

周期：3-4 周
目标：把内存态 worker 替换为可生产运行的隔离转换平台。

实施内容：

- uploads、conversions、artifacts 落库。
- 对象存储接入 S3/MinIO。
- 队列接入：建议先 Postgres queue，后续可替换 Redis/NATS。
- worker 支持 retry、cancel、timeout、dead letter、过期清理。
- sandbox runner 支持 no network、cgroup、disk quota、process limit。
- 固定 TeX runtime image。
- 编译日志脱敏和 artifact 化。

验收标准：

```text
恶意 zip 不能写出 workspace。
超时任务会失败并清理临时目录。
worker 崩溃后 queued/running 任务可恢复或明确失败。
并发任务不会互相污染输出。
```

### P13：转换质量与 Profile 商业化

周期：3-5 周
目标：建立可以对外承诺的质量边界。

实施内容：

- 扩展真实样本库。
- 引入 failure/golden fixture。
- 完整实现 `semantic-verify`。
- 为每个 profile 建立 quality dashboard。
- 增强 LuaTeX Node 采集。
- 增强 XeLaTeX Hook 事件协议。
- 完成复杂表格、公式、引用、图片尺寸、bibliography 的失败分类。

验收标准：

```text
每个首期 profile 有最少 PoC 样本。
DOCX openability 达到 PoC 门槛。
失败样本有稳定错误码和修复建议。
质量指标可按 profile/backend/date 追踪。
```

### P14：三平台发布与自动升级

周期：2-4 周
目标：形成可安装、可升级、可回滚的桌面产品。

实施内容：

- Windows MSI/MSIX。
- macOS DMG/pkg + codesign + notarization。
- Linux AppImage/deb/rpm。
- release artifact SHA256。
- manifest 签名。
- updater 下载、校验、安装执行。
- 失败回滚和手动下载 fallback。

验收标准：

```text
三个平台均可安装、启动、登录、转换、导出诊断包。
更新检查能下载并校验 artifact。
签名校验失败时拒绝安装。
升级失败有明确错误和回滚/手动路径。
```

### P15：监控、支持与合规

周期：2-3 周
目标：让商业服务可运营、可追责、可支持。

实施内容：

- structured logs。
- metrics endpoint。
- tracing。
- dashboard。
- alert rules。
- admin/support console。
- 数据保留策略。
- 隐私与用户授权诊断流程。
- 依赖许可证审计。
- 安全审计 checklist。

验收标准：

```text
可按 job_id 追踪一次转换的全链路。
支持人员可下载诊断包和查看失败阶段。
用户可删除上传与产物。
依赖许可证风险有清单和处理结论。
```

### P16：Enterprise 能力

周期：后续版本
目标：支撑机构客户和私有化部署。

实施内容：

- 私有化部署包。
- SSO/SAML/OIDC。
- 租户隔离。
- 审计报表。
- 自定义模板/Profile 管理。
- 自有字体/宏包白名单。
- 离线 license。

---

## 七、近期优先级清单

### 立即做

1. 完成 P10 PoC 收口，避免 Preview 功能继续横向扩张。
2. 将 `semantic-verify` 从占位升级为最小可用质量验证器。
3. 完成桌面端 GUI 手工验收矩阵。
4. 为 recent jobs 增加 report path、打开 report、导出诊断包联动。
5. 对 server/worker 加入 zip guard、大小限制和基础 timeout。

### 随后做

1. PostgreSQL store。
2. JWT + refresh token 生产化。
3. usage ledger 和 quota reservation。
4. object storage + persistent artifacts。
5. sandbox worker。

### 暂缓做

1. Enterprise SSO。
2. 模板市场。
3. AI 自动修复宏。
4. 完整在线协作。
5. 高级 BI 报表。

---

## 八、主要风险与对策

| 风险 | 影响 | 对策 |
|---|---|---|
| 转换质量在真实模板上波动 | 影响付费转化和信任 | 建立 profile 样本库、质量 dashboard、失败分类 |
| TeX 输入安全风险 | 影响云端发布 | sandbox、禁 shell escape、无网络、资源限制 |
| 支付/用量账本不准确 | 影响收入和客服 | quota reservation、usage event ledger、幂等 webhook |
| 三平台安装升级复杂 | 影响桌面产品发布 | 分平台验收，先手动下载，后自动升级 |
| Slint 生态边界 | 影响复杂 UI | 保持轻客户端，复杂支付/报告可用系统浏览器或 WebView |
| 缺少真实客户样本 | 质量承诺不可验证 | PoC 阶段以样本回收作为商业合作条件 |
| 旧引擎和新引擎耦合 | 难以对比效果 | 保持 V1/V2 独立路径，只在调度层选择 |

---

## 九、商业发布准入标准

### 受控 PoC 准入

- paper3 三路径稳定输出。
- 至少 7 profile fixture 全部通过。
- 桌面端完成登录、云端转换、下载、report、诊断包。
- server 支持基础用量限制和转换任务。
- 每次失败有错误码和 report。

### 付费 Beta 准入

- 生产 auth。
- 支付 provider test/live 切换。
- usage ledger。
- PostgreSQL + object storage + persistent queue。
- sandbox worker。
- 三平台安装包。
- 真实样本库达到 Beta 门槛。

### GA 准入

- 三平台签名发布和自动升级。
- 监控告警和支持后台。
- SLA 与数据保留策略。
- 质量指标达到 GA 门槛。
- 安全与许可证审计完成。
- 用户删除数据、导出诊断授权和隐私说明完成。

---

## 十、结论

当前项目已经具备商业化的技术核心和 Preview 产品雏形，但距离正式商业发布还缺生产级工程体系。

接下来不宜继续单纯增加转换特性，而应优先完成：

```text
PoC 收口
生产 auth / billing / usage
持久化 cloud worker
sandbox
质量基准
三平台发布
诊断与运维
```

只要 P10-P12 完成，项目即可进入受控 PoC 到邀请制 Beta；P13-P15 完成后，才适合推进付费 Beta 和正式 GA。
