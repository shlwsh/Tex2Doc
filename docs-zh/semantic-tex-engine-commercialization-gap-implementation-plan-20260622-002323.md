# Semantic TeX Engine 商业化差距评估与实施方案

**文档版本**：20260622-002323  
**评估日期**：2026-06-22  
**基准报告**：`docs-zh/semantic-tex-engine-progress-report-20260621-180000.md`  
**补充进展**：

- `docs-zh/semantic-tex-engine-p5-p6-development-progress-20260621-232121.md`
- `docs-zh/semantic-tex-engine-p7-worker-progress-20260621-233323.md`
- `docs-zh/semantic-tex-engine-p8-p9-progress-20260621-235726.md`
- 当前工作区 P7 semantic-worker、P8 nightly regression、P9 updater 骨架实现

**目标**：结合当前开发进展，评估项目距离商业化发布还缺哪些关键能力，并给出可落地、可验收的技术实施方案。

---

## 一、结论摘要

当前项目已经从“语义引擎原型”推进到“可演示的商业化技术预览”阶段，但还没有达到公开收费发布或企业级交付要求。

综合判断：

| 发布级别 | 当前就绪度 | 判断 |
|---|---:|---|
| 内部 Preview | 80% | 可用于内部演示、paper3 对比、7 profile fixture 回归和受控 PoC |
| 邀请制 Beta | 50% | 需要补齐真实账号、用量扣减、持久化任务、云端 sandbox、桌面端云转换闭环 |
| 正式商业 GA | 30% | 需要生产计费、签名升级、三平台安装包、SLA、监控、隐私合规和大样本质量基准 |
| Enterprise | 15% | 需要私有化部署、SSO、租户隔离、审计、企业模板定制和管理员控制台 |

当前最重要的判断：

```text
核心转换资产已经具备商业化潜力，
但产品闭环、生产安全、质量承诺、计费交付和运维体系尚未生产化。
```

因此建议商业化路线为：

```text
技术 Preview
  -> 邀请制 Beta
  -> Pro Desktop + Cloud GA
  -> Team / Enterprise 交付
```

不建议立即公开自助收费发布。

---

## 二、当前事实基线

### 2.1 已经具备的商业化资产

| 资产 | 当前状态 | 商业价值 |
|---|---|---|
| V1 Rust 规则引擎 | 保持独立 | 可作为稳定 fallback 和效果对照 |
| V2 Semantic TeX Engine | 已独立存在 | 是后续商业化主路径 |
| RuleBased / XeLaTeX / LuaTeX 三路径 | 已建立 | 可覆盖无 runtime、中文 CTeX、长期 LuaTeX 语义采集 |
| 7 类 Journal Profile | 已实现 | 可形成首期市场定位 |
| ActiveProfile / ProfileRef | 已推进 | 支撑自动检测、profile-aware 渲染和质量门禁 |
| RuleEngine / CompatibilityAnalyzer | 已接入方向 | 支撑模板宏泛化和兼容性诊断 |
| QualityGate | 已有 V2 基础 | 可作为商业成功/失败判断基础 |
| semantic CLI | 已产品化推进 | 可服务专业用户、CI 和桌面端 |
| paper3 三路径脚本 | 已有 | 可做演示、回归和销售样例 |
| P8 nightly regression | 已有 28 fixture 首期回归 | 支撑 Preview 质量门禁 |

### 2.2 P5-P9 最新进展

| 阶段 | 当前状态 | 已完成 | 仍未生产化 |
|---|---|---|---|
| P5 Slint Desktop MVP | in_progress | Linux 构建通过；本地转换、profile、quality、report、history 已接 UI | 缺真实 GUI 操作验收、文件选择器、登录/用量/订阅、云端转换、三平台安装包 |
| P6 商业 API | in_progress | client 覆盖 auth/usage/billing/uploads/conversions/releases；server 有 `/v1` 合约端点 | auth 是 demo token；无 JWT、数据库、支付 provider、真实额度扣减 |
| P7 云端 Worker | in_progress | 内存态 upload/job/queue/worker 已有；worker 主路径可调用 `SemanticTexEngine`，旧 `doc_core` 作为 fallback | 无持久化、对象存储、sandbox、资源限制、崩溃恢复、横向扩展 |
| P8 回归体系 | in_progress | 7 profiles x 4 fixtures 全量 nightly 回归 28/28 通过 | 缺 Word/LibreOffice 打开验证、公式/表格/图片/样式质量指标、真实失败样本库 |
| P9 自动升级 | in_progress | 桌面端 updater manifest/SHA256 校验骨架；server 返回合法 sha256 | 缺真实签名验签、artifact 下载、安装器执行、MSI/DMG/AppImage、代码签名和公证 |

### 2.3 当前验证情况

已完成的关键验证：

```text
cargo check -p doc-commercial-api-client
cargo check -p doc-server
cargo test -p doc-server --test api -- --nocapture
cargo test -p doc-desktop-slint updater -- --nocapture
ALLOW_FAILURES=true scripts/nightly_regression.sh
```

当前 nightly regression 已确认：

| 指标 | 结果 |
|---|---:|
| Total fixtures | 28 |
| Succeeded | 28 |
| Failed | 0 |
| DOCX openable by ZIP header | 28 |
| Reports generated | 28 |
| Profile detection matched | 28 |
| Panic detected | 0 |

注意：这里的 DOCX openable 目前只是 ZIP 结构层面的可打开前置检查，还不是 Word/LibreOffice 实际打开验证。

---

## 三、当前不能商业化 GA 的核心原因

### 3.1 云端执行仍是内存态 preview

当前 `crates/server/src/state.rs` 以 `HashMap` 保存 uploads/jobs，适合集成测试和 demo，不适合生产：

- 服务重启后任务丢失。
- 上传文件、DOCX、report 全部在内存中。
- 无任务过期清理。
- 无幂等创建、重试、取消和恢复。
- 无横向扩展能力。

生产目标应切换为：

```text
PostgreSQL: users / jobs / subscriptions / usage / releases
Object Storage: uploads / docx / reports / logs
Redis or Queue Service: conversion queue / rate limit / locks
```

### 3.2 TeX 输入尚未安全隔离

TeX 项目是高风险不可信输入。商业云端转换必须具备：

- 禁止 shell escape。
- 限制网络访问。
- 限制文件系统读写范围。
- 限制 CPU、内存、磁盘、进程数、运行时间。
- 按 job 隔离临时目录。
- 编译日志脱敏。
- 失败后清理临时文件。

当前 worker 尚未接入 Docker/rootless container、namespace、seccomp/cgroup 或等价 sandbox，因此不能开放公网自助上传转换。

### 3.3 账号、套餐、用量和支付仍是合约模拟

当前 API 形态已经接近商业产品，但数据是固定值或 demo token：

- 无密码哈希。
- 无 JWT access/refresh token。
- 无 auth middleware。
- 无订阅状态同步。
- 无 Stripe 或等价支付回调。
- 无额度扣减。
- 无额度不足阻断。
- 无发票、退款、套餐变更策略。

这意味着当前可以做 API contract demo，不能做真实收费。

### 3.4 质量承诺还缺真实样本基准

商业用户关注的不是“能生成 DOCX”，而是：

- Word 是否能打开。
- 结构是否可编辑。
- 标题、摘要、图表、公式、引用是否保真。
- 缺失资源和未知宏是否可诊断。
- 是否能稳定处理真实投稿模板。

当前有 28 个首期 fixture，但还不足以支撑商业承诺。Beta 至少应达到每 profile 10+ 真实/半真实样本，GA 至少每 profile 30+ 样本，并建立失败分类和质量趋势。

### 3.5 桌面端还不是完整商业客户端

当前 Slint 客户端已有本地转换 MVP，但商业客户端还需要：

- 文件/目录选择器和拖拽。
- 登录/注册/退出。
- 安全保存 token。
- 用量和套餐展示。
- 云端上传、创建任务、轮询、下载。
- 失败报告和可复制诊断包。
- 自动升级 UI。
- Windows/macOS/Linux 安装包和签名。

### 3.6 发布和合规体系尚未建立

正式商业发布需要：

- 隐私政策和数据删除策略。
- 服务条款。
- 用户文件保留周期。
- 错误日志脱敏。
- 发布版本签名。
- 安全漏洞响应机制。
- 监控告警和支持流程。

这些目前都不是代码层面的小补丁，而是商业产品必须补齐的交付体系。

---

## 四、商业化产品边界建议

### 4.1 首期产品定位

建议首期产品定义为：

```text
Tex2Doc Desktop + Cloud
面向中文学术论文、SCI/CS 期刊论文的 TeX/LaTeX/CTeX -> DOCX 高保真转换工具
```

首期不承诺“通用 TeX 全覆盖”，而是明确支持：

| 场景 | 首期策略 |
|---|---|
| 中文学术论文 | 重点支持 ctexart、JOS、医学论文 |
| SCI/CS 投稿 | 支持 TACL、CVPR、Nature、Springer、generic |
| arXiv | generic fallback |
| 复杂 TikZ/minted | 降级输出并在 report 中提示 |
| 完全自定义 class | 先走 generic + 兼容性诊断 |

### 4.2 套餐建议

| 套餐 | 转换路径 | 适用用户 |
|---|---|---|
| Free | 本地 rule/semantic 预览，少量云端额度 | 试用用户 |
| Pro | 云端 semantic worker，更多次数，高质量报告 | 论文作者、研究生 |
| Team | 团队额度、批量转换、历史记录 | 实验室、投稿服务团队 |
| Enterprise | 私有化部署、自定义 profile、SSO、审计 | 机构、出版社、企业 |

### 4.3 商业成功指标

| 指标 | Preview | Beta | GA |
|---|---:|---:|---:|
| 支持 profile | 7 | 7+ | 10+ |
| realistic fixture / profile | 3 | 10 | 30+ |
| DOCX 实际打开率 | ZIP 检查 | 98% | 99% |
| 无崩溃转换率 | 95% | 98% | 99.5% |
| Profile 自动识别准确率 | 90% | 95% | 97% |
| P95 云端转换耗时 | 不承诺 | < 120s | < 60s |
| 账号/用量/计费 | demo | 测试模式 | 生产模式 |
| 三平台安装包 | dev build | beta build | 签名发布 |

---

## 五、目标技术架构

### 5.1 产品入口

```text
CLI
  -> semantic-detect / semantic-analyze / semantic-convert / semantic-verify

Desktop Slint App
  -> Local Convert
  -> Cloud Convert
  -> Account / Usage / Billing
  -> Update / Diagnostics

Cloud API
  -> Auth / Billing / Usage
  -> Upload / Conversion Job
  -> Worker Pool
  -> Artifacts / Reports
```

### 5.2 生产云端架构

```text
Axum API
  -> Auth Middleware
  -> Body Limit / Rate Limit
  -> Upload Service
  -> Conversion Service
  -> Usage Metering
  -> Billing Webhook
  -> Release Service
  -> Admin / Support API

Queue
  -> semantic-worker
  -> quality-worker
  -> cleanup-worker

Worker Sandbox
  -> normalize project
  -> detect profile
  -> analyze compatibility
  -> execute SemanticTexEngine
  -> verify DOCX
  -> store artifacts
  -> meter usage

Storage
  -> PostgreSQL
  -> Object Storage
  -> Redis
```

### 5.3 转换执行器抽象

当前 P7 已经具备 semantic/legacy 分支雏形，后续应升级为正式执行器接口：

```rust
pub trait ConversionExecutor {
    fn execute(&self, input: ConversionInput) -> Result<ConversionOutput>;
}
```

建议实现：

| Executor | 作用 |
|---|---|
| `SemanticExecutor` | 主路径，调用 `SemanticTexEngine` |
| `LegacyRuleExecutor` | 旧 `doc_core` fallback 和对照 |
| `SandboxedExecutor` | 生产 worker 外壳，负责隔离、限额、超时 |
| `QualityVerifyExecutor` | 对 DOCX 做结构、Word/LibreOffice、样式和资源验证 |

---

## 六、分阶段实施方案

## P10：Preview 商业化硬化

周期建议：1-2 周。

目标：把当前 demo 能力固化为可演示、可复现、可交付给早期用户试用的 Preview。

任务：

1. 更新 P7 文档，明确 worker 主路径已切到 `SemanticTexEngine`，旧引擎仅 fallback。
2. 将 `scripts/nightly_regression.sh` 接入 CI。
3. `commercial_verify.sh` 读取 semantic report，输出统一质量 JSON。
4. 对 paper3 和 7 profiles 生成固定回归包。
5. Slint 客户端完成一次真实 GUI 本地转换冒烟。
6. `semantic-verify` 与 nightly 统计补齐 docx_bytes、profile、backend、quality_status、compatibility_score。
7. 形成 Preview release checklist。

验收：

```text
7 profiles x 4 fixtures nightly 通过
paper3 三路径输出稳定
Desktop Linux GUI 可手动完成一次转换
所有生成 DOCX 均有 report
失败能输出用户可理解错误
```

## P11：生产账号、用量与计费基础

周期建议：2-4 周。

目标：把 P6 demo API 变成可支撑 Beta 的账号和用量系统。

任务：

1. 引入 PostgreSQL 迁移：
   - users
   - sessions
   - subscriptions
   - usage_ledger
   - uploads
   - conversion_jobs
   - artifacts
   - release_manifests
2. 实现密码哈希，建议 Argon2id。
3. 实现 JWT access/refresh token。
4. 实现 auth middleware。
5. 实现用量扣减：
   - conversion_created
   - conversion_succeeded
   - conversion_failed_refund_policy
6. 接入 Stripe 或等价 billing provider 的测试模式。
7. 实现 billing webhook 幂等处理。
8. API client 和 desktop 接入登录、me、usage、plans、checkout。

验收：

```text
注册/登录/刷新/me 可用
未登录不能创建云端转换
额度不足不能创建转换
成功转换写入 usage ledger
checkout 和 billing portal 至少在测试模式可用
```

## P12：生产级云端 Worker 与安全隔离

周期建议：3-5 周。

目标：把内存态 worker 升级为可承载真实用户文件的安全云端转换系统。

任务：

1. 使用数据库任务表替换内存 jobs。
2. 使用对象存储保存 upload/docx/report/log。
3. 队列从内存 mpsc 升级为 Redis Stream、PostgreSQL job queue 或专用 MQ。
4. 每个 job 独立 workspace。
5. 引入 sandbox：
   - rootless container 或 namespace 隔离
   - 禁止网络
   - 只读 runtime 镜像
   - 限制 CPU/memory/disk/time
   - 禁止 shell escape
6. worker 状态机生产化：

```text
queued -> normalizing -> detecting -> analyzing -> compiling -> rendering -> verifying -> succeeded
                                                                -> failed
                                                                -> expired
                                                                -> canceled
```

7. 支持 job 取消、超时、重试、过期清理。
8. 收集 stdout/stderr/log 并脱敏。

验收：

```text
worker 重启后未完成任务可恢复或标记 failed
恶意 TeX 不能读取宿主机文件
恶意 TeX 不能访问网络
超时任务被终止
每个任务都有可下载 report 和脱敏日志
```

## P13：Desktop Beta 商业闭环

周期建议：3-5 周。

目标：让 Slint 桌面端成为真实 Beta 用户可用的入口。

任务：

1. 文件选择器：
   - 选择 `.tex`
   - 选择 `.zip`
   - 选择项目目录
   - 自动推断 main tex
2. 本地转换：
   - profile/backend/quality 选择
   - 后台任务
   - 取消任务
   - 打开输出目录
3. 云端转换：
   - 打包项目
   - 上传 zip
   - 创建 conversion
   - 轮询状态
   - 下载 DOCX/report
4. 账号：
   - 登录/注册/退出
   - refresh token
   - token 安全存储
5. 套餐与用量：
   - 当前套餐
   - 剩余额度
   - billing portal
6. 自动升级：
   - 调用 release manifest
   - 展示新版本
   - 下载并校验
   - 安装前确认
7. 诊断包：
   - report
   - logs
   - runtime info
   - app version

验收：

```text
Windows/macOS/Linux 至少 beta build 可运行
用户可以从登录到云端转换完成闭环
错误状态在 UI 中可读
token 不以明文写入普通配置文件
```

## P14：质量基准与失败样本库

周期建议：持续推进，Beta 前至少 2 周集中建设。

目标：把“感觉能转换”变成“可量化承诺”。

任务：

1. 扩展样本：
   - Beta：每 profile 10+ realistic fixture
   - GA：每 profile 30+ realistic fixture
2. 新增质量指标：
   - DOCX 实际打开验证
   - 公式 OMML 成功率
   - 表格结构成功率
   - 图片完整率
   - style coverage
   - unresolved references
   - raw fallback count
   - unknown macro count
3. 失败分类：
   - unsupported_package
   - missing_asset
   - runtime_missing
   - sandbox_timeout
   - parse_error
   - docx_verify_failed
4. nightly 输出趋势报告：
   - conversion_stats.json
   - conversion_stats.md
   - per-profile trend
   - failed samples index

验收：

```text
Beta 样本集 DOCX 实际打开率 >= 98%
每次回归能看出质量分变化
失败样本都有分类和复现命令
```

## P15：正式发布、升级与运维

周期建议：4-6 周。

目标：满足正式商业 GA 的发布、升级、监控和支持要求。

任务：

1. 三平台打包：
   - Windows MSI 或 MSIX
   - macOS DMG
   - Linux AppImage 或 deb/rpm
2. 代码签名：
   - Windows Authenticode
   - macOS Developer ID + notarization
   - Linux artifact checksum + signature
3. release manifest 生产化：
   - platform
   - arch
   - channel
   - version
   - sha256
   - signature
   - rollout percentage
4. updater 真实验签：
   - Ed25519 或等价签名
   - 内置公钥
   - 防回滚
5. 监控：
   - API latency
   - worker queue depth
   - conversion success rate
   - sandbox timeout rate
   - billing webhook failure
6. 支持：
   - 用户可导出诊断包
   - admin 可查看 job 状态
   - 错误码文档
7. 合规：
   - 隐私政策
   - 数据删除策略
   - 文件保留周期
   - 日志脱敏

验收：

```text
三平台签名安装包可下载和升级
API/worker 有生产监控和告警
用户可自助删除上传文件和转换产物
关键错误码有帮助文档
```

## P16：Enterprise 能力

周期建议：GA 后推进。

目标：支撑高价值企业、出版社和机构客户。

任务：

1. 私有化部署包。
2. 企业 license。
3. SSO/SAML/OIDC。
4. 租户隔离。
5. 管理员控制台。
6. 审计日志。
7. 自定义 profile 和模板。
8. 批量转换 API。
9. 离线模式和内网 runtime 镜像。

---

## 七、商业化发布门禁

### 7.1 Preview 门禁

必须满足：

- P0-P4 保持通过。
- P5 Linux GUI 能完成本地转换。
- P7 semantic worker 能完成 paper3 云端转换。
- P8 28 fixtures nightly 通过。
- P9 updater manifest/SHA256 骨架测试通过。
- 文档明确“不承诺生产 SaaS SLA”。

### 7.2 Beta 门禁

必须满足：

- 真实账号/JWT/用量扣减。
- 云端 worker 使用持久化任务表和对象存储。
- sandbox 阻断高风险输入。
- 每 profile 10+ realistic fixture。
- Desktop 支持登录和云端转换。
- Stripe 或等价支付测试模式打通。
- 用户可下载 report 和日志。

### 7.3 GA 门禁

必须满足：

- 生产支付和订阅同步。
- 三平台签名安装包。
- 自动升级真实签名验签。
- 每 profile 30+ realistic fixture。
- DOCX 实际打开率 >= 99%。
- 无崩溃转换率 >= 99.5%。
- P95 云端转换耗时 < 60s。
- 隐私政策、服务条款、数据删除机制上线。
- 生产监控、告警和支持流程上线。

---

## 八、近期优先级建议

### 立即做

1. 补一份 P7 semantic worker 最新进展报告，修正旧文档中“worker 调用 doc_core”的过时描述。
2. 将 `nightly_regression.sh` 纳入 CI。
3. 增加 DOCX LibreOffice headless 打开验证。
4. 完成 Slint GUI 手动冒烟和截图留档。
5. 定义数据库 schema 和对象存储路径规范。

### 下一阶段做

1. 实现 PostgreSQL 任务表和用户表。
2. 实现 JWT auth middleware。
3. 引入 sandbox worker。
4. 桌面端接入云端转换。
5. 支付测试模式闭环。

### GA 前必须做

1. 三平台安装包。
2. 代码签名和自动升级签名。
3. 生产监控告警。
4. 大样本质量基准。
5. 隐私与数据删除合规。

---

## 九、总体判断

当前项目的核心技术方向是成立的：语义引擎、期刊 profile、质量报告、三路径转换、桌面端和云端 API 都已经有了可运行基础。

但商业化的剩余工作已经从“能否转换”转向“是否能安全、稳定、可计费、可支持地交付给真实用户”。因此后续研发不应只继续堆转换特性，而应优先完成：

```text
生产级云端 worker
账号计费闭环
桌面端商业入口
安全隔离
质量基准
发布与升级体系
```

建议将下一目标定义为：

```text
Tex2Doc Preview Release
  时间目标：1-2 周
  范围：受控用户、本地转换、paper3/7 profile 演示、云端 semantic worker demo

Tex2Doc Invite Beta
  时间目标：6-10 周
  范围：账号、用量、云端 sandbox、桌面云转换、每 profile 10+ 样本

Tex2Doc GA
  时间目标：3-6 个月
  范围：生产支付、三平台签名安装、自动升级、SLA、监控、合规、大样本质量承诺
```
