# Semantic TeX Engine 商业化就绪度评估与实施方案
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



**文档版本**：20260621-234252  
**评估日期**：2026-06-21  
**基准文档**：`docs-zh/semantic-tex-engine-progress-report-20260621-180000.md`  
**补充进展**：

- `docs-zh/semantic-tex-engine-p5-p6-development-progress-20260621-232121.md`
- `docs-zh/semantic-tex-engine-p7-worker-progress-20260621-233323.md`
- 当前工作区 P8 realistic fixtures 初步补齐情况

**目标**：基于当前开发进展，判断项目距离商业化发布还缺哪些能力，并给出可实施、可验收的技术方案。

---

## 一、结论摘要

当前项目已经从“语义引擎原型”推进到“技术预览接近可交付”的阶段，但还没有达到商业化收费发布的要求。

综合判断：

| 发布级别 | 当前就绪度 | 判断 |
|---|---:|---|
| 内部技术预览 Preview | 约 75% | 可继续推进，需补齐 nightly regression、GUI 冒烟和基础 release 校验 |
| 邀请制 Beta | 约 45% | 还缺真实账号、订阅、用量、持久化任务、云端 sandbox、桌面端登录与云端转换闭环 |
| 正式商业 GA | 约 25% | 还缺支付生产接入、自动升级签名、三平台安装包、SLA、监控、隐私合规、安全隔离和大样本质量基准 |
| 企业级发布 | 约 15% | 还缺私有化部署、管理员控制台、SSO、审计、租户隔离、企业模板定制 |

建议不要直接公开收费发布。推荐路线是：

```text
Preview 可演示
  -> 邀请制 Beta 小范围真实用户验证
  -> Pro SaaS / Desktop 商业 GA
  -> Enterprise 私有化与模板定制
```

当前最关键的工程判断是：

```text
核心语义引擎已经有商业化价值，
但商业产品闭环、运行时安全、持续质量评估、计费与交付体系仍未生产化。
```

---

## 二、当前开发进展复盘

### 2.1 已具备的核心资产

| 资产 | 当前状态 | 商业价值 |
|---|---|---|
| V1 Rust 规则转换引擎 | 已有，且保持独立路径 | 可作为稳定 fallback 和对照组 |
| V2 Semantic TeX Engine | 已有独立路径 | 是未来商业化核心 |
| RuleBased / XeLaTeX Hook / LuaTeX Node 三路径 | 架构已建立 | 可覆盖离线、中文论文、长期语义采集 |
| 7 类 Journal Profile | 已有 | 可形成首期差异化卖点 |
| ActiveProfile 链路 | 已推进 | 支撑自动检测、profile-aware 渲染和质量门禁 |
| RuleEngine | 已接入期刊规则方向 | 可持续扩展模板宏兼容能力 |
| QualityGate | 已升级到 V2 方向 | 可作为商业成功/失败判断基础 |
| semantic CLI | 已产品化推进 | 可服务专业用户、CI 和桌面端 |
| paper3 三路径对比 | 已有脚本和结果 | 可做回归基准与演示案例 |

### 2.2 P5/P6/P7 新进展

| 阶段 | 当前状态 | 已完成 | 尚未生产化 |
|---|---|---|---|
| P5 Slint Desktop MVP | in_progress | Linux `cargo check` 通过；本地语义转换、profile/quality/report/job history 已接入 UI | 缺真实 GUI 操作验收、文件选择器、登录/用量/订阅、自动升级、三平台安装包 |
| P6 商业 API | in_progress | client 已覆盖 auth/usage/billing/uploads/conversions/releases；server 有 `/v1` 和 `/api/v1` 合约端点 | auth 是 demo token；usage/plans/billing 是固定数据；无 JWT、无数据库、无支付 provider |
| P7 云端 Worker | in_progress | 已有内存态 upload/job/queue/worker；可用 paper3 zip 产出真实 DOCX/report | worker 仍调用旧 `doc_core::convert_zip()`；无持久化、对象存储、sandbox、额度扣减、崩溃恢复 |
| P8 真实样本回归 | started | 每个 profile 已补 3 个 realistic `.tex` fixture | 缺 nightly regression 脚本、统计报告、失败样本库、真实 Word 打开验证 |
| P9 自动升级分发 | pending | server 有 release manifest 占位端点 | 缺 updater、sha256/签名校验、MSI/DMG/AppImage、代码签名、公证 |

### 2.3 当前不能商业发布的核心原因

1. **云端 worker 还不是语义引擎 worker**  
   P7 当前 worker 调用旧 `doc_core::convert_zip()`，不是 `doc_compiler_engine::SemanticTexEngine` 的云端执行器。商业宣传如果定位为“语义 TeX 引擎”，云端转换链路必须切换到语义引擎，旧引擎只能作为 fallback。

2. **真实用户任务没有生产隔离**  
   TeX 文档属于不可信输入，可能执行 shell escape、读文件、拉网络资源、消耗 CPU/内存。没有 sandbox 前不能开放公网转换服务。

3. **账号、订阅、用量仍是模拟合约**  
   API 端点形态已经有，但没有真实用户、JWT、用量扣减、支付回调、订阅状态同步和额度阻断。

4. **质量指标还不能支撑收费承诺**  
   当前有 QualityGate 和 fixtures，但还缺大样本成功率、DOCX 可打开率、版式偏差、语义覆盖率、公式/表格/图片质量指标。

5. **桌面端还不是商业客户端**  
   Slint 客户端已具备本地转换基础，但还缺登录、套餐、用量、云端转换、转换历史、错误诊断、自动升级与安装包交付。

---

## 三、商业化目标定义

### 3.1 首期商业产品形态

建议首期定义为：

```text
Tex2Doc Desktop + Cloud Preview
面向中文学术论文、SCI/CS 期刊论文的 TeX -> DOCX 高保真转换工具
```

不要一开始承诺“通用 TeX 全覆盖”。首期商业边界应明确为：

| 范围 | 首期承诺 |
|---|---|
| 文档类型 | 学术论文、期刊投稿、中文 CTeX、SCI/CS 模板 |
| 支持 profile | generic、chinese-academic、jos-paper、tacl、cvpr、nature、springer |
| 转换路径 | 本地语义转换 + 云端高兼容转换 |
| 输出格式 | DOCX + JSON/HTML 诊断报告 |
| 计费维度 | 本地免费/基础额度；云端转换按次数或套餐 |
| 用户群 | 论文作者、科研团队、医学论文编辑、投稿服务机构 |

### 3.2 商业成功标准

商业化不是“能生成 DOCX”，而是满足以下标准：

| 指标 | Preview | Beta | GA |
|---|---:|---:|---:|
| 支持 profile | 7 | 7+ | 10+ |
| realistic fixture / profile | 3 | 10 | 30+ |
| DOCX 可打开率 | 95% | 98% | 99% |
| 无崩溃转换率 | 95% | 98% | 99.5% |
| profile 自动识别准确率 | 90% | 95% | 97% |
| 引用/图片缺失可诊断率 | 90% | 95% | 98% |
| P95 云端转换耗时 | 不承诺 | < 120s | < 60s |
| 客户端三平台安装 | Linux dev | Win/mac/Linux beta | Win/mac/Linux 签名发布 |
| 支付/用量闭环 | 无 | 沙箱/测试模式 | 生产模式 |

---

## 四、目标技术架构

### 4.1 双产品入口

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

### 4.2 统一转换执行器

必须把 P7 worker 从旧 `doc_core` 调用抽象出来：

```rust
#[async_trait]
pub trait ConversionExecutor {
    async fn execute(&self, input: ConversionInput) -> Result<ConversionOutput>;
}
```

建议实现：

| Executor | 用途 |
|---|---|
| `LocalSemanticExecutor` | 桌面端本地转换，调用 `SemanticTexEngine` |
| `CloudSemanticExecutor` | 云端 worker 主路径，调用 `SemanticTexEngine` |
| `LegacyRuleExecutor` | 旧 `doc_core` fallback 和回归对照 |
| `SandboxedTexExecutor` | 生产云端转换，封装容器/进程隔离 |

云端主流程应调整为：

```text
upload zip
  -> normalize project
  -> detect profile
  -> compatibility analyze
  -> execute semantic engine
  -> quality gate
  -> store DOCX/report
  -> meter usage
```

### 4.3 商业 SaaS 架构

```text
API Gateway / Axum
  -> Auth Middleware
  -> Rate Limit / Body Limit
  -> User / Subscription Service
  -> Upload Service
  -> Conversion Service
  -> Usage Metering
  -> Billing Webhook
  -> Release Service

Worker
  -> Queue Consumer
  -> Sandbox Executor
  -> SemanticTexEngine
  -> QualityGate
  -> Artifact Store

Storage
  -> PostgreSQL: users/jobs/subscriptions/usage
  -> Object Store: uploads/docx/reports/logs
  -> Redis: queue/rate-limit/cache
```

---

## 五、必须补齐的商业化工作

### 5.1 引擎生产化

| 工作 | 当前状态 | 目标实现 |
|---|---|---|
| 云端 worker 切语义引擎 | 当前用旧 `doc_core` | P7 worker 改为 `SemanticTexEngine` 主路径，旧引擎 fallback |
| XeLaTeX/LuaTeX runtime 检测 | 架构已有 | 检测 runtime 可用性、版本、字体和 package |
| backend fallback 报告 | 初步存在 | 报告必须明确 fallback 原因和质量影响 |
| profile-specific 渲染 | 已接入 style_map | 每个 profile 补齐 style coverage 和 Word 样式验证 |
| 公式/表格/图片指标 | 部分已有 | 形成公式 OMML 成功率、表格结构成功率、图片完整率 |

验收标准：

```text
semantic engine 云端路径和桌面路径输出同结构 report。
旧 doc_core 只能作为 fallback，并在 report 中显式标记。
```

### 5.2 质量体系生产化

新增 `QualityGate V3`：

```rust
pub struct CommercialQualityReport {
    pub status: QualityStatus,
    pub score: u8,
    pub profile: String,
    pub backend: String,
    pub docx_openable: bool,
    pub semantic_coverage: f32,
    pub layout_confidence: f32,
    pub formula_success_rate: f32,
    pub table_success_rate: f32,
    pub image_success_rate: f32,
    pub unresolved_references: usize,
    pub raw_fallback_blocks: usize,
    pub blocking_issues: Vec<QualityIssue>,
    pub warnings: Vec<QualityIssue>,
}
```

质量检查分层：

| 层级 | 检查 |
|---|---|
| ZIP 结构 | `[Content_Types].xml`、`word/document.xml`、rels、styles |
| Word 可打开性 | LibreOffice headless 打开/另存验证 |
| 语义覆盖 | heading/paragraph/table/figure/equation/reference 识别率 |
| 资源完整 | image missing、bib missing、include missing |
| 样式覆盖 | style_map role 是否全部映射到 DOCX style |
| 版式风险 | overfull、fallback、runtime fallback、未知宏 |

### 5.3 桌面端商业闭环

P5 需要升级为 P5-GA：

| 模块 | 必需能力 |
|---|---|
| 文件选择 | 选择 TeX 项目目录、main tex、输出目录 |
| 本地转换 | profile/quality/backend 选择，后台任务，取消任务 |
| 云端转换 | 上传 zip，创建 job，轮询状态，下载 DOCX/report |
| 登录 | email/password、token refresh、安全存储 token |
| 用量 | 当前套餐、剩余额度、历史消耗 |
| 订阅 | 打开 checkout / billing portal |
| 报告 | 质量分、失败原因、profile 检测、backend fallback |
| 历史 | 最近任务、输出路径、报告路径、失败日志 |
| 更新 | release manifest、sha256、签名校验、平台安装器 |

推荐状态模型：

```rust
pub struct DesktopAppState {
    pub auth: AuthState,
    pub usage: UsageState,
    pub settings: Settings,
    pub jobs: JobStore,
    pub updater: UpdateState,
}
```

### 5.4 账号、订阅与用量

当前 P6 是合约端点，需升级为真实服务：

数据库表建议：

```sql
users(id, email, password_hash, display_name, created_at, disabled_at)
sessions(id, user_id, refresh_token_hash, device_id, expires_at)
plans(id, name, monthly_conversions, storage_bytes, price_cents, active)
subscriptions(id, user_id, plan_id, provider, provider_subscription_id, status, current_period_end)
usage_events(id, user_id, job_id, event_type, units, created_at)
uploads(id, user_id, object_key, file_name, bytes, sha256, created_at, expires_at)
conversion_jobs(id, user_id, upload_id, profile, quality, backend, status, created_at, updated_at)
conversion_artifacts(id, job_id, kind, object_key, bytes, sha256, created_at)
release_manifests(id, channel, platform, version, url, sha256, signature, created_at)
```

关键规则：

- 创建 job 前检查额度。
- job 成功后扣减额度。
- 因系统错误失败不扣减。
- 因用户输入错误可记录但不扣减或按低成本策略扣减。
- 所有用量事件必须可审计、可回滚。

### 5.5 云端安全与 sandbox

TeX 是高风险输入，必须做隔离。

首期建议：

```text
Docker / containerd sandbox
  readonly image
  network disabled
  per-job workdir
  cpu limit
  memory limit
  disk quota
  process count limit
  timeout kill
  no shell escape by default
```

后续增强：

```text
gVisor / Firecracker
  stronger kernel isolation
  tenant isolation
  audit log
```

worker 产物策略：

- 原始 zip、编译中间文件、DOCX、report 分开存储。
- 临时目录 job 完成后清理。
- 默认 7 到 30 天保留。
- 企业版允许自定义保留策略。

### 5.6 自动升级与交付

P9 必须包含：

| 平台 | 安装包 | 签名要求 |
|---|---|---|
| Windows | MSI 或 NSIS | Authenticode |
| macOS | DMG / PKG | Developer ID + notarization |
| Linux | AppImage / deb / rpm | sha256 + optional GPG |

release manifest：

```json
{
  "channel": "stable",
  "platform": "windows-x86_64",
  "version": "0.3.0",
  "download_url": "https://downloads.tex2doc.cn/desktop/0.3.0/Tex2Doc.msi",
  "sha256": "...",
  "signature": "...",
  "min_supported_version": "0.2.0",
  "release_notes": "..."
}
```

客户端必须执行：

```text
manifest 签名校验
  -> 版本比较
  -> 下载
  -> sha256 校验
  -> 平台安装器启动
```

---

## 六、阶段实施计划

### C0：商业化基线冻结（2 到 3 天）

目标：

```text
明确“什么叫转换成功”，冻结接口 schema、质量报告 schema 和首期 profile 范围。
```

任务：

1. 冻结 `ConversionReport` / `QualityGate` JSON schema。
2. 定义 Preview/Beta/GA 三档发布门禁。
3. 明确旧引擎与语义引擎的产品定位：旧引擎只做 fallback。
4. 更新 `docs-zh/plan-0621.md`，区分 `in_progress`、`MVP done`、`production pending`。

验收：

- 所有后续开发都有统一验收口径。
- CLI、desktop、server 三方使用同一 report schema。

### C1：Preview 可交付（1 到 2 周）

目标：

```text
可演示、可复现、可对外邀请少量技术用户试用，但不收费或只做免费 preview。
```

任务：

1. 完成 P8 nightly regression 脚本。
2. 每个 profile 跑 `minimal + 3 realistic`。
3. 输出 `conversion_stats.json` 和 `conversion_stats.md`。
4. Slint 桌面端完成 Linux GUI 操作验收。
5. 云端 worker 改为支持 semantic engine 执行器选项。
6. P9 实现 release manifest 解析与 sha256 校验骨架。
7. `commercial_verify.sh` 读取 compile report，不只检查 zip 结构。

验收：

```bash
cargo test
./scripts/verify_journal_profiles.sh --all
./scripts/nightly_regression.sh
cargo check -p doc-desktop-slint
cargo test -p doc-server --test api -- --nocapture
```

Preview 放行条件：

- 7 profiles 全部可转换。
- DOCX zip 结构通过率 >= 95%。
- 未出现 panic。
- 失败样本必须有 report，而不是静默失败。

### C2：邀请制 Beta（3 到 6 周）

目标：

```text
真实用户可注册、登录、消耗额度、上传项目、云端转换、下载结果。
```

任务：

1. 引入 PostgreSQL，替换内存 `HashMap`。
2. 引入对象存储 trait，并提供本地 FS + S3/MinIO 实现。
3. 实现 JWT access/refresh token。
4. 实现 usage metering 和额度阻断。
5. worker 使用 sandbox 执行器。
6. 桌面端接入登录、用量、云端转换。
7. release manifest 支持 beta channel。
8. 建立真实用户样本文档回收机制。

验收：

- 至少 20 个真实项目试用。
- Beta 用户任务可追踪、可恢复、可下载。
- 云端转换任务失败不影响 API 进程。
- worker 重启后 queued/running 任务有明确恢复策略。

### C3：商业 GA（8 到 12 周）

目标：

```text
可正式收费，支持三平台桌面端，具备基础 SLA 和运维体系。
```

任务：

1. Stripe 或等价支付 provider 生产接入。
2. Billing webhook 同步 subscription 状态。
3. Windows/macOS/Linux 安装包构建流水线。
4. 代码签名、公证、sha256、manifest 签名。
5. 监控：API latency、worker queue depth、success rate、panic count。
6. 告警：job failure spike、storage error、payment webhook failure。
7. 数据保留和删除策略。
8. 用户协议、隐私政策、论文数据处理说明。
9. 支持工单和故障诊断包导出。

GA 放行指标：

| 指标 | 阈值 |
|---|---:|
| DOCX 可打开率 | >= 99% |
| 无崩溃转换率 | >= 99.5% |
| profile 识别准确率 | >= 97% |
| P95 云端转换耗时 | <= 60s |
| 任务状态一致性 | 100% |
| 用量扣减可审计 | 100% |

### C4：企业级增强（3 到 6 个月）

目标：

```text
支持高校、期刊服务商、医学编辑机构的私有化部署和模板定制。
```

任务：

1. 多租户隔离。
2. SSO/SAML/OIDC。
3. 管理员控制台。
4. 审计日志。
5. 自定义 journal profile 上传与验证。
6. 私有模型/本地 LLM 宏推断插件。
7. 私有对象存储和离线授权。

---

## 七、优先级清单

### 最高优先级

1. **云端 worker 切入语义引擎主路径**  
   商业定位和技术实现必须一致。

2. **sandbox 执行器**  
   不解决 sandbox，不能开放公网 TeX 转换。

3. **QualityGate V3 与 nightly regression**  
   没有量化质量，就不能承诺商业效果。

4. **账号、用量、持久化任务表**  
   没有用户和任务状态，就没有 SaaS 产品闭环。

5. **桌面端登录 + 云端转换闭环**  
   这是用户实际付费入口。

### 中优先级

1. 自动升级和三平台打包。
2. 支付 provider 生产接入。
3. 对象存储和下载签名 URL。
4. 真实样本库扩展到每 profile 10+。
5. profile style coverage 自动检查。

### 后置但必须规划

1. 企业私有化部署。
2. SSO 和审计。
3. 模板 marketplace。
4. AI 宏推断与人工审核闭环。

---

## 八、风险与缓解策略

| 风险 | 级别 | 原因 | 缓解 |
|---|---|---|---|
| TeX runtime 安全风险 | 高 | 用户输入可触发文件/进程/资源风险 | sandbox、禁网、限时、限资源 |
| 转换质量不稳定 | 高 | LaTeX 宏生态复杂 | profile 分层、质量门禁、失败样本库 |
| 商业承诺过宽 | 高 | 通用 TeX 很难一次做好 | 首期聚焦中文学术和期刊模板 |
| 桌面端跨平台成本 | 中 | Slint + runtime + installer 差异 | 先 Linux/mac/Windows 冒烟，再签名发布 |
| 支付/订阅状态不一致 | 中 | webhook 与本地状态存在竞态 | subscription 状态以 provider webhook 为准，usage event 可审计 |
| 云端成本失控 | 中 | TeX 编译 CPU/内存消耗大 | quota、rate limit、timeout、队列优先级 |
| 数据隐私风险 | 高 | 用户论文可能未发表 | 明确保留策略、加密存储、用户主动删除、企业私有化选项 |

---

## 九、下一步具体开发安排

建议立即按以下顺序推进：

1. **完成 P8 nightly regression**
   - 新增 `scripts/nightly_regression.sh`。
   - 跑 7 profiles x 4 fixtures。
   - 输出 `conversion_stats.json` / `conversion_stats.md`。

2. **完成 P9 updater 骨架**
   - desktop 解析 release manifest。
   - sha256 校验。
   - 签名校验接口预留。
   - server manifest 从占位升级为可配置。

3. **P7 worker 引入 `ConversionExecutor`**
   - 先保留旧 `doc_core` executor。
   - 新增 semantic executor。
   - report 标注 executor/backend。

4. **P6 引入持久化设计**
   - 先实现 trait 和 repository 层。
   - 后接 PostgreSQL。

5. **P5 桌面端接云端 API**
   - 登录。
   - 用量。
   - 上传 zip。
   - 创建 conversion。
   - 轮询并下载。

6. **建立 Preview 发布门禁**
   - 每次发布必须通过 cargo test、profile verification、nightly regression、server API test、desktop check。

---

## 十、商业化发布建议

短期不要直接收费发布。建议采用：

```text
第 1 阶段：技术 Preview
  免费，面向内部和熟悉 LaTeX 的试用用户

第 2 阶段：邀请制 Beta
  小范围真实论文试用，收集失败样本，验证云端成本

第 3 阶段：Pro 订阅
  收费开放云端高质量转换、批量转换、质量报告

第 4 阶段：Enterprise
  私有化部署、期刊模板定制、机构用量管理
```

当前最合理的商业化落点是：

```text
中文学术论文 + SCI/CS 期刊模板 + DOCX 投稿转换
```

不建议首期宣传为：

```text
通用 TeX 到 Word 全自动高保真转换
```

因为这会放大宏兼容、版式还原和用户预期风险。

---

## 十一、验收清单

### Preview 验收

- [ ] 7 profiles x 4 fixtures nightly 全部有结果报告。
- [ ] `paper3` 三路径对比脚本稳定输出。
- [ ] Slint 桌面端至少 Linux GUI 手动转换通过。
- [ ] server API 集成测试通过。
- [ ] 所有失败都有结构化 report。

### Beta 验收

- [ ] 用户注册/登录/token refresh 可用。
- [ ] 上传、转换、下载、报告 API 使用数据库和对象存储。
- [ ] worker 在 sandbox 中执行。
- [ ] 用量扣减可审计。
- [ ] 桌面端支持云端转换闭环。
- [ ] 至少 20 个真实项目样本通过回归。

### GA 验收

- [ ] 支付生产接入。
- [ ] Windows/macOS/Linux 安装包可下载、可签名验证。
- [ ] 自动升级链路可用。
- [ ] 监控告警可用。
- [ ] 数据保留/删除/隐私文档完成。
- [ ] DOCX 可打开率和转换稳定性达到 GA 指标。

---

## 十二、最终判断

截至本评估，项目具备商业化的技术基础，但还处于“商业产品工程化前夜”。

可以对外展示的能力：

- 语义引擎架构。
- 7 profiles 泛化。
- paper3 三路径对比。
- CLI 转换。
- 桌面端本地转换 MVP。
- 云端转换 API/worker MVP。

尚不能商业收费承诺的能力：

- 生产级账号订阅。
- 安全云端 TeX 执行。
- 大样本质量稳定性。
- 三平台安装与自动升级。
- SLA 与数据隐私合规。

推荐下一步目标不是“马上 GA”，而是：

```text
在 1 到 2 周内完成 Preview 发布门禁，
在 3 到 6 周内完成邀请制 Beta 产品闭环，
在 8 到 12 周内冲刺商业 GA。
```
