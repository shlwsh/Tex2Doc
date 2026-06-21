# Semantic TeX Engine 商业化就绪度评估与实施方案

**文档版本**：20260622-005850  
**评估日期**：2026-06-22  
**基准文档**：`docs-zh/semantic-tex-engine-progress-report-20260621-180000.md`  
**补充参考**：

- `docs-zh/semantic-tex-engine-p5-p6-development-progress-20260621-232121.md`
- `docs-zh/semantic-tex-engine-p7-semantic-worker-progress-20260622-002137.md`
- `docs-zh/semantic-tex-engine-p8-p9-progress-20260621-235726.md`
- `docs-zh/semantic-tex-engine-p8-word-open-regression-progress-20260622-004126.md`
- `docs-zh/semantic-tex-engine-commercialization-gap-implementation-plan-20260622-002323.md`

**目标**：结合 20260621-180000 进展报告之后的 P5-P9 开发进展，评估项目距离商业化发布还缺哪些工作，并提出可执行、可验收的技术实施方案。

---

## 一、总体结论

当前项目已经具备“商业化技术预览”的基础，但尚未达到公开收费 GA 发布要求。

核心判断：

```text
转换核心已经可演示，
商业产品闭环尚未生产化。
```

当前更适合的发布级别：

| 发布级别 | 当前就绪度 | 结论 |
|---|---:|---|
| 内部 Preview | 85% | 可用于团队内部验证、paper3 演示、7 profile fixture 回归 |
| 受控 PoC | 70% | 可给少量合作用户试用，但必须人工跟进失败样本 |
| 邀请制 Beta | 55% | 需要补齐真实账号、用量、云端持久化、安全沙箱、桌面云转换 |
| 付费 Beta | 40% | 需要接入支付、账单、退款、升级、发布签名、稳定回归基准 |
| 正式 GA | 30% | 需要 SLA、监控告警、合规、三平台安装包、质量承诺 |
| Enterprise | 15% | 需要私有化部署、SSO、租户隔离、审计和模板定制平台 |

因此建议商业化路线为：

```text
内部 Preview
  -> 受控 PoC
  -> 邀请制 Beta
  -> 付费 Beta
  -> Pro Desktop + Cloud GA
  -> Team / Enterprise
```

不建议当前直接公开自助收费发布。

---

## 二、当前开发进展基线

### 2.1 语义转换核心

截至当前工作区，项目已经具备以下核心能力：

| 能力 | 当前状态 | 商业价值 |
|---|---|---|
| V1 Rust rule-based 引擎 | 保持独立 | 稳定 fallback 与效果对照 |
| V2 Semantic TeX Engine | 已独立实现 | 商业化主路径 |
| RuleBased / XeLaTeX Hook / LuaTeX Node 三路径 | 已建立 | 兼顾无 runtime、中文 CTeX、长期 LuaTeX 语义采集 |
| 7 类 Journal Profile | 已实现 | 首期目标市场明确 |
| ProfileStyleMap | 已接入 DOCX 渲染 | 可按期刊映射 Word 样式 |
| QualityGateResult | 已接入编译报告 | 可作为自动验收入口 |
| semantic CLI | 已提供 detect/analyze/convert/verify 入口 | 支撑专业用户、CI、桌面端 |
| paper3 三路径验证脚本 | 已有 | 支撑演示和回归对比 |

需要特别保留的架构边界：

```text
旧 Rust 规则引擎和新语义引擎必须保持独立。
商业云端 worker 可以按 engine 参数选择路径，
但不能把二者耦合成不可验证的一条混合实现。
```

### 2.2 P5 桌面端进展

当前 Slint 桌面端已经具备：

- Linux 构建链路可通过。
- 本地转换按钮接入 `SemanticTexEngine::compile_dir_to_docx()`。
- 支持 profile、quality、输出路径、报告摘要、任务历史。
- 支持打开输出目录。
- 已有 updater 模块骨架。

仍处于 preview 的原因：

- 还没有真实文件/目录选择器和拖拽。
- 还没有登录、注册、套餐、用量、云端转换 UI。
- 还没有 token 安全存储。
- 还没有三平台安装包、代码签名、公证和升级验证。
- 还没有真实 GUI 操作验收矩阵。

### 2.3 P6 商业 API 进展

当前商业 API 已经从静态合约推进到 preview 服务端：

- `doc-commercial-api-client` 已覆盖 auth、usage、billing、uploads、conversions、releases。
- server 已提供 `/v1` 与 `/api/v1` 兼容端点。
- 账号相关端点可返回 demo access/refresh token。
- 用户端点、用量端点、上传、创建转换、查询转换、下载、报告、billing portal/checkout 已加入 Bearer token 门禁。
- 已有内存态 `cloud_conversions_used` 和 `PREVIEW_CLOUD_CONVERSION_LIMIT`。
- 创建云转换前会进行基础额度扣减，额度不足返回 402。

仍处于 preview 的原因：

- token 是 `demo-access-*` / `demo-refresh-*`，不是 JWT。
- 没有密码哈希、邮箱验证、重置密码、refresh token 轮换。
- 用量存储在内存 `HashMap`，服务重启即丢失。
- 没有数据库用户、订阅、usage event ledger。
- billing URL 是模拟地址，没有支付 provider 和 webhook。
- 没有租户隔离、RBAC、审计。

### 2.4 P7 云端 Worker 进展

当前 worker 已经具备：

- `/v1/uploads` 保存上传 zip 到 `ServerState`。
- `/v1/conversions` 创建 queued job 并投递到 `tokio::sync::mpsc` 队列。
- worker 默认调用 `SemanticTexEngine::compile_zip_to_docx()`。
- `legacy-rule` / `doc-core` 作为显式或 fallback 路径保留。
- report 输出 executor、backend、profile、quality_status、compatibility_score、warnings。
- 成功产物可通过下载端点返回真实 DOCX。

仍处于 preview 的原因：

- uploads/jobs/docx/report 均在内存中。
- 没有 PostgreSQL、对象存储、任务恢复、重试、取消、过期清理。
- 没有 Docker/rootless container/namespace/seccomp/cgroup 等 sandbox。
- 没有 CPU、内存、磁盘、运行时长和进程数限制。
- 没有横向扩展和多 worker 调度。

### 2.5 P8 回归体系进展

当前回归体系已经具备：

- 7 个 profile，每个 `minimal + 3 realistic` fixture。
- `scripts/nightly_regression.sh` 可遍历 fixture 执行 semantic-convert。
- 已输出 `results.jsonl`、`conversion_stats.json`、`conversion_stats.md`。
- 已验证 DOCX ZIP header、必需 part、XML well-formed。
- 可选启用 LibreOffice headless 打开验证。
- 支持 `NIGHTLY_WORD_OPEN_REQUIRED=true` 强门禁模式。
- 已修复 DOCX 根节点重复 namespace 问题。

仍处于 preview 的原因：

- 当前 realistic fixture 数量不足以支撑 GA 质量承诺。
- LibreOffice/Word 实际打开验证还受环境限制，本地可能 skipped。
- 还缺公式、表格、图片、引用、样式覆盖率指标。
- 还没有失败样本库和失败分类仪表盘。
- 还没有真实用户模板的大样本统计。

### 2.6 P9 自动升级进展

当前 P9 已经具备：

- 桌面端 updater manifest 解析。
- 版本比较、SHA256 校验、签名状态占位。
- 服务端 release manifest 返回合法 sha256 格式。

仍处于 preview 的原因：

- 没有真实 release artifact 下载。
- 没有真实 Ed25519/minisign 或平台签名验签。
- 没有 Windows MSI、macOS DMG、Linux AppImage/deb/rpm。
- 没有代码签名、macOS notarization、Windows SmartScreen 信誉积累。
- updater 尚未接入 Slint UI 和安装器执行。

---

## 三、商业化发布的主要缺口

### 3.1 产品闭环缺口

当前已有转换能力，但产品闭环仍不完整：

```text
用户注册
  -> 登录
  -> 选择本地/云端转换
  -> 上传项目
  -> 查看额度
  -> 等待云端任务
  -> 下载 DOCX
  -> 查看质量报告
  -> 额度扣减
  -> 订阅升级
  -> 自动升级客户端
```

上述链路目前只有部分 API contract 和本地转换 UI，尚未形成完整商业用户体验。

### 3.2 生产云端缺口

必须从内存态 preview 升级为生产架构：

```text
PostgreSQL
  users / sessions / subscriptions / usage_events / uploads / conversions / artifacts

Object Storage
  uploaded_zip / generated_docx / compile_report / logs / diagnostics

Queue
  conversion queue / retry / dead letter / priority

Sandbox Worker
  per-job isolated workspace / timeout / cgroup / seccomp / no network
```

### 3.3 账号计费缺口

正式商业化必须补齐：

- Argon2id 密码哈希。
- JWT access token。
- refresh token 轮换与撤销。
- 邮箱验证与找回密码。
- 订阅表、套餐表、权益表。
- usage event ledger，不只保存累计数字。
- Stripe 或等价支付 provider。
- webhook 签名验证、幂等处理、订阅状态同步。
- 失败转换的额度返还策略。

### 3.4 安全隔离缺口

TeX 输入不能信任。云端转换必须具备：

- 禁用 shell escape。
- 默认禁止网络。
- 限制输入 zip 解压路径，防 zip slip。
- 限制 CPU、内存、磁盘、进程数、运行时长。
- 每 job 临时目录隔离。
- 运行用户降权。
- 日志脱敏。
- 完成后清理临时目录。

### 3.5 质量承诺缺口

商业用户买的不是“生成一个 docx 文件”，而是：

- Word 能打开。
- 内容可编辑。
- 标题、摘要、作者、图、表、公式、引用尽量保真。
- 未支持内容能准确诊断。
- 失败可复现、可解释、可人工修复。

因此必须建立 profile 级质量指标，而不是只看转换是否成功。

### 3.6 分发与运维缺口

正式发布还需要：

- 三平台安装包。
- 版本发布流水线。
- artifact 签名。
- 自动升级灰度。
- API 监控、worker 监控、队列监控。
- 错误追踪和诊断包。
- 备份、恢复、数据保留和删除策略。
- 隐私政策、服务条款、退款规则。

---

## 四、目标商业化技术架构

### 4.1 总体架构

```text
Desktop Slint App
  ├─ Local Convert
  │    ├─ SemanticTexEngine
  │    └─ Legacy Rule Engine
  │
  └─ Cloud Convert
       ├─ Auth API
       ├─ Usage API
       ├─ Billing API
       ├─ Upload API
       ├─ Conversion Job API
       └─ Release API

Cloud API Server
  ├─ Auth Service
  ├─ Billing Service
  ├─ Usage Service
  ├─ Upload Service
  ├─ Conversion Orchestrator
  ├─ Artifact Service
  └─ Release Manifest Service

Worker Pool
  ├─ Sandbox Runner
  ├─ Semantic Engine Runner
  ├─ Legacy Rule Runner
  ├─ Quality Verifier
  └─ Artifact Publisher

Data Plane
  ├─ PostgreSQL
  ├─ Object Storage
  ├─ Redis / Queue
  └─ Metrics / Logs / Traces
```

### 4.2 双引擎执行边界

商业版本必须明确引擎路径：

| 路径 | 用途 | 是否扣云端额度 |
|---|---|---|
| `local-semantic` | 桌面本地语义转换 | 不扣云端额度 |
| `cloud-semantic` | 云端默认高质量转换 | 扣云端额度 |
| `cloud-legacy-rule` | 旧 Rust 规则引擎 fallback 或对照 | 按策略扣减 |
| `cloud-compare` | 同时跑 semantic 与 legacy，用于诊断 | 高级套餐或内部 |

实现约束：

- 旧规则引擎不因商业化改造而改变默认行为。
- 新语义引擎通过 adapter 调用旧组件，但不把旧路径变成隐式依赖。
- report 中必须记录 executor、backend、fallback_from、profile、quality gate。

### 4.3 数据模型建议

首期生产表：

| 表 | 作用 |
|---|---|
| `users` | 用户基础信息 |
| `auth_identities` | 邮箱密码、OAuth 等登录身份 |
| `refresh_tokens` | refresh token 哈希、过期、撤销 |
| `plans` | 套餐定义 |
| `subscriptions` | 当前订阅状态 |
| `usage_events` | 额度事件流水 |
| `uploads` | 上传项目元数据 |
| `conversions` | 转换任务状态 |
| `artifacts` | DOCX、report、log 存储索引 |
| `billing_events` | 支付 provider webhook 事件 |
| `release_artifacts` | 桌面端发布包 |
| `audit_logs` | 登录、下载、删除、管理员操作 |

关键原则：

```text
额度扣减必须用 usage_events 事件流水实现，
不能只维护一个 used counter。
```

建议事件类型：

```text
conversion_reserved
conversion_completed
conversion_failed_refunded
conversion_failed_charged
manual_credit_granted
subscription_reset
```

### 4.4 云端任务状态机

生产状态机建议：

```text
created
  -> uploaded
  -> queued
  -> reserved_quota
  -> normalizing
  -> detecting
  -> analyzing
  -> compiling
  -> rendering
  -> verifying
  -> publishing
  -> completed

failure branches:
  -> failed_user_input
  -> failed_engine
  -> failed_timeout
  -> failed_sandbox
  -> failed_internal
  -> cancelled
  -> expired
```

任务必须支持：

- 幂等创建。
- 查询状态。
- 取消任务。
- 超时失败。
- 重试策略。
- 死信队列。
- artifact 过期清理。

---

## 五、分阶段实施方案

### 阶段 C0：商业化基线冻结

周期：1 周  
目标：把当前 preview 能力整理成可稳定演示的 baseline。

实施项：

1. 冻结 `paper3`、7 profile fixture、nightly regression 的当前通过状态。
2. 整理三路径输出：
   - sh/rule 路径 DOCX
   - Rust rule engine DOCX
   - Semantic engine DOCX
3. 给每个输出配套 report、log、质量摘要。
4. 完成 P6 auth/usage 最新改造的集成测试回归。
5. 输出 `Preview Acceptance Report`。

验收标准：

| 指标 | 门槛 |
|---|---:|
| paper3 三路径转换 | 3/3 成功 |
| 7 profile fixture | 28/28 成功 |
| DOCX required parts | 100% |
| XML well-formed | 100% |
| P6/P7 API 测试 | 通过 |
| 桌面端 cargo check | 通过 |

### 阶段 C1：Beta 质量基准

周期：2-3 周  
目标：把“可转换”升级为“可度量、可承诺、可回归”。

实施项：

1. 每个 profile 扩展到至少 10 个 realistic fixture。
2. `semantic-verify` 完整实现：
   - DOCX package 结构检查
   - XML well-formed 检查
   - Word/LibreOffice 打开验证
   - style coverage
   - image relationship 检查
   - bookmark/hyperlink 检查
   - OMML 公式检查
3. 引入失败分类：
   - profile-detection
   - unsupported-package
   - macro-lowering
   - formula-omml
   - table-span
   - image-missing
   - docx-invalid
   - runtime-backend
4. 建立 `quality_baseline.json`，记录每个 profile 的历史趋势。
5. CI 中增加 nightly summary 门禁。

验收标准：

| 指标 | Beta 门槛 |
|---|---:|
| realistic fixture / profile | >= 10 |
| DOCX 实际打开率 | >= 98% |
| profile 检测准确率 | >= 95% |
| 无 panic 转换率 | >= 99% |
| 未解析引用为 0 的样本占比 | >= 90% |
| 质量报告生成率 | 100% |

### 阶段 C2：生产级云端转换

周期：3-5 周  
目标：把内存态 worker 升级成可恢复、可扩展、可隔离的生产服务。

实施项：

1. 引入 PostgreSQL：
   - users
   - uploads
   - conversions
   - artifacts
   - usage_events
2. 引入对象存储：
   - original zip
   - normalized project
   - generated docx
   - report json
   - compile logs
3. 引入队列：
   - Redis Streams、PostgreSQL advisory queue 或专用消息队列均可。
4. Worker sandbox：
   - 每 job 独立 workdir。
   - 禁止网络。
   - 禁止 shell escape。
   - 限制 CPU/memory/disk/time/process。
   - 清理临时文件。
5. artifact 生命周期：
   - Free/Preview 保留 7 天。
   - Pro 保留 30-90 天。
   - Enterprise 可配置。
6. 任务恢复：
   - API 重启不丢任务。
   - worker 崩溃后 queued/running 任务可重新分派。

验收标准：

| 指标 | 门槛 |
|---|---:|
| API 重启后任务可查询 | 100% |
| worker 崩溃后任务可恢复 | 通过演练 |
| 单 job 超时控制 | 100% |
| ZIP slip 防护 | 通过恶意样本测试 |
| 并发转换 | >= 10 jobs |
| P95 转换耗时 | < 120s for Beta |

### 阶段 C3：账号、订阅和用量

周期：3-4 周  
目标：形成可收费的账号权益闭环。

实施项：

1. Auth：
   - 邮箱注册/登录。
   - Argon2id 密码哈希。
   - JWT access token。
   - refresh token 轮换。
   - 登出和撤销。
2. Usage：
   - 额度预留。
   - 失败返还。
   - 幂等扣减。
   - 月度重置。
3. Billing：
   - 支付 provider checkout。
   - customer portal。
   - webhook 签名验证。
   - 订阅状态同步。
   - 套餐变更。
4. API middleware：
   - auth guard。
   - plan entitlement guard。
   - rate limit。
   - request id。
5. 管理工具：
   - 查看用户。
   - 调整额度。
   - 重放 webhook。
   - 手工退款/补偿。

验收标准：

| 指标 | 门槛 |
|---|---:|
| 未授权访问受保护端点 | 401 |
| 额度不足创建任务 | 402 |
| webhook 重放 | 幂等 |
| refresh token 轮换 | 通过 |
| 支付测试模式订阅 | 端到端通过 |
| 失败任务额度返还 | 通过 |

### 阶段 C4：桌面商业客户端

周期：4-6 周  
目标：把 Slint MVP 升级为用户可直接使用的商业客户端。

实施项：

1. 文件操作：
   - 文件夹选择器。
   - `.tex` 主文件选择。
   - 拖拽 zip/project folder。
   - 输出目录选择。
2. 本地转换：
   - profile 自动检测。
   - quality 选择。
   - 质量报告查看。
   - 最近任务历史。
3. 云端转换：
   - 登录/注册。
   - token 安全保存。
   - 上传进度。
   - job 轮询。
   - DOCX 下载。
   - report 下载。
   - 失败诊断展示。
4. 商业入口：
   - 用量显示。
   - 套餐显示。
   - 打开 checkout/portal。
5. 自动升级：
   - 查询 release manifest。
   - 下载 artifact。
   - SHA256 校验。
   - 签名验签。
   - 平台安装器执行。
6. 诊断包：
   - app log。
   - compile report。
   - engine backend。
   - profile detection。
   - sanitized project summary。

验收标准：

| 指标 | 门槛 |
|---|---:|
| Windows 本地转换 | 通过 |
| macOS 本地转换 | 通过 |
| Linux 本地转换 | 通过 |
| 云端转换闭环 | 通过 |
| token 安全存储 | 通过 |
| 自动升级检测 | 通过 |
| 诊断包生成 | 通过 |

### 阶段 C5：三平台分发和发布工程

周期：3-5 周  
目标：让客户端可以安全安装、升级和回滚。

实施项：

1. Windows：
   - MSI 或 NSIS。
   - 代码签名。
   - 安装/卸载测试。
2. macOS：
   - `.app` bundle。
   - DMG。
   - codesign。
   - notarization。
3. Linux：
   - AppImage。
   - deb/rpm 可选。
4. Release manifest：
   - platform
   - arch
   - channel
   - version
   - download_url
   - sha256
   - signature
   - release_notes
5. 灰度发布：
   - stable
   - beta
   - nightly
6. 回滚：
   - 保留最近 N 个版本 artifact。
   - manifest 可指定最低可用版本。

验收标准：

| 指标 | 门槛 |
|---|---:|
| 三平台安装包 | 生成成功 |
| 三平台签名 | 通过 |
| macOS notarization | 通过 |
| manifest 验签 | 通过 |
| 升级失败回滚 | 通过演练 |

### 阶段 C6：运维、安全和合规

周期：持续，Beta 前完成基础版  
目标：从工程 demo 升级为可运营服务。

实施项：

1. Observability：
   - structured tracing。
   - request id。
   - metrics。
   - logs。
   - error tracking。
2. 告警：
   - API 5xx。
   - queue backlog。
   - worker failure rate。
   - conversion timeout。
   - storage error。
3. 合规：
   - 隐私政策。
   - 服务条款。
   - 数据删除。
   - 文件保留周期。
   - 日志脱敏。
4. 安全：
   - 依赖漏洞扫描。
   - upload malware/zip bomb 防护。
   - secret 管理。
   - webhook 签名。
   - admin 操作审计。

验收标准：

| 指标 | 门槛 |
|---|---:|
| API/worker metrics | 已接入 |
| 失败任务可追踪 | 100% |
| 用户数据删除 | 可执行 |
| 上传保留策略 | 生效 |
| secrets 不入库不入日志 | 通过审计 |

### 阶段 C7：企业版能力

周期：GA 后迭代  
目标：支撑机构、出版社、投稿服务团队。

实施项：

- 组织/团队。
- 多用户额度池。
- 自定义 profile。
- 模板规则编辑器。
- 批量转换。
- 私有化部署。
- SSO/SAML/OIDC。
- 审计日志。
- 管理员控制台。

---

## 六、近期优先级清单

建议把后续开发拆成 10 个可验收 work packages：

| 优先级 | Work Package | 目标 |
|---|---|---|
| P0 | Preview baseline freeze | 固化 paper3、7 profile、P6/P7 API、P8 regression 当前状态 |
| P1 | `semantic-verify` 完整实现 | 从脚本验证升级为正式 CLI/API 质量验证 |
| P2 | Auth/JWT/Usage Ledger | 取代 demo token 和内存 usage counter |
| P3 | Persistent Job Store | 取代内存 uploads/jobs/docx/report |
| P4 | Worker Sandbox | 支撑公网上传转换的安全底线 |
| P5 | Desktop Cloud Convert | Slint 接入登录、上传、轮询、下载、报告 |
| P6 | Billing Test Mode | 支付 provider 测试模式端到端跑通 |
| P7 | Realistic Fixture 70+ | 7 profile x 10 样本，质量趋势入库 |
| P8 | Release Packaging | 三平台安装包、签名、manifest、升级 |
| P9 | Observability & Compliance | 监控、告警、数据保留、隐私条款 |

---

## 七、商业化验收矩阵

### 7.1 邀请制 Beta 验收

| 类别 | 门槛 |
|---|---|
| 转换 | 7 profile x 10 realistic fixtures |
| 打开 | DOCX 实际打开率 >= 98% |
| API | 真实 JWT、usage ledger、持久化 jobs |
| Worker | sandbox、timeout、resource limit |
| Desktop | 登录、本地转换、云转换、下载报告 |
| Billing | 测试模式 checkout/webhook 通过 |
| Release | 至少 Windows/Linux beta 安装包 |
| 运维 | 基础 metrics/logs/error tracking |

### 7.2 付费 Beta 验收

| 类别 | 门槛 |
|---|---|
| 转换 | 每 profile >= 20 样本 |
| 打开 | DOCX 实际打开率 >= 99% |
| 计费 | 真实支付、小额收费、退款策略 |
| 升级 | 三平台升级检查和签名校验 |
| 支持 | 诊断包、失败样本回收流程 |
| SLA | 明确 Beta SLA 和免责边界 |

### 7.3 GA 验收

| 类别 | 门槛 |
|---|---|
| 转换 | 每 profile >= 30 样本 |
| 稳定 | 无崩溃转换率 >= 99.5% |
| 性能 | P95 云端转换 < 60s，复杂文档例外 |
| 分发 | Windows/macOS/Linux 签名安装包 |
| 合规 | 隐私、条款、删除、保留策略 |
| 运维 | 备份、恢复、告警、值班流程 |

---

## 八、主要风险与缓解方案

| 风险 | 影响 | 缓解 |
|---|---|---|
| 用户模板差异过大 | 转换失败率高 | profile registry + compatibility analyzer + 失败样本库 |
| TeX 云端执行安全风险 | 公网服务不可上线 | sandbox、禁网、资源限制、输入清洗 |
| DOCX 在 Word 中打开失败 | 直接影响付费转化 | LibreOffice/Word-open 强门禁、OOXML 单元测试 |
| 公式/表格保真不足 | 学术场景核心体验受损 | OMML 覆盖率、table span 指标、人工样本分类 |
| 账号计费用 demo 实现替代生产实现 | 无法收费 | JWT、usage ledger、billing webhook 优先落地 |
| 三平台签名成本高 | 客户端发布受阻 | 先 Linux/Windows beta，再 macOS notarization |
| LibreOffice 验证环境不稳定 | CI 假失败或跳过过多 | verifier preflight + required 模式分环境启用 |
| 旧规则引擎被新语义路径污染 | 对照基线丢失 | 保持独立 crate/adapter/report 字段 |

---

## 九、下一步建议

建议马上推进的 5 件事：

1. 完成 P6 最新 auth/usage 改造的完整测试回归，并新增进展报告。
2. 将 `semantic-verify` 从预留命令升级为真实验证入口，复用 nightly regression 中的 DOCX 检查逻辑。
3. 设计并实现 PostgreSQL schema 与 object storage adapter，先替换 `ServerState` 的 uploads/jobs/artifacts。
4. 在 Slint 桌面端接入商业 API client，先打通登录、usage、cloud conversion 三个界面闭环。
5. 扩展每个 profile 至 10 个 realistic fixture，并把 conversion_stats 纳入 CI 可比较基线。

---

## 十、最终判断

当前项目距离商业化不是“缺一个功能”，而是缺一组生产化系统：

```text
质量基准
账号计费
云端持久化
安全沙箱
桌面产品闭环
三平台分发
运维合规
```

但核心转换技术已经具备可商业化的起点，尤其是：

- 新旧引擎独立存在，便于效果对比。
- XeLaTeX/LuaTeX/RuleBased 三路径已经建立。
- Journal Profile 泛化已经形成差异化定位。
- P7 semantic worker 已能通过 API 产出真实 DOCX。
- P8 nightly regression 已经开始从“能生成”走向“可验证”。

建议以“中文学术论文 + SCI/CS 投稿 DOCX 转换”为首期商业化边界，先做受控 PoC 和邀请制 Beta，避免过早承诺通用 TeX 全覆盖。
