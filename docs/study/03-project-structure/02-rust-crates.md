# `crates/` 详尽说明

> 本节按 crate 列出：**作用、入口、关键文件、依赖、测试**。每个 crate 末尾给出「修改前的最小 checklist」。

---

## 0. 当前 Workspace 总览（2026-06-20）

当前根 `Cargo.toml` 声明 15 个 crate：

| crate 目录 | 包名 | 定位 |
|---|---|---|
| `crates/core` | `doc-core` | FFI/WASM/HTTP 兼容门面，保留旧 `convert_*` API |
| `crates/compiler-engine` | `doc-compiler-engine` | Semantic TeX Engine facade，显式编排 source/dir/zip/VFS → Document Graph → DOCX |
| `crates/utils` | `doc-utils` | VFS、路径、图片、字体工具 |
| `crates/semantic-ast` | `doc-semantic-ast` | `Document`、`Block`、`StandardDocument` 等语义模型 |
| `crates/latex-reader` | `doc-latex-reader` | IncludeGraph、Logos/Rowan 解析、规则降级 collector |
| `crates/docx-writer` | `doc-docx-writer` | OOXML 序列化、样式、图片、页眉页脚、ZIP 打包 |
| `crates/bib` | `doc-bib` | Bib/BibLaTeX 解析 |
| `crates/mathml` | `doc-mathml` | LaTeX math → MathML/OMML |
| `crates/wasm` | `doc-wasm` | Web/扩展 WASM 入口 |
| `crates/native` | `doc-native` | 桌面端 FFI 入口 |
| `crates/server` | `doc-server` | HTTP 服务入口 |
| `crates/tex-facade` | `doc-tex-facade` | xelatex/tectonic/latexmk oracle 编译封装 |
| `crates/docx-pdf` | `doc-docx-pdf` | LibreOffice headless DOCX → PDF |
| `crates/quality` | `doc-quality` | 结构/文本/视觉质量对比 |
| `crates/cli` | `doc-engine` | 统一 CLI：convert、tex-compile、docx-to-pdf、verify-pdf、build、AST/render dump、docx diff |

最新新增的战略入口是 `doc-compiler-engine`。它目前复用 `doc-latex-reader` 和 `doc-docx-writer`，但把语义采集、Document Graph、DOCX 渲染阶段显式暴露出来，后续可替换为 LuaHook/XDV collector 或新增 HTML/Markdown renderer。

---

## 1. `crates/core/` — `doc-core`

### 1.1 作用
**对外门面（FFI / WASM / HTTP 唯一入口）**。整合 `doc-latex-reader` + `doc-docx-writer`，暴露四个公共函数。

### 1.2 目录树

```
crates/core/
├── Cargo.toml                   # 依赖 doc-utils / semantic-ast / latex-reader / docx-writer / bib
├── src/
│   ├── lib.rs                   # 模块声明 + 公共 re-export
│   ├── convert.rs               # 四个转换入口
│   ├── error.rs                 # CoreError 枚举（Io/Parse/Serialize/Unsupported）
│   ├── options.rs               # ConvertOptions / BibStyle / Attachment
│   └── result.rs                # ConvertResult / ProgressEvent / ProgressPhase
└── tests/
    ├── end_to_end.rs            # 端到端（小夹具）
    ├── ieee_fixtures.rs         # IEEE 模板夹具
    ├── paper3_e2e.rs            # 真实工程：paper3 main-jos
    └── output/
        ├── hello.docx
        └── sample.docx
```

### 1.3 关键 API

| 函数 | 用途 | 何时用 |
|------|------|--------|
| `convert_sync(main_tex, source, opts)` | 单文件 + 源文本 | V1 M1-M2 测试；不推荐产线 |
| `convert_dir(project_root, main_tex, opts)` | 真实项目根目录 | CLI / 桌面端 |
| `convert_zip(zip_bytes, main_tex_path, opts)` | 内存 zip | WASM / HTTP |
| `convert_stream(...)` | 异步进度事件（M5-M6 占位） | 暂未启用 |

### 1.4 关键数据结构

```rust
// error.rs
pub enum CoreError {
    Io(String),
    Parse(String),
    Serialize(String),
    Unsupported(String),
}

// options.rs
pub enum BibStyle { Numeric, AuthorYear }
pub struct ConvertOptions {
    pub bib_style: BibStyle,
    pub template: Option<Vec<u8>>,
    pub attachments: Vec<Attachment>,
    pub template_bytes: Option<Vec<u8>>,
}

// result.rs
pub enum ProgressPhase { Reading, Parsing, Lowering, Serializing, Packing, Done }
pub struct ProgressEvent { phase, ratio, message }
pub struct ConvertResult { docx: Vec<u8>, warnings: Vec<String> }
```

### 1.5 测试

| 文件 | 覆盖 |
|------|------|
| `tests/end_to_end.rs` | 最小 docx 头、段落实体 |
| `tests/ieee_fixtures.rs` | IEEE 模板覆盖：嵌套列表 / 表格 / 公式 / 引用 |
| `tests/paper3_e2e.rs` | 真实工程 8 千行 LaTeX + 6 个 include + BibTeX |

### 1.6 修改前 checklist

* [ ] 跑 `cargo test -p doc-core` 全过
* [ ] 跑 `cargo test -p doc-core --test paper3_e2e`（最严格）
* [ ] 公共 API 变更需同步更新 `doc-wasm` / `doc-native` / `doc-server` / `flutter_app/lib/`
* [ ] 跑 `gitnexus impact({target: "doc_core::convert_zip"})`

---

## 1A. `crates/compiler-engine/` — `doc-compiler-engine`

### 1A.1 作用

**Semantic TeX Engine facade**。这是新一代 TeX → DOCX 主链路的稳定编排层：当前复用 rule-based `doc-latex-reader`，但已经把“语义采集、Document Graph、DOCX Renderer”显式建模，便于后续接入 LuaHook、XDV layout collector、独立公式引擎和多 renderer。

### 1A.2 目录树

```
crates/compiler-engine/
├── Cargo.toml
├── src/
│   └── lib.rs                   # SemanticTexEngine + CompileOptions + DocumentGraph + CompileReport
└── examples/
    └── paper3_to_docx.rs        # paper3 目录输入 -> DOCX 示例二进制
```

### 1A.3 关键 API

| API | 用途 |
|---|---|
| `SemanticTexEngine::compile_source_to_docx` | 单文件字符串输入 |
| `SemanticTexEngine::compile_dir_to_docx` | 真实目录输入，paper3 脚本使用该入口 |
| `SemanticTexEngine::compile_zip_to_docx` | zip 包输入 |
| `SemanticTexEngine::compile_vfs_to_graph` | 只生成 `DocumentGraph`，用于调试/后续多 renderer |
| `SemanticTexEngine::compile_vfs_to_docx` | VFS → DOCX |

### 1A.4 关键类型

```rust
pub enum EngineProfile {
    GenericArticle,
    ChineseAcademic,
    JosPaper,
    MedicalJournal,
}

pub struct CompileOptions {
    pub profile: EngineProfile,
    pub template_bytes: Option<Vec<u8>>,
    pub page_setup: Option<doc_docx_writer::PageSetup>,
    pub collect_standard_ast: bool,
    pub enable_bibliography: bool,
}

pub struct DocumentGraph {
    pub document: Document,
    pub standard_document: Option<StandardDocument>,
    pub image_assets: ImageAssets,
    pub report: CompileReport,
}
```

### 1A.5 阶段报告

`CompileReport` 记录以下阶段：

```text
SourceMount
IncludeGraph
TexParse
SemanticCollect
DocumentGraph
DocxRender
```

paper3 当前输出约 250 个语义块、10 个图片资产，生成 DOCX 大约 3.0 MB。

### 1A.6 使用与测试

```bash
cargo test -p doc-compiler-engine

bash scripts/build_paper3_compiler_engine_docx.sh

cargo run -p doc-compiler-engine --example paper3_to_docx -- \
  --project-root examples/paper3/latex \
  --main-tex examples/paper3/latex/main-jos.tex \
  --profile jos-paper \
  --out examples/paper3/output/to-docx/paper3-compiler-engine.docx
```

### 1A.7 修改前 checklist

* [ ] 跑 `cargo test -p doc-compiler-engine`
* [ ] 跑 `bash scripts/build_paper3_compiler_engine_docx.sh`
* [ ] 若改 `CompileOptions` 或 `CompileArtifact`，同步更新 `examples/paper3_to_docx.rs` 和 `docs-zh/semantic-tex-engine-docx-implementation-plan.md`
* [ ] 若改主链路行为，检查 `doc-core::convert_dir/convert_zip` 是否应迁移到该 facade

---

## 2. `crates/utils/` — `doc-utils`

### 2.1 作用
**通用工具库**：虚拟文件系统、路径解析、图片处理、字体探测与映射。**唯一允许持有 VFS / 字体相关代码的 crate**。

### 2.2 目录树

```
crates/utils/
├── Cargo.toml                   # 依赖 thiserror / serde / image
├── src/
│   ├── lib.rs                   # 模块声明 + 公共 re-export
│   ├── error.rs                 # DocError（VfsMissing/Io/InvalidPath/ImageDecode/Unsupported）
│   ├── vfs.rs                   # VirtualFs（BTreeMap<PathBuf, Vec<u8>>）
│   ├── path.rs                  # PathResolver + parse_graphics_path
│   ├── image.rs                 # SupportedFormat / ImageMeta / ImageAssets
│   ├── fontmap.rs               # FontMap（CTeX→Office 默认映射）
│   └── fontdetect.rs            # FontDetector（系统字体探测 + Office 映射）
└── tests/
    └── proptest.rs              # VFS 属性测试
```

### 2.3 关键类型

```rust
// vfs.rs
pub struct VirtualFs { files: BTreeMap<PathBuf, Vec<u8>> }
impl VirtualFs {
    pub fn new() -> Self;
    pub fn insert<P: Into<PathBuf>>(&mut self, path: P, bytes: Vec<u8>);
    pub fn read<P: AsRef<Path>>(&self, path: P) -> DocResult<&[u8]>;
    pub fn mount_dir(&mut self, root: &Path) -> io::Result<usize>;
    pub fn first_existing<'a, I, P>(&self, candidates: I) -> Option<PathBuf>;
    pub fn paths(&self) -> impl Iterator<Item = &PathBuf>;
}

// path.rs
pub struct PathResolver { base_dir: Option<PathBuf>, graphics_paths: Vec<PathBuf> }
pub fn parse_graphics_path(body: &str) -> DocResult<Vec<PathBuf>>;

// image.rs
pub enum SupportedFormat { Png, Jpeg }
pub struct ImageMeta { width: u32, height: u32, format: SupportedFormat }
pub struct ImageAssets { inner: HashMap<String, Vec<u8>> }

// fontmap.rs
pub struct OfficeFont { ascii: String, east_asia: String }
pub struct FontMap { map: HashMap<String, OfficeFont> }
pub fn default_map() -> FontMap;

// fontdetect.rs
pub enum FontStatus { Available, Embed, Fallback }
pub struct FontProbe { name: String, status: FontStatus, recommended: String, system_path: Option<PathBuf> }
pub struct FontDetector { system_dirs: Vec<PathBuf>, fallback: String, office_map: HashMap<String, String> }
pub fn probe_font(name: &str) -> FontProbe;
```

### 2.4 测试

| 文件 | 覆盖 |
|------|------|
| `src/vfs.rs::tests` | 插入/读取、缺失报错、Windows 路径归一化、first_existing |
| `src/path.rs::tests` | graphicspath 解析、未闭合报错、相对 base_dir 解析 |
| `src/image.rs::tests` | PNG 探测、尺寸、重编码往返 |
| `src/fontmap.rs::tests` | 默认映射、自定义覆盖 |
| `src/fontdetect.rs::tests` | 默认创建、未知字体 Fallback、Office 映射命中 |
| `tests/proptest.rs` | VFS 属性测试 |

### 2.5 修改前 checklist

* [ ] `cargo test -p doc-utils` 全过
* [ ] 新增 VFS API 需同步更新 `doc-latex-reader::IncludeGraph`
* [ ] 新增 image format 需同步更新 `doc-docx-writer::pack_with_assets`
* [ ] font 映射新增需考虑 Windows / macOS / Linux 三平台

---

## 3. `crates/semantic-ast/` — `doc-semantic-ast`

### 3.1 作用
**核心语义块模型（长期资产）**。Reader 与 Writer 的解耦点。任何 LaTeX 语法特性都在此消融为「强类型 Enum」。

### 3.2 目录树

```
crates/semantic-ast/
├── Cargo.toml                   # 依赖 serde / serde_json
├── src/
│   ├── lib.rs                   # Document / MetaData / Block / TextRun / TextStyle / BibEntry
│   ├── span.rs                  # Span / SourceId
│   └── visit.rs                 # Visitor trait + BlockCounter
```

### 3.3 关键类型

```rust
// lib.rs
pub struct MetaData {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub abstract_text: Option<String>,
    pub keywords: Vec<String>,
}

pub struct Document { pub metadata: MetaData, pub blocks: Vec<Block> }

pub enum Block {
    Heading { level: u8, text: String, number: Option<String>, span: Span },
    Paragraph { runs: Vec<TextRun>, span: Span },
    List { is_ordered: bool, items: Vec<Vec<Block>>, span: Span },
    Table { rows: Vec<TableRow>, caption: Option<String>, number: Option<String>, span: Span },
    Figure { path: String, caption: Option<String>, scale: f32, number: Option<String>, span: Span },
    Equation { latex: String, is_block: bool, span: Span },
    Bibliography { entries: Vec<BibEntry> },
    RawFallback { text: String, span: Span },
}

pub struct TableRow { pub cells: Vec<TableCell> }
pub struct TableCell { pub runs: Vec<TextRun>, pub colspan: u32, pub rowspan: u32, pub bg_color: Option<String> }

pub struct TextRun { pub text: String, pub style: TextStyle, pub span: Span }
pub enum TextStyle { Plain, Bold, Italic, BoldItalic, Code, MathInline }

pub struct BibEntry { pub key: String, pub authors: Vec<String>, pub title: String, pub year: String, pub venue: Option<String> }

// span.rs
pub struct SourceId(pub u32);
pub struct Span { pub start: u32, pub end: u32, pub source: SourceId }

// visit.rs
pub trait Visitor {
    fn visit_document(&mut self, doc: &Document);
    fn visit_block(&mut self, b: &Block);
    fn visit_run(&mut self, r: &TextRun);
}
pub struct BlockCounter(pub usize);
```

### 3.4 测试

| 文件 | 覆盖 |
|------|------|
| `src/lib.rs::tests` | serde 往返、TextRun 助手 |
| `src/visit.rs::tests` | Visitor 计数 |

### 3.5 修改前 checklist

* [ ] `cargo test -p doc-semantic-ast` 全过
* [ ] 任何 enum 变体变更都属于**破坏性**：必须同步更新 `doc-latex-reader::lower`（生产方）与 `doc-docx-writer::serializer`（消费方）
* [ ] 新增字段需考虑 serde 兼容（可选字段 + `#[serde(default)]`）

---

## 4. `crates/latex-reader/` — `doc-latex-reader`

### 4.1 作用
**LaTeX 解析器**。从源文本构建语法树，再降级到 `doc-semantic-ast::Document`。M1-M7 累积。

### 4.2 目录树

```
crates/latex-reader/
├── Cargo.toml                   # 依赖 doc-utils / doc-semantic-ast / logos / rowan / thiserror / serde
├── src/
│   ├── lib.rs                   # 模块声明 + 公共 re-export
│   ├── lexer.rs                 # Logos 词法（TokKind）
│   ├── green.rs                 # Rowan 语法树节点类型（SyntaxKind + Lang）
│   ├── parser.rs                # Pass-2：朴素 Rowan 树构建
│   ├── lower.rs                 # CST → 语义 AST 降级（M3 完整版）
│   ├── include.rs               # IncludeGraph（Pass-1：include 拓扑 + 拼接）
│   └── expand.rs                # 宏展开（\newcommand / \providecommand / \renewcommand）
└── tests/
    ├── insta_snapshots.rs       # insta 快照测试
    ├── proptest.rs              # 属性测试
    └── snapshots/
        ├── insta_snapshots__list_doc.snap
        ├── insta_snapshots__simple_doc.snap
        └── insta_snapshots__table_doc.snap
```

### 4.3 关键 API

```rust
// lib.rs 公共 re-export
pub use expand::expand_macros;
pub use green::{GreenNode, SyntaxKind, SyntaxNode};
pub use include::IncludeGraph;
pub use lower::lower_to_document;
pub use parser::{parse as parse_tex, Parse};
```

### 4.4 模块责任

* **lexer.rs**（Logos 词法）：定义 `TokKind` 枚举（Command / LBrace / RBrace / Comment / Whitespace / Newline / LineBreak / Par / Dollar / DollarDollar / Error），映射到 `SyntaxKind`。
* **green.rs**（Rowan 节点类型）：定义 `SyntaxKind` 枚举 + `Lang: Language` impl + 类型别名。
* **parser.rs**（Pass-2 解析）：朴素无文法硬编码，仅做 `\begin{...}` 配对 + `{...}` 配对 + Group/Env 容器；不匹配自动补。
* **lower.rs**（Pass-3 降级）：**核心**——
  1. 宏展开（`expand_macros_in`）
  2. 跳过 preamble（`\begin{document}` 之前）
  3. 顶层行扫描 + 环境优先
  4. 段落级命令（`\section` 等）+ 顶层 metadata 命令剥离
  5. 段落 buffer + 触发 flush
  6. 段落内联清洗（`\textbf` / `\textit` / `\cite` / `\href` / `tabular` 嵌套检测）
  7. inline math 抽出为 Equation 块
  8. citation 编号（`\cite{k}` → `[n]`）
  9. heading / figure / table 自动编号
  10. 错误降级（未匹配 → RawFallback）
* **include.rs**（Pass-1 拓扑）：扫描 `\input` / `\include` / `\subfile` / `\graphicspath`；构建 DAG（`IncludeGraph`）；Kahn 拓扑排序；`join()` 拼接为单流文本 + source_map。
* **expand.rs**（宏展开）：支持 `\newcommand{\X}{body}` / `\providecommand` / `\renewcommand` / `[n]` 参数；单 pass，跨段共享宏表。

### 4.5 测试

| 文件 | 覆盖 |
|------|------|
| `src/lexer.rs::tests` | section / 括号 / 注释 |
| `src/parser.rs::tests` | 配对 / 未闭合 / 多余右括号 |
| `src/lower.rs::tests` | 50+ 单元测试：heading、paragraph、textbf、itemize、enumerate、tabular、figure、equation、href、unbalanced、inline math、cite、tabular nested 等 |
| `src/include.rs::tests` | include 扫描、build 拓扑、cycle 检测 |
| `src/expand.rs::tests` | define/expand、word boundary、providecommand、body 中文 |
| `tests/insta_snapshots.rs` | 段落/列表/表格快照 |
| `tests/proptest.rs` | 属性测试 |
| `tests/snapshots/*.snap` | insta 快照数据 |

### 4.6 修改前 checklist

* [ ] `cargo test -p doc-latex-reader` 全过
* [ ] 若修改 `lower.rs` 中段落处理逻辑，跑 `cargo test -p doc-core --test paper3_e2e` 验证真实工程
* [ ] 新增 LaTeX 命令需：lexer 加 token（如有）+ lower 加分支 + 测试
* [ ] 跑 `gitnexus impact({target: "doc_latex_reader::lower"})`

---

## 5. `crates/docx-writer/` — `doc-docx-writer`

### 5.1 作用
**OOXML 序列化与 ZIP 打包**。把 `doc-semantic-ast::Document` 写入符合 ECMA-376 最小子集的 docx。

### 5.2 目录树

```
crates/docx-writer/
├── Cargo.toml                   # 依赖 doc-utils / doc-semantic-ast / doc-mathml / quick-xml / zip / image / base64
├── src/
│   ├── lib.rs                   # 模块声明 + 公共 re-export
│   ├── model.rs                 # OOXML 扁平结构体（Paragraph / Run）
│   ├── packer.rs                # .docx 打包（CONTENT_TYPES / _rels / document.xml / styles.xml）
│   ├── serializer.rs            # AST → OOXML 元素序列化
│   ├── styles.rs                # styles.xml 默认样式表 + apply_font_probes
│   └── template.rs              # reference.docx 解析与合并（M7 简化）
└── tests/
    └── fixtures/                # 内部 docx 夹具（不入仓 docx）
```

### 5.3 关键 API

```rust
// packer.rs
pub fn pack(doc: &Document) -> Result<Vec<u8>, DocxWriteError>;
pub fn pack_with_template(doc: &Document, template_bytes: Option<&[u8]>) -> Result<Vec<u8>, DocxWriteError>;
pub fn pack_with_assets(doc: &Document, template_bytes: Option<&[u8]>, image_assets: Option<&ImageAssets>) -> Result<Vec<u8>, DocxWriteError>;

// serializer.rs
pub fn serialize_document(doc: &Document, image_assets: Option<&ImageAssets>) -> Vec<u8>;

// styles.rs
pub const STYLE_TITLE: &str;
pub const STYLE_HEADING1..3: &str;
pub const STYLE_BODY: &str;
pub const STYLE_LIST_BULLET: &str;
pub const STYLE_LIST_NUMBER: &str;
pub const STYLE_CAPTION: &str;
pub const STYLE_TABLE_HEADER: &str;
pub fn write_styles() -> Vec<u8>;
pub fn apply_font_probes(styles_xml: &mut Vec<u8>, probes: &[FontProbe]);

// template.rs（M7 简化版）
pub struct TemplateStyles { by_id: BTreeMap<String, String>, name_to_id: BTreeMap<String, String> }
pub fn parse_template(docx_bytes: &[u8]) -> Result<TemplateStyles, TemplateError>;
pub fn parse_styles_xml(xml: &str) -> TemplateStyles;
pub fn merge_styles(target_xml: &mut Vec<u8>, template: &TemplateStyles);
```

### 5.4 模块责任

* **model.rs**：扁平数据模型（`Paragraph` / `Run`），仅 V1 最小字段。
* **packer.rs**：构建 docx 包结构：固定常量 `CONTENT_TYPES` / `ROOT_RELS` / `DOC_RELS` + `document.xml` + `styles.xml`；`zip::ZipWriter` 用 `Deflated` 压缩。
* **serializer.rs**：序列化每个 `Block`：
  * Heading → 段落 + 样式 ID
  * Paragraph → 普通段落 + run 样式（bold/italic）
  * List → 缩进式（V1 简化）
  * Table → `<w:tbl>` + 边框 + 单元格（含 `colspan` / `bg_color`）
  * Figure → `<w:drawing>` + `<wp:inline>` + base64（`word/media/imageN.png`），回退占位
  * Equation → `<m:oMath>`（调 `doc-mathml::to_omml`）
  * Bibliography → "参考文献" 标题 + `[key] title (year)`
  * RawFallback → 原文段落
* **styles.rs**：默认 9 个样式（Title/Heading1-3/BodyText/ListBullet/ListNumber/Caption/TableHeader），全 Calibri；`apply_font_probes` 按 `FontProbe` 替换 `w:ascii` / `w:hAnsi` / `w:eastAsia` / `w:cs`。
* **template.rs**：解析 `reference.docx` → 提取 `<w:style ...>` 块 → 按 `w:styleId` 同名覆盖 / 缺失补全。

### 5.5 测试

| 文件 | 覆盖 |
|------|------|
| `src/packer.rs::tests` | pack_minimal（PK 头 + 长度断言） |
| `src/template.rs::tests` | parse_styles_xml_basic、merge_adds_missing、round_trip_via_zip |
| `src/styles.rs::tests` | apply_font_probes 空 / fallback 替换 |

### 5.6 修改前 checklist

* [ ] `cargo test -p doc-docx-writer` 全过
* [ ] 跑 `cargo test -p doc-core --test paper3_e2e` 端到端验证
* [ ] 验证脚本：`scripts/verify_paper3.mjs` 内容断言不挂
* [ ] 修改 OOXML 命名空间时同步更新 WASM / 桌面端

---

## 6. `crates/bib/` — `doc-bib`

### 6.1 作用
**BibLaTeX 解析**。M3 完整实现，支持常见条目类型与字段。

### 6.2 目录树

```
crates/bib/
├── Cargo.toml                   # 依赖 doc-semantic-ast / serde
└── src/
    └── lib.rs                   # parse / parse_raw / BibRawEntry
```

### 6.3 关键 API

```rust
pub fn parse(bib: &str) -> Vec<BibEntry>;     // 直接产出 BibEntry
pub fn parse_raw(bib: &str) -> Vec<BibRawEntry>; // 中间结构
pub struct BibRawEntry { entry_type: String, key: String, fields: Vec<(String, String)> }
```

### 6.4 支持的字段

* `author` / `title` / `year` / `booktitle` / `journal` / `publisher` / `url`
* 条目类型：`@inproceedings` / `@article` / `@book` / `@misc` / `@techreport`

### 6.5 错误降级
未闭合自动补；非法条目跳过；`title` 缺失则条目丢弃。

### 6.6 测试

* 内联在 `lib.rs` 末尾（`#[cfg(test)]` 模块）

### 6.7 修改前 checklist

* [ ] `cargo test -p doc-bib` 全过
* [ ] 新增字段时同步 `from_raw` 与 `BibEntry` 结构

---

## 7. `crates/mathml/` — `doc-mathml`

### 7.1 作用
**公式管道**：LaTeX 数学子集 → `MathExpr` AST → MathML（Presentation）+ OMML（Office MathML）。

### 7.2 目录树

```
crates/mathml/
├── Cargo.toml                   # 依赖 doc-semantic-ast / quick-xml / thiserror
└── src/
    ├── lib.rs                   # 模块声明 + 公共 re-export
    ├── expr.rs                  # MathExpr 枚举
    ├── latex.rs                 # LaTeX 数学子集解析
    ├── mathml.rs                # MathML 序列化
    └── omml.rs                  # OMML 序列化
```

### 7.3 关键 API

```rust
pub use expr::MathExpr;
pub use latex::parse_latex_math;
pub use mathml::to_mathml;
pub use omml::to_omml;
```

### 7.4 关键类型

```rust
// expr.rs
pub enum MathExpr {
    Number(String),
    Ident(String),
    Text(String),
    Op(char),
    Space,
    Sub { base, sub },
    Sup { base, sup },
    SubSup { base, sub, sup },
    Frac { num, den },
    Sqrt { body, index: Option<Box<MathExpr>> },
    Fenced { open, body, close },
    Function { name, arg },
    Matrix { rows: Vec<Vec<MathExpr>> },
    Raw(String),
    Seq(Vec<MathExpr>),
}
```

### 7.5 支持的 LaTeX 语法

* 数字 / 字母标识符（隐式 italic）
* 二元运算符：`+ - * / = < >`
* 上下标：`x^{...}` / `x_{...}`
* 分式：`\frac{a}{b}`
* 根式：`\sqrt{...}` / `\sqrt[n]{...}`
* 括号：`\left( ... \right)`
* 三角函数：`\sin` `\cos` `\tan`（直译为函数应用）
* 希腊字母：`\alpha` ... `\omega`（V1 子集）
* 矩阵：`\begin{matrix} ... \end{matrix}`（V1 占位：仅 `\\` / `&` 解析）

### 7.6 保护
* 嵌套深度限制 `MAX_EXPR_DEPTH = 100`，超出后降级为 `Raw`。

### 7.7 修改前 checklist

* [ ] `cargo test -p doc-mathml` 全过
* [ ] 新增语法需 latex.rs / omml.rs / mathml.rs 三处都加分支
* [ ] 跑 `cargo test -p doc-core --test paper3_e2e` 验证真实工程公式

---

## 8. `crates/wasm/` — `doc-wasm`

### 8.1 作用
**WASM 桥接（cdylib）**。暴露 `convert_zip` / `convert_zip_to_docx` / `version` 三个 `#[wasm_bindgen]` 函数给 JS。

### 8.2 目录树

```
crates/wasm/
├── Cargo.toml                   # 依赖 doc-core / wasm-bindgen / serde-wasm-bindgen / js-sys / zip
├── src/
│   └── lib.rs                   # 全部代码（5 KB）
```

### 8.3 关键 API

```rust
#[wasm_bindgen]
pub fn convert_zip(zip_bytes: &[u8], main_tex_path: &str, options_js: Option<String>)
    -> Result<JsValue, JsValue>;          // 返回 { docx, docx_len, warnings }

#[wasm_bindgen]
pub fn convert_zip_to_docx(zip_bytes: &[u8], main_tex_path: &str, options_js: Option<String>)
    -> Result<js_sys::Uint8Array, JsValue>; // 返回 Uint8Array

#[wasm_bindgen]
pub fn version() -> String;
```

### 8.4 配置

* `crate-type = ["cdylib", "rlib"]`
* 默认 features 关闭
* `console_error_panic_hook` 可选 feature

### 8.5 修改前 checklist

* [ ] `cargo build -p doc-wasm --target wasm32-unknown-unknown`
* [ ] 跑 `npm run build:wasm`
* [ ] 复制 `flutter_app/wasm/pkg/` → `flutter_app/web/wasm/` + `extension/popup/wasm/`
* [ ] 跑 `node scripts/e2e_paper3.mjs` 端到端

---

## 9. `crates/native/` — `doc-native`

### 9.1 作用
**原生 cdylib 桥接（dart:ffi）**。暴露 `extern "C"` 函数给 Flutter 桌面端。

### 9.2 目录树

```
crates/native/
├── Cargo.toml                   # 依赖 doc-core / serde / serde_json / thiserror
│                                # 允许 unsafe_op_in_unsafe_fn
├── src/
│   └── lib.rs                   # 全部 FFI 函数
```

### 9.3 关键 API

```rust
#[no_mangle] pub unsafe extern "C" fn doc_engine_version() -> *const c_char;
#[no_mangle] pub unsafe extern "C" fn doc_engine_last_error() -> *const c_char;
#[no_mangle] pub unsafe extern "C" fn doc_engine_free(ptr: *mut u8);
#[no_mangle] pub unsafe extern "C" fn doc_engine_convert_zip(
    zip_ptr: *const u8, zip_len: usize,
    main_tex_ptr: *const u8, main_tex_len: usize,
    out_docx_ptr: *mut *mut u8, out_docx_len: *mut usize,
    out_warnings_ptr: *mut *mut u8, out_warnings_len: *mut usize,
) -> c_int;
```

### 9.4 内存契约
* 入参：C 字符串 / 字节 + 长度
* 出参：malloc 分配（`libc::malloc` / `memcpy`），Dart 端读完必须 `doc_engine_free`
* 错误：写入 thread-local `LAST_ERROR`，Dart 调 `doc_engine_last_error()` 取出

### 9.5 修改前 checklist

* [ ] `cargo build -p doc-native`（dev / release 两套都验）
* [ ] 跑 `dart run bin/native_smoke.dart`
* [ ] 函数签名变更需同步 `flutter_app/lib/native_bridge.dart`
* [ ] 出参契约变更需同步 `flutter_app/bin/native_smoke.dart`

---

## 10. `crates/server/` — `doc-server`

### 10.1 作用
**Axum HTTP 服务端**。提供 `/api/v1/health` / `/api/v1/version` / `/api/v1/convert`。

### 10.2 目录树

```
crates/server/
├── Cargo.toml                   # 依赖 doc-core / axum / tower / tower-http / tokio / tokio-util / memchr / http / bytes / serde / serde_json / thiserror / tracing / tracing-subscriber / mime / async-trait
├── src/
│   ├── lib.rs                   # 模块声明 + build_router re-export
│   ├── main.rs                  # 二进制入口（tokio::main）
│   ├── routes.rs                # HTTP 路由
│   ├── error.rs                 # ServerError + HTTP 状态码映射
│   └── limits.rs                # MAX_BODY = 50 MiB
└── tests/
    └── api.rs                   # HTTP API 集成测试（reqwest）
```

### 10.3 关键 API

* `GET /api/v1/health` → `{"status":"ok"}`
* `GET /api/v1/version` → `{"name":..., "version":...}`
* `POST /api/v1/convert`（multipart）：
  * `file`：项目 zip 字节（≤ 50 MiB）
  * `main_tex`：可选，缺省 `main-jos.tex`
  * 返回：`application/vnd.openxmlformats-officedocument.wordprocessingml.document`

### 10.4 限制
* 单请求体 50 MiB（`tower_http::RequestBodyLimitLayer` + `axum::body::to_bytes(_, MAX_BODY)`）
* docx 至少 4 KiB + `PK\x03\x04` 魔数（routes.rs 内部断言）

### 10.5 测试

* `tests/api.rs`：使用 `reqwest` + `rustls-tls` 测三接口

### 10.6 修改前 checklist

* [ ] `cargo test -p doc-server` 全过
* [ ] 跑 `cargo run -p doc-server` + `node scripts/e2e_server.mjs`

---

## 11. `crates/cli/`（占位）

* 当前**未实现**（`exclude` 自 workspace）。
* V2 计划：用 `clap` 提供 `tex2doc convert ...` CLI 工具。

---

## 12. 跨 crate 改动优先级

| 改动类型 | 影响 | 应同步更新的 crate |
|----------|------|---------------------|
| `doc-semantic-ast` 新增枚举变体 | 全部 | `doc-latex-reader::lower` + `doc-docx-writer::serializer` + `doc-wasm` / `doc-native` / `doc-server` |
| `doc-core` API 变更 | 全部前端 | `doc-wasm` + `doc-native` + `doc-server` + `flutter_app/lib/*` |
| `doc-utils` VFS 变更 | 间接 | `doc-latex-reader::IncludeGraph` |
| `doc-mathml` 新增语法 | 单点 | `doc-docx-writer::serializer::write_equation` |
| `doc-bib` 新增字段 | 单点 | `doc-docx-writer::serializer::Block::Bibliography` |

---

## 13. 进一步阅读

* [05-key-tech/](../05-key-tech/) — 深入解析每个 crate 的关键模块
