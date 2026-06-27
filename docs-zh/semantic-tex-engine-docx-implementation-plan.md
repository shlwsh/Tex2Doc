# Semantic TeX Engine：XeLaTeX/CTeX 到 DOCX 高保真实现方案
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



> 当前实现进度、缺口和开发任务清单见：[semantic-tex-engine-progress-and-task-plan.md](./semantic-tex-engine-progress-and-task-plan.md)
>
> 最新独立双路径审核方案见：[Semantic TeX Engine 独立 DOCX 转换路径方案（20260620-112803）](./semantic-tex-engine-independent-docx-plan-20260620-112803.md)
>
> 最新双后端语义采集审核方案见：[Semantic TeX Engine 双后端语义采集方案（20260620-115348）](./semantic-tex-engine-dual-backend-design-20260620-115348.md)
>
> 最新开发进展报告见：[Semantic TeX Engine 开发进展报告（20260620-124347）](./semantic-tex-engine-development-report-20260620-124347.md)

## 1. 目标定位

当前项目已经具备 `latex-reader -> semantic-ast -> docx-writer` 的基础链路，但它仍更像一个转换管线。新的目标是把它提升为可演进的语义化 TeX 编译器：

```text
TeX/CTeX source
  -> Semantic Collector
  -> Semantic AST / StandardDocument
  -> Document Graph
  -> DOCX Renderer
```

本阶段新增 `doc-compiler-engine` crate，作为这条管线的稳定 facade。它先复用现有规则解析器和 DOCX 写出器，后续可以逐步替换为 LuaHook、XDV 版式恢复、独立 OMML 公式引擎和模板系统。

## 2. 当前项目映射

| 目标模块 | 当前可复用模块 | 说明 |
|---|---|---|
| Semantic Collector | `doc-latex-reader` | 现阶段用 include graph、Logos/Rowan 解析、规则降级采集语义 |
| Semantic AST | `doc-semantic-ast` | 已有 `Document`、`Block`、`StandardDocument` |
| Formula Engine | `doc-mathml` + `doc-docx-writer` | 已有 LaTeX math 到 MathML/OMML 的基础能力 |
| Document Graph | `StandardDocument` | 作为跨 renderer 的统一文档图 |
| DOCX Renderer | `doc-docx-writer` | 负责 document.xml、styles.xml、rels、media、page setup |
| VFS / Assets | `doc-utils` | 统一 zip、目录、内存文件输入 |
| CLI / WASM / Server | `doc-core`、`cli`、`wasm`、`server` | 保持现有 Rust 转换路径不变；新语义引擎作为独立第二路径 |

## 3. 新增 crate

路径：

```text
crates/compiler-engine
```

包名：

```text
doc-compiler-engine
```

核心类型：

```rust
SemanticTexEngine
CompileOptions
CompileArtifact
DocumentGraph
CompileReport
EngineProfile
```

当前公开入口：

```rust
compile_source_to_docx(main_tex, source, options)
compile_dir_to_docx(project_root, main_tex, options)
compile_zip_to_docx(zip_bytes, main_tex_path, options)
compile_vfs_to_graph(main_tex, vfs, options)
compile_vfs_to_docx(main_tex, vfs, options)
```

这让调用方可以选择：

- 只要 DOCX：直接调用 `compile_*_to_docx`。
- 要调试语义：调用 `compile_vfs_to_graph`，检查 `Document`、`StandardDocument`、图片资产和阶段报告。
- 后续要接 HTML/Markdown renderer：复用 `DocumentGraph`，替换 renderer。

## 4. 编译阶段设计

### 4.1 SourceMount

输入形态统一为 `VirtualFs`：

- 单文件：把 `main.tex` 直接插入 VFS。
- 目录：递归挂载项目根目录。
- zip：解包到 VFS，并拒绝包含 `..` 的不安全路径。

输出：

```text
VirtualFs
main_tex POSIX path
SourceBundle
```

### 4.2 IncludeGraph

使用现有：

```rust
IncludeGraph::build(vfs, main_tex)
IncludeGraph::join(vfs)
```

目标是把多文件 TeX 工程合并为一条带来源信息的输入流。后续需要加强：

- `\input` / `\include` 路径相对性。
- `\graphicspath` 对图片路径的影响。
- 子文件宏定义对主文件的作用域。

### 4.3 TexParse

当前实现：

```rust
parse_tex(&joined.text)
```

这是 rule-based collector 的第一层。它不追求完整 TeX 展开，而是把中文学术论文常见结构稳定识别出来。

后续替换目标：

```text
LuaHook collector
XeTeX/XDV layout collector
Rule engine + LLM fallback
```

### 4.4 SemanticCollect

当前实现复用：

```rust
lower_to_document(...)
lower_to_document_with_cite_map(...)
```

输出 `doc_semantic_ast::Document`，包含：

- `Heading`
- `Paragraph`
- `List`
- `Table`
- `Figure`
- `Equation`
- `Algorithm`
- `Bibliography`
- `RawFallback`

引用处理策略：

- 优先读取同名 `.bbl`，保持 BibTeX 编号顺序。
- 其次读取 `references.bib` 或 `<main>.bib`。
- 解析失败不阻断主流程，回退到普通语义降级。

### 4.5 DocumentGraph

当前图模型：

```rust
StandardDocument::from_legacy_document(&document, source, profile_id)
```

它承载：

- 源文件清单。
- 元数据。
- 块节点。
- 编号状态。
- bibliography 状态。
- resource index。
- diagnostics。

后续应把 `StandardDocument` 从“legacy 包装”升级为主模型，`Document` 只作为兼容层。

### 4.6 AssetCollect

当前策略：

- 收集 PNG/JPEG。
- PDF 图片尝试用 `pdfium-render` 渲染为 PNG。
- 为图片注册完整路径、basename、PDF 原路径和 PNG 别名。

后续加强：

- 读取 LaTeX width/height/scale。
- 与 `graphicspath` 统一。
- 支持 SVG/EPS 的可控降级。

### 4.7 DocxRender

当前实现：

```rust
doc_docx_writer::pack_with_page_setup(
    &document,
    template_bytes,
    Some(&image_assets),
    page_setup,
)
```

生成：

```text
[Content_Types].xml
_rels/.rels
word/document.xml
word/styles.xml
word/_rels/document.xml.rels
word/media/*
word/header*.xml
word/footer*.xml
```

后续目标：

- `template.docx` 样式继承更强。
- 独立 numbering.xml。
- 表格、公式、caption、bookmark 的样式映射可配置。

## 5. Profile 设计

当前内置：

```rust
GenericArticle
ChineseAcademic
JosPaper
MedicalJournal
```

Profile 不应只是页面大小，而应逐步控制：

- 文档类白名单。
- 标题/摘要/关键词抽取规则。
- 字体映射。
- caption 命名规则。
- 参考文献样式。
- DOCX 样式名称映射。
- 兼容性评分阈值。

优先级建议：

```text
JosPaper
ChineseAcademic
MedicalJournal
GenericArticle
```

原因是当前项目已经有软件学报论文样例和 JOS 页眉页脚逻辑，最容易做出可验证闭环。

## 6. 公式引擎方案

短期：

- 继续复用 `doc-mathml` 和 `doc-docx-writer` 内的公式输出能力。
- `Block::Equation { latex, is_block }` 保留原始 LaTeX。

中期：

```text
LaTeX formula
  -> math lexer
  -> math parser
  -> Math AST
  -> OMML renderer
```

优先支持：

```text
\frac
\sqrt
\sum
\prod
\int
matrix
cases
subscript/superscript
\hat
\bar
\vec
```

长期：

- 为未知公式命令提供 fallback：保留原 LaTeX、生成图片或生成近似 OMML。
- 给每个公式节点输出 diagnostics，避免静默丢失。

## 7. 表格方案

不要从 XDV 线条恢复表格作为主路径。主路径应在语义阶段捕获：

```text
\begin{tabular}
\begin{longtable}
\multicolumn
\multirow
booktabs
```

短期目标：

- `tabular` 基础行列。
- `\hline` / `booktabs` 转换为边框策略。
- `\caption` 绑定表格。

中期目标：

- `multicolumn` / `multirow`。
- 列宽推断。
- 表格内段落、公式、脚注。

DOCX 输出目标：

```text
<w:tbl>
  <w:tblPr>
  <w:tr>
  <w:tc>
```

## 8. 图片与浮动体方案

语义采集阶段识别：

```text
\begin{figure}
\includegraphics[width=.8\textwidth]{fig.png}
\caption{...}
\label{...}
```

Document Graph 中应保存：

```text
path
caption
label
scale
width expression
source span
```

DOCX 阶段：

- 复制到 `word/media/*`。
- 写入 `document.xml.rels`。
- 用 DrawingML 输出尺寸。
- caption 写入独立段落并绑定 bookmark。

## 9. 引用与交叉引用方案

需要新增显式 `ReferenceGraph`：

```rust
HashMap<String, NodeId>
```

输入：

```text
\label
\ref
\eqref
\autoref
\cite
```

输出：

- DOCX bookmark。
- 内部 hyperlink。
- citation 文本。
- 未解析引用 diagnostics。

当前 `doc-compiler-engine` 已经为 `.bbl` 和 `.bib` 预留主链路，下一步应把 citation 从段落文本提升为结构化 inline。

## 10. CTeX/XeLaTeX 支持路线

优先支持：

```text
ctexart
ctexrep
article
report
IEEEtran
elsarticle
llncs
acmart
jos / rjthesis
医学期刊模板
```

不建议一开始追求任意 TeX。真正可交付的顺序应是：

```text
中文学术论文
软件学报
医学论文
SCI/英文期刊
通用 TeX
```

## 11. LuaHook/XDV 后续接入

当前 `SemanticCollect` 是规则降级。未来可以新增：

```text
crates/semantic-collector
crates/xdv-parser
```

建议 trait：

```rust
trait SemanticCollector {
    fn collect(&self, source: &SourceBundle, vfs: &VirtualFs) -> Result<DocumentGraph>;
}
```

实现：

- `RuleBasedCollector`：当前 `doc-latex-reader`。
- `LuaHookCollector`：LuaTeX callback 采集宏语义。
- `XdvLayoutCollector`：解析 XDV glyph/rule/special，补充版式信息。
- `AiAssistedCollector`：未知宏语义推断，仅作为可审计 fallback。

XDV 不应负责恢复所有语义，它只负责补足：

- 字体。
- 字号。
- 行距。
- 位置。
- 页。
- glyph 级 layout。

## 12. 兼容性分析器

建议新增：

```text
crates/compatibility-analyzer
```

扫描：

```text
\documentclass
\usepackage
\newcommand
\renewcommand
\DeclareRobustCommand
\begin{tikzpicture}
minted/listings
```

输出：

```json
{
  "score": 92,
  "profile": "jos-paper",
  "unsupported": ["tikz", "minted"],
  "warnings": []
}
```

该分析器应在编译前运行，给用户明确预期。

## 13. 测试策略

最小测试矩阵：

| 层级 | 测试 |
|---|---|
| 单元 | parser/lower/image assets/docx pack |
| crate smoke | `doc-compiler-engine` 单文件和 zip 编译 |
| 样例 E2E | `examples/paper3` 软件学报样例 |
| 结构断言 | docx zip parts、document.xml 内容、media count |
| 视觉回归 | DOCX 转 PDF 后与 oracle PDF 比对 |
| 兼容性 | CTeX、IEEEtran、elsarticle、医学模板 fixture |

新增引擎已经包含：

- 单文件 TeX 到 DOCX smoke。
- zip TeX 到 DOCX smoke。
- 阶段报告断言。
- 标题语义块断言。

## 14. 独立双路径计划

第一步已经完成：

```text
新增 doc-compiler-engine
新增中文技术方案
不替换现有 core/cli 主链路
```

第二步：

```text
保留 doc-core 现有转换引擎
保留 doc-engine convert/build 默认路径
为 doc-compiler-engine 建立独立 profile、renderer、测试和脚本
```

第三步：

```text
生成旧 Rust 引擎 DOCX 与新 Semantic Engine DOCX
输出双路径差异报告
以 paper3 作为首个高保真对照样例
```

第四步：

```text
在新路径内逐步实现 ReferenceGraph、OMML、兼容性分析、LuaHook/XDV
旧路径只作为稳定基线维护，不参与新引擎能力迁移
```

## 15. 研发里程碑

### M1：语义编译 facade

- 新增 `doc-compiler-engine`。
- 支持 source/dir/zip/VFS。
- 输出 `CompileReport`。
- 复用现有 AST 与 DOCX writer。

### M2：Profile 化

- JOS profile 默认页眉页脚和页面设置。
- 中文学术论文 profile。
- 医学论文 profile。
- 样式映射表外置。

### M3：结构增强

- 引用图。
- 表格多行多列。
- 图片尺寸表达式。
- caption/bookmark/hyperlink。

### M4：公式引擎

- 独立 Math AST。
- LaTeX math 到 OMML。
- diagnostics 与 fallback。

### M5：LuaHook/XDV

- LuaHook collector 原型。
- XDV parser 原型。
- layout metadata 合并到 Document Graph。

### M6：兼容性与 AI fallback

- compatibility analyzer。
- rule engine。
- LLM-assisted macro inference。
- 可审计宏语义规则库。

## 16. 工程原则

- 语义优先：不要从 XDV 倒推所有语义。
- XDV 补版式：glyph/layout 只补充视觉信息。
- DOCX 不手搓散乱 XML：继续由 renderer 集中写 OOXML。
- 失败可降级：未知宏进入 diagnostics 或 RawFallback，不直接 panic。
- Profile 先行：先做好中文学术论文和投稿模板，再扩通用 TeX。
