# Doc-engine 完善设计实现方案
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



> 版本：v2.0  
> 日期：2026-06-17  
> 参考输入：`docs/Doc-engine_质量分析报告_v1.8_20260617.md`、`docs/文档转换引擎实现算法总结_v1.0_20260617.md`、`scripts/build_docx.sh`、`scripts/build_jos_docx.py`

## 1. 背景与目标

Tex2Doc 当前 Rust V2 主链路已经具备完整的工程骨架：`IncludeGraph` 拓扑、宏展开、Logos/Rowan 解析、Semantic AST、DOCX Writer、PDF 校验模块均已成形。但质量分析报告指出，paper3 这类软件学报论文在真实转换中仍存在三类阻断问题：

- 图片未稳定进入 `word/media/`，导致 DOCX 文件体积异常小，PDF 中图片缺失。
- `algorithm`、复杂 `tabular`、`\ref`、部分数学上下标没有被语义化，出现 Raw LaTeX 泄漏。
- JOS 版式的页眉、首页题头、脚注、双语前置内容、参考文献、分页保持等细节没有被模板级还原，导致与 Oracle PDF 页数和字符分布差距较大。

本轮移植后的 `scripts/build_docx.sh` 已经证明另一条可行路径：先用 TeX 工具链生成 PDF/BBL 等侧产物，再用专用结构解析器直接构建 JOS WordprocessingML，最后用校验脚本闭环验证。该路径在 `examples/paper3/latex/main-jos.tex` 上已生成通过校验的 DOCX：8 张图、51 条英文参考文献、DOCX/PDF 字符比例 0.982。

因此，本方案的目标不是推翻 Rust 主引擎，而是建立“两阶段收敛”：

1. **短期产品化**：将 `build_docx.sh + build_jos_docx.py + verify_jos_docx.py` 固化为 paper3/JOS 高保真转换通路，作为质量基准和回归 oracle。
2. **中期回灌主引擎**：把 Python 高保真转换器中的解析、编号、图片、算法表格、样式和校验算法，逐步回迁到 Rust 的 `latex-reader`、`semantic-ast`、`docx-writer`、`quality`。
3. **长期泛化**：形成模板 Profile 驱动的学术论文 DOCX 引擎，不再依赖单篇论文硬编码。

## 2. 设计原则

### 2.1 质量优先于抽象纯度

质量报告显示，纯通用 AST 路径在复杂期刊模板上会先遇到图片、算法、表格和版式问题。高保真输出必须允许模板 Profile 参与解析和渲染。JOS 这类模板不应被当成普通 article，而应有明确的 `JosProfile`，承载页面尺寸、页眉页脚、标题区、前置内容、表格线型、算法伪代码、参考文献样式等规则。

### 2.2 以 TeX 侧产物补齐语义

`build_docx.sh` 的关键价值是承认 BibTeX 和 TeX 编译器已经能可靠产出若干事实：

- `main-jos.bbl` 提供最终参考文献顺序与条目文本。
- `main-jos.pdf` 提供校验基准。
- `pdftotext` 能提供内容覆盖和字符比例校验。
- `pdftoppm` 可在源图为 PDF 时兜底转 PNG。

这些侧产物不应被视为临时脚本依赖，而应进入设计：高保真学术转换可以是“源文档解析 + TeX 侧产物校准”的混合管线。

### 2.3 解析阶段必须语义化，不能 RawFallback

算法总结中已经给出 lowering 目标：将 LaTeX 特性消融为标准块枚举。质量报告中的主要失败都来自特定环境落入 RawFallback。因此后续主引擎必须显式支持：

- `algorithm2e`：`\KwIn`、`\KwOut`、`\ForEach`、`\If`、`\Return`、`\tcp*`。
- `figure`：图片路径、宽度、caption、label。
- `table/tabular`：caption、label、行列、booktabs 规则、列定义清洗。
- `equation`：编号、label、上下标。
- `description/list`：中文参考文献和作者简介。
- `rjabstract/rjkeywords/rjtitle/rjauthor/rjinfor/rjhead`：JOS 元数据。

### 2.4 校验必须成为构建的一部分

`build_docx.sh` 将生成和校验串在同一入口中，这是正确方向。后续所有转换入口都应输出机器可读校验 JSON，并对 P0/P1 失败设置非零退出码。

### 2.5 以文档标准作为解析和映射基线

转换引擎不应只依赖项目内经验规则，而应显式引入源格式和目标格式的标准体系作为基线。

对 `.docx`：

- 以 Office Open XML 为目标格式标准，核心参考 ECMA-376 与 ISO/IEC 29500。
- 物理包结构遵循 OPC：ZIP package、part、relationship、content type、`_rels`。
- 逻辑内容遵循 WordprocessingML：`word/document.xml`、styles、settings、numbering、header/footer、media、field code。
- XML 结构应尽可能对齐 OOXML Schema 约束，避免只生成“Word 恰好能打开”的脆弱 XML。

对 `.tex` / LaTeX：

- 承认 TeX/LaTeX 是宏编程语言和事实标准，不存在类似 OOXML 的完整静态 XSD。
- 以 TeXbook、LaTeX2e/LaTeX3 文档、常用宏包说明，以及 TeX Live/XeTeX/LuaTeX/pdfTeX 的实际编译行为作为规范基线。
- 对静态可解析结构建立 parser 规则；对必须依赖引擎求值的结果，通过 `.aux`、`.bbl`、`.log`、`.pdf` 等 TeX artifact 补齐。

设计含义是：`Source CST` 和 `Standard Doc AST` 必须记录规则来源，例如 `latex2e.section`、`algorithm2e.ForEach`、`ooxml.w_p`、`opc.relationship.image`，让转换行为可追溯、可升级、可审计。

## 3. 当前可用转换算法拆解

`scripts/build_docx.sh` 实际形成了四阶段流水线。

### 3.1 输入与依赖检查

```text
ROOT = 仓库根目录
PAPER_ROOT = examples/paper3
LATEX_DIR = examples/paper3/latex
FORMAT_JSON = docs/format/jos_2025_docx_format_definitions.json
TEX_SRC = latex/main-jos.tex
PDF_SRC = latex/main-jos.pdf
BBL_SRC = latex/main-jos.bbl
OUTPUT_DIR = examples/paper3/output/to-docx
```

依赖检查覆盖：

- `python3`
- `pdftotext`
- `pdftoppm`
- `latexmk`
- `xelatex`
- `bibtex`
- `Pillow`
- JOS 格式 JSON
- `main-jos.tex`

设计结论：CLI 层应暴露 `doctor` 或 `preflight`，把这些依赖状态写入报告，避免转换失败后才暴露缺工具。

### 3.2 TeX 侧产物生成

当前逻辑：

```text
if main-jos.pdf 和 main-jos.bbl 都存在且比 main-jos.tex 新:
    复用 PDF/BBL
else:
    cd latex && latexmk -xelatex -bibtex -interaction=nonstopmode -halt-on-error main-jos.tex
```

设计结论：

- PDF/BBL 是高保真转换的显式输入，不能只作为测试临时文件。
- 判断是否重建不能只比较 `main-jos.tex`，还应纳入 `sections/**/*.tex`、`references.bib`、`rjthesis.cls`、图片文件的 mtime 或内容 hash。
- Rust 主引擎的 `tex-facade` 应新增 `BuildArtifacts { pdf, bbl, aux, log }`，供转换和质量模块共享。

### 3.3 DOCX 生成

当前调用：

```text
python3 scripts/build_jos_docx.py \
  --root examples/paper3 \
  --format docs/format/jos_2025_docx_format_definitions.json \
  --output output.docx
```

`build_jos_docx.py` 的核心算法：

1. 读取 `latex/main-jos.tex`。
2. 递归展开 `\input{...}`，收集正文和附录区。
3. 从 `sections/zh/00_abstract.tex` 读取 `\newcommand` 摘要和关键词宏。
4. 从 `latex/main-jos.bbl` 读取 BibTeX 输出，建立 `cite_key -> number` 映射。
5. 扫描所有章节，建立 `label -> number` 映射。
6. 解析 front matter：中文题名、作者、单位、中文摘要、关键词、中图分类号、中文/英文引用格式、英文题名/作者/单位/摘要/关键词、页眉、首页脚注。
7. 解析正文块：heading、paragraph、list item、table、figure、algorithm、equation。
8. 用 `DocxProfile` 和格式 JSON 写入 WordprocessingML。
9. 打包 `[Content_Types].xml`、`_rels/.rels`、`word/document.xml`、`word/styles.xml`、`word/settings.xml`、header/footer、relationships、`word/media/*`。

设计结论：这套算法已经覆盖质量报告中所有 P0/P1 的关键缺口，应作为 Rust 回迁的“参考实现”。

### 3.4 DOCX/PDF/格式一致性校验

当前调用：

```text
python3 scripts/verify_jos_docx.py \
  --docx output.docx \
  --pdf latex/main-jos.pdf \
  --tex-root examples/paper3 \
  --format docs/format/jos_2025_docx_format_definitions.json \
  --report report.md \
  --json-report report.json
```

校验覆盖：

- DOCX 是否可解包，`word/document.xml` 是否存在。
- 页面尺寸、边距、分栏是否符合格式 JSON。
- 图片数量、图题、图片段落样式。
- 表格数量、编号表题、边框、字体、禁止跨页拆分。
- 算法表格结构、行号、注释列、竖线、行 cantSplit。
- 页眉页码字段、首页期刊信息制表位、偶奇页页眉。
- 正文引用是否上标。
- 公式段落、上下标 run、LaTeX 残留。
- 参考文献悬挂缩进和条目数。
- 关键内容覆盖和 DOCX/PDF 字符比例。

设计结论：`quality` crate 应吸收这些检查，形成模板 Profile 驱动的质量门禁。

## 4. 目标架构

### 4.1 总体结构

```text
                    ┌──────────────────────────┐
                    │ CLI / Script / WASM / API │
                    └────────────┬─────────────┘
                                 │
                  ┌──────────────▼──────────────┐
                  │ Conversion Orchestrator      │
                  │ - preflight                  │
                  │ - artifact build             │
                  │ - profile selection          │
                  │ - quality gate               │
                  └──────────────┬──────────────┘
                                 │
        ┌────────────────────────┼────────────────────────┐
        │                        │                        │
┌───────▼────────┐     ┌─────────▼─────────┐    ┌─────────▼────────┐
│ TeX Artifacts  │     │ LaTeX Reader       │    │ Format Profile    │
│ pdf/bbl/aux/log│     │ include/macro/CST  │    │ JOS/page/styles   │
└───────┬────────┘     └─────────┬─────────┘    └─────────┬────────┘
        │                        │                        │
        └──────────────┬─────────▼─────────┬──────────────┘
                       │ Standard Doc AST   │
                       │ semantic + format  │
                       │ metadata + source  │
                       └─────────┬─────────┘
                                 │
                       ┌─────────▼─────────┐
                       │ AST Dump           │
                       │ md + json review   │
                       └─────────┬─────────┘
                                 │
                 ┌───────────────▼───────────────┐
                 │ Mapping Registry               │
                 │ standard AST -> target rules   │
                 └───────────────┬───────────────┘
                                 │
                       ┌─────────▼─────────┐
                       │ DOCX Render Tree   │
                       │ blocks/runs/rels   │
                       └─────────┬─────────┘
                                 │
                       ┌─────────▼─────────┐
                       │ Renderer / Packer  │
                       │ OOXML + media ZIP  │
                       └─────────┬─────────┘
                                 │
                       ┌─────────▼─────────┐
                       │ Quality Report     │
                       │ md + json + gate   │
                       └───────────────────┘
```

### 4.2 双路径策略

#### 路径 A：JOS 高保真路径

适用场景：

- 软件学报模板。
- paper3 这类含双语摘要、算法、图表、参考文献、作者简介的论文。
- 对格式和内容完整性要求高于转换泛化能力。

实现策略：

- 短期继续使用 `build_jos_docx.py`。
- 输出作为 Rust V2 的 oracle。
- 校验必须通过才能认为转换成功。

#### 路径 B：通用 Rust 主引擎

适用场景：

- 一般 LaTeX 文档、Web/WASM/扩展端上传转换。
- 不依赖本地 TeX 工具链或不要求严格期刊版式。

实现策略：

- 继续以 `IncludeGraph -> MacroExpand -> CST -> Standard Doc AST -> Mapping Registry -> Render Tree -> DOCX` 为主。
- 从路径 A 回迁算法环境、表格、图片、引用、样式、校验。
- 对不支持的模板显式降级，不宣称高保真。

### 4.3 Profile 模型

新增或完善 `FormatProfile`：

```text
FormatProfile
  id: "jos-2025"
  page_setup
  styles
  front_matter_rules
  header_footer_rules
  citation_rules
  table_rules
  figure_rules
  algorithm_rules
  reference_rules
  quality_thresholds
```

短期可由 JSON + Python dataclass 承载；中期在 Rust 中对应为 `docx-writer::PageSetup`、`styles::StyleSet`、`quality::Thresholds` 的组合。

### 4.3.1 标准知识库模型

除 `FormatProfile` 外，还需要一层“标准知识库”，用于保存 TeX/LaTeX 与 OOXML 的标准规则、宏包规则和映射规则。Profile 只描述某个模板如何取舍这些标准规则。

```text
Standards Knowledge Base
  tex/
    primitives
    latex2e commands
    latex environments
    package rules
    engine artifacts
  ooxml/
    opc package parts
    wordprocessingml elements
    styles
    relationships
    content types
    field codes
  mappings/
    standard_ast_to_docx
    profile_overrides
    diagnostics policies
```

核心实体：

```text
SyntaxRule
  id
  standard_source
  command_or_element
  parse_pattern
  semantic_role
  metadata_schema
  diagnostics

MappingRule
  id
  source_ast_kind
  target_format
  target_render_node
  style_policy
  resource_policy
  validation_rules

ProfileOverride
  profile_id
  rule_id
  override_fields
  reason
```

这样可以把“标准规则”和“JOS 模板特例”分开：`\section`、`\caption`、`\label` 属于 LaTeX 基础规则；`algorithm2e` 属于宏包规则；`JOSHeading1`、首页 masthead、参考文献悬挂缩进属于 `jos-2025` Profile override。

### 4.4 标准文档 AST

后续主架构应从“TeX 直接到 DOCX”升级为“TeX 到标准文档 AST，再由映射层到 DOCX”。标准 AST 是 Tex2Doc 内部的稳定语义边界，既不是原始 TeX CST，也不是 DOCX XML 的简化版，而是系统专用的文档中间表示。

#### 4.4.1 分层关系

```text
TeX Source
  ↓
Source CST
  - 保留原始命令、环境、分组、source span、注释、错误恢复信息
  ↓
Standard Doc AST
  - 提炼文档语义、格式意图、编号、交叉引用、资源、模板元数据
  ↓
Target Render Tree
  - 面向 DOCX/HTML/PDF 等目标格式的渲染节点
  ↓
Renderer / Packer
```

`Source CST` 用于调试、错误定位和可逆追踪；`Standard Doc AST` 用于所有目标格式共享；`Target Render Tree` 用于承载目标格式特定结构，例如 DOCX 的 relationships、section properties、header/footer、field code、media part。

#### 4.4.2 AST 顶层模型

```rust
pub struct StandardDocument {
    pub schema_version: String,
    pub source: SourceBundle,
    pub artifacts: BuildArtifacts,
    pub profile: FormatProfileRef,
    pub metadata: DocumentMetadata,
    pub numbering: NumberingState,
    pub bibliography: BibliographyState,
    pub resources: ResourceIndex,
    pub blocks: Vec<BlockNode>,
    pub diagnostics: Vec<Diagnostic>,
}
```

核心要求：

- `schema_version` 必须显式记录，后续 AST dump、质量报告和映射规则都以此兼容。
- `source` 保存 main tex、include 文件、source span、hash，保证 AST 可追溯到 TeX 源。
- `artifacts` 保存 PDF、BBL、AUX、LOG 等 TeX 侧产物路径与 hash。
- `profile` 指向 `jos-2025` 等模板 Profile。
- `metadata` 保存题名、作者、摘要、关键词、页眉页脚、投稿模板字段。
- `numbering` 保存章节、图、表、算法、公式、参考文献编号状态。
- `resources` 保存图片、PDF 图转换结果、字体、外部链接、media id。
- `diagnostics` 保存未解析命令、缺失图片、未解析引用、RawFallback 等问题。

#### 4.4.3 节点模型

```rust
pub struct BlockNode {
    pub id: NodeId,
    pub kind: BlockKind,
    pub source_span: Option<SourceSpan>,
    pub label: Option<String>,
    pub number: Option<NumberingValue>,
    pub style_intent: StyleIntent,
    pub layout: LayoutHints,
    pub metadata: NodeMetadata,
    pub children: Vec<BlockNode>,
}

pub enum BlockKind {
    FrontMatter(FrontMatterKind),
    Heading { level: u8, runs: Vec<InlineNode> },
    Paragraph { runs: Vec<InlineNode> },
    Figure(FigureNode),
    Table(TableNode),
    Algorithm(AlgorithmNode),
    Equation(EquationNode),
    List(ListNode),
    Bibliography(BibliographyNode),
    AuthorBio(Vec<InlineNode>),
    RawFallback { raw: String, reason: String },
}
```

`BlockNode` 必须同时保存语义和格式意图。比如 `FigureNode` 不仅要有图片路径，还要有 caption、label、width factor、解析到的 media resource、期望 caption 样式和 keep-with-next 规则。

#### 4.4.4 行内模型

```rust
pub enum InlineNode {
    Text {
        text: String,
        style: InlineStyle,
        source_span: Option<SourceSpan>,
    },
    Citation {
        keys: Vec<String>,
        rendered: String,
        number_refs: Vec<usize>,
    },
    CrossRef {
        label: String,
        target: Option<NodeId>,
        rendered: String,
    },
    MathInline {
        latex: String,
        normalized: String,
        runs: Vec<InlineNode>,
    },
    Link {
        url: String,
        text: String,
    },
}
```

`InlineStyle` 至少覆盖：

- bold / italic / code
- superscript / subscript
- font hint
- language hint
- citation role
- math role

这可以解决当前正文引用、公式上下标、`\textbf`、`\textit` 在 writer 阶段重复猜测的问题。

#### 4.4.5 格式细节抽象

标准 AST 不直接存储 DOCX XML，但应保存足够的格式意图：

```rust
pub struct StyleIntent {
    pub semantic_role: SemanticRole,
    pub profile_style: Option<String>,
    pub font_hint: Option<FontHint>,
    pub paragraph_hint: ParagraphHint,
}

pub struct LayoutHints {
    pub keep_next: bool,
    pub keep_lines: bool,
    pub allow_split: bool,
    pub width: Option<LengthExpr>,
    pub alignment: Option<Alignment>,
    pub spacing: Option<SpacingHint>,
}
```

原则是：AST 保存“这是什么、希望如何排版”，Render Tree 才保存“DOCX 具体 XML 应怎么写”。例如算法块在 AST 中是 `AlgorithmNode`，带行号、缩进、注释、竖线指引；到了 DOCX Render Tree 才变成固定列宽 `w:tbl`。

### 4.5 AST Markdown 可核验输出

为方便人工核验，标准 AST 必须支持 Markdown dump。该 dump 不是最终论文正文，而是“结构化审计视图”，用于确认 TeX 语法是否被正确提炼。

#### 4.5.1 输出命令

建议新增 CLI：

```bash
doc-engine ast-dump \
  --root examples/paper3 \
  --main-tex latex/main-jos.tex \
  --profile jos-2025 \
  --format md \
  --out examples/paper3/output/main-jos.ast.md
```

同步支持 JSON：

```bash
doc-engine ast-dump \
  --root examples/paper3 \
  --main-tex latex/main-jos.tex \
  --profile jos-2025 \
  --format json \
  --out examples/paper3/output/main-jos.ast.json
```

#### 4.5.2 Markdown dump 结构

```markdown
# AST Dump: main-jos.tex

## Document Metadata

| Field | Value | Source |
|---|---|---|
| title_zh | ... | latex/main-jos.tex:42 |
| running_header | ... | latex/main-jos.tex:40 |

## Build Artifacts

| Artifact | Path | Hash | Status |
|---|---|---|---|
| bbl | latex/main-jos.bbl | ... | loaded |
| pdf | latex/main-jos.pdf | ... | loaded |

## Numbering

| Kind | Label | Number | Source |
|---|---|---:|---|
| figure | fig:arch | 1 | sections/zh/03_system.tex:24 |
| table | tab:compare | 5 | sections/zh/06_experiments.tex:11 |

## Blocks

### [B001] Heading level=1 number=1

- source: `sections/zh/01_intro.tex:1`
- style_intent: `JOSHeading1`
- text: `1 引言`

### [B018] Figure number=1 label=fig:arch

- source: `sections/zh/03_system.tex:24`
- image: `../figures/fig1_system_overview.png`
- width: `0.9\textwidth`
- caption: `图 1 ...`
- layout: `keep_next=true`

### [B044] Algorithm number=1 label=alg:attention

| line | indent | code | comment |
|---:|---:|---|---|
| 1 | 0 | foreach ... do | |
| 2 | 1 | if ... then | |

## Diagnostics

| Severity | Code | Message | Source |
|---|---|---|---|
| warning | unresolved-ref | ... | ... |
```

#### 4.5.3 dump 验收标准

- 每个 `figure/table/algorithm/equation` 都有 label、number、source span。
- 每个 `\cite` 都能在 dump 中看到 key、编号、渲染文本。
- 每个 `\ref` 都能看到 label、目标节点、渲染文本。
- `RawFallback` 节点必须出现在 Diagnostics 中，不能静默混入正文。
- Markdown dump 可以直接由评审人员读出文档结构是否正确。

### 4.6 转换语法映射管理

标准 AST 之后必须增加映射管理层，避免 writer 直接理解 TeX 或模板细节。

```rust
pub struct MappingRegistry {
    pub profile_id: String,
    pub block_mappers: HashMap<BlockKindDiscriminant, BlockMapper>,
    pub inline_mappers: HashMap<InlineKindDiscriminant, InlineMapper>,
    pub style_mappers: StyleMapperSet,
    pub resource_mapper: ResourceMapper,
    pub diagnostics_policy: DiagnosticsPolicy,
}
```

JOS Profile 中的映射示例：

| Standard AST | DOCX Render Tree | JOS 样式/规则 |
|---|---|---|
| `Heading(level=1)` | `ParagraphRenderNode` | `JOSHeading1` |
| `Paragraph` | `ParagraphRenderNode` | `JOSBody` |
| `Citation` | `RunRenderNode` | superscript |
| `Figure` | `DrawingRenderNode + Caption` | `JOSImage` + `JOSCaption` |
| `Table` | `TableRenderNode` | 左右开口、内部细线、表头加粗 |
| `Algorithm` | `TableRenderNode` | 固定列宽、行号、缩进竖线、注释列 |
| `Equation` | `MathRenderNode` | OMML 或 JOSCode |
| `BibliographyEntry` | `ParagraphRenderNode` | `JOSReference` 悬挂缩进 |

映射层职责：

- 根据 Profile 选择目标样式和 layout hints。
- 将 AST 中的资源 id 映射为 DOCX relationship id。
- 将 AST 中的 field intent 映射为 Word field code，例如 `PAGE`。
- 对未支持节点执行策略：失败、占位、RawFallback、或降级文本。
- 输出映射诊断，供质量报告使用。

映射规则应来自标准知识库，而不是散落在 Rust match 分支或 Python if/else 中。代码层只负责执行规则、校验规则和提供少量不可声明化的算法插件，例如 `algorithm2e` 递归解析、表格列宽计算、图片尺寸换算。

### 4.6.1 规则存储方案

语法标准和映射关系涉及大量元数据，必须规划存储方式。可选方案如下。

| 方案 | 优点 | 缺点 | 适用阶段 |
|---|---|---|---|
| 纯文件化 YAML/JSON/TOML | 易版本控制、易 review、适合随代码发布、无需运行时依赖 | 查询能力弱，规则关系复杂时校验成本高 | Phase 1-3 |
| SQLite 本地数据库 | 支持索引、查询、迁移、版本表、规则依赖关系，适合规则数量增长 | 二进制库文件不利于 code review，需要导入/导出机制 | Phase 4 以后 |
| 混合方案 | 源规则用文件管理，构建时编译为 SQLite/cache，兼顾 review 与查询性能 | 工具链复杂度更高 | 推荐最终方案 |

推荐采用分阶段混合方案：

1. **开发期源文件化**：所有规则以 `standards/**/*.yaml` 或 `profiles/**/*.yaml` 存储，进入 Git 管理。
2. **构建期校验**：用 schema 校验规则文件，生成 `standards.lock.json`，固定规则版本和 hash。
3. **运行期可选编译缓存**：CLI/Server 可将规则编译到 SQLite，提升查询、版本比对、规则依赖追踪能力。
4. **发布期双产物**：发布包同时包含源规则文件和编译后的 cache；cache 可删除后重建。

### 4.6.2 建议目录结构

```text
standards/
  tex/
    core.yaml
    latex2e.yaml
    packages/
      algorithm2e.yaml
      natbib.yaml
      graphicx.yaml
      booktabs.yaml
  ooxml/
    opc.yaml
    wordprocessingml.yaml
    relationships.yaml
    field-codes.yaml
  mappings/
    standard-ast-to-docx.yaml
profiles/
  jos-2025/
    profile.yaml
    styles.yaml
    front-matter.yaml
    quality-thresholds.yaml
    overrides.yaml
```

示例规则：

```yaml
id: latex2e.section
source: LaTeX2e
syntax:
  command: "\\section"
  arguments:
    - kind: required_group
semantic:
  ast_kind: Heading
  fields:
    level: 1
    title: arg0
numbering:
  counter: section
diagnostics:
  on_unclosed_group: error
```

映射规则：

```yaml
id: map.heading1.docx.jos
source_ast_kind: Heading
when:
  level: 1
target:
  render_node: Paragraph
  style: JOSHeading1
  keep_next: false
  keep_lines: true
validation:
  required_text: true
```

### 4.6.3 数据库存储设计

当规则规模扩大后，可引入 SQLite 作为本地规则索引与缓存。

建议表：

```sql
rules(
  id text primary key,
  kind text,
  namespace text,
  version text,
  source text,
  body_json text,
  hash text,
  enabled integer
)

rule_dependencies(
  rule_id text,
  depends_on text
)

profiles(
  id text primary key,
  version text,
  body_json text,
  hash text
)

profile_overrides(
  profile_id text,
  rule_id text,
  body_json text
)

mapping_rules(
  id text primary key,
  profile_id text,
  source_ast_kind text,
  target_format text,
  body_json text,
  priority integer
)

schema_versions(
  name text primary key,
  version text,
  migrated_at text
)
```

运行策略：

- 默认从文件读取，保证开发透明。
- 如果存在 SQLite cache 且 hash 匹配，则直接加载 cache。
- 如果规则文件变化，则重建 cache。
- CI 必须验证文件规则和 cache 生成结果一致。

这避免把规则“锁死”在数据库里，也避免在规则复杂后只能靠线性扫描文件。

### 4.7 DOCX Render Tree

DOCX Writer 不应直接遍历 Standard AST 写 XML，而应先生成目标渲染树：

```rust
pub struct DocxRenderTree {
    pub package: PackagePlan,
    pub document: Vec<DocxBlock>,
    pub styles: StylePlan,
    pub sections: Vec<SectionPlan>,
    pub headers: Vec<HeaderPlan>,
    pub footers: Vec<FooterPlan>,
    pub relationships: Vec<RelationshipPlan>,
    pub media: Vec<MediaPlan>,
}
```

优势：

- 可以在写 ZIP 前检查 relationships、media、content types 是否完整。
- 可以单独 dump render tree，定位“AST 正确但 DOCX 映射错误”的问题。
- 可以对 LibreOffice 兼容性做目标树级修正，而不污染 AST。

建议支持：

```bash
doc-engine render-dump \
  --input examples/paper3/output/main-jos.ast.json \
  --target docx \
  --out examples/paper3/output/main-jos.docx-render.md
```

### 4.8 AST 与现有模块的关系

现有 `semantic-ast` crate 应升级为 Standard AST 的承载层，而不是只保存粗粒度 `Document/Block/TextRun`。

迁移策略：

1. 保留现有 `Document` API，新增 `StandardDocument`。
2. `lower_to_document` 先产出 `StandardDocument`，再提供兼容转换到旧 `Document`。
3. `docx-writer` 新增 `render_standard_document(profile, doc) -> DocxRenderTree`。
4. 旧 writer 路径逐步切换到 Render Tree。
5. `quality` 同时读取 AST dump 和 DOCX dump，做源结构与目标结构一致性校验。

## 5. 核心算法完善方案

### 5.1 TeX Artifact Builder

#### 问题

当前 `build_docx.sh` 只比较 `main-jos.tex` 与 PDF/BBL 的 mtime。如果章节文件或参考文献变化，可能错误复用旧 PDF/BBL。

#### 方案

引入 artifact manifest：

```json
{
  "main": "latex/main-jos.tex",
  "inputs": {
    "latex/main-jos.tex": "sha256",
    "latex/sections/zh/01_intro.tex": "sha256",
    "latex/references.bib": "sha256",
    "latex/rjthesis.cls": "sha256"
  },
  "outputs": {
    "pdf": "latex/main-jos.pdf",
    "bbl": "latex/main-jos.bbl"
  },
  "engine": "latexmk -xelatex -bibtex",
  "built_at": "..."
}
```

重建条件：

- manifest 不存在。
- 任意输入 hash 变化。
- PDF/BBL 缺失。
- TeX 引擎版本变化。

Rust 迁移点：

- `tex-facade` 新增 `build_artifacts(project_root, main_tex, profile) -> BuildArtifacts`。
- CLI `build` 命令增加 `--refresh-artifacts` 和 `--reuse-artifacts`。

### 5.2 参考文献与引用编号

#### 当前有效算法

Python 参考实现从 `.bbl` 解析：

```text
parse_bbl:
  split \bibitem{key}
  key_to_num[key] = index
  clean_bibitem(body)
```

正文中：

```text
\cite{a,b,c} -> [1,2-4]
```

#### 完善方案

在 `bib` crate 中统一实现：

- `BblDocument { entries: Vec<BibEntry>, key_to_num: HashMap<String, usize> }`
- `compress_numbers(Vec<usize>) -> String`
- `CitationResolver::resolve(keys) -> TextRun::Superscript("[1,2-4]")`

验收标准：

- 正文数字引用全部为 Word 上标 run。
- 参考文献条目数不少于 `.bbl` 条目数。
- 不出现 `\cite`、`[?]`，除非报告明确列出未解析 key。

### 5.3 Label 编号与交叉引用

#### 当前有效算法

Python 参考实现按章节文本扫描环境：

```text
for env in table/figure/algorithm/equation:
  count += 1
  if \label{key}: label_map[key] = number
```

正文中：

```text
\ref{fig:xxx} -> 1
```

#### 完善方案

Rust lowering 阶段增加 `NumberingContext`：

```text
NumberingContext
  section_no
  subsection_no
  figure_no
  table_no
  algorithm_no
  equation_no
  labels: HashMap<String, LabelRef>
```

两遍处理：

1. **Collect pass**：扫描所有章节环境和 label，建立编号。
2. **Render pass**：正文归一化时替换 `\ref{}`，并按上下文生成“图 1”“表 2”“算法 1”或裸编号。

验收标准：

- DOCX/PDF 关键标记覆盖中所有“图/表/算法/公式”引用命中。
- 正文不出现 `fig:`、`tab:`、`alg:`、`eq:` 字面量。

### 5.4 Algorithm2e 语义化与渲染

#### 当前有效算法

Python 参考实现支持：

- `\KwIn` / `\KwOut` -> 输入输出行。
- `\ForEach{cond}{body}` -> 递归解析子行，缩进加一。
- `\If{cond}{body}` -> 递归解析子行，缩进加一。
- `\Return{value}` -> return 行。
- `\tcp*{comment}` -> 右侧注释列。
- `\;` -> 逻辑语句结束。
- `guides` / `end_guides` -> 块竖线与收口线。

#### Rust 数据模型

```rust
pub struct AlgorithmBlock {
    pub caption: String,
    pub label: Option<String>,
    pub io: Vec<AlgorithmIo>,
    pub rows: Vec<AlgorithmRow>,
}

pub struct AlgorithmRow {
    pub line_no: usize,
    pub indent: usize,
    pub guides: Vec<usize>,
    pub end_guides: Vec<usize>,
    pub code: Vec<TextRun>,
    pub comment: Option<String>,
}
```

#### DOCX 渲染

不要用普通 `JOSCode` 段落模拟算法。应沿用 Python 参考实现，渲染为固定布局表格：

- 第一列：行号。
- 中间若干窄列：缩进竖线。
- 主代码列：关键词加粗，变量支持上下标。
- 右注释列：`// comment` 右对齐。
- 每行 `w:cantSplit`。
- 标题行上下边框。

验收标准：

- `Algorithm 1` 文本存在。
- 行号连续。
- `Input:`、`Output:` 存在。
- 注释列存在。
- 不出现 `\ForEach`、`\If`、`\tcp*`。

### 5.5 表格解析与版式

#### 当前有效算法

Python 参考实现：

- 从 `table` 环境提取 `\caption`、`\label`。
- 从 `tabular` 或 `tabular*` 提取列内容。
- 删除 `\toprule`、`\midrule`、`\bottomrule`、`\hline`。
- 按非嵌套 `&` 切列，按 `\\` 切行。
- 单元格文本走 `latex_to_text`。
- DOCX 中表格左右开口，内部横线和竖线细线，表头加粗，字号 7.5pt。

#### 完善方案

`semantic-ast` 的表格模型从 `Vec<Vec<String>>` 扩展为：

```rust
Table {
  caption,
  label,
  rows: Vec<TableRow>,
  rules: TableRules,
}

TableCell {
  runs: Vec<TextRun>,
  colspan: usize,
  align: CellAlign,
  is_header: bool,
}
```

支持优先级：

1. `booktabs` 基础表格。
2. `tabular*` 宽度参数。
3. `\multicolumn`。
4. `p{}`、`m{}`、`@{}` 列定义清理。

验收标准：

- 编号表题数等于源文档 `table` 环境数。
- 表格中不出现 `{}{@{}ll}`、`\toprule` 等源码。
- 表题与表格同页，所有行 `cantSplit`。

### 5.6 图片资产与 PDF 图片兜底

#### 当前有效算法

Python 参考实现：

- 解析 `\includegraphics[width=0.9\textwidth]{...}`。
- 若源为 PDF，优先查同名 PNG；不存在时调用 `pdftoppm -png -singlefile -r 220` 转换。
- 用 Pillow 读取像素尺寸，按正文宽度和 width_factor 计算 EMU。
- 写入 `word/media/imageN.ext` 和 `document.xml.rels` 图片关系。

#### 完善方案

Rust 主链路修复 `image_assets` 传递链：

```text
convert_dir/convert_zip
  -> collect_project_assets
  -> parse_tex_with_vfs
  -> Document.blocks Figure(path)
  -> pack_with_page_setup(image_assets)
  -> serializer embeds image
  -> packer writes word/media/*
```

新增 `ImageResolver`：

- 按 `\graphicspath`、主文件目录、项目根目录、zip 条目路径查找。
- 支持 `.png/.jpg/.jpeg`。
- PDF 图片在本地 CLI 下转 PNG；WASM 下报告 unsupported 并给出占位。

验收标准：

- `word/media/` 中图片数量等于源 `figure` 数。
- DOCX 文件大小进入合理区间。
- 校验脚本图片数、图题对应全部通过。

### 5.7 数学公式与上下标

#### 当前有效算法

Python 参考实现分两层：

- 行内简单数学由 `clean_math` 转为 Unicode/文本，如 `\pm`、`\leq`、`\geq`、`\rightarrow`。
- 输出 run 时识别 `[1,2]`、`^`、`_`，生成 Word 上标/下标 run。

#### 完善方案

短期：

- 对简单公式继续使用文本 run + 上下标。
- 修复 `_` 残留，避免 `d_`、`l_N` 等泄漏。

中期：

- `mathml` crate 对 block equation 输出 OMML。
- 公式编号右对齐或追加到居中公式行尾。

验收标准：

- 公式段落不出现 `bigl`、`bigr`、裸 `^X`。
- 上标、下标 run 数均大于 0。
- `\pm`、`\leq`、`\geq` 等符号正确。

### 5.8 JOS Front Matter 与 Header/Footer

#### 当前有效算法

Python 参考实现从主 TeX 提取：

- `\rjtitle`
- `\rjauthor`
- `\rjinfor`
- `\rjhead`
- `\footnotetext`
- 中文/英文引用格式行
- 英文标题、作者、机构
- `\AbstractContentZh`、`\AbstractContentEn`
- `\KeywordsZh`、`\KeywordsEn`

DOCX 输出：

- 首页 header：软件学报 masthead 三行，左右制表位对齐。
- 奇数页 header：运行页眉 + 页码字段。
- 偶数页 header：`Journal of Software 软件学报` + 页码字段。
- 首页 footer：收稿/修改/采用时间，上边框。
- `w:titlePg` + odd/even headers。

#### 完善方案

将这些规则纳入 `JosProfile`：

```text
JosProfile
  first_header_rows
  odd_header_source = rjhead
  even_header_text
  first_footer_source = footnotetext
  front_matter_order
  front_matter_spacing
```

验收标准：

- 首页期刊信息有 3 个右对齐制表位。
- 页眉页码字段存在于奇偶页 header。
- PDF 和 DOCX 页眉文本均命中。
- front matter 不重复，不泄漏 `\AbstractContentZh` 等宏名。

### 5.9 标准 AST 构建与 Markdown Dump

#### 构建算法

标准 AST 构建应在 lowering 阶段完成，但要显式拆成四个 pass：

```text
Pass A: Source collect
  - IncludeGraph 收集 main/include/source span
  - TeX Artifact Builder 收集 pdf/bbl/aux/log
  - ResourceResolver 收集 graphicspath、图片、字体、链接

Pass B: Semantic collect
  - 提取 front matter 元数据
  - 扫描 section/figure/table/algorithm/equation 编号
  - 解析 bbl 建立 cite_key -> number
  - 建立 label -> NodeId/number/source span

Pass C: AST build
  - paragraph/list/table/figure/algorithm/equation 全部产出 BlockNode
  - inline text/citation/ref/math/link 全部产出 InlineNode
  - 未支持结构产出 RawFallback + Diagnostic

Pass D: Normalize and validate
  - 绑定 style_intent/profile_style
  - 绑定 layout hints
  - 检查资源、引用、编号完整性
  - 输出 StandardDocument
```

#### Markdown dump 算法

```text
write_ast_markdown(doc):
  写 Document Metadata 表
  写 Build Artifacts 表
  写 Resource Index 表
  写 Numbering/Label 表
  for block in doc.blocks:
      写 block id/kind/source/label/number/style/layout
      if block 是 paragraph/heading:
          写 inline run 摘要
      if block 是 figure:
          写 image/caption/width/resource id
      if block 是 table:
          写 caption + 行列预览
      if block 是 algorithm:
          写 line/indent/code/comment 表
      if block 是 equation:
          写 latex/normalized/number
  写 Diagnostics 表
```

Markdown dump 必须稳定排序，便于 Git diff 和人工 review。JSON dump 必须与 Markdown dump 来源一致，不允许两套逻辑分别生成。

#### 与质量报告的关系

质量模块应同时比较三层对象：

1. `StandardDocument`：源文档是否被正确理解。
2. `DocxRenderTree`：AST 是否被正确映射到 DOCX 渲染计划。
3. `.docx` ZIP/XML：最终包是否完整、可打开、可校验。

这可以把错误定位从“DOCX 质量差”细化为：

- parser/lowering 错：AST dump 中缺节点或 RawFallback。
- mapping 错：AST 正确但 render dump 样式/关系错误。
- renderer 错：render tree 正确但 ZIP/XML 缺部件。

## 6. 实施路线图

### Phase 0：固定高保真基线

目标：把已通过的 Python/JOS 路径作为稳定基线。

任务：

- 保留 `scripts/build_docx.sh` 作为 JOS 高保真入口。
- 将输出目录固定为 `examples/paper3/output/to-docx`。
- 将 `verify_jos_docx.py` 的动态源文档推导能力保持为默认行为。
- 增加 `docs/format/jos_2025_docx_format_definitions.json` 的版本说明。
- 在 CI 或本地验证命令中加入：

```bash
./scripts/build_docx.sh
```

通过标准：

- 生成 DOCX。
- `passed=True`。
- 图片数、表格数、算法结构、参考文献、页眉页脚、字符比例全部通过。

### Phase 1：建立标准 AST 骨架

目标：先形成可审计的标准文档 AST，而不是继续在 writer 中补特例。

任务：

1. 在 `semantic-ast` 新增 `StandardDocument`、`BlockNode`、`InlineNode`、`StyleIntent`、`LayoutHints`。
2. `latex-reader` lowering 输出 `StandardDocument`，保留旧 `Document` 兼容转换。
3. 增加 `ast-dump --format md/json`。
4. 将 paper3 的 front matter、章节、图、表、算法、公式、引用、参考文献全部呈现在 AST Markdown dump。
5. `RawFallback` 必须进入 Diagnostics。
6. 建立 `standards/tex`、`standards/ooxml`、`profiles/jos-2025` 的最小规则文件，记录规则来源与 schema version。

通过标准：

- `main-jos.ast.md` 可人工核验文档结构。
- 每个源 `figure/table/algorithm/equation` 均有 AST 节点、编号、label、source span。
- 引用和交叉引用在 AST 中已解析，不等待 DOCX writer 猜测。
- AST 节点可追溯到规则 id，例如 `latex2e.section`、`algorithm2e.ForEach`、`ooxml.w_p`。

### Phase 2：修复 Rust 主链路 P0

目标：解决质量报告中最致命的功能断裂。

任务：

1. 修复图片资产传递链，确保 `word/media/` 写入。
2. 将 Python 的 `parse_algorithm_rows` 回迁到 `latex-reader/src/algorithm.rs`。
3. 在标准 AST 中完善 `AlgorithmNode`。
4. 新增 `MappingRegistry`，将 `AlgorithmNode` 映射为 DOCX 算法表格 Render Tree。
5. 对 LibreOffice 兼容性做最小验证：DOCX 可打开，可转换 PDF，图片存在。

通过标准：

- `scripts/compare_paper3.sh` 不再出现图片缺失。
- DOCX 文件体积接近含图文档合理区间。
- 不出现 algorithm RawFallback。

### Phase 3：修复 Rust 主链路 P1

目标：达到内容可用。

任务：

1. 实现 `.bbl` 引用顺序解析和正文引用上标。
2. 实现两遍 label 编号与 `\ref` 替换。
3. 改造标准表格 AST，支持 booktabs/table/tabular*。
4. 修复公式上下标和常见数学符号。
5. 按 JOS 样式 JSON 对齐字体、行距、段前段后、页边距。

通过标准：

- DOCX/PDF 字符比例不低于 0.90。
- 关键 token 全命中。
- LaTeX 残留为 0。
- 表格、图、算法、公式、引用校验通过。

### Phase 4：映射注册表、Profile 化与多端统一

目标：不再让 JOS 规则散落在脚本和 writer 中，而是由 Profile + Mapping Registry 管理。

任务：

- 引入 `FormatProfile` 文件格式。
- `jos-2025` Profile 驱动 front matter、style、header/footer、quality thresholds。
- 引入 `MappingRegistry`，管理 Standard AST 到 DOCX Render Tree 的 block/inline/style/resource 映射。
- 增加 `render-dump --target docx --format md/json`，用于核验映射结果。
- 建立 `standards.lock.json`，记录规则文件、Profile、映射规则的版本和 hash。
- 评估并实现可选 SQLite cache：文件规则仍是源，SQLite 仅作为运行期索引和缓存。
- CLI 增加：

```bash
doc-engine build \
  --root examples/paper3 \
  --main-tex latex/main-jos.tex \
  --profile jos-2025 \
  --out examples/paper3/output/main-jos.docx \
  --verify
```

- WASM/Web 路径对不能调用 TeX 工具链的能力显式降级，报告缺失项。

通过标准：

- Python 路径与 Rust 路径的 AST dump、render dump、质量报告字段一致。
- Profile 可独立版本化。
- 新文档可通过 Profile 添加，而不是改 writer 代码。
- 修改规则文件后可以重新生成 lock/cache，并通过 CI 一致性检查。

### Phase 5：质量体系产品化

目标：每次转换都有可解释、可追踪、可回归的质量结果。

任务：

- `quality` crate 吸收 `verify_jos_docx.py` 中的结构检查。
- 输出 `report.md`、`report.json`、失败项列表。
- 质量报告同时引用 AST dump、render dump、DOCX XML 检查结果。
- 建立阈值分级：
  - P0：DOCX 无法解包、图片缺失、算法 RawFallback、关键章节缺失。
  - P1：引用未解析、公式残留、表格结构异常、字符比例低。
  - P2：样式、间距、分页保持不足。
- 将 `examples/paper3` 固化为回归样本。

通过标准：

- 任意 PR 可看到 DOCX 质量差异。
- 校验失败可以定位到 AST node、mapping rule、render node、relationship 或 XML 部件。

## 7. 文件级改造建议

| 模块 | 文件 | 改造点 |
|---|---|---|
| CLI/编排 | `crates/cli/src/main.rs` | 新增 `build --profile --verify` 高保真构建入口 |
| TeX 侧产物 | `crates/tex-facade` | 输出 `BuildArtifacts`，支持 latexmk/xelatex/bibtex |
| 标准知识库 | `standards/` | 文件化维护 TeX/LaTeX、OOXML/OPC、映射基础规则 |
| Profile | `profiles/jos-2025/` | 文件化维护 JOS 模板规则、样式、阈值、override |
| 规则加载 | 新增 `crates/standards` 或 `crates/profile` | 加载 YAML/JSON 规则，校验 schema，生成 lock/cache |
| Include/VFS | `crates/latex-reader/src/include.rs` | 将 `graphicspath` 结果暴露给图片解析 |
| 归一化 | `crates/latex-reader/src/normalize.rs` | 对齐 Python 26 步归一化与上下标 run |
| 降级 | `crates/latex-reader/src/lower.rs` | 输出 `StandardDocument`，完成两遍编号、front matter、figure/table/algorithm/equation |
| 算法 | `crates/latex-reader/src/algorithm.rs` | 移植递归 algorithm2e parser |
| Bib | `crates/bib` | `.bbl` 顺序解析、引用压缩 |
| AST | `crates/semantic-ast/src/lib.rs` | 新增标准 AST：`StandardDocument`、`BlockNode`、`InlineNode`、metadata、diagnostics |
| AST Dump | `crates/semantic-ast` / `crates/cli` | 输出 `*.ast.md` 与 `*.ast.json`，用于人工核验 |
| 映射 | `crates/docx-writer` 或新增 `crates/mapping` | `MappingRegistry`：Standard AST 到 DOCX Render Tree 的映射管理 |
| 规则缓存 | 可选 SQLite cache | 运行期索引 `SyntaxRule`、`MappingRule`、`ProfileOverride`，由文件规则编译生成 |
| Render Tree | `crates/docx-writer` | 新增 `DocxRenderTree`、`render-dump`、package/media/rels plan |
| DOCX | `crates/docx-writer/src/serializer.rs` | 从 DOCX Render Tree 写 XML，支持图片、算法表格、JOS front matter、header/footer |
| 打包 | `crates/docx-writer/src/packer.rs` | 确认 `word/media`、rels、content types 完整 |
| 样式 | `crates/docx-writer/src/styles.rs` | 从 Profile/JSON 生成样式，减少硬编码 |
| 质量 | `crates/quality` | 联合 AST dump、render dump、DOCX XML、PDF 文本比对 |

## 8. 验收指标

### 8.1 paper3/JOS 高保真指标

必须满足：

- DOCX 可解包，ZIP 魔数正确。
- `main-jos.ast.md` 和 `main-jos.ast.json` 可生成，且包含 metadata、numbering、resources、blocks、diagnostics。
- AST 中 `RawFallback` 数量为 0，或所有 RawFallback 都有明确非阻断说明。
- `main-jos.docx-render.md` 可生成，且 relationships/media/content types 计划完整。
- AST dump 中关键节点包含标准规则 id 和 Profile 规则 id。
- `standards.lock.json` 可生成，记录 TeX/LaTeX、OOXML、mapping、profile 规则版本。
- `word/document.xml`、`word/styles.xml`、`word/_rels/document.xml.rels` 存在。
- `word/media/*` 图片数等于源 figure 数。
- 表格数、图题数、算法数与源文档一致。
- 正文引用全部上标，且无 `\cite` 残留。
- `\ref` 不泄漏 `fig:`、`tab:`、`alg:`、`eq:`。
- Algorithm 行号连续，含 Input/Output、注释列、缩进竖线。
- 参考文献条目数不少于 `.bbl` 条目数。
- DOCX/PDF 字符比例不低于 0.95。
- 关键标记覆盖 100%。
- 页眉页脚、首页 masthead、页面尺寸、边距符合 JOS Profile。

### 8.2 通用引擎指标

可分级：

- 基础可用：章节、段落、列表、普通表格、图片、参考文献可输出。
- 学术可用：标准 AST 可完整表达公式、引用、label、算法、复杂表格，且无源码泄漏。
- 期刊高保真：AST dump、render dump、Profile 校验通过，PDF 字符比例和结构指标达阈值。

## 9. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| Python 路径成为长期孤岛 | Rust 主引擎继续质量不足 | 将 Python 设为 oracle，并逐项建立 Rust 回迁任务和验收 |
| Profile 过度绑定 paper3 | 无法泛化到其他 JOS 文档 | 所有硬编码先进入 `jos-2025` Profile，再抽象公共字段 |
| TeX 工具链依赖限制 WASM | Web 端无法高保真 | WASM 明确走降级路径，本地/服务器走高保真路径 |
| LibreOffice 兼容性不稳定 | DOCX→PDF 仍失败 | 校验 DOCX XML 部件，必要时用 python-docx/模板容器对照生成兼容 XML |
| 表格/算法解析边界复杂 | RawFallback 回潮 | 对每个不支持结构生成质量失败项，不静默通过 |
| 标准 AST 过度膨胀 | 实现复杂度失控 | 先覆盖 JOS/paper3 的必须节点，schema version 化，按 Profile 渐进扩展 |
| AST dump 与实际渲染不一致 | 人工核验失去意义 | Markdown/JSON dump 必须来自同一 `StandardDocument`，render dump 必须来自同一 `DocxRenderTree` |
| 规则文件和 SQLite cache 不一致 | 运行行为不可复现 | 文件规则是唯一源，cache 必须可删除重建，CI 校验 hash 与 lock |
| 标准规则与模板特例混杂 | 难以扩展其他模板 | 标准规则、mapping、Profile override 分目录、分 schema 管理 |

## 10. 近期执行清单

1. 将 `scripts/build_docx.sh` 写入 README/用户指南，作为当前 JOS 高保真转换命令。
2. 给 `scripts/build_docx.sh` 增加 manifest/hash 级重建判断，避免章节变更后复用旧 BBL/PDF。
3. 在 `scripts/verify_jos_docx.py` 中继续保持动态源文档统计，不再写死 paper3 标题和图表数量。
4. 先定义 `StandardDocument` schema，并输出 `main-jos.ast.md` / `main-jos.ast.json`。
5. 建立最小 `standards/` 与 `profiles/jos-2025/` 规则文件，先覆盖 section、caption、label、cite、figure、table、algorithm、OOXML paragraph/run/table/image relationship。
6. 新增 Rust 单测：以 `scripts/build_jos_docx.py` 输出为 oracle，逐项对比 AST 中的图片、表格、算法、引用。
7. 优先修复 Rust `docx-writer` 图片嵌入链路，但入口改为 AST -> Mapping -> Render Tree。
8. 移植 Python algorithm2e parser 到 `latex-reader/src/algorithm.rs`，产出 `AlgorithmNode`。
9. 移植 `.bbl` 引用顺序和 `label_map` 两遍编号，先体现在 AST dump 中。
10. 建立 `MappingRegistry` 和 `DocxRenderTree` 雏形，输出 render dump。
11. 生成 `standards.lock.json`；SQLite cache 暂作为可选优化，不作为首版硬依赖。
12. 将 `verify_jos_docx.py` 的 JSON schema 固定下来，为 `quality` crate 迁移做准备。

## 11. 结论

质量报告说明 Rust V2 的问题不是“缺一个更好的样式表”，而是特定学术结构没有被语义化，且图片、引用、算法和校验闭环存在断裂。算法总结说明当前工程架构已经具备承载这些能力的分层基础。`build_docx.sh` 证明了可行的高保真路线：TeX 侧产物、JOS 专用解析、直接 OOXML、格式校验四者结合，可以在 paper3 上得到可验收 DOCX。

后续应以这条已验证路径作为质量基准，先产品化 JOS 高保真入口，再建立标准文档 AST，把 TeX 文件中的语义、格式细节、元数据、编号、资源和诊断统一沉淀到可 dump、可核验的中间表示。标准规则应显式对齐 TeX/LaTeX 事实标准、常用宏包规则和 OOXML/OPC 国际标准；规则和映射关系以文件化知识库作为唯一源，必要时编译为本地 SQLite cache 以支持查询、更新和版本追踪。最终目标是形成 `TeX -> Standard Doc AST -> Mapping Registry -> DOCX Render Tree -> Renderer/Packer -> Quality Report` 的学术 DOCX 转换系统：通用引擎负责理解源文档，标准 AST 负责稳定表达，映射层负责模板和目标格式规则，渲染器负责输出，质量模块负责证明转换结果。
