# Semantic TeX Engine 商业化技术实现方案

**方案版本**：20260621-151221  
**目标**：将当前语义 TeX→DOCX 引擎推进到可商业化发布  
**适用范围**：新语义引擎路径，不影响原有 Rust doc 转换引擎  
**当前判断**：可技术预览和定向 PoC，暂不满足正式收费发布  
**输出目录**：`docs-zh`  

---

## 一、商业化目标定义

本项目的商业化目标不是做一个“能偶尔转换”的脚本，而是形成一个可持续交付的学术论文 TeX→DOCX 产品：

```text
面向作者、实验室、出版社和期刊平台
提供高可信的 TeX/LaTeX/CTeX → DOCX 转换能力
核心价值是期刊 Profile 泛化、结构语义保留、中文论文支持和可审计质量报告
```

商业化首期不追求覆盖所有 TeX，而是聚焦以下场景：

1. 中文学术论文。
2. 软件学报/JOS 类论文。
3. ACL/TACL、CVPR/ICCV、Nature、Springer 等常见投稿模板。
4. arXiv/generic 论文降级转换。
5. 以 DOCX 可编辑、结构可读、引用可追踪为核心，不承诺完全复刻 PDF 视觉。

---

## 二、发布级别与商业化门槛

### 2.1 技术预览版

用途：

- 内部演示。
- 定向给早期用户试用。
- 验证期刊 Profile 泛化方向。

允许：

- 部分模板需要 fallback。
- DOCX 结构正确但版式不完全一致。
- 部分复杂宏以 RawFallback 或文本降级输出。

必须满足：

- 7 类首期 Profile 能自动检测。
- 7 个 minimal fixture 能生成 DOCX。
- 编译报告包含 profile、backend、compatibility、fallback、raw fallback 计数。
- 验证脚本稳定通过。

### 2.2 邀请制 Beta

用途：

- 面向真实用户小规模试点。
- 可开始收集付费意向，但不建议开放自助付费。

必须满足：

- 每个首期 Profile 至少有 10 篇真实或半真实 fixture。
- DOCX 可打开率 100%。
- minimal fixture raw fallback 数量为 0 或可解释。
- 真实 fixture raw fallback 数量低于 Profile 阈值。
- 兼容性评分低于阈值时必须阻断或明确降级。
- 上传文件、编译日志、生成文件具备安全隔离。
- 用户可下载转换报告。

### 2.3 正式收费 GA

用途：

- SaaS 公开发布。
- API 收费。
- 桌面端或企业版授权。

必须满足：

- 端到端转换成功率可量化。
- 有稳定 CLI/API。
- 有任务队列、隔离执行、资源配额、超时控制。
- 有隐私与数据删除策略。
- 有质量门禁与可观测性。
- 有失败分级与用户可理解报告。
- 支持版本化 Profile 和回归测试。

---

## 三、当前工程基线

### 3.1 已完成的基础能力

当前项目已经具备商业化的技术基础：

| 能力 | 状态 | 主要位置 |
|---|---|---|
| 新语义编译入口 | 已有 | `crates/compiler-engine/src/lib.rs` |
| RuleBased / XeLaTeXHook / LuaTeXNode 后端 | 已有 | `crates/compiler-engine/src/lib.rs` |
| JournalDetector | 已有 | `crates/compiler-engine/src/journal_detector.rs` |
| ProfileRegistry | 已有 | `crates/compiler-engine/src/profiles.rs` |
| 7 类期刊 Profile TOML | 已有 | `crates/compiler-engine/profiles/*.toml` |
| CompatibilityAnalyzer | 已有 | `crates/compatibility-analyzer/src/lib.rs` |
| RuleEngine | 已有 | `crates/rule-engine` |
| 引用图 ReferenceGraph | 已有 | `crates/semantic-collector` |
| XDV / LayoutGraph 原型 | 已有 | `crates/xdv-parser` |
| DOCX writer | 已有 | `crates/docx-writer` |
| PDF 质量验证基础 | 已有 | `crates/quality`、`crates/cli/src/pdf_verify.rs` |

### 3.2 当前不能直接商业化的断点

| 断点 | 影响 |
|---|---|
| `EngineProfile` 仍是固定 enum | `tacl/cvpr/nature/springer` 无法成为完整编译 Profile |
| JournalDetector 结果没有全链路生效 | 检测到的 Profile 主要参与 backend selector，未完整驱动 compatibility、RuleEngine、renderer |
| `journal_rules(profile_id)` 未接入编译管线 | 期刊宏规则存在但不能稳定影响转换结果 |
| Profile `style_map` 未接入 DOCX writer | 不同模板无法形成可靠 DOCX 样式差异 |
| Profile `quality` 未接入 quality gate | 无法形成商业化验收与失败阻断 |
| runtime hook 只做通用采集 | 缺少按 Profile 注入特定宏 hook 的机制 |
| E2E 验证脚本不可作为门禁 | 当前脚本依赖不存在的 `$PROJECT_ROOT/cargo`，且没有逐 fixture 断言输出 |
| 多期刊 CLI 未打通 | `doc-compiler-engine` 没有正式 bin，example 也不支持所有 Profile |
| 真实样本集不足 | minimal fixture 不能代表商业场景 |

---

## 四、商业化产品架构

### 4.1 总体架构

```text
User Upload
  ↓
Project Normalizer
  ↓
Security Sandbox
  ↓
JournalDetector
  ↓
ProfileRegistry
  ↓
CompatibilityAnalyzer
  ↓
Semantic Compile Pipeline
     ├─ RuleBasedBackend
     ├─ XeLaTeXHookBackend
     └─ LuaTeXNodeBackend
  ↓
Profile-aware RuleEngine
  ↓
Semantic AST / DocumentGraph
  ↓
Profile-aware DOCX Renderer
  ↓
Quality Gate
  ↓
DOCX + Report + Diagnostics
```

### 4.2 商业化服务架构

```text
Web / CLI / API / Desktop
  ↓
API Gateway
  ↓
Auth / Billing / Rate Limit
  ↓
Conversion Job Queue
  ↓
Worker Pool
     ├─ fast-worker: rule-based only
     ├─ tex-worker: XeLaTeX/LuaLaTeX runtime
     ├─ verify-worker: DOCX/PDF quality check
     └─ render-worker: optional rasterization
  ↓
Object Storage
  ↓
Result Delivery
```

### 4.3 Worker 类型

| Worker | 能力 | 适用套餐 |
|---|---|---|
| `fast-worker` | RuleBased 转换，无外部 TeX | Free / Preview |
| `semantic-worker` | RuleBased + Profile + DOCX | Pro |
| `tex-worker` | XeLaTeX/LuaLaTeX hook、sidecar、XDV | Pro / Team |
| `quality-worker` | DOCX 可打开、结构验证、PDF diff | Team / Enterprise |
| `enterprise-worker` | 专属模板、自定义 Profile、私有化部署 | Enterprise |

---

## 五、核心技术改造方案

## P0：商业化发布基线固化

### 目标

把“可以测试”变成“可以作为发布门禁复现”。

### 任务

1. 修复 `scripts/verify_journal_profiles.sh`。
2. 使用 PATH 中的 `cargo`，不要硬编码 `$PROJECT_ROOT/cargo`。
3. 逐个 fixture 运行真实检测，而不是只跑全局 test。
4. DOCX 生成失败必须导致脚本失败，不能只 warning。
5. 输出 JSON 报告，记录每个 profile 的检测结果、兼容性分数、backend、产物路径。

### 建议命令

```bash
scripts/verify_journal_profiles.sh --all
scripts/verify_journal_profiles.sh --profile-id tacl
scripts/verify_journal_profiles.sh --skip-runtime
```

### 验收

```text
7 个 minimal fixture 全部生成 DOCX
脚本退出码准确
报告中每个 selected_profile_id 与预期一致
输出 examples/journals/output/*.docx
```

---

## P1：动态 Profile 全链路接入

### 当前问题

`CompileOptions.profile` 当前依赖 `EngineProfile` enum，而 enum 只覆盖：

```text
generic-article
chinese-academic
jos-paper
medical-journal
```

这导致 `tacl/cvpr/nature/springer` 只能被检测，不能完整成为编译 Profile。

### 目标设计

新增动态 Profile 引用：

```rust
pub struct ActiveProfile {
    pub id: String,
    pub spec: ProfileSpecFile,
    pub source: ProfileSource,
}

pub enum ProfileSource {
    ExplicitId,
    ExplicitPath,
    AutoDetected,
    Fallback,
}
```

`CompileOptions` 改造为：

```rust
pub struct CompileOptions {
    pub profile: EngineProfile,
    pub profile_id: Option<String>,
    pub profile_path: Option<PathBuf>,
    pub profile_auto_detect: bool,
    ...
}
```

中期可进一步废弃 enum：

```rust
pub struct CompileOptions {
    pub profile_ref: ProfileRef,
    ...
}

pub enum ProfileRef {
    Auto,
    Id(String),
    Path(PathBuf),
}
```

### 处理流程

```text
resolve_active_profile(options, vfs)
  ↓
explicit path
  ↓
explicit id
  ↓
JournalDetector
  ↓
generic fallback
```

### 必须接入的模块

| 模块 | 改造 |
|---|---|
| `compile_vfs_to_graph` | 先解析 `ActiveProfile`，后续都使用它 |
| `CompatibilityAnalyzer` | 使用 `ProfileKind::from_id(active_profile.id)` |
| `select_auto_backend` | 使用 `active_profile.spec.backend` |
| `RuleEngine` | 加载 `active_profile.spec.macro_rules` 和 `journal_rules(active_profile.id)` |
| `DocxRenderer` | 使用 `active_profile.spec.style_map`、caption/citation policy |
| `CompileReport` | 增加 active profile 来源、检测置信度、fallback 信息 |

### 验收

```text
--profile-id tacl 能完整编译
--profile-id cvpr 能完整编译
--profile-id nature 能完整编译
--profile-id springer 能完整编译
--profile auto 能自动选择以上 Profile
```

---

## P2：正式 CLI 与 API 内核

### 当前问题

`doc-compiler-engine` 目前主要是 library 和 example，没有稳定商业入口。现有 `paper3_to_docx` example 只支持有限 Profile，不适合作为产品入口。

### CLI 设计

新增正式命令：

```bash
tex2doc semantic convert \
  --project-root examples/journals/tacl \
  --main-tex minimal.tex \
  --profile auto \
  --out examples/journals/output/tacl.docx \
  --report examples/journals/output/tacl.report.json
```

支持：

```bash
tex2doc semantic detect --project-root . --main-tex main.tex
tex2doc semantic analyze --project-root . --main-tex main.tex --profile auto
tex2doc semantic convert --profile auto
tex2doc semantic verify --docx output.docx --report report.json
```

### Rust API 设计

```rust
pub struct ConvertRequest {
    pub project_root: PathBuf,
    pub main_tex: PathBuf,
    pub profile_ref: ProfileRef,
    pub backend: SemanticBackendKind,
    pub output: PathBuf,
    pub report: Option<PathBuf>,
}

pub struct ConvertResponse {
    pub docx_path: PathBuf,
    pub report: CommercialCompileReport,
    pub quality: QualityGateResult,
}
```

### HTTP API 设计

```http
POST /v1/conversions
GET  /v1/conversions/{id}
GET  /v1/conversions/{id}/download/docx
GET  /v1/conversions/{id}/report
DELETE /v1/conversions/{id}
```

### 验收

```text
CLI 能完成 7 个 fixture 转换
API 能提交任务、查询状态、下载结果
每次转换都有 machine-readable report
```

---

## P3：Profile-aware RuleEngine 接入

### 当前问题

期刊宏规则已经定义，但编译管线没有按 Profile 加载这些规则。

### 改造目标

```rust
fn build_rule_engine(active_profile: &ActiveProfile) -> RuleEngine {
    let mut engine = RuleEngine::new();
    for rule in journal_rules(&active_profile.id) {
        engine.registry_mut().register(rule);
    }
    for rule in profile_macro_rules_to_runtime(&active_profile.spec.macro_rules) {
        engine.registry_mut().register(rule);
    }
    engine
}
```

`apply_rule_engine_to_document` 改为：

```rust
fn apply_rule_engine_to_document(
    document: Document,
    active_profile: &ActiveProfile,
    report: &mut CompileReport,
) -> Document
```

### 输出要求

RuleEngine 决策必须进入报告：

```json
{
  "rule_engine": {
    "loaded_builtin_rules": 18,
    "loaded_journal_rules": 8,
    "loaded_profile_rules": 4,
    "unknown_macros": 3,
    "fallback_macros": ["mytable", "customcaption"]
  }
}
```

### 验收

```text
\citet / \citep 在 tacl 中不再作为普通未知宏
\IEEEkeywords 在 jos-paper 中输出 metadata/keywords
\institute 在 springer 中输出 affiliation
\keywords 在 chinese-academic 中输出 keywords
```

---

## P4：Profile-aware Runtime Hook

### 目标

让 XeLaTeX/LuaTeX runtime 采集从“通用 hook”升级为“通用 hook + Profile hook”。

### 设计

```text
common-hook.tex
profile-hooks/
  jos-paper.tex
  tacl.tex
  cvpr.tex
  nature.tex
  springer.tex
  chinese-academic.tex
```

LuaTeX：

```text
common.lua
profile-hooks/
  tacl.lua
  cvpr.lua
  nature.lua
```

注入顺序：

```text
common hook
  ↓
profile hook
  ↓
source document
```

### 事件 schema

```json
{
  "schema": "semantic-event-v3",
  "profile_id": "tacl",
  "origin": "runtime-luatex",
  "type": "metadata",
  "key": "shorttitle",
  "value": "A short title",
  "source": {
    "path": "main.tex",
    "line": 12
  }
}
```

新增事件：

```rust
Metadata { key, value, span }
Author { text, span }
Affiliation { text, span }
KeywordList { text, separator, span }
EnvironmentBegin { name, span }
EnvironmentEnd { name, span }
```

### 验收

```text
LuaTeX sidecar 包含 profile_id
TACL 的 \shorttitle/\name/\address 可采集
Springer 的 \institute/\email 可采集
Chinese academic 的 \keywords 可采集
```

---

## P5：Profile-aware DOCX 渲染

### 当前问题

Profile 中的 `style_map`、caption policy、citation policy 已有配置入口，但未成为 DOCX writer 的稳定输入。

### 渲染模型

新增中间层：

```rust
pub struct DocxRenderContext {
    pub profile: ActiveProfile,
    pub style_map: StyleMap,
    pub caption_policy: CaptionPolicy,
    pub citation_policy: CitationPolicy,
    pub page_setup: PageSetup,
}
```

Renderer 接口：

```rust
pub trait ProfileDocxRenderer {
    fn render(
        &self,
        doc: &Document,
        graph: &ReferenceGraph,
        context: &DocxRenderContext,
    ) -> Result<Vec<u8>, RenderError>;
}
```

### 必须支持的样式映射

| Semantic role | DOCX style |
|---|---|
| `metadata.title` | `Title` |
| `metadata.author` | `Author` |
| `heading.level1` | `Heading1` |
| `heading.level2` | `Heading2` |
| `paragraph.body` | `BodyText` |
| `caption.figure` | `FigureCaption` |
| `caption.table` | `TableCaption` |
| `equation.display` | `Equation` |
| `bibliography.item` | `Bibliography` |

### Caption policy

```text
jos-paper: 图 / 表 / 式，section-scoped
tacl: Figure / Table，arabic
cvpr: Figure / Table，arabic
nature: Figure / Table，arabic
springer: Fig. / Table，arabic
chinese-academic: 图 / 表 / 式，chapter-or-section
generic: Figure / Table，arabic
```

### 验收

```text
7 个 fixture 生成的 DOCX 样式名称符合 Profile
caption 前缀符合 Profile
引用链接和书签仍可用
公式 OMML fallback 报告准确
```

---

## P6：Quality Gate 与商业报告

### 目标

商业化必须让用户知道：

```text
转换成功了吗
为什么失败
哪些地方被降级
是否适合提交给期刊
```

### QualityGateResult

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

### 必须检查

| 检查项 | 失败级别 |
|---|---|
| DOCX zip 可打开 | blocking |
| `word/document.xml` 存在 | blocking |
| compatibility score >= profile threshold | configurable |
| raw fallback blocks <= profile threshold | warning/blocking |
| unresolved references <= profile threshold | warning |
| image assets missing | warning/blocking |
| runtime fallback occurred | warning |
| unsupported package detected | warning/blocking |
| profile detection confidence too low | warning/blocking |

### 报告格式

输出：

```text
report.json
report.md
semantic-events.jsonl
quality-summary.txt
```

报告 JSON 示例：

```json
{
  "profile": {
    "selected": "tacl",
    "source": "auto-detected",
    "confidence": 0.95
  },
  "backend": {
    "requested": "auto",
    "selected": "luatex-node",
    "fallback": false
  },
  "compatibility": {
    "score": 88,
    "unsupported": [],
    "warnings": ["biblatex"]
  },
  "quality": {
    "status": "passed_with_warnings",
    "score": 84
  }
}
```

### 验收

```text
每次转换都有 report.json
质量失败时不输出“成功”状态
CLI/API 返回明确错误码
```

---

## P7：真实样本回归体系

### 目标

从 minimal fixture 升级为商业可用的回归语料。

### 样本分层

```text
fixtures/
  minimal/
    7 profiles × 1
  realistic/
    7 profiles × 10
  stress/
    complex tables
    tikz
    minted/listings
    biblatex
    ctex fonts
    missing images
  customer-redacted/
    sanitized real documents
```

### 指标

| 指标 | Beta 阈值 | GA 阈值 |
|---|---:|---:|
| DOCX openable | 100% | 100% |
| minimal profile detection | 100% | 100% |
| realistic profile detection | >= 95% | >= 98% |
| conversion success | >= 90% | >= 97% |
| blocking quality failure | 可报告 | 可报告 |
| unhandled panic | 0 | 0 |
| job timeout recoverability | 100% | 100% |

### 自动化

```bash
scripts/commercial_verify.sh \
  --suite minimal,realistic \
  --out target/commercial-verify
```

产物：

```text
target/commercial-verify/summary.json
target/commercial-verify/summary.md
target/commercial-verify/docx/
target/commercial-verify/reports/
```

---

## 六、SaaS 技术实现方案

## 6.1 API Gateway

职责：

- 用户认证。
- 文件大小限制。
- 频率限制。
- 任务创建。
- 计费事件记录。

建议接口：

```http
POST /v1/conversions
Content-Type: multipart/form-data

fields:
  file: zip or tex project
  main_tex: string
  profile: auto | profile-id
  quality_level: preview | standard | strict
```

返回：

```json
{
  "id": "conv_20260621_xxx",
  "status": "queued",
  "estimated_seconds": 30
}
```

## 6.2 Job Queue

推荐任务状态：

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

任务 payload：

```json
{
  "job_id": "conv_xxx",
  "tenant_id": "tenant_xxx",
  "input_uri": "s3://bucket/input.zip",
  "main_tex": "main.tex",
  "profile_ref": "auto",
  "quality_level": "standard",
  "limits": {
    "timeout_seconds": 120,
    "max_files": 500,
    "max_unzipped_mb": 200
  }
}
```

## 6.3 Worker Sandbox

TeX 运行期必须隔离：

- 容器内运行。
- 只读 TeXLive 镜像。
- 禁止网络。
- 限制 CPU、内存、磁盘、进程数。
- 禁止 shell escape，除非 Enterprise 专属隔离环境。
- 每个 job 独立临时目录。

推荐限制：

```text
CPU: 2 cores
Memory: 2 GB
Timeout: 120 s preview, 300 s pro
Disk: 512 MB preview, 2 GB pro
Process: 64
Network: disabled
```

## 6.4 Object Storage

对象：

```text
input.zip
normalized-project.zip
output.docx
report.json
report.md
semantic-events.jsonl
logs.txt
```

默认保留策略：

| 套餐 | 保留时间 |
---|---:|
| Free | 24 小时 |
| Pro | 7 天 |
| Team | 30 天 |
| Enterprise | 可配置 |

## 6.5 Billing Meter

计费维度：

```rust
pub struct BillingUsage {
    pub pages_estimated: u32,
    pub input_bytes: u64,
    pub runtime_backend_used: bool,
    pub quality_verification_used: bool,
    pub conversion_seconds: u32,
    pub profile_id: String,
    pub success: bool,
}
```

商业化初期建议按次数计费，不按页精细计费：

```text
Free: 每月 10 次，generic/chinese preview
Pro: 每月 200 次，7 类 Profile，标准质量报告
Team: 批量转换，质量门禁，团队历史
Enterprise: 私有部署，自定义 Profile，SLA
```

---

## 七、安全与合规设计

### 7.1 输入安全

必须检查：

- zip path traversal。
- symlink。
- 过大文件。
- 过深目录。
- 过多文件。
- 非 UTF-8 文件名。
- 隐藏二进制或可执行文件。

### 7.2 TeX 安全

禁用或限制：

```text
\write18
\openout
\input{/absolute/path}
\include{/absolute/path}
shell-escape
网络访问
```

### 7.3 数据隐私

最低要求：

- 默认不训练模型。
- 默认不人工查看用户文档。
- 明确文件保留时间。
- 用户可删除任务。
- 企业版支持私有化部署。

### 7.4 日志脱敏

日志中不要保存：

- 全文 TeX。
- 作者邮箱。
- 机构详细信息。
- 图片内容。

报告中可保存：

- 宏名。
- package 名。
- profile id。
- 错误码。
- 兼容性统计。

---

## 八、可观测性与运营指标

### 8.1 技术指标

```text
conversion_success_rate
profile_detection_accuracy
runtime_backend_failure_rate
rule_based_fallback_rate
raw_fallback_block_count
unsupported_package_rate
docx_open_failure_rate
quality_gate_failure_rate
p95_conversion_seconds
```

### 8.2 产品指标

```text
upload_to_success_rate
first_conversion_success_rate
download_rate
retry_rate
paid_conversion_rate
profile_distribution
top_failure_reasons
```

### 8.3 错误码体系

```text
E_INPUT_ZIP_INVALID
E_INPUT_MAIN_TEX_MISSING
E_PROFILE_LOW_CONFIDENCE
E_COMPAT_UNSUPPORTED
E_RUNTIME_UNAVAILABLE
E_RUNTIME_TIMEOUT
E_DOCX_RENDER_FAILED
E_QUALITY_GATE_FAILED
E_INTERNAL_PANIC
```

---

## 九、商业化开发路线图

## M0：商业化门禁修复

周期：1 周

任务：

1. 修复 `verify_journal_profiles.sh`。
2. 新增正式 `semantic convert/detect/analyze` CLI。
3. 加入 7 个 fixture 的真实 DOCX 生成断言。
4. 输出 `report.json`。

验收：

```text
7 minimal fixture 全通过
失败时退出码准确
CI 可跑
```

## M1：动态 Profile 全链路

周期：2 周

任务：

1. 引入 `ActiveProfile`。
2. JournalDetector 结果驱动 compatibility。
3. JournalDetector 结果驱动 RuleEngine。
4. JournalDetector 结果驱动 DOCX page setup、style、caption。
5. `tacl/cvpr/nature/springer` 支持显式 `--profile-id`。

验收：

```text
--profile-id tacl/cvpr/nature/springer 均可转换
profile 自动检测和显式指定一致
```

## M2：质量门禁和商业报告

周期：2 周

任务：

1. 实现 `QualityGateResult`。
2. 接入 compatibility score。
3. 接入 raw fallback 阈值。
4. 接入 unresolved reference 阈值。
5. 输出 Markdown 和 JSON 报告。

验收：

```text
质量失败能阻断
报告可解释
用户能知道降级点
```

## M3：真实样本集

周期：3 到 4 周

任务：

1. 每个 Profile 收集 10 个 realistic fixture。
2. 建立 fixture license 与脱敏策略。
3. 建立 nightly regression。
4. 输出 profile 成功率统计。

验收：

```text
70 篇 realistic fixture 可自动跑
转换成功率 >= 90%
DOCX openable = 100%
```

## M4：SaaS MVP

周期：4 到 6 周

任务：

1. API Gateway。
2. Job Queue。
3. Worker Sandbox。
4. Object Storage。
5. 用户下载和删除。
6. 基础计费事件。

验收：

```text
用户可上传 zip
后台异步转换
可下载 DOCX 和 report
任务超时可恢复
```

## M5：Beta 商业试点

周期：4 周

任务：

1. 邀请 10 到 30 个真实用户。
2. 收集失败样本。
3. 建立 profile backlog。
4. 增强 RuleEngine 和 Profile。
5. 完成隐私、条款、数据删除。

验收：

```text
Beta 用户首转成功率 >= 80%
真实转换成功率 >= 90%
严重故障 0
```

## M6：正式 GA

周期：6 到 8 周

任务：

1. 支付/订阅。
2. 团队空间。
3. API key。
4. 运行监控。
5. SLA 基础。
6. 私有化部署包。

验收：

```text
公开注册可用
付费链路可用
转换与质量指标达标
支持工单闭环
```

---

## 十、商业化验收矩阵

| 能力 | Preview | Beta | GA |
|---|---:|---:|---:|
| 7 Profile 自动检测 | 必须 | 必须 | 必须 |
| 7 Profile DOCX 生成 | 必须 | 必须 | 必须 |
| 动态 Profile 全链路 | 建议 | 必须 | 必须 |
| Profile-aware RuleEngine | 建议 | 必须 | 必须 |
| Profile-aware DOCX style | 可部分 | 必须 | 必须 |
| Quality gate | 可报告 | 必须 | 必须 |
| 真实样本回归 | 可少量 | 每类 10 篇 | 每类 30 篇以上 |
| SaaS job queue | 不要求 | 必须 | 必须 |
| 安全沙箱 | 不要求 | 必须 | 必须 |
| 计费系统 | 不要求 | 可选 | 必须 |
| 用户数据删除 | 不要求 | 必须 | 必须 |
| 监控告警 | 不要求 | 必须 | 必须 |

---

## 十一、首期开发清单

建议立即执行的开发清单：

1. 修复 `scripts/verify_journal_profiles.sh` 的 cargo 路径与退出码。
2. 新增 `doc-compiler-engine` 或 `cli` 的正式 semantic 子命令。
3. 引入 `ActiveProfile`，让自动检测结果成为编译上下文。
4. `CompatibilityAnalyzer` 使用 detected profile。
5. `RuleEngine` 加载 `journal_rules(profile_id)`。
6. Profile TOML `macro_rules` 转成运行期 `MacroRule`。
7. Profile `quality` 进入 `QualityGateResult`。
8. Profile `style_map` 接入 DOCX writer。
9. 7 个 minimal fixture 端到端生成 DOCX。
10. 新增 `commercial_verify.sh` 作为商业化门禁。

---

## 十二、风险与处理策略

| 风险 | 商业影响 | 处理 |
|---|---|---|
| TeX 模板差异巨大 | 转换失败率高 | 限定首期 Profile，低置信度提示用户显式选择 |
| LuaLaTeX/XeLaTeX 环境不稳定 | SaaS 任务失败 | 容器化 TeXLive，runtime health check |
| 复杂宏无法识别 | DOCX 语义缺失 | RuleEngine + RawFallback 报告 + 客户样本闭环 |
| DOCX 版式不符预期 | 用户退款 | 明确产品定位为结构 DOCX，不承诺 PDF 级复刻 |
| 用户论文隐私 | 法务风险 | 默认短期保留、可删除、不训练、不人工查看 |
| TikZ/minted 等高复杂包 | 转换质量不稳 | compatibility 阻断或 rasterize fallback |
| 质量门禁缺失 | 用户误以为成功 | GA 前必须实现 QualityGateResult |

---

## 十三、结论

当前项目已经具备商业化的技术基础，但还没有达到正式收费发布标准。最核心的差距不是“再多支持几个宏”，而是把已经存在的 Profile、JournalDetector、RuleEngine、CompatibilityAnalyzer、DOCX Renderer 和 Quality Gate 串成一条可审计、可验证、可运营的商业化转换链路。

建议将近期目标定义为：

```text
先完成 Preview 可演示
再完成 Beta 可试点
最后完成 GA 可收费
```

其中最优先的技术工作是：

```text
ActiveProfile 全链路
Profile-aware RuleEngine
Profile-aware DOCX Renderer
QualityGateResult
真实 fixture 回归
正式 CLI/API
安全沙箱与任务队列
```

完成这些后，Semantic TeX Engine 才能从“语义转换工程原型”升级为“可以商业化运营的 TeX→DOCX 产品内核”。
