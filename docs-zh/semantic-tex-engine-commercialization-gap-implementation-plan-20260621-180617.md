# Semantic TeX Engine 商业化差距评估与实施方案

**方案版本**：20260621-180617  
**基准文档**：`docs-zh/semantic-tex-engine-progress-report-20260621-180000.md`  
**目标**：评估当前项目距离商业化发布的剩余工作，并提出可执行技术实施方案  
**结论**：当前已达到技术预览基础，但距离邀请制 Beta 和正式商业 GA 仍需补齐产品闭环、质量门禁、账号订阅、端到端验证和部署运维能力  

---

## 一、总体判断

结合 2026-06-21 18:00 开发进展，项目已经从“语义引擎原型”推进到“商业化基础设施初具雏形”的阶段。

已经具备：

- 3 条编译路径：RuleBased、XeLaTeX Hook、LuaTeX Node。
- 7 类期刊 Profile：generic、chinese-academic、jos-paper、tacl、cvpr、nature、springer。
- JournalDetector、ProfileRegistry、CompatibilityAnalyzer、RuleEngine。
- ProfileStyleMap 初步接入 DOCX writer。
- QualityGateResult 初步接入编译报告。
- `semantic-detect`、`semantic-analyze`、`semantic-convert` CLI 入口。
- `commercial_verify.sh` 质量脚本。
- `doc-commercial-api-client` 骨架。
- `doc-desktop-slint` Slint 桌面 UI 骨架。

但当前仍不建议直接公开收费发布。准确定位应为：

| 发布级别 | 当前状态 | 结论 |
|---|---|---|
| 技术预览 Preview | 接近可用 | 还需修复门禁脚本和 CLI 语义 profile 链路 |
| 邀请制 Beta | 尚未达到 | 缺账号、订阅、用量、云端任务、真实样本回归 |
| 正式商业 GA | 明显不足 | 缺支付、SLA、安全沙箱、自动升级、三平台签名分发 |

推荐下一阶段目标：

```text
先把当前工程推进到 Preview 可交付
再建设 Beta 所需的账号/订阅/云端转换/桌面端闭环
最后补齐 GA 级别的部署、监控、安全与运维
```

---

## 二、已完成能力与可商业化价值

### 2.1 语义引擎核心

| 能力 | 当前状态 | 商业价值 |
|---|---|---|
| `SemanticTexEngine` | 已有 | 可作为商业转换内核 |
| RuleBased 后端 | 已有 | 无 TeX runtime 的快速转换 |
| XeLaTeX Hook 后端 | 已有 | 中文/CTeX/XeLaTeX 生态兼容 |
| LuaTeX Node 后端 | 已有 | 长期高泛化语义采集基础 |
| ReferenceGraph | 已有 | 引用/交叉引用可审计 |
| LayoutGraph/XDV | 原型已有 | 后续高保真版式校准 |

### 2.2 期刊泛化

| 能力 | 当前状态 | 商业价值 |
|---|---|---|
| 7 Profile TOML | 已有 | 首期可宣传多模板支持 |
| JournalDetector | 已有 | 用户无需手动选模板 |
| Profile-aware backend selector | 初步已有 | 可按模板选择 runtime |
| CompatibilityAnalyzer | 已有 | 转换前给出风险评分 |
| RuleEngine 期刊规则 | 已定义 | 可处理模板特定宏 |

### 2.3 商业化入口

| 能力 | 当前状态 | 商业价值 |
|---|---|---|
| 语义 CLI | 已有入口 | 专业用户和 CI 可用 |
| Slint 桌面端 | UI 骨架 | PC 客户端商业入口 |
| API client | DOCX analysis 骨架 | 云服务 SDK 起点 |
| QualityGate | 初步已有 | 可生成转换质量判断 |
| commercial verify | 初步脚本 | CI 门禁雏形 |

---

## 三、关键商业化缺口

## 3.1 ActiveProfile 全链路仍未闭合

当前 CLI 和编译选项仍大量依赖 `EngineProfile` enum。该 enum 主要覆盖：

```text
generic-article
chinese-academic
jos-paper
medical-journal
```

但商业化首期要求：

```text
generic
chinese-academic
jos-paper
tacl
cvpr
nature
springer
```

当前问题：

- `semantic-convert --profile tacl` 被映射到 `EngineProfile::JosPaper`。
- `semantic-convert --profile cvpr` 也被映射到 JOS。
- `nature/springer` 被映射到 GenericArticle。
- JournalDetector 检测结果没有完整驱动 compatibility、RuleEngine、style_map、quality。
- Profile TOML 中的 `macro_rules`、`quality`、`style_map` 没有全链路生效。

影响：

```text
用户以为选择了 TACL/CVPR/Nature/Springer，但实际编译策略并不完整匹配。
这在商业化中属于高风险问题。
```

## 3.2 RuleEngine 期刊规则尚未真正进入编译主链路

虽然 `journal_rules(profile_id)` 已存在，但当前 `apply_rule_engine_to_document()` 仍是：

```rust
let mut engine = RuleEngine::new();
```

缺少：

- 按 ActiveProfile 加载 `journal_rules(profile_id)`。
- 按 Profile TOML 加载 `macro_rules`。
- 将 RuleEngine 决策写入 CompileReport。
- 对未知宏形成可计数、可审计、可展示的 fallback 报告。

影响：

```text
期刊宏规则“定义了”，但对真实转换结果影响有限。
```

## 3.3 QualityGate 仍是技术门禁，不是商业门禁

当前 QualityGate 检查项主要是：

- compatibility score。
- unresolved references。
- OMML fallback。
- docx bytes。

缺少商业化必需项：

- Profile TOML 的 `quality.min_compatibility_score`。
- `max_raw_fallback_blocks`。
- missing image 统计。
- DOCX zip 深度结构校验。
- DOCX 可由 Word/LibreOffice 打开验证。
- style_map 覆盖率。
- profile detection confidence 阻断策略。
- runtime fallback 风险等级。
- 用户可读质量说明。

影响：

```text
当前 QualityGate 可用于开发阶段 sanity check，
但不能作为“可收费转换成功”的标准。
```

## 3.4 CLI 尚未达到产品级

已有 CLI 子命令：

```text
semantic-detect
semantic-analyze
semantic-convert
semantic-verify
```

问题：

- `semantic-verify` 仍是 not implemented。
- `semantic-convert --profile auto` 实际走 GenericArticle，而不是完整 ActiveProfile。
- profile 解析存在 tacl/cvpr/nature/springer 映射降级。
- 缺少统一 JSON output schema。
- 缺少机器可读错误码。
- 缺少商业任务报告格式。

## 3.5 `commercial_verify.sh` 仍是浅层脚本

当前脚本只做：

- 文件大小。
- `word/styles.xml` 存在。
- `word/document.xml` 存在。
- `word/_rels/document.xml.rels` 存在。

问题：

- 没有读取 `CompileReport`。
- 没有检查 QualityGateResult。
- 没有检查 Profile。
- 没有检查 raw fallback。
- 没有检查 unresolved references。
- 没有检查 image missing。
- 没有检查实际 style 覆盖。
- Bash 中存在顶层 `local` 用法风险，应修复。

## 3.6 Slint 桌面端仍是 UI 骨架

当前 `doc-desktop-slint` 只有：

- `MainWindow`。
- 项目路径输入。
- Detect Profile 按钮。
- status 文本。

缺少：

- 文件/目录选择。
- 本地转换调用。
- 云端转换调用。
- 账号登录。
- token/keychain。
- 用量显示。
- 套餐订阅入口。
- 自动升级。
- 转换历史。
- 报告展示。
- 后台任务状态。

结论：

```text
当前 Slint 端可作为 UI feasibility proof，
不能作为商业 PC 客户端。
```

## 3.7 商业 API Client 仅覆盖质量分析

当前 `doc-commercial-api-client` 主要支持：

- `submit_analysis(docx)`。
- `get_analysis_result(job_id)`。

缺少：

- 注册/登录。
- token refresh。
- 设备激活。
- plans/usage。
- checkout/portal。
- 上传项目。
- 创建 conversion job。
- 下载 DOCX/report。
- 删除任务。
- release/update manifest。

## 3.8 服务端仍不是商业 SaaS 后端

当前 `crates/server` 是 MVP：

- `/api/v1/health`
- `/api/v1/version`
- `/api/v1/convert`

并且主要调用旧 `doc_core::convert_zip`。

商业化缺口：

- 没有认证。
- 没有异步任务队列。
- 没有对象存储。
- 没有 usage/billing。
- 没有云端 semantic engine worker。
- 没有 TeX runtime sandbox。
- 没有转换报告 API。
- 没有删除/保留策略。

## 3.9 真实样本和质量基准不足

当前已有 minimal fixture 和 paper3 回归，但商业化还需要：

- 每个 Profile 至少 10 篇 realistic fixture。
- 隐私脱敏样本。
- 失败样本库。
- nightly regression。
- 转换成功率统计。
- 用户真实文档闭环。

---

## 四、商业化目标架构

### 4.1 产品架构

```text
Desktop / CLI / Web
  ↓
Project Normalizer
  ↓
JournalDetector
  ↓
ActiveProfile
  ↓
CompatibilityAnalyzer
  ↓
Semantic Engine
  ├─ RuleBased
  ├─ XeLaTeXHook
  └─ LuaTeXNode
  ↓
Profile-aware RuleEngine
  ↓
Profile-aware DOCX Renderer
  ↓
QualityGate V2
  ↓
DOCX + Report + Diagnostics
```

### 4.2 SaaS 架构

```text
Client
  ↓
API Gateway
  ↓
Auth / Billing / Usage
  ↓
Upload Service
  ↓
Conversion Job Queue
  ↓
Worker Pool
  ├─ rule-worker
  ├─ tex-worker
  ├─ quality-worker
  └─ report-worker
  ↓
Object Storage
  ↓
Result API
```

### 4.3 桌面端架构

```text
Slint UI
  ↓
Desktop App State
  ├─ Auth State
  ├─ Usage State
  ├─ Job State
  └─ Update State
  ↓
Local Engine Adapter
  ├─ SemanticTexEngine
  └─ doc-core fallback
  ↓
Commercial API Client
  ├─ auth
  ├─ usage
  ├─ conversion jobs
  ├─ billing portal
  └─ release update
```

---

## 五、设计实施方案

## P0：Preview 发布门禁修复

周期：3 到 5 天

目标：

```text
让当前技术预览版可稳定演示、可验证、可复现。
```

任务：

1. 修复 `commercial_verify.sh` 顶层 `local` 风险。
2. 修复 `verify_journal_profiles.sh`，确保使用 PATH 中的 `cargo`。
3. `semantic-verify` 实现基础 DOCX 结构验证。
4. `semantic-convert --profile auto` 输出 detected profile。
5. CLI 报告统一输出 JSON。
6. 7 个 minimal fixture 通过 CLI 端到端生成 DOCX。

验收：

```bash
cargo test -p doc-compiler-engine
cargo test -p doc-docx-writer
cargo test -p doc-compatibility-analyzer
cargo test -p doc-rule-engine
./scripts/verify_journal_profiles.sh --all
./scripts/commercial_verify.sh --docx examples/journals/output/tacl.docx
```

退出码必须可靠。

---

## P1：ActiveProfile 全链路重构

周期：1 到 2 周

目标：

```text
让 auto/tacl/cvpr/nature/springer 真正成为编译上下文，而不是检测标签。
```

新增类型：

```rust
pub struct ActiveProfile {
    pub id: String,
    pub spec: ProfileSpecFile,
    pub source: ProfileSource,
    pub detection: Option<JournalDetectionReport>,
}

pub enum ProfileSource {
    ExplicitId,
    ExplicitPath,
    AutoDetected,
    Fallback,
}
```

改造 `CompileOptions`：

```rust
pub struct CompileOptions {
    pub profile_ref: ProfileRef,
    pub semantic_backend: SemanticBackendKind,
    pub allow_backend_fallback: bool,
    ...
}

pub enum ProfileRef {
    Auto,
    Id(String),
    Path(PathBuf),
    Legacy(EngineProfile),
}
```

改造流程：

```text
compile_vfs_to_graph
  ↓
resolve_active_profile(options, vfs)
  ↓
CompatibilityAnalyzer(active_profile)
  ↓
select_backend(active_profile)
  ↓
RuleEngine(active_profile)
  ↓
DOCX Renderer(active_profile)
  ↓
QualityGate(active_profile)
```

验收：

```bash
doc-engine semantic-convert --profile tacl ...
doc-engine semantic-convert --profile cvpr ...
doc-engine semantic-convert --profile nature ...
doc-engine semantic-convert --profile springer ...
doc-engine semantic-convert --profile auto ...
```

报告中必须显示：

```json
{
  "active_profile": {
    "id": "tacl",
    "source": "auto_detected",
    "confidence": 0.95
  }
}
```

---

## P2：Profile-aware RuleEngine 接入

周期：1 周

目标：

```text
把期刊宏规则从“定义存在”变成“转换生效”。
```

实现：

```rust
fn build_rule_engine(active_profile: &ActiveProfile) -> RuleEngine {
    let mut engine = RuleEngine::new();
    for rule in journal_rules(&active_profile.id) {
        engine.registry_mut().register(rule);
    }
    for rule in toml_macro_rules(&active_profile.spec.macro_rules) {
        engine.registry_mut().register(rule);
    }
    engine
}
```

改造：

```rust
apply_rule_engine_to_document(document, active_profile, report)
```

报告新增：

```rust
pub struct RuleEngineReport {
    pub builtin_rules: usize,
    pub journal_rules: usize,
    pub profile_rules: usize,
    pub unknown_macros: usize,
    pub fallback_macros: Vec<String>,
}
```

验收：

- TACL：`\citet`、`\citep` 不再作为普通未知宏。
- JOS：`\IEEEkeywords` 进入 keywords/metadata。
- Springer：`\institute` 进入 affiliation。
- Chinese：`\keywords`、`\zhabstract` 进入 metadata。

---

## P3：QualityGate V2

周期：1 到 2 周

目标：

```text
把质量门禁从开发 sanity check 升级为商业成功/失败判断。
```

新增检查：

| 检查 | 来源 | 默认等级 |
|---|---|---|
| compatibility score | CompatibilityReport | Error |
| unresolved references | ReferenceGraph | Warning/Error 可配置 |
| raw fallback count | Document/Report | Error |
| missing images | ImageAssets | Error |
| docx zip structure | DOCX bytes | Error |
| styles coverage | style_map + document.xml | Warning |
| profile detection confidence | JournalDetector | Warning |
| runtime fallback | BackendReport | Warning |
| OMML fallback ratio | Formula report | Warning |
| DOCX openable | LibreOffice/zip check | Error |

Profile TOML：

```toml
[quality]
min_compatibility_score = 80
max_raw_fallback_blocks = 5
max_unresolved_references = 0
allow_runtime_fallback = true
require_docx_openable = true
```

QualityGateResult V2：

```rust
pub struct QualityGateResult {
    pub status: QualityStatus,
    pub score: u8,
    pub checks: Vec<QualityCheck>,
    pub blocking_issues: Vec<QualityIssue>,
    pub warnings: Vec<QualityIssue>,
}

pub enum QualityStatus {
    Passed,
    PassedWithWarnings,
    Failed,
}
```

验收：

- 质量失败时 CLI 返回非 0 或明确 `status=failed`。
- 报告可解释失败原因。
- commercial verify 读取 report.json，而不是只读 DOCX zip。

---

## P4：CLI 产品化

周期：1 周

目标：

```text
把 CLI 做成专业用户、CI 和桌面端可复用的稳定接口。
```

命令保持：

```text
semantic-detect
semantic-analyze
semantic-convert
semantic-verify
```

补齐：

- 所有命令支持 `--json`。
- 所有失败输出标准错误码。
- `semantic-convert` 支持 `--profile auto|id|path`。
- `semantic-convert` 支持 `--quality strict|standard|preview`。
- `semantic-verify` 真正实现。
- `--report` JSON schema 固定版本。

错误码：

```text
E_INPUT_INVALID
E_MAIN_TEX_MISSING
E_PROFILE_LOW_CONFIDENCE
E_COMPAT_UNSUPPORTED
E_CONVERT_FAILED
E_DOCX_INVALID
E_QUALITY_FAILED
```

验收：

```bash
doc-engine semantic-detect --project-root examples/journals/tacl --json
doc-engine semantic-convert --project-root examples/journals/tacl --main-tex minimal.tex --profile auto --out out.docx --report out.json
doc-engine semantic-verify --docx-file out.docx --report verify.json
```

---

## P5：Slint Desktop MVP

周期：2 到 3 周

目标：

```text
把 doc-desktop-slint 从 UI 骨架推进到可试用 PC 客户端。
```

功能：

1. 选择 `.tex` / `.zip` / 项目目录。
2. 自动识别 main tex。
3. 调用 `semantic-detect` 或直接调用 `JournalDetector`。
4. 展示 Profile、compatibility score、backend。
5. 调用 `SemanticTexEngine` 本地转换。
6. 输出 DOCX。
7. 展示 QualityGate。
8. 打开输出目录。
9. 保存最近任务。

工程模块：

```text
desktop-slint/src/
├── app_state.rs
├── commands.rs
├── local_convert.rs
├── job.rs
├── report.rs
└── settings.rs
```

验收：

- Windows/Linux 至少一平台能运行。
- UI 不阻塞。
- 可转换 `examples/journals/generic`。
- 可展示 report。

---

## P6：账号、订阅、用量 API

周期：3 到 4 周

目标：

```text
让商业 API client 从“质量分析 client”升级为商业账户 client。
```

新增 API：

```http
POST /v1/auth/register
POST /v1/auth/login
POST /v1/auth/refresh
GET  /v1/me
GET  /v1/usage
GET  /v1/plans
POST /v1/billing/checkout
POST /v1/billing/portal
POST /v1/uploads
POST /v1/conversions
GET  /v1/conversions/{id}
GET  /v1/conversions/{id}/download/docx
GET  /v1/conversions/{id}/report
```

客户端模块：

```text
auth.rs
usage.rs
billing.rs
conversions.rs
uploads.rs
releases.rs
```

验收：

- Slint 客户端可登录。
- 可显示套餐和用量。
- 可打开订阅页面。
- 额度不足时阻断云端转换。

---

## P7：云端转换 SaaS Worker

周期：4 到 6 周

目标：

```text
提供可收费的云端高质量转换能力。
```

服务端改造：

```text
api-gateway
auth-service
billing-service
upload-service
conversion-service
worker-service
quality-service
report-service
```

首期可用单体 Axum 实现，但内部按模块拆分。

任务状态：

```text
queued
normalizing
detecting
analyzing
compiling
rendering
verifying
succeeded
failed
expired
```

Worker sandbox：

- 禁止网络。
- 禁止 shell escape。
- CPU/memory/disk/time 限制。
- 每 job 独立目录。
- TeXLive runtime health check。

验收：

- 客户端上传项目。
- 云端异步转换。
- 下载 DOCX/report。
- 失败有错误码和诊断。

---

## P8：真实样本回归与质量指标

周期：持续，首期 2 到 4 周

目标：

```text
用真实论文证明产品能力，而不是只证明 minimal fixture。
```

样本矩阵：

| Profile | Preview | Beta | GA |
|---|---:|---:|---:|
| generic | 3 | 10 | 30 |
| chinese-academic | 3 | 10 | 30 |
| jos-paper | 3 | 10 | 30 |
| tacl | 3 | 10 | 30 |
| cvpr | 3 | 10 | 30 |
| nature | 3 | 10 | 30 |
| springer | 3 | 10 | 30 |

指标：

| 指标 | Preview | Beta | GA |
|---|---:|---:|---:|
| DOCX openable | 100% | 100% | 100% |
| profile detection accuracy | 95% | 98% | 99% |
| conversion success | 80% | 90% | 97% |
| unhandled panic | 0 | 0 | 0 |
| quality report generated | 100% | 100% | 100% |

---

## P9：自动升级与三平台分发

周期：3 到 5 周

目标：

```text
让 PC 客户端具备商业分发能力。
```

分发：

| 平台 | 包 |
|---|---|
| Windows | MSI/MSIX/NSIS |
| macOS | .app + DMG + notarization |
| Linux | AppImage + deb |

自动升级：

```text
release manifest
sha256
签名校验
stable/beta channel
回滚策略
```

验收：

- 三平台安装包可安装。
- 客户端可检查更新。
- 签名失败拒绝更新。

---

## 六、商业化实施路线图

### 阶段 A：Preview 可交付

周期：2 周

交付：

- P0、P1、P2、P4。
- CLI 可稳定转换 7 Profile minimal fixture。
- report.json 和 QualityGate V2 可用。
- Slint 可本地转换 1 到 2 个 fixture。

发布对象：

- 内部用户。
- 技术演示。
- 早期合作方。

### 阶段 B：Beta 可试点

周期：4 到 6 周

交付：

- P5、P6、P8 初版。
- Slint 客户端可登录、显示用量、本地转换。
- API client 支持账号和用量。
- 每 Profile 至少 10 篇样本。

发布对象：

- 邀请制用户。
- 实验室。
- 合作期刊/编辑。

### 阶段 C：Cloud Pro 可收费

周期：6 到 8 周

交付：

- P7 云端 worker。
- 上传、队列、转换、下载。
- 订阅和用量扣减。
- 转换失败不扣量策略。

发布对象：

- Pro 用户。
- 小团队。

### 阶段 D：GA 正式发布

周期：8 到 12 周

交付：

- P9 三平台分发。
- 自动升级。
- 监控告警。
- 安全沙箱。
- 客户支持和数据删除策略。

发布对象：

- 自助注册用户。
- 团队用户。
- 企业客户。

---

## 七、优先级清单

### 立即做

1. ActiveProfile 全链路。
2. RuleEngine profile 接入。
3. semantic-verify 实现。
4. commercial_verify 读取 report.json。
5. Slint 本地转换 MVP。

### 其次做

1. QualityGate V2。
2. API client auth/usage/plans。
3. Slint 登录和用量页面。
4. 真实样本 fixture。
5. cloud conversion API。

### 后续做

1. 自动升级。
2. 支付订阅。
3. 三平台签名分发。
4. 企业离线授权。
5. AI macro fallback 产品化。

---

## 八、风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| Profile 检测与实际编译策略不一致 | 商业误导 | ActiveProfile 全链路作为 P1 |
| QualityGate 过浅 | 用户误判转换成功 | V2 gate + report.json |
| Slint 只是骨架 | PC 客户端无法试用 | 本地转换 MVP 优先 |
| 云端 runtime 不稳定 | 转换失败率高 | 容器化 + health check + fallback |
| 真实样本不足 | 商业承诺无依据 | 每 Profile fixture 指标化 |
| 支付和用量没有服务端权威 | 套餐失控 | usage/billing 必须服务端判断 |
| 用户隐私 | 商业信任风险 | 本地模式 + 上传前提示 + 删除策略 |

---

## 九、最终判断

截至 2026-06-21 18:00，当前项目的商业化状态是：

```text
技术内核：基本具备
期刊泛化：架构具备，执行链路需闭合
质量门禁：初版具备，商业级不足
CLI：入口具备，产品级不足
桌面端：骨架具备，功能闭环不足
API client：分析能力具备，账号/订阅/转换不足
SaaS 服务：MVP 旧转换存在，商业云转换不足
真实验证：paper3 和 minimal 有，商业样本不足
```

因此，最合理的商业化推进策略是：

```text
先用 2 周完成 Preview 级闭环
再用 4 到 6 周完成邀请制 Beta
最后用 8 到 12 周完成可收费 GA
```

商业化成败的关键不是再增加单点功能，而是把已有的语义引擎、Profile、RuleEngine、DOCX renderer、QualityGate、CLI、Slint 客户端和 SaaS API 串成一条可靠、可审计、可收费、可运维的产品链路。
