# Semantic TeX Engine 详细进展报告（20260620-193855）
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



## 1. 报告范围

本报告基于当前最新架构规划：

```text
docs-zh/semantic-tex-engine-dual-backend-design-20260620-115348.md
```

以及当前开发计划：

```text
docs-zh/semantic-tex-engine-progress-and-task-plan.md
```

目标是汇总截至 2026-06-20 19:38 的实现进展、验证结果、剩余缺口和下一步计划安排。

当前代码状态：

```text
latest commit: 748a842 feat: add semantic collector output model
working tree: clean
```

## 2. 总体结论

当前项目已经具备一条可运行的新 Semantic TeX Engine 路径：

```text
TeX/CTeX Source
  -> doc-compiler-engine
  -> Backend Selector
  -> RuleBasedBackend / XeLaTeXHookBackend / LuaTeXNodeBackend
  -> CollectedDocument
  -> DocumentGraph
  -> DOCX Renderer + semantic post-process
  -> DOCX
```

它已经能够在 paper3 上输出可验证 DOCX，并与旧 Rust rule 路径和 sh 路径并行对比。

但它还不是完整的 Semantic TeX Engine。当前完成的是“可运行的独立语义引擎门面 + 双后端原型 + 学术论文关键能力初版”，剩余重点在：

- XDV parser。
- LuaTeX node/layout 语义增强。
- 独立 `semantic-collector` crate。
- 独立 `compatibility-analyzer` crate。
- AI/rule fallback。
- 外置 profile/rule 系统。
- 更高保真 DOCX renderer。

## 3. 已完成能力矩阵

| 模块 | 状态 | 当前证据 |
|---|---|---|
| 独立 semantic engine 路径 | 已完成初版 | `doc-compiler-engine` 可 source/dir/zip/VFS 到 DOCX |
| 旧 Rust rule 路径隔离 | 已完成 | `doc-core` 未依赖 `doc-compiler-engine`，`cargo test -p doc-core` 通过 |
| 双后端策略 | 已完成初版 | `RuleBasedBackend`、`XeLaTeXHookBackend`、`LuaTeXNodeBackend`、Auto selector 已落地 |
| Collector 输出模型 | 已完成内置初版 | `SemanticCollector`、`CollectedDocument`、`BuildSidecar` 已落地 |
| JOS / 中文学术 profile | 已完成初版 | `EngineProfile::JosPaper`、`ProfileSpec`、JOS page setup 已落地 |
| ReferenceGraph | 已完成初版 | label/ref/eqref/autoref/cite 结构化，paper3 unresolved=0 |
| DOCX bookmark/hyperlink | 已完成初版 | paper3 bookmarks=25，hyperlinks=35 |
| OMML 块级公式 | 已完成初版 | paper3 omml-equations=4，fallbacks=0 |
| 表格 span | 已完成初版 | `multicolumn`、基础 `multirow` 到 `gridSpan/vMerge` |
| 图片尺寸表达式 | 已完成初版 | `FigureSizing`、relative/absolute unit -> EMU |
| 兼容性分析器 | 已完成内置初版 | paper3 score=76，unsupported=0，warnings=2 |
| paper3 验证脚本 | 已完成初版 | 三路径、双引擎、semantic backend 对比脚本均已存在 |

## 4. 当前架构落地情况

### 4.1 编译阶段

当前 `doc-compiler-engine` 阶段为：

```text
SourceMount
CompatibilityAnalyze
IncludeGraph
TexParse
SemanticCollect
DocumentGraph
DocxRender
```

新增的关键报告字段包括：

```text
backend
compatibility
profile_spec
semantic_event_count
layout_node_count
sidecar_count
reference_label_count
reference_edge_count
citation_count
unresolved_reference_count
bookmark_count
hyperlink_count
omml_equation_count
omml_equation_fallback_count
```

### 4.2 后端选择

`SemanticBackendKind::Auto` 当前策略：

- 检测 `ctex` / `xeCJK` / `fontspec` / XeTeX 字体命令时优先 `XeLaTeXHookBackend`。
- 检测 LuaTeX 特征或通用 LaTeX 且 `lualatex` 可用时优先 `LuaTeXNodeBackend`。
- runtime 不可用或失败且允许 fallback 时回退 `RuleBasedBackend`。
- paper3 当前自动选择 `xelatex-hook`。

### 4.3 Collector 输出模型

当前已新增：

```rust
SemanticCollector
SemanticCollectorInput
CollectedDocument
BuildSidecar
RuleBasedCollector
```

`CollectedDocument` 作为后续拆出 `semantic-collector` crate 的中间模型，当前包含：

```text
legacy Document
optional StandardDocument
ImageAssets
SemanticEvent list
LayoutGraph
EngineDiagnostic list
BuildSidecar list
```

`SemanticBackendArtifact` 暂时作为 `CollectedDocument` 的类型别名保留。

## 5. paper3 最新验证结果

三路径验证脚本：

```bash
bash scripts/build_paper3_three_docx.sh 15
```

最新输出：

| path | docx | bytes | media |
|---|---|---:|---:|
| sh | `v15-论文稿件-jos-sh-20260620-192754.docx` | 3,079,377 | 10 |
| rust-rule | `v15-论文稿件-jos-20260620-192754-rust-rule.docx` | 3,055,363 | 10 |
| semantic-engine | `v15-论文稿件-jos-20260620-192754-semantic-engine-xelatex_hook.docx` | 3,057,574 | 10 |

semantic-engine 报告：

```text
compatibility-score: 76
compatibility-unsupported: 0
compatibility-warnings: 2
compatibility-custom-macros: 46
sidecars: 1
reference-labels: 35
reference-edges: 46
citations: 36
unresolved-references: 0
bookmarks: 25
hyperlinks: 35
omml-equations: 4
omml-equation-fallbacks: 0
backend-requested: xelatex-hook
backend-selected: xelatex-hook
profile-id: jos-paper
profile-page-setup: jos-paper3
```

双引擎对比脚本：

```bash
bash scripts/compare_paper3_dual_engines.sh 15
```

最新输出：

| engine | bytes | media | paragraphs | tables | drawings | text chars |
|---|---:|---:|---:|---:|---:|---:|
| rust-rule | 3,055,363 | 10 | 653 | 12 | 20 | 41,963 |
| semantic-engine auto | 3,057,574 | 10 | 653 | 12 | 20 | 42,535 |

对比报告：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-192735-dual-engines-comparison-report.md
```

## 6. 已执行验证

当前已执行过的关键验证：

```bash
cargo test -p doc-compiler-engine collector -- --nocapture
cargo test -p doc-compiler-engine sidecar -- --nocapture
cargo test -p doc-compiler-engine compatibility -- --nocapture
cargo test -p doc-compiler-engine
cargo test -p doc-core
bash scripts/compare_paper3_dual_engines.sh 15
bash scripts/build_paper3_three_docx.sh 15
```

最新结果：

```text
doc-compiler-engine collector: 1 passed
doc-compiler-engine sidecar: 1 passed
doc-compiler-engine compatibility: 2 passed
doc-compiler-engine: 21 passed, 1 ignored
doc-core: 5 passed
paper3 dual engines: generated
paper3 three-docx: generated
```

已知 warning：

- `doc-latex-reader` 有既存 unused/dead_code warning。
- `doc-docx-writer` 有既存 unused helper warning。
- `cargo fmt` 在 stable rustfmt 下会提示部分 nightly-only format options 无法启用。

这些 warning 不阻塞当前功能验证。

## 7. 仍未完成内容

### 7.1 T11 XDV parser

状态：待实现。

目标：

- 新增 `crates/xdv-parser`。
- 解析 XDV/DVI 指令头和最小 opcode 子集。
- 输出 FontDef、Glyph、SetChar、Push、Pop、Rule、Special 等结构。
- 暂不接入 DOCX，只做独立 parser 和 fixture 单测。

### 7.2 T12 LuaHook collector 增强

状态：待实现。

目标：

- 设计 LuaHook 输出协议 v2。
- 捕获 section、caption、label、includegraphics、tabular、equation、citation。
- 增强 LuaTeX node tree 采集，向 `LayoutGraph` 输出行、盒子、字体、字号等信息。

### 7.3 T13 AI fallback 与可审计规则库

状态：待实现。

目标：

- 对未知宏提供可选推断入口。
- 默认离线，不调用网络。
- AI 推断必须可审计、可缓存、可禁用。
- 引入 rule engine + optional LLM engine 双层结构。

### 7.4 crate 拆分

状态：待实现。

候选拆分：

```text
crates/semantic-collector
crates/compatibility-analyzer
crates/xdv-parser
```

当前这些能力仍位于 `doc-compiler-engine` 内部。

### 7.5 Profile 外置化

状态：待实现。

目标：

- 将当前 Rust 内置 `EngineProfile::spec()` 迁移为 YAML/TOML 可配置规则。
- 支持 JOS、中文学术、医学期刊、SCI/IEEE/Elsevier 等 profile 外置。
- profile 应包含文档类白名单、页面、字体、caption、引用、兼容性阈值、DOCX style 映射。

### 7.6 高保真 DOCX renderer 增强

状态：部分完成。

仍待增强：

- inline math OMML。
- Word 字段型交叉引用。
- 复杂表格多段落/脚注。
- TikZ 降级策略。
- minted/listings 样式保留。
- 更完整的 figure/caption/float 语义。

## 8. 下一步计划安排清单

### P1：T11 XDV parser 原型

优先级：最高。

任务清单：

- [ ] 新增 `crates/xdv-parser` crate。
- [ ] 在 workspace 注册 `doc-xdv-parser`。
- [ ] 定义 `XdvDocument`、`XdvCommand`、`FontDef`、`GlyphNode`、`RuleNode`、`SpecialNode`。
- [ ] 实现字节级 reader，支持 big-endian 数值读取。
- [ ] 解析 DVI/XDV preamble、bop/eop、push/pop、set_char、set_rule、font selection、font def、special。
- [ ] 增加 fixture 单测。
- [ ] 增加错误类型和 offset 诊断。
- [ ] 暂不接入 `doc-compiler-engine`，先保持独立可测。

验收：

```bash
cargo test -p doc-xdv-parser
```

### P2：LayoutGraph 数据接入设计

优先级：高。

任务清单：

- [ ] 定义 XDV glyph -> layout node 的转换层。
- [ ] 设计 `LayoutGlyph`、`LayoutLine`、`LayoutPage`。
- [ ] 保留 font id、glyph code、x/y 坐标、rule 尺寸、special 信息。
- [ ] 不直接影响 DOCX 渲染，先输出 `LayoutGraph` 统计。

验收：

```bash
cargo test -p doc-compiler-engine layout
```

### P3：LuaTeX collector v2

优先级：高。

任务清单：

- [ ] 定义 sidecar JSONL v2 schema。
- [ ] 增加 event version、source path、line、macro name。
- [ ] 捕获 caption/table/equation/graphics/citation/reference。
- [ ] 采集 post_linebreak_filter 的 box/glyph/font 信息。
- [ ] 与 XDV layout 数据统一到 `LayoutGraph`。

验收：

```bash
cargo test -p doc-compiler-engine luatex -- --ignored --nocapture
```

### P4：拆分 `semantic-collector`

优先级：中。

任务清单：

- [ ] 新增 `crates/semantic-collector`。
- [ ] 迁移 `SemanticCollector`、`CollectedDocument`、`BuildSidecar`。
- [ ] 保留 `doc-compiler-engine` facade。
- [ ] 确保 `doc-core` 不依赖新 crate。

验收：

```bash
cargo test -p doc-semantic-collector
cargo test -p doc-compiler-engine
cargo test -p doc-core
```

### P5：拆分 `compatibility-analyzer`

优先级：中。

任务清单：

- [ ] 新增 `crates/compatibility-analyzer`。
- [ ] 迁移 `CompatibilityReport`、`CompatibilityIssue`。
- [ ] 支持 profile-aware 阈值。
- [ ] 为 TikZ/minted/listings/biblatex/custom macro 增加规则表。

验收：

```bash
cargo test -p doc-compatibility-analyzer
cargo test -p doc-compiler-engine compatibility
```

### P6：AI fallback / Rule engine

优先级：中低，必须默认关闭。

任务清单：

- [ ] 定义 unknown macro audit record。
- [ ] 定义 rule cache 文件格式。
- [ ] 默认离线，不调用网络。
- [ ] 支持人工规则优先，AI 推断只作为可选插件。
- [ ] 所有推断结果进入 diagnostics/report。

验收：

```bash
cargo test -p doc-compiler-engine ai_fallback
```

### P7：Profile 外置化

优先级：中。

任务清单：

- [ ] 设计 profile YAML/TOML schema。
- [ ] 将 JOS profile 导出为外置样例。
- [ ] 支持内置 profile fallback。
- [ ] 支持命令行指定 profile file。

验收：

```bash
cargo test -p doc-compiler-engine profile
cargo run -p doc-compiler-engine --example paper3_to_docx -- --profile jos-paper ...
```

## 9. 风险与控制

| 风险 | 控制方式 |
|---|---|
| XDV 指令覆盖不足 | 先做 fixture 驱动，逐步扩大 opcode |
| LuaTeX 与 XeLaTeX 版式差异 | 保留双后端，不强行迁移 paper3 到 LuaTeX |
| 影响旧 Rust rule 路径 | 所有新能力默认留在 `doc-compiler-engine`，`doc-core` 测试必跑 |
| AI fallback 不可复现 | 默认关闭，结果缓存，可审计，可 diff |
| profile 规则污染 | JOS/中文/医学/通用 profile 分离，禁止隐式跨 profile 规则 |

## 10. 建议执行顺序

```text
1. T11 XDV parser 独立 crate
2. XDV -> LayoutGraph 最小转换
3. LuaTeX collector v2 schema
4. semantic-collector crate 拆分
5. compatibility-analyzer crate 拆分
6. profile 外置化
7. AI/rule fallback
8. DOCX renderer 高保真增强
```

下一步建议立即执行：

```text
T11 XDV parser 原型
```
