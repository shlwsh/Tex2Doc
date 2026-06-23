# Semantic TeX Engine 商业化差距评估与实施路线图

**文档版本**：20260622-014918  
**评估日期**：2026-06-22  
**基准报告**：`docs-zh/semantic-tex-engine-progress-report-20260621-180000.md`  
**评估目标**：结合当前开发进展，判断项目距离商业化发布还缺哪些关键能力，并提出可执行的技术实施方案。

---

## 一、总体结论

当前 Tex2Doc / Semantic TeX Engine 已经从“技术验证”推进到“商业化 Preview 原型”阶段，但还不能直接公开收费发布。

当前最有商业价值的资产已经形成：

- 新旧两条 DOCX 转换路径保持独立：旧 Rust rule-based 引擎可作为 fallback 与效果对照，新 Semantic TeX Engine 作为主商业路径。
- 新语义引擎已支持 RuleBased、XeLaTeX Hook、LuaTeX Node 三种后端策略。
- 已具备 7 类首期 Journal Profile：JOS、TACL/ACL、CVPR/ICCV、Nature、Springer、中文学报、generic/arXiv。
- `paper3` 三路径验证、profile fixture、质量门禁、CLI 子命令、商业 API client、Slint 桌面端、云端 worker preview、自动升级骨架均已启动。
- 桌面端已从简单 UI 骨架推进到本地转换、云端转换、路径选择、注册/登录/刷新/退出、用量、套餐、checkout/portal、安全 token 存储 preview。

但商业化发布的阻断项仍集中在五个方面：

```text
生产级账号计费
生产级云端转换与安全隔离
大样本转换质量基准
三平台客户端安装/签名/升级
运维、合规、支持与故障闭环
```

建议当前定位：

| 发布级别 | 当前就绪度 | 判断 |
|---|---:|---|
| 内部 Preview | 92% | 可继续内部演示、paper3 验证和开发联调 |
| 受控 PoC | 78% | 可给少量合作用户试用，但需要人工支持和样本回收 |
| 邀请制 Beta | 65% | 需要补齐 GUI 验收、诊断包、生产 auth 雏形和 sandbox |
| 付费 Beta | 48% | 需要支付、用量账本、持久化任务和安装包 |
| 正式 GA | 32% | 需要 SLA、监控、签名发布、质量基准和合规流程 |
| Enterprise | 15% | 需要私有化、SSO、审计、租户隔离和模板定制平台 |

商业化策略不应从“公开自助 SaaS”直接开始，而应按下面路径推进：

```text
内部 Preview
  -> 受控 PoC
  -> 邀请制 Beta
  -> 付费 Beta
  -> Pro Desktop + Cloud GA
  -> Team / Enterprise
```

---

## 二、当前进展基线

### 2.1 转换核心

已经具备商业化核心雏形：

| 能力 | 当前状态 | 商业意义 |
|---|---|---|
| V1 Rust rule-based 引擎 | 保持独立 | 可作为稳定 fallback、对比基线和低依赖离线路径 |
| V2 Semantic TeX Engine | 已建立独立 facade | 商业化主路径 |
| XeLaTeX Hook 后端 | 已能接入 sidecar 语义事件 | 适配中文 CTeX、xeCJK、fontspec 生态 |
| LuaTeX Node 后端 | 已有 node/macro sidecar 原型 | 长期提升语义采集泛化能力 |
| RuleBased 后端 | 已可无 TeX runtime 运行 | 保证环境不足时仍可输出 DOCX |
| Auto backend selector | 已按模板信号选择 | 降低用户配置成本 |
| JournalDetector | 已支持首期 profile | 支撑期刊模板商业场景 |
| ProfileStyleMap | 已接 DOCX 渲染 | 支撑不同期刊样式差异化 |
| QualityGate | 已接编译报告 | 支撑自动验收和付费质量报告 |
| semantic CLI | 已新增 detect/analyze/convert/verify 入口 | 支撑专业用户、CI 和云端 worker |

核心缺口：

- LuaTeX Node 采集仍是原型级，尚未形成完整、稳定、可量化的段落/表格/公式/引用语义采集能力。
- XDV/LayoutGraph 尚未充分参与高保真版式恢复。
- `semantic-verify` 仍需要从占位升级为真实 DOCX 质量验证器。
- 公式、表格、交叉引用、图片尺寸、bibliography 在复杂模板上的覆盖率尚未以大样本统计证明。

### 2.2 桌面客户端

Slint 桌面端已具备可演示闭环：

```text
选择目录/zip
  -> 本地转换或云端转换
  -> 登录/注册/刷新/退出
  -> 查看用量
  -> 查询套餐
  -> 打开 checkout/portal
  -> 保存 DOCX/report
  -> recent jobs
  -> refresh token 安全存储 preview
```

仍缺商业客户端能力：

- 未完成 Windows/macOS/Linux 三平台真实安装包验收。
- 未完成 GUI 自动化或手工矩阵验收。
- 拖拽 project/zip 尚未完成。
- updater 仍未完整接入 UI 和安装执行。
- token 安全存储仍是 preview adapter，Linux/Windows/macOS 都需要 Beta 级验证。
- 没有用户可理解的失败诊断页、诊断包导出和问题上报链路。

### 2.3 商业 API 与云端 Worker

当前 preview API 已覆盖：

- auth/register、auth/login、auth/refresh。
- me、usage、plans。
- billing checkout/portal。
- uploads。
- conversions create/get/download/report。
- releases manifest。
- `/v1` 与 `/api/v1` 双路径兼容。
- Bearer token 门禁、preview 用量扣减、额度不足 402。
- 上传 zip 后进入 in-memory worker，调用 `SemanticTexEngine` 输出 DOCX/report，失败时可 fallback legacy rule。

核心缺口：

- token 是 demo token，不是 JWT。
- 密码、refresh token、订阅、用量、任务、产物均未生产级持久化。
- worker、upload、docx、report 仍是内存态。
- billing URL 和套餐仍是 mock/preview。
- 没有支付 webhook、用量账本、幂等处理和失败返还。
- 没有 sandbox、安全资源限制、任务恢复和对象存储。

### 2.4 回归与质量体系

当前已有：

- 7 profile 的 minimal + realistic fixture。
- nightly regression 脚本。
- DOCX ZIP 结构检查。
- XML well-formed 检查。
- 可选 LibreOffice headless 打开验证。

商业化缺口：

- realistic fixture 数量不足，缺真实客户模板和失败样本库。
- Word 实际打开验证未成为强制门禁。
- 缺公式、表格、图片、引用、样式、字体、中文断行的覆盖率指标。
- 缺 profile 维度的质量趋势 dashboard。
- 缺转换失败分类、错误码稳定规范和用户可读修复建议。

---

## 三、商业化发布必须补齐的工作

### 3.1 产品闭环

商业用户完整链路应为：

```text
下载并安装客户端
  -> 注册/登录
  -> 查看套餐与额度
  -> 选择 TeX 工程目录或 zip
  -> 自动识别 main tex / profile / backend
  -> 本地或云端转换
  -> 查看进度和诊断
  -> 下载 DOCX/report/logs
  -> 用量扣减或失败返还
  -> 订阅升级/管理
  -> 客户端自动升级
```

当前已经覆盖其中约 50% 的交互骨架，但仍需补齐：

- main tex 自动识别与冲突选择。
- 失败诊断页：缺包、缺字体、缺图片、未解析引用、TeX runtime 不存在、额度不足。
- 诊断包导出：输入摘要、profile、backend、report、日志、失败阶段、版本信息。
- 任务历史持久化与重新下载。
- 用户可见的 privacy/data retention 提示。

### 3.2 生产级账号与计费

必须从 demo auth 升级为生产 auth：

- Argon2id 密码哈希。
- JWT access token。
- refresh token hash 存储、轮换、撤销。
- 设备维度 session 管理。
- 邮箱验证与找回密码。
- RBAC 或至少 user/admin 分权。
- 订阅、套餐、权益、额度。
- usage event ledger。
- quota reservation：创建任务预占、成功确认、失败返还。
- Stripe 或等价支付 provider。
- webhook 签名验证、幂等处理、状态同步。

### 3.3 生产级云端转换

必须把当前 in-memory worker 迁移到生产架构：

```text
API Server
  -> PostgreSQL: users / sessions / plans / subscriptions / usage_events / uploads / conversions / artifacts
  -> Object Storage: upload zip / generated docx / report / logs / diagnostic bundle
  -> Queue: queued / running / retry / dead letter
  -> Sandbox Worker: isolated TeX runtime + SemanticTexEngine
```

关键能力：

- zip slip 防护。
- 上传大小、文件数量、单文件大小限制。
- 禁用 shell escape。
- worker 默认无网络。
- 每 job 独立 workspace。
- CPU、内存、磁盘、进程数、wall-clock timeout 限制。
- TeXLive/XeLaTeX/LuaLaTeX runtime 版本固定。
- 编译日志脱敏。
- 任务完成后自动清理。
- worker 崩溃后任务可恢复或可明确标记失败。

### 3.4 高保真转换质量基准

商业用户购买的不是“能生成 DOCX”，而是：

```text
可打开
可编辑
结构尽可能正确
失败时可解释
在目标期刊模板上有稳定预期
```

建议 Beta/GA 指标：

| 指标 | Beta 门槛 | GA 门槛 |
|---|---:|---:|
| DOCX 实际打开率 | >= 98% | >= 99% |
| 无崩溃转换率 | >= 98% | >= 99.5% |
| Profile 自动识别准确率 | >= 95% | >= 97% |
| main tex 自动识别准确率 | >= 95% | >= 98% |
| 缺失资源可诊断率 | >= 95% | >= 99% |
| 未解析引用定位率 | >= 90% | >= 98% |
| 公式 fallback 可见率 | 100% 记录 | 100% 分类并定位 |
| 表格结构成功率 | >= 85% | >= 93% |
| P95 云端转换耗时 | < 120s | < 60s |

首期样本库建议：

| Profile | Beta 样本数 | GA 样本数 |
|---|---:|---:|
| jos-paper | 20 | 50 |
| chinese-academic | 30 | 80 |
| tacl | 15 | 40 |
| cvpr | 15 | 40 |
| nature | 10 | 25 |
| springer | 20 | 50 |
| generic/arXiv | 50 | 150 |

### 3.5 三平台客户端发布

商业桌面端必须补齐：

- Windows MSI/MSIX。
- macOS DMG/pkg + notarization。
- Linux AppImage/deb/rpm。
- 代码签名。
- release artifact SHA256。
- manifest 签名。
- updater UI 与安装执行。
- 回滚策略。
- 版本兼容策略。
- crash/error report。

---

## 四、目标技术架构

### 4.1 商业化总体架构

```text
Desktop Slint App
  |-- Local Convert: legacy rule path / SemanticTexEngine
  |-- Cloud Convert: commercial-api-client
  |-- Account / Usage / Billing
  |-- Update / Diagnostics

Commercial API Server
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
  |-- TeX runtime image
  |-- SemanticTexEngine
  |-- legacy fallback
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

### 4.2 推荐 crate 拆分

保留现有 preview server，但逐步拆出生产模块：

| Crate | 职责 |
|---|---|
| `doc-commercial-domain` | 用户、套餐、订阅、用量、任务、产物领域模型 |
| `doc-commercial-store` | PostgreSQL repository、migration、transaction |
| `doc-commercial-auth` | 密码哈希、JWT、refresh token、auth middleware |
| `doc-commercial-billing` | provider trait、Stripe adapter、webhook |
| `doc-commercial-worker` | queue consumer、sandbox runner、artifact writer |
| `doc-commercial-observability` | metrics、logs、trace、quality event |
| `doc-desktop-slint` | 桌面 UI，继续依赖 `doc-commercial-api-client` |

### 4.3 数据模型草案

```text
users
  id, email, password_hash, display_name, email_verified_at, created_at, updated_at

refresh_tokens
  id, user_id, token_hash, device_name, expires_at, revoked_at, created_at

plans
  id, name, monthly_conversions, max_upload_bytes, max_retention_days, price_cents, currency

subscriptions
  id, user_id, plan_id, provider, provider_customer_id, provider_subscription_id,
  status, current_period_start, current_period_end

usage_events
  id, user_id, conversion_id, event_type, units, idempotency_key, created_at

uploads
  id, user_id, object_key, file_name, bytes, sha256, status, created_at, expires_at

conversions
  id, user_id, upload_id, main_tex, profile, engine, backend, quality,
  status, error_code, error_message, created_at, started_at, finished_at

artifacts
  id, conversion_id, kind, object_key, bytes, sha256, created_at

audit_logs
  id, user_id, action, resource_type, resource_id, metadata_json, created_at
```

### 4.4 API 合约升级

保留现有 preview endpoint，GA 前补齐：

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

### P10：Preview 收口与 PoC 准入

周期：1-2 周  
目标：把当前 preview 变成可交给合作用户的受控 PoC。

实施内容：

- 完成 Linux GUI 真实操作验收。
- 完成 paper3 本地转换、云端转换、三路径输出对比。
- 完成拖拽目录/zip。
- 完成 updater UI 接入，但安装执行可先保持占位。
- recent jobs 持久化 job id、状态、输出路径、错误。
- 增加诊断包导出：profile、backend、quality、report、错误、日志摘要、版本。
- nightly regression 输出 `conversion_stats.md`。
- 为 `semantic-verify` 做最小实现：DOCX ZIP、XML、styles、rels、document body、openability 结果。

验收标准：

```text
桌面端可完成：
登录 -> 用量 -> 选择 paper3 -> 云端转换 -> 下载 DOCX/report -> recent jobs 可见。

脚本可完成：
paper3 三路径输出
server API integration test
desktop cloud_convert unit test
nightly regression
commercial verify
```

### P11：账号、订阅、用量生产化

周期：2-3 周  
目标：替换 demo token 和内存用量，形成真实商业账户系统。

实施内容：

- 新增 PostgreSQL migration。
- 实现 `doc-commercial-auth`：
  - Argon2id 密码哈希。
  - JWT access token。
  - refresh token hash 存储、轮换、撤销。
  - auth middleware。
- 实现 `doc-commercial-store`：
  - users、refresh_tokens、plans、subscriptions、usage_events。
  - transaction 包装。
- 实现 usage ledger：
  - conversion 创建时预占额度。
  - 成功时确认。
  - 失败/取消/超时时返还。
- 接入 billing provider trait。
- Stripe test mode adapter。
- webhook 签名验证和幂等。

验收标准：

```text
服务重启后用户、session、套餐、用量不丢失。
重复 webhook 不重复发放额度。
额度不足稳定返回 402。
失败任务可返还预占额度。
refresh token 可撤销，退出登录后不可继续刷新。
```

### P12：云端 Worker 生产化与 Sandbox

周期：3-4 周  
目标：把内存态 worker 替换为可生产运行的隔离转换平台。

实施内容：

- 对象存储接入：S3/MinIO。
- uploads、conversions、artifacts 落库。
- 队列接入：Redis/NATS/Postgres queue 三选一，建议先 Postgres queue 降低部署复杂度。
- worker 支持 retry、cancel、timeout、dead letter、过期清理。
- zip slip 防护和上传限制。
- sandbox runner：
  - rootless container 或 Linux namespace。
  - no network。
  - cgroup CPU/memory。
  - disk quota。
  - process limit。
  - wall-clock timeout。
- 固定 TeX runtime image。
- 编译日志脱敏和 artifact 化。

验收标准：

```text
恶意 zip 不能写出 workspace。
超时任务会失败并清理临时目录。
worker 崩溃后 queued/running 任务可恢复或标记失败。
并发 N 个任务不会互相污染输出。
```

### P13：转换质量与 Profile 商业化

周期：3-5 周  
目标：从“能转换”升级为“有可承诺质量边界”。

实施内容：

- 扩展真实样本库：
  - 每个 profile 引入 minimal、realistic、failure、golden 四类样本。
  - 建立客户样本脱敏流程。
- 实现 `semantic-verify`：
  - DOCX 结构。
  - Word/LibreOffice openability。
  - styles.xml 覆盖。
  - rels/media 检查。
  - bookmark/hyperlink/REF 字段检查。
  - OMML fallback 分类。
- 引入 profile quality dashboard：
  - open rate。
  - crash-free rate。
  - profile detect accuracy。
  - unresolved reference count。
  - formula fallback ratio。
  - table span success/fallback ratio。
- 增强 LuaTeX Node 采集：
  - paragraph、glyph/font、list、section、caption、table、math、label/ref/cite。
- 增强 XeLaTeX Hook：
  - CTeX、IEEEtran、acl、springer、nature 常见宏事件覆盖。
- 形成错误码规范：
  - `missing_main_tex`
  - `unsupported_package`
  - `missing_asset`
  - `tex_runtime_failed`
  - `quota_exceeded`
  - `quality_gate_failed`
  - `sandbox_timeout`

验收标准：

```text
每个首期 profile 至少达到 Beta 样本门槛。
所有失败都有稳定 error_code 和用户可读 message。
DOCX openability 作为 CI 阻断门禁。
quality report 可直接展示给桌面端和云端用户。
```

### P14：桌面客户端 Beta 产品化

周期：2-3 周  
目标：让 Slint 客户端达到邀请制 Beta 可用水平。

实施内容：

- 三平台安装包：
  - Windows MSI/MSIX。
  - macOS DMG/pkg。
  - Linux AppImage/deb。
- 系统凭据存储 Beta 化：
  - macOS Keychain。
  - Windows Credential Manager 或稳定 DPAPI adapter。
  - Linux Secret Service。
- 登录 session 自动恢复。
- 任务进度条与阶段状态。
- 失败诊断可复制、可导出。
- billing checkout/portal 真实接入。
- updater UI：
  - 检查新版本。
  - 展示 release notes。
  - 下载 artifact。
  - SHA256 + manifest signature 校验。
  - 调用平台安装器。
- 隐私和数据保留说明入口。

验收标准：

```text
Windows/macOS/Linux 各完成一次：
安装 -> 登录 -> 本地转换 -> 云端转换 -> 下载 -> 查看报告 -> 退出登录 -> 重启恢复 session。
```

### P15：运维、监控和支持闭环

周期：2-3 周  
目标：让商业服务可观察、可定位、可支持。

实施内容：

- tracing span：request_id、user_id、job_id、profile、backend、runtime、duration。
- metrics：
  - API latency。
  - queue depth。
  - worker success/fail。
  - conversion duration。
  - quota errors。
  - quality gate failures。
- 日志脱敏。
- 错误聚合。
- 管理后台最小版：
  - 用户查询。
  - conversion 查询。
  - artifact/log/report 下载。
  - quota 手工调整。
- 支持流程：
  - 诊断包上传。
  - 失败样本归档。
  - profile/rule 回归用例生成。

验收标准：

```text
任一用户失败任务可在 10 分钟内定位到：
输入摘要、profile、backend、失败阶段、错误码、日志、artifact 状态。
```

### P16：付费 Beta 与 GA 准入

周期：4-6 周  
目标：从邀请制 Beta 推进到可收费 Beta，再推进 GA。

实施内容：

- 价格与套餐：
  - Free trial。
  - Pro monthly。
  - Team。
  - Enterprise contact sales。
- Stripe live mode。
- 发票、退款、取消、套餐升级/降级。
- 隐私政策、服务条款、数据删除策略。
- 数据保留周期。
- 安全审计清单。
- 客户反馈到工程 issue 的闭环。

GA 准入标准：

```text
DOCX 实际打开率 >= 99%
无崩溃转换率 >= 99.5%
Profile 自动识别准确率 >= 97%
P95 云端转换耗时 < 60s
三平台安装包签名发布
支付、用量、退款、取消流程全链路通过
生产监控和告警覆盖 API/worker/billing
```

---

## 六、近期 30 天建议排期

### 第 1 周：Preview 收口

- 完成 updater UI 接入。
- 完成拖拽 project/zip。
- 完成 `semantic-verify` 最小实现。
- 完成 paper3 三路径 + cloud convert GUI 验收。
- 输出受控 PoC 操作手册。

### 第 2 周：PoC 稳定化

- 诊断包导出。
- recent jobs 持久化。
- nightly regression 统计表。
- 失败错误码第一版。
- 桌面端 Linux 安装包预研。

### 第 3-4 周：生产 auth / store 起步

- PostgreSQL migration。
- Argon2id + JWT + refresh token hash。
- usage ledger。
- Stripe test mode checkout/webhook。
- conversions/uploads/artifacts 表结构。

### 第 5-6 周：worker 与质量

- 对象存储和 queue。
- sandbox runner 第一版。
- Word/LibreOffice openability 强制门禁。
- 每个 profile 增补真实样本。

---

## 七、风险与取舍

### 7.1 最大技术风险

| 风险 | 影响 | 建议 |
|---|---|---|
| TeX 输入可执行性与安全风险 | 云端被攻击 | P12 前不得公开自助云端上传 |
| 复杂模板转换质量不稳定 | 商业口碑受损 | Beta 前建立 profile 样本库和质量 dashboard |
| 桌面端三平台差异 | 客户端安装失败 | 尽早做 Windows/macOS/Linux packaging 验收 |
| 支付/用量不一致 | 直接影响收入和信任 | usage ledger 必须 transaction 化和幂等 |
| runtime 环境差异 | 用户本地/云端结果不一致 | 云端固定 TeX runtime image，本地转换清晰提示 runtime 版本 |

### 7.2 建议取舍

首期商业化不要承诺“通用 TeX 全覆盖”，而应明确支持范围：

```text
中文学术论文
JOS/软件学报场景
通用 arXiv/article
部分 SCI/会议模板
```

首期产品定位建议：

```text
Tex2Doc Pro Desktop
  本地转换 + 云端高质量转换 + 诊断报告 + 期刊 Profile
```

暂不建议首期承诺：

- 完整 TikZ 还原。
- 所有自定义宏自动理解。
- 所有期刊模板 100% 保真。
- 私有化部署。
- Word 模板在线编辑平台。

---

## 八、结论

项目目前已经具备商业化原型的技术底座，但离正式商业化发布仍有明显距离。当前核心不是继续堆功能数量，而是把已形成的转换能力产品化、生产化、可验证化。

最优先的下一步是：

```text
P10 Preview 收口
  -> P11 生产 auth / usage / billing
  -> P12 storage / queue / sandbox worker
  -> P13 quality benchmark
  -> P14 desktop packaging / updater
```

只要 P10-P13 完成，项目就可以进入邀请制 Beta；P14-P16 完成后，才适合进入付费 Beta 和正式 GA。
