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
>
> ProfileSpec 开发报告：[Semantic TeX Engine ProfileSpec 开发报告（20260620-131521）](./semantic-tex-engine-development-report-20260620-131521.md)
>
> ReferenceGraph 开发报告：[Semantic TeX Engine ReferenceGraph 开发报告（20260620-132709）](./semantic-tex-engine-development-report-20260620-132709.md)
>
> DOCX bookmark/hyperlink 开发报告：[Semantic TeX Engine DOCX 引用链接开发报告（20260620-134723）](./semantic-tex-engine-development-report-20260620-134723.md)
>
> OMML 公式接入开发报告：[Semantic TeX Engine OMML 公式接入开发报告（20260620-135937）](./semantic-tex-engine-development-report-20260620-135937.md)
>
> 表格 span 渲染开发报告：[Semantic TeX Engine 表格 span 渲染开发报告（20260620-182504）](./semantic-tex-engine-development-report-20260620-182504.md)
>
> 图片尺寸表达式开发报告：[Semantic TeX Engine 图片尺寸表达式开发报告（20260620-184710）](./semantic-tex-engine-development-report-20260620-184710.md)

## 1. 当前结论

当前实现已经完成了 `doc-compiler-engine` 语义编译 facade，并能通过规则解析链把 TeX/CTeX 工程转换为 DOCX。它目前是“可运行的语义转换引擎门面”，还不是完整的 Semantic TeX Engine。

完成度判断：

| 里程碑 | 状态 | 说明 |
|---|---|---|
| M1 语义编译 facade | 已完成 | `doc-compiler-engine` 已支持 source/dir/zip/VFS 到 DOCX，并输出阶段报告 |
| M2 Profile 化 | 部分完成 | 已有 `ProfileSpec` 初版，JOS/中文学术/医学期刊具备页面、字体、caption、引用策略；规则尚未 YAML/TOML 外置 |
| M3 结构增强 | 部分完成 | 表格已支持 `multicolumn` 与基础 `multirow` 到 DOCX `gridSpan/vMerge`；图片已支持 `includegraphics` 尺寸表达式采集、Graph 元数据和 DOCX EMU 换算初版；`ReferenceGraph` 初版已结构化 label/ref/eqref/autoref/cite；语义 DOCX 后处理已支持 bookmark/hyperlink 初版 |
| M4 公式引擎 | 部分完成 | `doc-mathml` 有 Math AST 与 OMML 输出；新语义路径已在 DOCX 后处理阶段接入块级公式 OMML 初版；旧 `doc-docx-writer` 默认块公式仍走文本化输出 |
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
11. `doc-compiler-engine` 语义路径可选执行 DOCX 后处理，把块级公式段落替换为 OMML，并根据 `ReferenceGraph` 为目标段落写入 bookmark、把已解析引用写为内部 hyperlink。

当前双后端相关边界：

- `XeLaTeXHookBackend` 已能 materialize VFS、注入 hook tex、调用 `xelatex`、解析 JSONL sidecar，并在 paper3 上严格选中。
- `LuaTeXNodeBackend` 已能 materialize VFS、注入 Lua collector、调用 `lualatex`、采集 macro events 与 `post_linebreak_filter` 段落事件。
- 默认 `Auto` 已启用模板特征选择：paper3 这类 `ctex` / `xeCJK` 模板会选择 `XeLaTeXHookBackend`，通用 LaTeX 在 `lualatex` 可用时会选择 `LuaTeXNodeBackend`，无可用 runtime 时回退 `RuleBasedBackend`。
- 用户显式指定 runtime backend 且允许 fallback 时，runtime 失败会回退到 `RuleBasedBackend`，并输出 `backend_fallback` warning。
- 用户显式指定 runtime backend 且关闭 fallback 时，runtime 失败会返回错误，用于验证该 backend 是否真的可用。
- 已提供 `parse_semantic_events_jsonl`，作为 XeLaTeX/LuaTeX sidecar 协议解析入口。

当前 profile 规则边界：

- `EngineProfile::spec()` 已返回 `ProfileSpec`。
- `ProfileSpec` 包含 document class 白名单、默认页面设置、字体策略、caption 策略和引用策略。
- `JosPaper` profile 已内置 `PageSetup::jos_paper3()`，`paper3_to_docx` example 不再手写 JOS 页面设置。
- `CompileReport.profile_spec` 会输出本次编译使用的 profile 规则摘要。
- 规则仍为 Rust 内置表，尚未外置到 YAML/TOML。

当前引用图边界：

- `doc-compiler-engine` 已定义 `ReferenceGraph`。
- 新语义路径会从 VFS TeX 源和 runtime semantic events 合并 label/ref/eqref/autoref/cite。
- `CompileReport` 会输出 label、cross-reference、citation、unresolved reference 计数。
- 未解析引用会进入 `EngineDiagnostic`，code 为 `unresolved_reference`。
- DOCX bookmark/hyperlink 已在新语义路径中接入初版：不修改 `doc-core` 与 `doc-docx-writer` 默认输出，而是在 `doc-compiler-engine` 打包后对 `word/document.xml` 做独立后处理。
- `CompileReport` 已新增 `bookmark_count`、`hyperlink_count`。

当前公式 OMML 边界：

- `doc-compiler-engine` 已直接依赖 `doc-mathml`。
- 新语义路径已新增 `CompileOptions.enable_omml_equations`，默认启用。
- 块级 `Block::Equation { is_block: true }` 会在打包后从 `JOSCode` 公式段替换为 `<m:oMath>`。
- 公式编号继续以普通 `w:t` 文本保留，方便 Word 显示和后续引用目标定位。
- 旧 `doc-docx-writer` 默认 `write_equation` 行为不变，旧 `doc-core` 路径仍输出 JOSCode 纯文本公式。
- `CompileReport` 已新增 `omml_equation_count`、`omml_equation_fallback_count`。

当前表格 span 边界：

- `TableCell` 已有 `colspan` / `rowspan` 字段，本轮没有修改 AST 结构。
- `doc-latex-reader::lower_table` 已支持 `\multicolumn{n}{spec}{text}` 和基础 `\multirow{n}{width}{text}`。
- `\multirow` 首行写入 `rowspan=n`，后续空占位单元格写入 `rowspan=0`，作为 DOCX `vMerge continue` 语义。
- `doc-docx-writer` 已按逻辑列数推断 `tblGrid`，并为跨列单元格输出加宽后的 `w:tcW` 与 `w:gridSpan`。
- `doc-docx-writer` 已输出 `w:vMerge w:val="restart"` / `continue`。
- 复杂 `multirow` 变体、跨行跨列组合冲突、表格内多段落与脚注仍待后续增强。

当前图片尺寸表达式边界：

- `Block::Figure` 已新增可选 `FigureSizing` 元数据，保存 `source_options`、`width_expr`、`height_expr`、`scale_expr` 与相对比例。
- `doc-latex-reader` 已能从 `\includegraphics[width=.8\textwidth,height=...,scale=...]{...}` 中采集 path 和 options。
- `StandardDocument` 的 `FigureNode` 已保留 sizing；`LayoutHints.width` 会写入类似 `0.8000\textwidth` 的归一化宽度提示。
- `doc-docx-writer` 已按 `PageSetup` 将 `\textwidth`、`\linewidth`、`\columnwidth`、`\textheight` 以及 `in/cm/mm/pt/bp/pc` 转为 EMU。
- `XeLaTeXHookBackend` 的 hook 已补充 `includegraphics` figure event，`width_expr` 与 LuaTeX sidecar 协议对齐。
- 无法解析尺寸表达式时仍回退旧的图片默认尺寸策略。

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
| sh | `v15-论文稿件-jos-sh-20260620-184633.docx` | 3,079,377 bytes | 10 |
| rust-rule | `v15-论文稿件-jos-20260620-184633-rust-rule.docx` | 3,055,363 bytes | 10 |
| semantic-engine | `v15-论文稿件-jos-20260620-184633-semantic-engine-xelatex_hook.docx` | 3,057,574 bytes | 10 |

semantic-engine 后端报告：

```text
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
backend-reason: XeLaTeXHookBackend explicitly requested; xelatex-hook available: found /usr/bin/xelatex
profile-id: jos-paper
profile-page-setup: jos-paper3
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
| rust-rule | `v15-论文稿件-jos-20260620-184633-dual-engines-rust-rule.docx` | 3,055,363 bytes | 10 | 653 | 12 | 20 | 41,963 |
| semantic-engine auto | `v15-论文稿件-jos-20260620-184633-dual-engines-semantic-engine-auto.docx` | 3,057,574 bytes | 10 | 653 | 12 | 20 | 42,535 |

对比报告：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-184633-dual-engines-comparison-report.md
```

结论：

- `semantic_backend=auto` 在 paper3 上选择 `xelatex-hook`。
- semantic log 已输出 `profile-id: jos-paper` 与 `profile-page-setup: jos-paper3`。
- ReferenceGraph 统计为 `reference-labels=35`、`reference-edges=46`、`citations=36`、`unresolved-references=0`。
- 语义 DOCX 链接统计为 `bookmarks=25`、`hyperlinks=35`。
- 语义 DOCX 公式统计为 `omml-equations=4`、`omml-equation-fallbacks=0`。
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

当前 `EngineProfile` 已提供：

```rust
GenericArticle
ChineseAcademic
JosPaper
MedicalJournal
```

并通过 `EngineProfile::spec()` 暴露：

- 文档类白名单。
- 默认页面设置。
- 字体策略。
- caption 命名策略。
- 引用策略。

仍缺少：

- front matter 抽取规则。
- DOCX style 映射。
- 兼容性评分阈值。
- YAML/TOML 外置规则。

### 3.3 引用图与 DOCX 内部链接已完成初版

当前 `.bbl/.bib` 可以影响引用编号和参考文献段落；新语义路径已经额外构建 `ReferenceGraph` 初版：

- `ReferenceGraph`。
- `label/ref/eqref/autoref/cite` 统一索引。
- 未解析引用 diagnostics。

当前已实现：

- `ReferenceGraph` 驱动 DOCX bookmark 初版。
- 已解析 `CrossReference` 驱动内部 hyperlink 初版。
- 未解析引用继续走 diagnostics 与纯文本 fallback。

仍待增强：

- Word 字段型交叉引用。
- heading/equation 的更精细目标定位。
- 引用源位置到 DOCX 段落的确定性映射。

### 3.4 公式 OMML 已完成语义路径初版

`doc-mathml` 已有：

- LaTeX math parser。
- `MathExpr`。
- MathML 输出。
- OMML 输出。

当前新语义路径已经在 `doc-compiler-engine` 中把块级 `Block::Equation` 接入 `doc-mathml::to_omml`，通过 DOCX 后处理替换 `JOSCode` 公式段。

仍保留的边界：

- `doc-docx-writer` 默认块级公式仍是 JOS 风格文本段，用于保持旧 `doc-core` 路径不变。
- inline math 尚未接入 OMML。
- 复杂公式 parser 仍会降级为 `MathExpr::Raw` 或简化 AST，后续需要提升覆盖率。

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

状态：已完成初版

目标：

- 将 `EngineProfile` 从简单枚举扩展为可查询规则。
- JOS、中文学术、医学期刊拥有明确页面、字体、caption、引用策略。

实现要点：

- 已新增 `ProfileSpec`、`FontPolicySpec`、`CaptionPolicySpec`、`CitationPolicySpec`、`PageSetupProfile`。
- 已增加 `EngineProfile::spec()`。
- 已把 JOS 默认 `PageSetup::jos_paper3()` 纳入 `JosPaper` profile。
- `CompileOptions::effective_page_setup()` 会在未显式覆盖时使用 profile 默认页面。
- `CompileReport.profile_spec` 会输出可审计的 profile 摘要。
- `paper3_to_docx` example 已去掉手写 JOS page setup，改由 `JosPaper` profile 提供。
- 已为后续 YAML/TOML 外置规则预留结构。

验收：

```bash
cargo test -p doc-compiler-engine profile
```

### T4 引用图与交叉引用

状态：已完成初版

目标：

- 新增 `ReferenceGraph`。
- 结构化 `label/ref/eqref/autoref/cite`。
- 对未解析引用输出 diagnostics。

实现要点：

- 已在 `doc-compiler-engine` 中定义 `ReferenceGraph`、`ReferenceLabel`、`CrossReference`、`CitationReference`、`UnresolvedReference`。
- 已从 VFS TeX 源轻量扫描 `label/ref/eqref/autoref/cite`，并合并 runtime semantic events。
- 已将 `ReferenceGraph` 挂到 `DocumentGraph`。
- 已在 `CompileReport` 输出引用图统计。
- 已对未解析引用输出 `unresolved_reference` diagnostics。
- 当前文本级引用替换能力保持不回退。

验收：

```bash
cargo test -p doc-latex-reader ref
cargo test -p doc-compiler-engine reference
```

### T5 DOCX bookmark/hyperlink

状态：已完成初版

目标：

- heading、figure、table、equation、algorithm 支持 bookmark。
- `\ref` 类引用可渲染为内部 hyperlink。

实现要点：

- 已选择独立后处理方案：只在 `doc-compiler-engine` 语义路径中处理 `word/document.xml`，不改变 `doc-docx-writer` 和 `doc-core` 默认输出。
- 已新增 `CompileOptions.enable_reference_links`，默认在语义路径启用。
- 已新增 `CompileReport.bookmark_count` 与 `CompileReport.hyperlink_count`。
- 已实现两阶段链接：先按 caption/heading/equation 等目标段落写 bookmark，再把已解析引用写为 `w:hyperlink w:anchor`。
- 已避免把 `<w:pgSz>` 等页设置标签误判为段落。
- 已收紧 figure/table/algorithm/theorem/proposition 的引用匹配，避免裸数字误链接到文献引用。
- 对无法解析目标的引用继续保留纯文本 fallback。

验收：

```bash
cargo test -p doc-compiler-engine bookmark
cargo test -p doc-compiler-engine
bash scripts/build_paper3_three_docx.sh 15
bash scripts/compare_paper3_dual_engines.sh 15
```

### T6 公式 OMML 端到端接入

状态：已完成初版

目标：

- `Block::Equation` 使用 `doc-mathml` 生成 OMML。
- 未支持公式保持文本 fallback，并输出 diagnostics。

实现要点：

- 已选择在 `doc-compiler-engine` 阶段做 DOCX 后处理，避免改变旧 `doc-docx-writer` 默认行为。
- 已新增 `CompileOptions.enable_omml_equations`，默认启用。
- 已新增 `CompileReport.omml_equation_count`、`CompileReport.omml_equation_fallback_count`。
- 已将块级 `Block::Equation` 通过 `doc_mathml::parse_latex_math` 与 `doc_mathml::to_omml` 生成 `<m:oMath>`。
- 已保留 JOS 公式编号为普通 `w:t` 文本。
- 已让 equation bookmark 目标匹配接受 `JOSCode` 公式段，使 OMML 公式段可作为引用目标。
- 已补充 `\frac` 的 DOCX XML 断言；`\sqrt`、上下标、矩阵已有 `doc-mathml` 单元测试，DOCX XML 断言后续继续扩展。

验收：

```bash
cargo test -p doc-compiler-engine omml
cargo test -p doc-compiler-engine
cargo test -p doc-mathml
cargo test -p doc-docx-writer block_equation_uses_jos_code_plain_text
bash scripts/build_paper3_three_docx.sh 15
bash scripts/compare_paper3_dual_engines.sh 15
```

### T7 表格增强

状态：已完成初版

目标：

- 支持 `multicolumn`、基础 `multirow`。
- 改善列宽推断。
- 表格内公式、段落、脚注可降级保留。

实现要点：

- 未修改 `TableCell` 结构，复用已有 `colspan` / `rowspan` 字段，保持旧 `doc-core` 路径模型兼容。
- `doc-latex-reader::lower_table` 已支持 `\multicolumn{n}{spec}{text}` 的列索引推进。
- `doc-latex-reader::lower_table` 已新增基础 `\multirow{n}{width}{text}` 解析，后续空占位单元格用 `rowspan=0` 表示 `vMerge continue`。
- `doc-docx-writer::write_table` 已按逻辑列数计算 `tblGrid`，跨列单元格宽度按 `colspan` 放大。
- DOCX writer 已输出 `w:gridSpan`、`w:vMerge restart/continue`。
- booktabs/hline 基础边框策略保持不变。
- 表格内复杂多段落、脚注与复杂跨行跨列组合仍是后续增强项。

验收：

```bash
cargo test -p doc-latex-reader multi -- --nocapture
cargo test -p doc-latex-reader table
cargo test -p doc-docx-writer table_colspan_and_rowspan_emit_grid_span_and_vmerge -- --nocapture
cargo test -p doc-docx-writer table
cargo test -p doc-core
cargo test -p doc-compiler-engine
bash scripts/build_paper3_three_docx.sh 15
bash scripts/compare_paper3_dual_engines.sh 15
```

### T8 图片尺寸表达式

状态：已完成初版

目标：

- 捕获 `\includegraphics[width=.8\textwidth,height=...,scale=...]`。
- Document Graph 保存原始尺寸表达式和归一化尺寸。

实现要点：

- 已新增 `FigureSizing`，保存原始 options、width/height/scale 表达式和归一化比例。
- 已扩展 `Block::Figure` 与 `FigureNode`，标准文档图保存 sizing，`LayoutHints.width` 保存归一化宽度提示。
- `doc-latex-reader` 已解析 `includegraphics` optional arguments。
- `doc-docx-writer` 已按 `PageSetup` 把相对正文尺寸和绝对单位转换为 EMU。
- `XeLaTeXHookBackend` 已补充 `includegraphics` sidecar event，LuaTeX 已有同类事件。
- 无法计算时保留当前默认尺寸。

验收：

```bash
cargo test -p doc-latex-reader figure
cargo test -p doc-docx-writer image
cargo test -p doc-semantic-ast
cargo test -p doc-core
cargo test -p doc-compiler-engine
bash scripts/build_paper3_three_docx.sh 15
bash scripts/compare_paper3_dual_engines.sh 15
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
cargo check -p doc-compiler-engine
cargo test -p doc-compiler-engine profile
cargo test -p doc-compiler-engine reference
cargo test -p doc-compiler-engine omml -- --nocapture
cargo test -p doc-compiler-engine
cargo test -p doc-mathml
cargo test -p doc-docx-writer block_equation_uses_jos_code_plain_text
cargo test -p doc-latex-reader multi -- --nocapture
cargo test -p doc-latex-reader table
cargo test -p doc-docx-writer table_colspan_and_rowspan_emit_grid_span_and_vmerge -- --nocapture
cargo test -p doc-docx-writer table
cargo test -p doc-semantic-ast
cargo test -p doc-latex-reader figure -- --nocapture
cargo test -p doc-docx-writer image -- --nocapture
cargo test -p doc-docx-writer figure
cargo test -p doc-compiler-engine luatex_runtime_collects_semantic_events -- --ignored --nocapture
cargo test -p doc-latex-reader ref
cargo test -p doc-core
bash scripts/build_paper3_three_docx.sh 15
bash scripts/compare_paper3_dual_engines.sh 15
bash scripts/compare_paper3_semantic_backends.sh
```

结果：

```text
doc-compiler-engine profile: 2 passed
doc-compiler-engine reference: 2 passed
doc-compiler-engine bookmark: 1 passed
doc-compiler-engine omml: 1 passed
doc-compiler-engine: 17 passed, 1 ignored
doc-mathml: 19 passed
doc-docx-writer block equation legacy behavior: 1 passed
doc-latex-reader multi: 7 passed
doc-latex-reader table: 7 unit tests plus snapshot_table passed
doc-docx-writer table_colspan_and_rowspan_emit_grid_span_and_vmerge: 1 passed
doc-docx-writer table: 6 passed
doc-semantic-ast: 13 passed
doc-latex-reader figure: 2 passed
doc-docx-writer image: 1 passed
doc-docx-writer figure: 2 passed
doc-compiler-engine luatex ignored integration: 1 passed
doc-latex-reader ref: 9 passed
doc-core: 5 passed
paper3 three-docx: sh/rust-rule/semantic-engine generated, semantic bookmarks=25, hyperlinks=35, omml-equations=4, omml-equation-fallbacks=0
paper3 dual engines: rust-rule/semantic-engine generated, paragraphs=653, tables=12, drawings=20 on both paths; comparison report, reference graph counts, bookmark/hyperlink counts, OMML counts and text diff generated
paper3 semantic backend compare: auto/rule-based/xelatex-hook/luatex-node generated
```

已知 warning：

- `doc-latex-reader` 存在 unused/dead_code warning。
- `doc-docx-writer` 存在 `formula_runs` / `clean_formula_latex` unused warning。

这些 warning 不阻塞当前测试；旧 writer 的公式纯文本路径仍保留，因此 unused helper 是否删除需要后续单独评估。

## 7. 下一步执行项

下一步建议进入 T9：

```text
T9 兼容性分析器
```

具体先做：

1. 盘点宏包、文档类、自定义宏、TikZ、minted/listings 等兼容性特征。
2. 设计 `CompatibilityReport`、score、unsupported、warnings 数据结构。
3. 优先在 `doc-compiler-engine` 内落地轻量扫描器，再评估是否拆出 `crates/compatibility-analyzer`。
4. 跑 `cargo test -p doc-compiler-engine compatibility` 与 paper3 三路径脚本。
