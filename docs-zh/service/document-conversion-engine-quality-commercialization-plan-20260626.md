# Tex2Doc 文档转换引擎质量提升与商业化整改技术方案

> 日期：2026-06-26  
> 输出目录：`docs-zh/service`  
> 适用范围：Tex2Doc LaTeX 到 DOCX 转换核心、云端转换服务、桌面端云转换入口、商业化质量与交付体系

## 1. 结论摘要

Tex2Doc 当前已经具备商业化雏形：纯 Rust 转换核心、语义 AST、期刊 Profile、模板继承、OMML 公式、引用链接、DOCX 打包、质量门禁、云端上传/转换/下载、账号与计费能力均已进入主链路。下一阶段不应只继续堆叠单点规则，而应把项目升级为“可度量、可解释、可扩展、可运营”的转换平台。

建议将整改目标拆为四条主线：

1. 转换质量主线：从“能生成 DOCX”升级为“语义完整、版面稳定、Word 无修复、质量可评分”。
2. 工程架构主线：从规则兜底升级为 Profile、Rule、Backend、Renderer、Quality 可插拔的分层引擎。
3. 服务商业化主线：从单机/演示型队列升级为具备 SLA、隔离、审计、重试、限流、成本控制的转换服务。
4. 产品推广主线：把期刊模板、转换报告、失败诊断、企业私有化部署沉淀为可销售资产。

建议优先投入质量闭环平台，而不是优先扩展更多入口端。只有转换结果可稳定量化，后续渠道推广和付费转化才有可持续基础。

## 2. 当前架构判断

根据代码图谱与源码核对，现有主链路如下：

```text
CLI / 云端 API / 桌面端云转换
  -> 上传 zip 或目录 / VFS 挂载
  -> ActiveProfile 解析与期刊自动识别
  -> CompatibilityAnalyzer 兼容性分析
  -> SemanticBackend 自动选择
     -> RuleBased / XeLaTeXHook / LuaTeXNode
     -> 失败时可 fallback 到 RuleBased
  -> SemanticEvent / DocumentGraph / ReferenceGraph / LayoutGraph
  -> docx-writer 打包 document.xml / styles.xml / media / relationships
  -> OMML 公式与引用链接后处理
  -> CompileReport + QualityGate
  -> 服务端 job report / DOCX / log / zip 下载
```

关键实现位置：

- `crates/compiler-engine/src/lib.rs`：转换主编排、Profile、后端选择、质量门禁、CompileReport。
- `crates/latex-reader/src/lower.rs`：LaTeX 语义 lowering，包括列表、captioned env 等。
- `crates/docx-writer/src/serializer.rs` 与 `packer.rs`：DOCX OOXML 序列化与打包。
- `crates/quality/src/docx_diff.rs`：DOCX 快照、段落对齐、格式差异比较。
- `crates/compatibility-analyzer/src/lib.rs`：包、文档类、特性兼容性评分。
- `apps/rust-service/src/routes.rs`：上传、转换任务、报告与下载 API。
- `apps/rust-service/src/worker_service.rs`：云转换 worker，语义引擎与 legacy fallback。
- `apps/rust-service/src/state.rs`：job 创建、入队、存储、查询与额度关联。

当前优势：

- 已有强类型语义模型与多后端路线，具备长期演进价值。
- 已有 Profile 与期刊自动检测，可承载商业模板包。
- 已有质量门禁字段，报告结构已经能向用户解释部分问题。
- 已有 DOCX diff crate，可扩展为回归平台核心。
- 已有账号、额度、充值、下载、反馈和桌面云转换入口，商业闭环雏形完整。

当前短板：

- 质量分数仍偏工程内部指标，和最终用户感知的“像不像、能不能改、会不会坏”之间存在差距。
- 服务端 legacy 路径仍用 DOCX 大小推断质量，商业交付可信度不足。
- fallback 虽然可用，但降级内容缺少细粒度审计与用户可解释呈现。
- DOCX diff 主要作为 CLI/离线能力存在，尚未成为每次转换的内置质量资产。
- 对复杂 LaTeX 宏包、表格、算法、浮动体、参考文献和 CJK 版式的覆盖需要体系化提升。
- 服务队列与 worker 需要更强的重试、隔离、观测、限流和 SLA 设计。

## 3. 商业级质量目标

建议定义“商业可用”不是一个笼统状态，而是一组硬指标：

| 指标 | GA 目标 | 企业版目标 |
| --- | ---: | ---: |
| DOCX 可打开且 Word 不触发修复 | >= 99.5% | >= 99.9% |
| 转换任务成功率 | >= 95% | >= 98% |
| 期刊 Profile 自动识别准确率 | >= 90% | >= 95% |
| 未解析引用占比 | <= 2% | <= 0.5% |
| 公式 OMML 成功转换率 | >= 90% | >= 97% |
| 核心样例视觉相似度 | >= 90 | >= 95 |
| P95 云端转换耗时 | <= 60 秒 | <= 30 秒或按页数计费 SLA |
| 失败报告可解释率 | >= 95% | >= 99% |
| 回归样例阻断覆盖 | >= 80% | >= 95% |

质量分数建议从单一 compatibility score 升级为多维评分：

| 维度 | 权重 | 说明 |
| --- | ---: | --- |
| 解析完整性 | 20% | include、宏、环境、错误恢复、未知命令比例 |
| 语义保真 | 25% | 标题、段落、列表、表格、图、公式、引用、参考文献 |
| DOCX 结构 | 20% | OOXML 合法性、style、relationship、media、field |
| 版面一致 | 20% | 页边距、字号、行距、表格宽度、浮动体位置、PDF 视觉差异 |
| 可编辑性 | 10% | Word 打开、段落样式、交叉引用可更新、公式可编辑 |
| 性能与稳定 | 5% | 耗时、内存、fallback、重试、超时 |

## 4. 目标架构

建议将转换引擎演进为六层架构：

```text
1. Intake & Preflight
   上传校验、main.tex 发现、zip 安全、包/类扫描、兼容性预测

2. Semantic Compiler
   include graph、宏展开、环境识别、引用图、语义事件、诊断事件

3. Backend Orchestrator
   RuleBased / XeLaTeXHook / LuaTeXNode / 后续商业运行时
   后端选择、fallback 策略、降级审计

4. DOCX Renderer
   Profile style pack、模板继承、OOXML emit、OMML、media、field、relationships

5. Quality Platform
   DOCX 结构检查、Word open 检查、PDF/图片视觉对比、golden corpus、质量报告

6. Commercial Service
   Job API、队列、worker pool、对象存储、额度计费、SLA、观测、审计、反馈闭环
```

核心原则：

- Semantic AST 是唯一可信中间层，所有 renderer 和报告都围绕它扩展。
- fallback 必须可审计，不能只作为内部日志存在。
- Profile 不只是配置，而是商业模板资产，应版本化、测试化、授权化。
- 质量报告是产品能力，不只是开发调试信息。
- 服务化优先保证隔离和可恢复，再追求吞吐。

## 5. 整改技术方案

### 5.1 P0：建立质量基线与样例资产

目标：先把“好坏”量化，否则后续优化无法证明价值。

改造内容：

- 建立 `quality-corpus` 元数据规范：来源、文档类型、页数、包列表、预期 profile、授权状态、golden DOCX/PDF。
- 将现有 examples、tests、docs 中的 JOS、paper2、paper3、通用 article 样例纳入第一批基线。
- 每个样例生成以下产物：输入 zip、输出 docx、CompileReport JSON、DOCX diff、PDF 渲染图、质量摘要。
- `crates/quality` 增加统一 `QualityRun` 模型，承接 DOCX diff、结构检查、视觉检查、性能指标。
- `apps/rust-service` 的转换报告增加 `quality_dimensions` 字段，避免只给一个分数。

验收标准：

- 至少 30 个真实/合成样例进入基线。
- 每个 PR 能跑核心 smoke corpus。
- 任一核心样例出现 Word 修复、DOCX 结构损坏、质量分下降超过阈值时阻断。

### 5.2 P1：语义覆盖率提升

目标：减少未知宏、raw fallback 和语义丢失。

改造内容：

- 在 `latex-reader` 与 `rule-engine` 之间建立宏/环境能力矩阵：
  - 支持级别：native、lowered、text fallback、unsupported。
  - 影响级别：阻断、降级、可忽略。
  - 用户提示：报告中的修复建议。
- 增强表格语义：
  - `tabular`、`tabularx`、`longtable`、`multirow`、`multicolumn` 分层支持。
  - 对不能精确表达的布局给出降级说明。
- 增强图与浮动体：
  - figure/table/algorithm/theorem 统一 caption、label、numbering、cross-reference 模型。
  - 图片路径解析、尺寸推断、DPI、缺图提示标准化。
- 增强参考文献：
  - `natbib`、`biblatex`、`.bbl`、`.bib` 四类输入建立统一 CitationGraph。
  - 支持 author-year、numeric、superscript 三类输出策略。
- 增强 CJK 与中文学术样式：
  - 中文标点、全半角、字体 fallback、行距、首行缩进、标题编号规则 Profile 化。
- 所有 fallback 都写入 `CompileReport.rule_engine` 与新的 `semantic_loss_events`。

验收标准：

- unknown macro 数量在核心 corpus 中下降 60%。
- 表格、图、公式、引用四类对象的保真率均可在报告中量化。
- 用户下载报告能明确看到“哪些内容被降级”和“如何修复源文件”。

### 5.3 P2：DOCX 渲染与 Word 兼容性加固

目标：输出结果不仅能打开，还要稳定、可编辑、样式一致。

改造内容：

- `docx-writer` 增加 OOXML 结构校验：
  - relationship 目标存在性。
  - media 引用完整性。
  - numbering/style 引用完整性。
  - document.xml、styles.xml、settings.xml、footnotes/endnotes 完整性。
- 引入 Word open regression：
  - Windows 环境优先使用 Word/LibreOffice 打开和另存检查。
  - CI 或无 GUI 环境使用 docx unzip + schema-lite + LibreOffice headless 兜底。
- 统一样式解析：
  - Profile style role 到 Word style id 的映射必须有覆盖率报告。
  - 模板继承时记录实际继承的 style、numbering、theme。
- 强化表格 layout：
  - 固定宽度、百分比宽度、跨列、不能分页、caption keep-next。
  - 针对期刊 Profile 建立表格 golden。
- 强化公式：
  - OMML 成功、fallback、不可编辑三种状态分别计数。
  - 对 fallback 公式输出图片或文本策略要按 Profile 可配置。
- 强化引用链接：
  - bookmark id 唯一性、hyperlink anchor 完整性、field 可更新性。

验收标准：

- 核心 corpus Word 打开修复率为 0。
- style map 覆盖率 >= 95%。
- 表格 golden 的 DOCX diff 无结构性回归。
- 公式 fallback 率进入质量分计算，而不是仅作为 warning。

### 5.4 P3：质量平台服务化

目标：把质量能力从离线工具变成商业服务的一部分。

改造内容：

- `crates/quality` 输出统一 JSON schema：
  - `quality_score`
  - `dimension_scores`
  - `blocking_issues`
  - `warnings`
  - `semantic_loss_events`
  - `visual_diff_summary`
  - `word_compatibility`
  - `suggested_fixes`
- `apps/rust-service/src/worker_service.rs` 停止使用 DOCX 大小推断质量。
  - semantic-engine 使用 CompileReport + QualityRun。
  - legacy-rule 也必须经过 DOCX 结构检查和最低限度 diff/打开检查。
- 转换完成后保存：
  - `report.json`
  - `conversion.log`
  - `quality.html` 或 markdown 摘要
  - 可选 PDF/PNG diff 产物
- 管理后台增加质量趋势：
  - 成功率、失败原因 Top N、profile 命中率、fallback 率、平均耗时、P95。
- 用户端报告分层：
  - 普通用户：成功/警告/失败、可下载、建议修复。
  - 专业用户：对象级差异、style 差异、未解析引用、宏包风险。

验收标准：

- 所有云端转换都有可下载质量报告。
- 所有失败都落到稳定错误码，不再只返回字符串。
- 管理后台能按 profile、引擎、版本查看质量趋势。

### 5.5 P4：服务可靠性、隔离与成本控制

目标：达到可对外推广的服务能力。

改造内容：

- Job 模型增强：
  - `queued/running/succeeded/failed/cancelled/expired` 状态机。
  - `attempt_count`、`last_error_code`、`worker_id`、`engine_version`、`profile_version`。
  - 创建转换接口支持 idempotency key，避免重复扣费。
- 队列与 worker：
  - 当前 channel 入队可作为单机基础，商业服务建议接入持久队列或 DB claim loop。
  - 支持超时、重试、死信、worker 心跳、stale job recovery。
  - 不同质量等级使用不同队列：preview、standard、strict、enterprise。
- 安全隔离：
  - zip 解压大小、文件数量、路径、压缩炸弹、可执行文件扫描。
  - 运行时 backend 调 TeX 时必须 sandbox、限制 CPU/内存/磁盘/网络。
  - 用户上传与输出分租户存储，设置过期策略。
- 成本控制：
  - 按页数、文件大小、耗时、backend 类型计算成本。
  - 严格质量或视觉 diff 可作为高级能力收费。
  - 对异常样例自动降级或转人工/异步处理。
- 可观测：
  - tracing span 覆盖 upload、preflight、compile、render、quality、store。
  - metrics 覆盖成功率、耗时、内存、失败码、fallback、队列积压。
  - 日志脱敏，避免泄露用户论文内容。

验收标准：

- worker crash 后任务可恢复。
- 重复提交不会重复扣费。
- 所有外部可见失败都有稳定错误码。
- P95/P99 延迟、队列积压、失败率有看板。

### 5.6 P5：商业产品化与推广能力

目标：把转换引擎变成可销售、可扩展、可合作的产品。

产品能力建议：

- Profile 商店：
  - 期刊/会议/学校模板作为版本化包管理。
  - 每个 Profile 附带样例、golden、质量阈值、授权说明。
  - 支持用户上传 reference.docx 生成私有 Profile。
- API/SDK：
  - 提供 REST API、Rust client、TypeScript client、Python quickstart。
  - 支持 webhook、批量任务、异步轮询、报告下载。
- 企业私有化：
  - 离线 license、内网部署、对象存储适配、审计日志、管理员配额。
  - 禁用云端数据留存或设置短周期自动删除。
- 专业报告：
  - “投稿前格式体检”作为独立付费产品。
  - 输出期刊差异、风险等级、可修复建议。
- 反馈闭环：
  - 用户反馈线程关联 job、profile、engine_version、quality report。
  - 高价值失败样例进入内部 corpus，修复后自动回归。

推广策略建议：

- 先聚焦 3 到 5 个高价值场景，而不是泛化所有 LaTeX：
  - 中文学术论文/JOS。
  - 通用 arXiv article。
  - ACL/TACL。
  - CVPR/IEEE。
  - Springer/Nature 初稿。
- 每个场景输出公开可展示的 before/after 样例、质量分、耗时和限制说明。
- 官网宣传避免承诺“100% 还原”，应表达为“可度量高保真转换 + 可解释格式诊断”。

## 6. 分阶段实施路线

| 阶段 | 周期 | 核心目标 | 主要交付 |
| --- | --- | --- | --- |
| M1 | 1-2 周 | 质量基线 | corpus 元数据、QualityRun schema、核心 smoke 回归、服务报告字段 |
| M2 | 3-5 周 | 语义覆盖 | 宏/环境能力矩阵、表格/图/公式/引用保真指标、semantic loss events |
| M3 | 6-8 周 | DOCX 稳定 | OOXML 校验、Word open regression、style 覆盖、视觉 diff 初版 |
| M4 | 9-12 周 | 服务化增强 | 持久队列、重试/死信、幂等扣费、稳定错误码、观测看板 |
| M5 | 13-16 周 | 商业发布 | Profile 包、API/SDK、企业配置、质量报告产品化、推广样例 |

推荐优先级：

1. 先做 `QualityRun` 与服务报告，快速形成可见价值。
2. 再补最影响用户感知的表格、公式、引用和 Word 可打开性。
3. 随后做 worker 可靠性和 SLA，支撑商业流量。
4. 最后扩展 Profile 商店和企业部署。

## 7. 模块级改造清单

| 模块 | 建议改造 | 风险 |
| --- | --- | --- |
| `crates/compiler-engine` | CompileReport V2、质量维度、fallback 审计、backend 版本记录 | 中，高调用面 |
| `crates/latex-reader` | 宏/环境能力矩阵、复杂表格和浮动体 lowering | 高，直接影响语义 |
| `crates/rule-engine` | Profile 规则包、未知宏分类、建议修复 | 中 |
| `crates/docx-writer` | OOXML 校验、style/numbering/media 完整性、表格 layout | 高，影响输出 |
| `crates/quality` | QualityRun、DOCX+PDF+Word 综合评分、JSON schema | 中 |
| `crates/compatibility-analyzer` | 从包级评分扩展到对象级风险预测 | 中 |
| `apps/rust-service` | 持久队列、幂等、错误码、报告存储、观测指标 | 中高 |
| `crates/commercial-api-client` | 报告 API、批量转换、webhook client | 低中 |
| `apps/slint-user` | 质量报告展示、失败建议、云端 job 状态细化 | 中 |

注意：对函数、类、方法进行实际代码修改前，必须按项目要求对目标符号运行 GitNexus impact 分析；若风险为 HIGH/CRITICAL，需先向用户说明影响面再动手。

## 8. 错误码与质量报告建议

建议新增稳定错误码：

| 错误码 | 含义 | 用户提示 |
| --- | --- | --- |
| `upload_invalid_zip` | zip 不合法或存在安全风险 | 请重新打包项目，避免嵌套危险路径 |
| `main_tex_not_found` | 主文件不存在 | 请指定 main.tex 或检查压缩包结构 |
| `preflight_unsupported_package` | 关键宏包不支持 | 可尝试 strict/enterprise 或移除不支持宏包 |
| `semantic_parse_failed` | 语义解析失败 | 查看报告中的源文件位置与宏命令 |
| `backend_runtime_unavailable` | TeX runtime 不可用 | 服务端降级或稍后重试 |
| `docx_render_failed` | DOCX 输出失败 | 联系支持并附带 job id |
| `word_compatibility_failed` | Word 打开/校验失败 | 已生成报告但不建议直接投稿 |
| `quality_gate_failed` | 质量门禁未通过 | 查看阻断项和修复建议 |
| `quota_exhausted` | 额度不足 | 购买或兑换额度 |
| `job_timeout` | 转换超时 | 尝试精简项目或使用企业队列 |

质量报告建议结构：

```json
{
  "job_id": "conv_xxx",
  "engine_version": "0.1.0",
  "profile": "jos-paper",
  "backend": {
    "requested": "auto",
    "selected": "rule-based",
    "fallback_from": "xelatex-hook",
    "reason": "runtime unavailable"
  },
  "quality_score": 88,
  "dimension_scores": {
    "parse": 95,
    "semantic": 86,
    "docx": 92,
    "visual": 80,
    "editable": 90,
    "performance": 85
  },
  "blocking_issues": [],
  "warnings": [],
  "semantic_loss_events": [],
  "artifacts": {
    "docx": "...",
    "report_json": "...",
    "log": "..."
  }
}
```

## 9. 主要风险与缓解

| 风险 | 表现 | 缓解 |
| --- | --- | --- |
| LaTeX 生态复杂度过高 | 长尾宏包无法完全支持 | Profile 聚焦、能力矩阵、明确 fallback 与限制说明 |
| 视觉一致性难量化 | 用户感觉“不像”但报告显示通过 | 引入 PDF/图片视觉 diff 与人工标注样例 |
| Word 兼容性环境差异 | WPS、Word、LibreOffice 行为不一致 | 分环境回归，至少保证 Word 主版本与 LibreOffice headless |
| Runtime backend 成本高 | XeLaTeX/LuaTeX 慢且资源占用高 | 队列分级、缓存、超时、企业队列计费 |
| 用户数据敏感 | 学术论文未发表内容泄露风险 | 默认短期留存、私有化部署、日志脱敏、访问审计 |
| Profile 维护成本 | 期刊模板频繁变更 | Profile 版本化、自动回归、用户上传样例反馈闭环 |

## 10. 下一步建议

建议立即启动三个并行工作包：

1. 质量平台工作包：定义 `QualityRun` schema，把 `crates/quality` 接入云端 worker，替换 DOCX 大小评分。
2. 核心样例工作包：整理 30 个 corpus 样例，建立 smoke/golden/visual 三层回归。
3. 服务可靠性工作包：补 job 状态机、幂等键、稳定错误码、重试与质量报告下载。

完成上述三项后，再进入复杂语义覆盖和 Profile 商店建设。这样能最快把当前项目从“工程演示可用”推进到“可度量交付、可支持付费用户”的商业化状态。
