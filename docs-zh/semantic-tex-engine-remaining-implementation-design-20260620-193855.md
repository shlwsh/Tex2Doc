# Semantic TeX Engine 待完成内容技术实现设计方案（20260620-193855）

## 1. 设计目标

本方案面向当前尚未完成的 Semantic TeX Engine 后续阶段，基于现有实现继续推进：

```text
doc-compiler-engine
  -> SemanticBackend
  -> SemanticCollector
  -> CollectedDocument
  -> DocumentGraph
  -> DOCX
```

设计原则：

- 不影响旧 `doc-core` / Rust rule DOCX 转换路径。
- 新能力优先在独立 crate 或 `doc-compiler-engine` 新路径内落地。
- 运行时 backend、规则 collector、layout collector 输出统一收敛到 `CollectedDocument` / `DocumentGraph`。
- 所有不确定推断必须可审计、可关闭、可复现。
- paper3 始终作为三路径回归样例。

## 2. 待完成工作分组

| 分组 | 对应任务 | 目标 |
|---|---|---|
| XDV parser | T11 | 从 XeLaTeX/XDV 输出获得字形、字体、rule、special 等版式底层信息 |
| LayoutGraph | B6/T11 | 将 XDV/LuaTeX layout 统一成版式图 |
| LuaTeX collector v2 | T12 | 提升 LuaTeX node tree 和 macro hook 采集质量 |
| semantic-collector crate | T10 后续 | 将 collector trait 和输出模型从 engine 中拆出 |
| compatibility-analyzer crate | T9 后续 | 将兼容性扫描器独立成可复用模块 |
| AI/rule fallback | T13 | 对未知宏提供可审计的规则和可选 AI 推断 |
| Profile 外置化 | M2 后续 | 将 JOS/中文/医学/SCI profile 从 Rust 内置迁移到配置文件 |
| DOCX 高保真增强 | M3/M4 后续 | inline math、字段引用、复杂表格、TikZ/minted 降级 |

## 3. T11：XDV parser 原型设计

### 3.1 crate 设计

新增 crate：

```text
crates/xdv-parser
```

包名：

```text
doc-xdv-parser
```

workspace 变更：

```toml
members = [
    ...
    "crates/xdv-parser",
]

[workspace.dependencies]
doc-xdv-parser = { path = "crates/xdv-parser" }
```

依赖建议：

```toml
thiserror = { workspace = true }
serde = { workspace = true }
```

第一阶段不强制引入 `nom`，先实现小型 byte reader；如果 opcode 覆盖扩大，再评估 `nom`。

### 3.2 模块结构

```text
crates/xdv-parser
├── Cargo.toml
└── src
    ├── lib.rs
    ├── error.rs
    ├── reader.rs
    ├── opcode.rs
    ├── model.rs
    └── parser.rs
```

### 3.3 数据模型

```rust
pub struct XdvDocument {
    pub preamble: Option<XdvPreamble>,
    pub pages: Vec<XdvPage>,
    pub fonts: Vec<FontDef>,
    pub commands: Vec<XdvCommand>,
}

pub struct XdvPreamble {
    pub id: u8,
    pub numerator: i32,
    pub denominator: i32,
    pub magnification: i32,
    pub comment: String,
}

pub struct XdvPage {
    pub number: i32,
    pub commands: Vec<XdvCommand>,
}

pub enum XdvCommand {
    SetChar { code: u32 },
    SetRule { height: i32, width: i32 },
    PutRule { height: i32, width: i32 },
    Push,
    Pop,
    MoveRight(i32),
    MoveDown(i32),
    SelectFont(u32),
    FontDef(FontDef),
    Special(Vec<u8>),
    Bop,
    Eop,
    Unknown { opcode: u8, offset: usize },
}

pub struct FontDef {
    pub id: u32,
    pub checksum: u32,
    pub scale: i32,
    pub design_size: i32,
    pub area: String,
    pub name: String,
}
```

后续 layout 层再将 `SetChar`、font state、position state 合成为：

```rust
pub struct GlyphNode {
    pub code: u32,
    pub font_id: Option<u32>,
    pub x: i64,
    pub y: i64,
}
```

### 3.4 parser 范围

第一阶段支持：

- preamble。
- bop/eop。
- push/pop。
- set_char short form。
- set_rule / put_rule。
- right/down movement。
- font selection。
- font definitions。
- special。
- postamble 可跳过但保留 offset 诊断。

不在第一阶段解决：

- 完整 XDV native font 扩展。
- OpenType glyph mapping。
- page physical unit 换算。
- line/paragraph clustering。
- 与 DOCX renderer 集成。

### 3.5 错误模型

```rust
pub enum XdvError {
    UnexpectedEof { offset: usize, needed: usize },
    InvalidOpcode { offset: usize, opcode: u8 },
    InvalidUtf8 { offset: usize },
    InvalidFormat { offset: usize, message: String },
}
```

所有错误必须带 offset，便于后续定位 TeX 输出问题。

### 3.6 验收测试

测试文件：

```text
crates/xdv-parser/tests/fixtures.rs
```

fixture 策略：

- 手写最小 DVI/XDV-like byte sequence。
- 覆盖 preamble、font def、bop/eop、push/pop、set_char、special。
- 不依赖外部 `xelatex`，保证 CI 稳定。

验收命令：

```bash
cargo test -p doc-xdv-parser
```

## 4. LayoutGraph 设计

### 4.1 目标

将 XDV parser 和 LuaTeX node collector 输出统一到现有：

```rust
LayoutGraph
LayoutNode
```

但当前 `LayoutNode` 只有：

```text
id
kind
page
```

后续需要扩展为更细的 layout 模型。

### 4.2 建议模型

```rust
pub struct LayoutGraph {
    pub pages: Vec<LayoutPage>,
    pub nodes: Vec<LayoutNode>,
}

pub struct LayoutPage {
    pub number: u32,
    pub width_sp: Option<i64>,
    pub height_sp: Option<i64>,
}

pub enum LayoutNode {
    Glyph(LayoutGlyph),
    Rule(LayoutRule),
    Special(LayoutSpecial),
    Line(LayoutLine),
    Block(LayoutBlock),
}

pub struct LayoutGlyph {
    pub page: u32,
    pub font_id: Option<u32>,
    pub code: u32,
    pub x_sp: i64,
    pub y_sp: i64,
}

pub struct LayoutRule {
    pub page: u32,
    pub x_sp: i64,
    pub y_sp: i64,
    pub width_sp: i64,
    pub height_sp: i64,
}
```

### 4.3 集成方式

第一阶段：

```text
XdvDocument -> LayoutGraph
```

只输出 glyph/rule/special。

第二阶段：

```text
LayoutGraph -> line clustering -> paragraph hints
```

第三阶段：

```text
LayoutGraph + SemanticEvent -> DocumentGraph enrichment
```

## 5. T12：LuaTeX collector v2 设计

### 5.1 sidecar schema v2

当前 JSONL event 是轻量 event。v2 建议增加：

```json
{
  "schema":"semantic-event-v2",
  "type":"heading",
  "source":{"path":"main.tex","line":12,"column":1},
  "macro":"section",
  "payload":{}
}
```

统一字段：

```text
schema
type
source
macro
payload
layout
diagnostics
```

### 5.2 macro hook 范围

必须覆盖：

- `\section`
- `\subsection`
- `\subsubsection`
- `\caption`
- `\label`
- `\ref`
- `\eqref`
- `\autoref`
- `\cite`
- `\includegraphics`
- `equation`
- `table`
- `tabular`

### 5.3 node tree 范围

LuaTeX node callback 初期只采：

- paragraph text。
- glyph code。
- font id。
- hlist/vlist 层级。
- glue。
- rule。

输出到：

```text
LayoutGraph
```

不在初期做：

- 完整宏展开追踪。
- 完整页面重排。
- Word 样式直接推断。

### 5.4 与 XeLaTeX 的关系

paper3 / CTeX / xeCJK 场景仍优先 XeLaTeX。

LuaTeX v2 的目标不是替代 XeLaTeX，而是为：

- 通用 LaTeX。
- 可运行 LuaLaTeX 的模板。
- 后续 node tree layout 采集。

提供更直接的语义底座。

## 6. `semantic-collector` crate 拆分设计

### 6.1 拆分条件

满足以下条件后再拆：

- `CollectedDocument` 字段稳定。
- `SemanticCollector` trait 不再频繁变化。
- RuleBased/XeLaTeX/LuaTeX 三类 collector 输出一致。
- paper3 三路径稳定通过。

### 6.2 crate API

```rust
pub trait SemanticCollector {
    fn name(&self) -> &'static str;
    fn collect(&self, input: &mut SemanticCollectorInput<'_>) -> Result<CollectedDocument, CollectorError>;
}

pub struct CollectedDocument {
    pub document: Document,
    pub standard_document: Option<StandardDocument>,
    pub image_assets: ImageAssets,
    pub events: Vec<SemanticEvent>,
    pub layout: Option<LayoutGraph>,
    pub diagnostics: Vec<CollectorDiagnostic>,
    pub sidecars: Vec<BuildSidecar>,
}
```

### 6.3 依赖方向

允许：

```text
doc-compiler-engine -> doc-semantic-collector
doc-semantic-collector -> doc-semantic-ast
doc-semantic-collector -> doc-utils
```

禁止：

```text
doc-core -> doc-compiler-engine
doc-core -> doc-semantic-collector
```

## 7. `compatibility-analyzer` crate 拆分设计

### 7.1 当前迁移对象

从 `doc-compiler-engine` 拆出：

```text
CompatibilityReport
CompatibilityIssue
analyze_compatibility
```

### 7.2 API

```rust
pub struct CompatibilityAnalyzer {
    rules: CompatibilityRules,
}

impl CompatibilityAnalyzer {
    pub fn analyze(&self, input: &CompatibilityInput<'_>) -> CompatibilityReport;
}

pub struct CompatibilityInput<'a> {
    pub profile_id: &'a str,
    pub files: Vec<CompatibilitySource<'a>>,
}
```

### 7.3 规则外置

建议规则：

```toml
[unsupported.package]
minted = "requires shell escape and external Pygments"
tikz = "requires rasterization or drawing fallback"

[warning.package]
listings = "code style may be downgraded"
biblatex = "best support is BibTeX/bbl flow"
```

## 8. AI fallback / Rule Engine 设计

### 8.1 默认行为

默认：

```text
AI fallback disabled
network disabled
unknown macro -> warning + conservative text fallback
```

### 8.2 Rule Engine

先实现确定性规则：

```rust
pub struct MacroRule {
    pub name: String,
    pub arity: usize,
    pub output: RuleOutput,
}

pub enum RuleOutput {
    Heading { level: u8, arg: usize },
    Paragraph { arg: usize },
    InlineText { arg: usize },
    Ignore,
}
```

### 8.3 AI Engine

AI 只能作为可选插件：

```text
unknown macro
  -> collect context
  -> rule cache lookup
  -> optional AI inference
  -> audit record
  -> user-reviewable rule
```

必须输出 audit：

```json
{
  "macro":"mycompanytable",
  "decision":"table",
  "confidence":0.72,
  "source":"ai",
  "prompt_hash":"...",
  "accepted":false
}
```

## 9. Profile 外置化设计

### 9.1 文件格式

建议：

```text
profiles/jos-paper.toml
profiles/chinese-academic.toml
profiles/medical-journal.toml
```

### 9.2 schema

```toml
id = "jos-paper"
display_name = "Journal of Software Paper"
document_classes = ["rjthesis", "ctexart"]

[page_setup]
kind = "jos-paper3"

[font_policy]
latin_main = "Times New Roman"
cjk_main = "SimSun/FangSong/KaiTi"
math = "Cambria Math"

[caption_policy]
figure_prefix = "图"
table_prefix = "表"
equation_prefix = "式"
numbering = "section-scoped"

[compatibility]
min_score = 70
fail_on_unsupported = false
```

### 9.3 加载顺序

```text
explicit profile file
  -> built-in profile id
  -> generic fallback
```

## 10. DOCX 高保真增强设计

### 10.1 inline math

当前块级公式已接入 OMML。后续需要：

```text
Inline::Formula -> OMML run
```

验收：

- `$a+b$` 在 Word 中为公式对象。
- 不破坏普通段落 run。

### 10.2 Word 字段引用

当前是 bookmark + hyperlink。后续目标：

```text
\ref{fig:a} -> REF field
\cite{...} -> citation field or styled run
```

第一阶段可继续保留 hyperlink，同时增加可选 field mode。

### 10.3 TikZ 降级

策略：

```text
TikZ source
  -> detect
  -> compile/rasterize to PNG/PDF
  -> insert image
  -> preserve source as alt/audit metadata
```

不直接承诺 editable drawing。

### 10.4 minted/listings

策略：

```text
minted/listings
  -> code block
  -> preserve language
  -> optional syntax highlight
  -> fallback plain monospace
```

## 11. 验证矩阵

| 工作项 | 单元测试 | 集成测试 | paper3 |
|---|---|---|---|
| XDV parser | `cargo test -p doc-xdv-parser` | fixture parser | 暂不要求 |
| LayoutGraph | `cargo test -p doc-compiler-engine layout` | XDV fixture -> layout | 后续接入 |
| LuaTeX v2 | ignored runtime test | lualatex runtime | 非 CTeX 样例优先 |
| semantic-collector split | `cargo test -p doc-semantic-collector` | compiler-engine | paper3 三路径 |
| compatibility split | `cargo test -p doc-compatibility-analyzer` | compiler-engine compatibility | paper3 score |
| AI fallback | offline unit tests | audit cache tests | 不默认启用 |
| profile externalization | profile parser tests | paper3 profile file | paper3 三路径 |

## 12. 近期执行计划

### Sprint A：XDV parser

- [ ] 新增 crate。
- [ ] 实现 byte reader。
- [ ] 实现最小 opcode parser。
- [ ] fixture 单测。
- [ ] 文档更新。

### Sprint B：LayoutGraph

- [ ] XDV commands -> glyph/rule/special layout nodes。
- [ ] layout count 写入 report。
- [ ] 暂不影响 DOCX。

### Sprint C：LuaTeX collector v2

- [ ] JSONL schema v2。
- [ ] macro hook 补齐。
- [ ] node tree layout 初版。
- [ ] runtime ignored tests。

### Sprint D：crate 拆分

- [ ] `semantic-collector`。
- [ ] `compatibility-analyzer`。
- [ ] 编译器 facade 保持稳定。

### Sprint E：可配置与 fallback

- [ ] profile TOML。
- [ ] rule engine。
- [ ] AI fallback audit cache。

## 13. 完成定义

后续阶段不能只以“能编译”为完成标准。每个任务完成时必须满足：

- 有独立单元测试。
- 有 paper3 或 fixture 验证。
- 不破坏旧 `doc-core`。
- 更新 `docs-zh/semantic-tex-engine-progress-and-task-plan.md`。
- 输出带时间戳开发报告。
- GitNexus detect_changes 已执行。

下一项推荐立即开发：

```text
T11 XDV parser 原型
```
