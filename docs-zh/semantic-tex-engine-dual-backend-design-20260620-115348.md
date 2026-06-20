# Semantic TeX Engine 双后端语义采集方案（20260620-115348）

> 状态：待审核
>
> 本方案只完善规划设计，不进入代码开发。审核通过前，不改动现有 `doc-core` Rust 转换引擎，也不改动当前 `doc-compiler-engine` 行为。
>
> 实现更新：2026-06-20 已按本方案完成双后端初版开发，详情见：[Semantic TeX Engine 开发进展报告（20260620-124347）](./semantic-tex-engine-development-report-20260620-124347.md)。

## 1. 结论

当前语义引擎方案应该升级为“双后端语义采集”架构：

```text
Semantic TeX Engine
  ├─ RuleBasedBackend       当前已有：Rust 规则解析，零 TeX 运行时依赖
  ├─ XeLaTeXHookBackend     第一代增强：兼容中文模板，用 hook/sidecar 获取语义
  └─ LuaTeXNodeBackend      第二代增强：访问 LuaTeX node tree，获取更强语义和版式信息
```

但这个“双后端”只属于新 `doc-compiler-engine` 路径，不替换、不迁移、不影响现有 `doc-core` 路径。

更准确的最终结构是两层独立：

```text
项目级双路径
  ├─ Path A: doc-core 现有 Rust 转换引擎
  └─ Path B: doc-compiler-engine 新 Semantic TeX Engine

Semantic Engine 内部多后端
  ├─ RuleBasedBackend
  ├─ XeLaTeXHookBackend
  └─ LuaTeXNodeBackend
```

## 2. 为什么需要双后端

### 2.1 XeLaTeX 的价值

XeLaTeX 适合作为第一代语义辅助后端，原因是：

- 中文论文、CTeX、软件学报、医学期刊模板大量默认使用 XeLaTeX。
- `fontspec`、`xeCJK`、`ctex` 生态成熟。
- 用户现有论文通常已经能用 XeLaTeX 编译。
- 第一阶段使用 XeLaTeX 更容易降低模板兼容风险。

但 XeLaTeX 不适合承担全部语义采集：

- 不能像 LuaTeX 一样直接访问 node tree。
- 如果依赖 XDV 反推语义，需要开发 XDV parser、layout rebuilder、line/paragraph detector，成本高。
- 更适合通过宏 hook、sidecar JSONL、aux/toc/lof/lot 等方式获取结构化语义事件。

### 2.2 LuaTeX 的价值

LuaTeX 更适合作为长期语义后端，原因是：

- 可以通过 Lua callback 访问 TeX 内部 node list。
- 可以在 `pre_linebreak_filter`、`post_linebreak_filter`、`hpack_filter` 等阶段获取 glyph、hlist、vlist、glue、kern 等信息。
- 可以直接输出结构化 AST 或 layout metadata，减少从 XDV/PDF 反推版式的工程量。
- 对未来 LuaMetaTeX 或更深层 TeX runtime 集成更友好。

但 LuaTeX 不能直接替代 XeLaTeX：

- 中文 CTeX/XeCJK 模板切换到 LuaLaTeX 可能产生字体、换行、页码差异。
- 用户模板经常写死 XeLaTeX 生态假设。
- 商业化第一阶段不能把用户可编译论文变成不可编译论文。

### 2.3 RuleBasedBackend 仍需保留

当前 Rust rule-based collector 仍有价值：

- 零外部 TeX 依赖。
- CI 和单元测试稳定。
- 可以作为 fallback。
- 可以作为双后端输出的对照基线。
- 对简单 article/report 场景足够快。

因此新语义引擎不应从“rule-based”直接跳到“LuaTeX-only”，而应采用可插拔 backend。

## 3. 总体架构

```text
TeX/CTeX Source
      │
      ▼
Backend Selector
      │
      ├───────────────┬──────────────────┬─────────────────┐
      ▼               ▼                  ▼                 ▼
RuleBasedBackend  XeLaTeXHookBackend  LuaTeXNodeBackend  FutureBackend
      │               │                  │
      └───────────────┴──────────────────┘
                      ▼
          Semantic Events / CollectedDocument
                      ▼
              Semantic AST / StandardDocument
                      ▼
                DocumentGraph
                      ▼
              Semantic DOCX Renderer
                      ▼
                    DOCX
```

## 4. 核心 Rust 接口设计

建议在 `doc-compiler-engine` 内先定义 trait，稳定后再拆到 `crates/semantic-collector`：

```rust
pub trait SemanticBackend {
    fn kind(&self) -> SemanticBackendKind;

    fn is_available(&self, env: &BackendEnvironment) -> BackendAvailability;

    fn collect(
        &self,
        input: &SemanticInput,
        options: &SemanticBackendOptions,
    ) -> Result<SemanticBackendArtifact, EngineError>;
}
```

后端枚举：

```rust
pub enum SemanticBackendKind {
    Auto,
    RuleBased,
    XeLaTeXHook,
    LuaTeXNode,
}
```

统一输入：

```rust
pub struct SemanticInput {
    pub main_tex: String,
    pub vfs: VirtualFs,
    pub project_root: Option<PathBuf>,
    pub profile: EngineProfile,
}
```

统一输出：

```rust
pub struct SemanticBackendArtifact {
    pub document: Document,
    pub standard_document: Option<StandardDocument>,
    pub events: Vec<SemanticEvent>,
    pub layout: Option<LayoutGraph>,
    pub diagnostics: Vec<EngineDiagnostic>,
    pub sidecars: Vec<BuildSidecar>,
}
```

语义事件：

```rust
pub enum SemanticEvent {
    Heading {
        level: u8,
        text: String,
        label: Option<String>,
        span: SourceSpan,
    },
    Paragraph {
        text: String,
        inlines: Vec<InlineEvent>,
        span: SourceSpan,
    },
    Figure {
        path: String,
        caption: Option<String>,
        label: Option<String>,
        width_expr: Option<String>,
        span: SourceSpan,
    },
    Table {
        rows: Vec<TableEventRow>,
        caption: Option<String>,
        label: Option<String>,
        span: SourceSpan,
    },
    Equation {
        latex: String,
        label: Option<String>,
        display: bool,
        span: SourceSpan,
    },
    Citation {
        keys: Vec<String>,
        span: SourceSpan,
    },
}
```

## 5. 后端一：RuleBasedBackend

### 5.1 当前状态

当前 `doc-compiler-engine` 实质就是 RuleBasedBackend：

```text
VirtualFs
  -> IncludeGraph
  -> parse_tex
  -> lower_to_document
  -> StandardDocument
```

### 5.2 后续改造

将当前逻辑包一层 backend：

```rust
pub struct RuleBasedBackend;
```

职责：

- 保持当前行为。
- 不依赖 `xelatex`、`luatex`、`lualatex`。
- 作为 Auto 模式下的 fallback。
- 作为 CI 默认后端。

## 6. 后端二：XeLaTeXHookBackend

### 6.1 定位

XeLaTeXHookBackend 是第一代 TeX runtime 辅助语义后端，目标不是从 XDV 反推所有语义，而是通过 hook 输出结构化 sidecar。

推荐数据流：

```text
TeX Source
  -> inject semantic hook package
  -> xelatex
  -> semantic sidecar JSONL
  -> sidecar parser
  -> SemanticEvent
  -> DocumentGraph
```

### 6.2 为什么不用 XDV 作为主语义来源

XDV 更适合补版式，不适合作为主语义来源：

- XDV 看到的是 glyph、rule、font、position，不是 section/table/citation 语义。
- 从 XDV 重建标题、段落、表格、列表会引入大量启发式。
- 对高保真 DOCX 来说，XDV 可作为 layout metadata，而不是语义主线。

因此 XeLaTeXHookBackend 的第一阶段应避免重 XDV parser，优先 hook semantic events。

### 6.3 Hook 输出方式

不建议依赖 shell-escape。推荐使用 TeX 原生写文件：

```latex
\newwrite\docsemanticout
\immediate\openout\docsemanticout=\jobname.docsem.jsonl
\immediate\write\docsemanticout{...}
```

hook 示例：

```latex
\let\docoldsection\section
\renewcommand{\section}[1]{%
  \immediate\write\docsemanticout{%
    {"type":"heading","level":1,"text":"#1"}%
  }%
  \docoldsection{#1}%
}
```

实际工程中应避免手写不安全 JSON，改用严格转义宏或更保守的字段编码：

```text
docsem:event heading level=1 id=... text-base64=...
```

Rust 侧再解析为 `SemanticEvent`。

### 6.4 可采集语义

第一阶段采集：

- `\title`
- `\author`
- `\section` / `\subsection` / `\subsubsection`
- `\caption`
- `\label`
- `\ref`
- `\cite`
- `\includegraphics`
- `tabular` begin/end
- equation begin/end

第二阶段采集：

- list/item。
- theorem/proof。
- algorithm2e。
- bibliography。
- page/layout anchor。

### 6.5 与 `doc-tex-facade` 的关系

现有 `doc-tex-facade` 已能调用：

```text
xelatex
tectonic
latexmk
```

但它当前定位是生成 oracle PDF。XeLaTeXHookBackend 可以复用其进程管理思路，但不应直接改变 `tex-compile` 的 oracle PDF 行为。

建议新增语义专用运行接口：

```rust
pub struct TeXSemanticRun {
    pub engine: EngineKind,
    pub pdf_path: Option<PathBuf>,
    pub log_path: PathBuf,
    pub semantic_sidecar: PathBuf,
    pub aux_files: Vec<PathBuf>,
}
```

## 7. 后端三：LuaTeXNodeBackend

### 7.1 定位

LuaTeXNodeBackend 是长期主力后端，目标是从 LuaTeX callback 和 node tree 中直接获得语义与版式信息。

推荐数据流：

```text
TeX Source
  -> inject Lua collector
  -> lualatex
  -> node callbacks + macro hooks
  -> semantic/layout sidecar JSONL
  -> SemanticEvent + LayoutGraph
  -> DocumentGraph
```

### 7.2 Lua callback

可用回调：

```lua
callback.register("process_input_buffer", ...)
callback.register("pre_linebreak_filter", ...)
callback.register("post_linebreak_filter", ...)
callback.register("hpack_filter", ...)
callback.register("shipout/before", ...)
```

采集内容：

- glyph 文本。
- font id。
- 字号。
- glue/kern。
- hlist/vlist。
- 行、段、页的 layout metadata。
- macro hook 输出的 section/caption/reference 事件。

### 7.3 LuaTeX 不只靠 node tree

LuaTeX node tree 能提供版式和低层结构，但 section/table/citation 仍应通过宏 hook 补充。推荐双通道：

```text
macro hooks -> semantic events
node callbacks -> layout events
```

合并后：

```text
SemanticEvent + LayoutEvent -> DocumentGraph
```

### 7.4 LuaTeX 兼容风险

LuaTeX 不是第一阶段默认后端，原因：

- `ctex` / `fontspec` / `xeCJK` 模板可能依赖 XeLaTeX 行为。
- 中文字体选择和换行可能与 XeLaTeX 不一致。
- 软件学报、医学模板可能需要额外适配。

因此 LuaTeXNodeBackend 的启用策略应是：

- 明确指定 `--semantic-backend luatex-node`。
- 或 Auto 模式判定模板 LuaLaTeX 兼容后启用。
- 如果 LuaTeX 编译失败，回退 XeLaTeXHookBackend 或 RuleBasedBackend。

## 8. Auto 后端选择策略

建议默认：

```text
Auto
  1. 如果用户显式指定 backend，使用用户指定。
  2. 如果没有可用 TeX runtime，使用 RuleBasedBackend。
  3. 如果模板强依赖 xeCJK/fontspec/ctex 且已有 XeLaTeX 可用，优先 XeLaTeXHookBackend。
  4. 如果模板无明显 XeTeX-only 特征且 LuaLaTeX 可用，优先 LuaTeXNodeBackend。
  5. 如果 runtime backend 失败，回退 RuleBasedBackend，并输出 diagnostic。
```

检测特征：

```text
\documentclass{ctexart}
\documentclass{ctexrep}
\usepackage{ctex}
\usepackage{xeCJK}
\setCJKmainfont
\setmainfont
\XeTeX
\directlua
\usepackage{luatexja}
```

后端选择结果必须写入 `CompileReport`：

```json
{
  "backend": "xelatex-hook",
  "reason": "ctex/xeCJK detected; xelatex available",
  "fallback": null
}
```

## 9. 与现有旧路径的边界

旧路径保持：

```text
doc-core -> doc-latex-reader -> doc-docx-writer
```

新路径新增：

```text
doc-compiler-engine -> SemanticBackend -> DocumentGraph -> Semantic Renderer
```

禁止：

- 让 `doc-core` 依赖 `doc-compiler-engine`。
- 让旧 `doc-engine convert` 默认调用 runtime backend。
- 为了实现 XeLaTeX/LuaTeX backend 而改变旧路径输出。

允许：

- 新路径复用 `doc-utils`、`doc-semantic-ast`、`doc-mathml`。
- 新路径参考或复制 `doc-core` 中的 JOS 兼容逻辑。
- 新路径使用 `doc-tex-facade` 的进程管理代码，但应保持 oracle PDF 路径行为不变。

## 10. DOCX 渲染策略

短期：

- 新语义引擎继续调用 `doc-docx-writer` 打包 DOCX。
- 通过独立 options/profile 控制新路径行为。

中期：

- 为新路径增加 `SemanticDocxRenderer`。
- 支持从 `DocumentGraph` 渲染，而不是只从 legacy `Document` 渲染。
- 为 OMML、bookmark、hyperlink、caption 编号、表格合并等新能力提供新 API。

关键原则：

- 如果修改 `doc-docx-writer`，必须保持旧路径默认行为不变。
- 新行为通过新函数、feature、options 或 semantic renderer 暴露。

## 11. 任务拆解

### B0 方案审核

状态：待审核

目标：

- 确认新语义引擎内部采用 RuleBased + XeLaTeXHook + LuaTeXNode 多后端路线。
- 确认旧 `doc-core` 不受影响。
- 确认第一阶段优先实现 XeLaTeXHookBackend，而 LuaTeXNodeBackend 作为第二阶段。

### B1 Backend trait 与报告字段

状态：待开发

目标：

- 在 `doc-compiler-engine` 内新增 `SemanticBackend` trait。
- 新增 `SemanticBackendKind`。
- `CompileReport` 增加 backend selection 字段。
- 当前逻辑包成 `RuleBasedBackend`。

验证：

```bash
cargo test -p doc-compiler-engine backend
```

### B2 XeLaTeXHookBackend 原型

状态：待开发

目标：

- 注入 hook package。
- 调用 XeLaTeX。
- 输出 semantic sidecar。
- Rust 解析 sidecar 为 `SemanticEvent`。

第一批 hook：

```text
title
author
section/subsection
caption
label/ref/cite
includegraphics
equation
tabular
```

验证：

```bash
cargo test -p doc-compiler-engine xelatex_hook
```

需要真实 XeLaTeX 的测试默认 `#[ignore]`，避免 CI 环境不稳定。

### B3 LuaTeXNodeBackend 原型

状态：待开发

目标：

- 注入 Lua collector。
- 通过 macro hook 输出 semantic events。
- 通过 node callback 输出 layout events。
- 合并为 `DocumentGraph` metadata。

验证：

```bash
cargo test -p doc-compiler-engine luatex_node
```

需要真实 LuaLaTeX 的测试默认 `#[ignore]`。

### B4 Auto selector

状态：待开发

目标：

- 根据模板特征、可用 runtime、用户选项选择 backend。
- 支持 fallback。
- 在 `CompileReport` 输出选择原因。

验证：

```bash
cargo test -p doc-compiler-engine backend_selector
```

### B5 双 runtime 后端对比脚本

状态：待开发

目标：

- 对 paper3 生成：
  - rule-based semantic DOCX。
  - xelatex-hook semantic DOCX。
  - 如可用，luatex-node semantic DOCX。
- 输出差异报告。

建议脚本：

```bash
scripts/compare_paper3_semantic_backends.sh
```

### B6 LayoutGraph 与 XDV/Lua layout 合并

状态：待开发

目标：

- 定义 `LayoutGraph`。
- LuaTeX node layout 首先落地。
- XDV layout parser 后续补充。

## 12. 推荐研发顺序

```text
B0 审核双后端方案
B1 Backend trait + RuleBasedBackend 封装
B2 XeLaTeXHookBackend 原型
B4 Auto selector
B5 后端对比脚本
B3 LuaTeXNodeBackend 原型
B6 LayoutGraph
```

理由：

- 先把接口和当前 rule-based 封装好，避免直接把 TeX runtime 写进主流程。
- 先做 XeLaTeXHookBackend，最大化兼容中文论文和现有模板。
- Auto selector 在 XeLaTeX 后端可用后再做，避免设计空转。
- LuaTeXNodeBackend 作为第二代增强，价值更高，但模板兼容风险也更高。

## 13. 验证策略

每次开发后必须确认旧路径未受影响：

```bash
cargo test -p doc-core
```

新路径基础验证：

```bash
cargo test -p doc-compiler-engine
```

需要 TeX runtime 的验证分层：

```bash
cargo test -p doc-compiler-engine -- --ignored xelatex
cargo test -p doc-compiler-engine -- --ignored luatex
```

纸面审核阶段不要求执行 runtime 测试。

## 14. 当前方案变更点

相对上一版独立路径方案，本版新增：

- 明确 Semantic Engine 内部采用多后端。
- 明确 XeLaTeXHookBackend 是第一代 runtime semantic backend。
- 明确 LuaTeXNodeBackend 是第二代 runtime semantic backend。
- 明确 XDV 只作为 layout 补充，不作为主语义来源。
- 明确 `doc-tex-facade` 可复用进程管理思路，但 oracle PDF 路径不被改变。
- 明确 runtime 测试默认应可 ignore，避免 CI 依赖本机 TeX 环境。

## 15. 待审核决策

需要审核确认：

1. 是否接受 Semantic Engine 内部从单一路径升级为多后端。
2. 是否接受第一代 runtime 后端优先做 XeLaTeXHookBackend。
3. 是否接受 LuaTeXNodeBackend 作为第二代增强，而不是立即替换 XeLaTeX。
4. 是否接受 TeX runtime 测试默认 `#[ignore]`。
5. 是否接受 XDV 只做 layout metadata，不做主语义恢复。
