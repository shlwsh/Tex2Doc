# Semantic TeX Engine 进展报告与开发任务清单

> 快照日期：2026-06-20
>
> 对照文档：[semantic-tex-engine-docx-implementation-plan.md](./semantic-tex-engine-docx-implementation-plan.md)
>
> 独立双路径审核方案：[Semantic TeX Engine 独立 DOCX 转换路径方案（20260620-112803）](./semantic-tex-engine-independent-docx-plan-20260620-112803.md)
>
> 双后端语义采集审核方案：[Semantic TeX Engine 双后端语义采集方案（20260620-115348）](./semantic-tex-engine-dual-backend-design-20260620-115348.md)
>
> 本轮开发报告：[Semantic TeX Engine 开发进展报告（20260620-124347）](./semantic-tex-engine-development-report-20260620-124347.md)
>
> Auto selector 开发报告：[Semantic TeX Engine Auto Selector 开发进展报告（20260620-125915）](./semantic-tex-engine-development-report-20260620-125915.md)
>
> 双引擎对比脚本开发报告：[Semantic TeX Engine 双引擎对比脚本开发报告（20260620-130628）](./semantic-tex-engine-development-report-20260620-130628.md)

## 1. 当前结论

当前实现已经完成了 `doc-compiler-engine` 语义编译 facade，并能通过规则解析链把 TeX/CTeX 工程转换为 DOCX。它目前是“可运行的语义转换引擎门面”，还不是完整的 Semantic TeX Engine。

完成度判断：

| 里程碑 | 状态 | 说明 |
|---|---|---|
| M1 语义编译 facade | 已完成 | `doc-compiler-engine` 已支持 source/dir/zip/VFS 到 DOCX，并输出阶段报告 |
| M2 Profile 化 | 部分完成 | 已有 `EngineProfile` 枚举，但 profile 规则尚未外置，JOS 页眉页脚仍主要在 `doc-core/docx-writer` 侧 |
| M3 结构增强 | 部分完成 | 表格、图片、引用已有文本级/块级处理；显式 `ReferenceGraph`、bookmark、hyperlink、图片尺寸表达式尚未完成 |
| M4 公式引擎 | 部分完成 | `doc-mathml` 有 Math AST 与 OMML 输出；DOCX writer 块公式仍走文本化输出 |
| M5 LuaHook/XDV | 部分完成 | 已在 `doc-compiler-engine` 内实现 backend trait、XeLaTeX hook sidecar、LuaTeX node/macro sidecar 原型、Auto selector 和 fallback；尚未拆出 `semantic-collector`、`xdv-parser` crate |
| M6 兼容性与 AI fallback | 未开始 | 尚无 `compatibility-analyzer`、rule engine、LLM fallback |

## 2. 已落地内容

### 2.1 Workspace 与 crate

当前 workspace 已包含 15 个 crate：

```text
core
utils
semantic-ast
latex-reader
docx-writer
bib
mathml
compiler-engine
wasm
native
server
tex-facade
docx-pdf
quality
cli
```

其中与本方案直接相关的新增/关键 crate：

| crate | 包名 | 当前作用 |
|---|---|---|
| `crates/compiler-engine` | `doc-compiler-engine` | Semantic TeX Engine facade |
| `crates/tex-facade` | `doc-tex-facade` | 调用 xelatex/tectonic/latexmk 生成 oracle PDF |
| `crates/docx-pdf` | `doc-docx-pdf` | LibreOffice DOCX 到 PDF |
| `crates/quality` | `doc-quality` | 结构、文本、视觉质量对比 |
| `crates/cli` | `doc-engine` | V2 命令入口 |

### 2.2 `doc-compiler-engine`

已实现核心类型：

```rust
SemanticTexEngine
CompileOptions
CompileArtifact
DocumentGraph
CompileReport
EngineProfile
SemanticBackend
SemanticBackendKind
RuleBasedBackend
XeLaTeXHookBackend
LuaTeXNodeBackend
```

已实现入口：

```rust
compile_source_to_docx
compile_dir_to_docx
compile_zip_to_docx
compile_vfs_to_graph
compile_vfs_to_docx
```

当前编译阶段：

```text
SourceMount
IncludeGraph
TexParse
SemanticCollect
DocumentGraph
DocxRender
```

当前实现方式：

1. 输入统一挂载到 `VirtualFs`。
2. `SemanticBackendKind::Auto` 扫描 `.tex/.sty/.cls/.ltx` 的模板特征，并结合 `xelatex` / `lualatex` 可用性选择 backend。
3. `ctex` / `xeCJK` / `fontspec` / XeTeX 字体命令优先选择 `XeLaTeXHookBackend`。
4. LuaTeX 特征或通用 LaTeX 在 `lualatex` 可用时优先选择 `LuaTeXNodeBackend`。
5. runtime 不可用或失败且允许 fallback 时，回退 `RuleBasedBackend`。
6. `RuleBasedBackend` 使用 `IncludeGraph::build/join` 展开 `\input` / `\include`。
7. `parse_tex` 使用现有 Logos/Rowan 解析器。
8. `lower_to_document` 或 `lower_to_document_with_cite_map` 生成旧 `Document`。
9. `StandardDocument::from_legacy_document` 生成标准文档图。
10. `doc_docx_writer::pack_with_page_setup` 输出 DOCX。

当前双后端相关边界：

- `XeLaTeXHookBackend` 已能 materialize VFS、注入 hook tex、调用 `xelatex`、解析 JSONL sidecar，并在 paper3 上严格选中。
- `LuaTeXNodeBackend` 已能 materialize VFS、注入 Lua collector、调用 `lualatex`、采集 macro events 与 `post_linebreak_filter` 段落事件。
- 默认 `Auto` 已启用模板特征选择：paper3 这类 `ctex` / `xeCJK` 模板会选择 `XeLaTeXHookBackend`，通用 LaTeX 在 `lualatex` 可用时会选择 `LuaTeXNodeBackend`，无可用 runtime 时回退 `RuleBasedBackend`。
- 用户显式指定 runtime backend 且允许 fallback 时，runtime 失败会回退到 `RuleBasedBackend`，并输出 `backend_fallback` warning。
- 用户显式指定 runtime backend 且关闭 fallback 时，runtime 失败会返回错误，用于验证该 backend 是否真的可用。
- 已提供 `parse_semantic_events_jsonl`，作为 XeLaTeX/LuaTeX sidecar 协议解析入口。

### 2.3 paper3 样例

已存在脚本：

```bash
bash scripts/build_paper3_compiler_engine_docx.sh
bash scripts/compare_paper3_dual_engines.sh
bash scripts/compare_paper3_semantic_backends.sh
bash scripts/build_paper3_three_docx.sh
```

输出目录：

```text
examples/paper3/output/to-docx
```

已观察到的 compiler-engine 产物：

```text
v13-论文稿件-jos-20260620-080507-compiler-engine.docx
大小约 3.05 MB
```

`scripts/compare_paper3_semantic_backends.sh` 只比较新 `doc-compiler-engine` 内部的 backend 选择路径：

```text
auto
rule-based
xelatex-hook
luatex-node
```

它不会调用或修改旧 `doc-core` 路径；runtime backend 会实际尝试采集语义，失败时按配置 fallback。

`scripts/build_paper3_three_docx.sh` 是最终 paper3 三路径验证脚本，输出：

```text
sh
rust-rule
semantic-engine
```

到：

```text
examples/paper3/output/to-docx
```

`scripts/compare_paper3_dual_engines.sh` 是旧 Rust 规则引擎与新 Semantic Engine 的双路径对比脚本，输出：

```text
rust-rule DOCX
semantic-engine DOCX
document.xml 文本摘要
关键短语命中表
document 文本 diff
```

到：

```text
examples/paper3/output/to-docx
```

2026-06-20 最新三路径验证结果：

| 路径 | 文件 | 大小 | media |
|---|---|---:|---:|
| sh | `v15-论文稿件-jos-sh-20260620-125728.docx` | 3,079,377 bytes | 10 |
| rust-rule | `v15-论文稿件-jos-20260620-125728-rust-rule.docx` | 3,055,363 bytes | 10 |
| semantic-engine | `v15-论文稿件-jos-20260620-125728-semantic-engine-xelatex_hook.docx` | 3,055,688 bytes | 10 |

semantic-engine 后端报告：

```text
backend-requested: xelatex-hook
backend-selected: xelatex-hook
backend-reason: XeLaTeXHookBackend explicitly requested; xelatex-hook available: found /usr/bin/xelatex
```

2026-06-20 最新 semantic backend 对比结果：

| requested | selected | fallback_from | 文件 | 大小 | media |
|---|---|---|---|---:|---:|
| auto | xelatex-hook |  | `paper3-20260620-125747-auto.docx` | 3,055,688 bytes | 10 |
| rule-based | rule-based |  | `paper3-20260620-125747-rule_based.docx` | 3,055,688 bytes | 10 |
| xelatex-hook | xelatex-hook |  | `paper3-20260620-125747-xelatex_hook.docx` | 3,055,688 bytes | 10 |
| luatex-node | rule-based | luatex-node | `paper3-20260620-125747-luatex_node.docx` | 3,055,688 bytes | 10 |

2026-06-20 最新双引擎对比结果：

| engine | 文件 | 大小 | media | paragraphs | tables | drawings | text chars |
|---|---|---:|---:|---:|---:|---:|---:|
| rust-rule | `v15-论文稿件-jos-20260620-130548-dual-engines-rust-rule.docx` | 3,055,363 bytes | 10 | 653 | 12 | 20 | 41,963 |
| semantic-engine auto | `v15-论文稿件-jos-20260620-130548-dual-engines-semantic-engine-auto.docx` | 3,055,688 bytes | 10 | 653 | 12 | 20 | 42,744 |

对比报告：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-130548-dual-engines-comparison-report.md
```

结论：

- `semantic_backend=auto` 在 paper3 上选择 `xelatex-hook`。
- 关键短语 `基于动态关注清单`、`微服务日志`、`Dynamic Attention List`、`DASM`、`Loki`、`DSB-Lite`、`系统总体设计`、`实验与分析` 在两条路径中均命中。
- 两条路径的段落数、表格数、图片数一致；文本 diff 已输出到 `*-document-text.diff`，用于后续差异分析。

### 2.4 V2 质量闭环

`doc-engine` CLI 已包含：

```text
convert
tex-compile
docx-to-pdf
verify-pdf
build
ast-dump
render-dump
docx-diff
```

其中 `tex-compile` 使用 `doc-tex-facade` 调用外部 TeX 引擎生成 oracle PDF；`build` 串联 DOCX、oracle PDF、DOCX PDF 和质量报告。

## 3. 关键缺口

### 3.1 `doc-core` 保持为独立旧路径

当前 `doc-core::convert_sync`、`convert_zip`、`convert_dir` 仍直接使用旧链路：

```text
doc-latex-reader -> doc-semantic-ast::Document -> doc-docx-writer
```

这不是缺陷，而是后续设计约束：现有 Rust 版本 DOCX 转换引擎作为稳定基线保留，不迁移到 `doc-compiler-engine`。新 Semantic TeX Engine 作为第二条独立路径存在，用于生成可对照的 DOCX 产物。

### 3.2 Profile 仍是枚举，不是规则系统

当前 `EngineProfile` 仅提供：

```rust
GenericArticle
ChineseAcademic
JosPaper
MedicalJournal
```

缺少：

- 文档类白名单。
- front matter 抽取规则。
- 字体映射。
- caption 命名策略。
- 参考文献样式。
- DOCX style 映射。
- 兼容性评分阈值。

### 3.3 引用图未结构化

当前 `.bbl/.bib` 可以影响引用编号和参考文献段落，但引用仍主要是段落文本。尚未实现：

- `ReferenceGraph`。
- `label/ref/eqref/autoref/cite` 统一索引。
- DOCX bookmark。
- 内部 hyperlink。
- 未解析引用 diagnostics。

### 3.4 公式未完成端到端 OMML

`doc-mathml` 已有：

- LaTeX math parser。
- `MathExpr`。
- MathML 输出。
- OMML 输出。

但 `doc-docx-writer` 的块级公式当前仍是 JOS 风格文本段，不直接调用 `doc-mathml::to_omml`。因此 M4 只能算“基础能力已具备，端到端未接通”。

### 3.5 LuaHook/XDV/兼容性分析缺口

尚无以下 crate：

```text
crates/semantic-collector
crates/xdv-parser
crates/compatibility-analyzer
```

也没有：

- 独立 `semantic-collector` crate。
- 独立的 XeLaTeX hook/sidecar collector crate。
- 独立的 LuaTeX node callback collector crate。
- `XdvLayoutCollector`。
- AI-assisted macro inference。

注意：`doc-compiler-engine` 内部已经有可运行的 `SemanticBackend` trait、`RuleBasedBackend`、`XeLaTeXHookBackend`、`LuaTeXNodeBackend` 原型，但它们仍属于 engine 内部实现，尚未稳定为独立 collector crate。

## 4. 开发任务拆解

任务按依赖顺序排列。每个任务完成后必须更新本报告的状态和验证记录。

### T0 文档与基线锁定

状态：进行中

目标：

- 输出当前进展报告。
- 明确已实现、部分实现、未实现内容。
- 固定后续任务顺序。

验收：

- `docs-zh/semantic-tex-engine-progress-and-task-plan.md` 存在。
- 原技术方案能链接到本进展报告。
- 报告包含完成度矩阵、缺口、任务清单、验证记录。

### T1 建立独立 Semantic Engine 路径边界

状态：部分完成

目标：

- 保持 `doc-core`、WASM、Native、Server、`doc-engine convert/build` 现有路径不变。
- 为 `doc-compiler-engine` 建立独立 profile、独立测试、独立脚本和独立输出命名。
- 明确新旧两条 DOCX 生成路径的边界和验证方式。

实现要点：

- 不让 `doc-core` 依赖 `doc-compiler-engine`。
- 不修改 `doc-core::convert_sync/convert_zip/convert_dir` 的默认行为。
- 新路径需要旧逻辑时，优先复制或通过底层库复用，不改变旧路径输出。
- 新增 semantic engine paper3 E2E，作为新路径验收。
- 新增 semantic backend 对比脚本，验证新语义引擎内部后端选择和 fallback 行为。

验收：

```bash
cargo test -p doc-core
cargo test -p doc-compiler-engine
bash scripts/build_paper3_compiler_engine_docx.sh
bash scripts/compare_paper3_semantic_backends.sh
bash scripts/build_paper3_three_docx.sh
```

### T2 双路径对比脚本与报告

状态：已完成初版

目标：

- 旧 Rust 引擎和新 Semantic Engine 对同一 paper3 输入分别生成 DOCX。
- 输出文件大小、media 数量、document.xml 文本摘要、关键短语命中和差异报告。
- 对比脚本不改变两条路径实现。

实现要点：

- 已新增 `scripts/compare_paper3_dual_engines.sh`。
- 旧路径继续调用现有 `doc-engine convert`，不依赖 `doc-compiler-engine`。
- 新路径调用 `doc-compiler-engine` 的 `paper3_to_docx` example。
- 报告输出到 `examples/paper3/output/to-docx`。
- 报告包含 DOCX 文件大小、zip part 数、media 数、paragraph/table/drawing 数、document.xml 文本摘要、关键短语命中表和 unified diff。

验收：

```bash
bash scripts/compare_paper3_dual_engines.sh
```

### T3 Profile 规则表

状态：待实现

目标：

- 将 `EngineProfile` 从简单枚举扩展为可查询规则。
- JOS、中文学术、医学期刊拥有明确页面、字体、caption、引用策略。

实现要点：

- 新增 `ProfileSpec`。
- 增加 `EngineProfile::spec()`。
- 把 JOS 默认 `PageSetup::jos_paper3()` 纳入 `JosPaper` profile。
- 为后续 YAML/TOML 外置规则预留结构。

验收：

```bash
cargo test -p doc-compiler-engine profile
```

### T4 引用图与交叉引用

状态：待实现

目标：

- 新增 `ReferenceGraph`。
- 结构化 `label/ref/eqref/autoref/cite`。
- 对未解析引用输出 diagnostics。

实现要点：

- 在 `doc-semantic-ast` 或 `doc-compiler-engine` 中定义 `ReferenceGraph`。
- 将 `collect_label_map` 的结果提升到 Document Graph。
- 保持当前文本替换能力不回退。

验收：

```bash
cargo test -p doc-latex-reader ref
cargo test -p doc-compiler-engine reference
```

### T5 DOCX bookmark/hyperlink

状态：待实现

目标：

- heading、figure、table、equation、algorithm 支持 bookmark。
- `\ref` 类引用可渲染为内部 hyperlink。

实现要点：

- 在 `doc-docx-writer` 增加 bookmark writer。
- 增加关系与字段测试。
- 对无法解析目标的引用保留纯文本 fallback。

验收：

```bash
cargo test -p doc-docx-writer bookmark
```

### T6 公式 OMML 端到端接入

状态：待实现

目标：

- `Block::Equation` 使用 `doc-mathml` 生成 OMML。
- 未支持公式保持文本 fallback，并输出 diagnostics。

实现要点：

- 让 `doc-docx-writer` 依赖 `doc-mathml`，或在 `doc-compiler-engine` 阶段预渲染公式。
- 保留 JOS 公式编号。
- 增加 `\frac`、`\sqrt`、上下标、矩阵、cases 的 DOCX XML 断言。

验收：

```bash
cargo test -p doc-mathml
cargo test -p doc-docx-writer equation
```

### T7 表格增强

状态：待实现

目标：

- 完整支持 `multicolumn`、基础 `multirow`。
- 改善列宽推断。
- 表格内公式、段落、脚注可降级保留。

实现要点：

- 扩展 `TableCell` 结构，明确 `grid_span`、`v_merge`。
- DOCX writer 输出 `w:gridSpan`、`w:vMerge`。
- 保持 booktabs/hline 基础边框策略。

验收：

```bash
cargo test -p doc-latex-reader table
cargo test -p doc-docx-writer table
```

### T8 图片尺寸表达式

状态：待实现

目标：

- 捕获 `\includegraphics[width=.8\textwidth,height=...,scale=...]`。
- Document Graph 保存原始尺寸表达式和归一化尺寸。

实现要点：

- 扩展 `Block::Figure` 或新增标准 graph resource metadata。
- 在 DOCX renderer 中按 page/profile 转换 EMU 尺寸。
- 无法计算时保留当前默认尺寸。

验收：

```bash
cargo test -p doc-latex-reader figure
cargo test -p doc-docx-writer image
```

### T9 兼容性分析器

状态：待实现

目标：

- 新增 `crates/compatibility-analyzer`。
- 编译前扫描宏包、文档类、自定义宏、TikZ、minted/listings。
- 输出 score、unsupported、warnings。

验收：

```bash
cargo test -p doc-compatibility-analyzer
```

### T10 Semantic Collector trait

状态：部分完成

目标：

- 新增 collector trait，把当前规则降级封装为 `RuleBasedCollector`。
- `SemanticTexEngine` 不再直接绑定 `doc-latex-reader` lowering 细节。

实现要点：

- 可先在 `doc-compiler-engine` 内定义 trait，稳定后再拆 crate。
- `RuleBasedCollector` 输出 `DocumentGraph` 或中间 `CollectedDocument`。

当前落地：

- 已在 `doc-compiler-engine` 内定义 `SemanticBackend` trait。
- 已将现有规则解析链封装为 `RuleBasedBackend`。
- 已实现 `XeLaTeXHookBackend` 最小可运行原型。
- 已实现 `LuaTeXNodeBackend` 最小可运行原型。
- 尚未拆分独立 `semantic-collector` crate。
- LuaTeX node tree 当前只输出段落文本事件，layout 坐标、box 尺寸与行/页聚类尚未落地。

验收：

```bash
cargo test -p doc-compiler-engine collector
```

## 4.1 双后端计划状态（2026-06-20 更新）

对照审核方案：[Semantic TeX Engine 双后端语义采集方案（20260620-115348）](./semantic-tex-engine-dual-backend-design-20260620-115348.md)

| 编号 | 状态 | 当前实现 |
|---|---|---|
| B0 方案审核 | 已完成文档草案 | 已输出带时间戳的双后端设计方案，明确新路径独立于 `doc-core` |
| B1 Backend trait 与报告字段 | 已完成初版 | `SemanticBackend`、`SemanticBackendKind`、`BackendSelectionReport`、`CompileReport.backend` 已落地 |
| B2 XeLaTeXHookBackend 原型 | 已完成初版 | 已生成 hook tex、调用 `xelatex`、解析 sidecar；paper3 严格选中 `xelatex-hook` |
| B3 LuaTeXNodeBackend 原型 | 已完成初版 | 已生成 Lua collector、调用 `lualatex`、采集 heading/paragraph/label/ref/cite/equation 等事件；paper3 因 `xeCJK` 强制 XeTeX 按设计 fallback |
| B4 Auto selector | 已完成初版 | `Auto` 已根据模板特征和 runtime 可用性选择 `XeLaTeXHookBackend` / `LuaTeXNodeBackend` / `RuleBasedBackend`；paper3 自动选择 `xelatex-hook` |
| B5 双 runtime 后端对比脚本 | 已完成初版 | 已新增 `scripts/compare_paper3_semantic_backends.sh` 与 `scripts/build_paper3_three_docx.sh` |
| B6 LayoutGraph 与 XDV/Lua layout 合并 | 数据结构初版 | `LayoutGraph`/`LayoutNode` 已预留；尚无 XDV/Lua layout 实际采集 |

### T11 XDV parser 原型

状态：待实现

目标：

- 新增 `crates/xdv-parser`。
- 解析 FontDef、Glyph、SetChar、Push、Pop、Rule、Special 的最小子集。

验收：

```bash
cargo test -p doc-xdv-parser
```

### T12 LuaHook collector 原型

状态：待实现

目标：

- 设计 LuaHook 输出协议。
- 捕获 section、caption、label、includegraphics、tabular 的语义 special。

验收：

```bash
cargo test -p doc-compiler-engine luahook
```

### T13 AI fallback 与可审计规则库

状态：待实现

目标：

- 对未知宏提供可选的外部推断入口。
- 所有推断结果必须可审计、可缓存、可禁用。

验收：

- 默认离线模式不调用网络。
- 未配置 AI 时行为完全确定。
- 规则缓存可 diff、可复现。

## 5. 推荐执行顺序

```text
T0 文档与基线锁定
T1 独立 Semantic Engine 路径边界
T2 双路径对比脚本与报告
T3 Profile 规则表
T4 ReferenceGraph
T5 DOCX bookmark/hyperlink
T6 公式 OMML 端到端
T7 表格增强
T8 图片尺寸表达式
T9 兼容性分析器
T10 Semantic Collector trait
T11 XDV parser 原型
T12 LuaHook collector 原型
T13 AI fallback
```

理由：

- T1/T2 先建立新旧引擎独立边界与对比机制，后续增强只进入新 Semantic Engine 路径。
- T3 先明确 profile 行为，避免 JOS、医学、通用论文互相污染。
- T4/T5 是学术论文高保真的核心，优先级高于 XDV。
- T6/T7/T8 是 DOCX 可见质量主要来源。
- T9 先给用户预期，再进入 T10-T13 的高级 collector 与 fallback。

## 6. 当前验证记录

已执行：

```bash
cargo fmt -p doc-compiler-engine
cargo test -p doc-compiler-engine
cargo test -p doc-compiler-engine luatex_runtime_collects_semantic_events -- --ignored --nocapture
cargo test -p doc-core
bash scripts/build_paper3_three_docx.sh 15
bash scripts/compare_paper3_dual_engines.sh 15
bash scripts/compare_paper3_semantic_backends.sh
```

结果：

```text
doc-compiler-engine: 10 passed, 1 ignored
doc-compiler-engine luatex ignored integration: 1 passed
doc-core: 5 passed
paper3 three-docx: sh/rust-rule/semantic-engine generated
paper3 dual engines: rust-rule/semantic-engine generated, comparison report and text diff generated
paper3 semantic backend compare: auto/rule-based/xelatex-hook/luatex-node generated
```

已知 warning：

- `doc-latex-reader` 存在 unused/dead_code warning。
- `doc-docx-writer` 存在公式相关 unused warning。

这些 warning 不阻塞当前测试，但应在 T6 公式端到端接入时清理。

## 7. 下一步执行项

下一步建议进入 T3：

```text
T3 Profile 规则表
```

具体先做：

1. 为 `EngineProfile` 增加 `ProfileSpec`，把 JOS、中文学术、医学期刊的页面、字体、caption、引用策略内聚到新语义路径。
2. 保持 `doc-core`、`doc-engine convert` 旧路径不依赖 `doc-compiler-engine`。
3. 跑 `cargo test -p doc-core -p doc-compiler-engine` 与 paper3 脚本，确认新旧路径仍可独立验证。
