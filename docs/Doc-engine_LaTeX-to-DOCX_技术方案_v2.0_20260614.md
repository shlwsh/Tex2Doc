# Doc-engine：LaTeX → DOCX 纯 Rust 核心 + Flutter 全平台转换工具 —— 细化技术方案（V2.0）

> 版本：V2.0（细化自 V1 方案）
> 基线文档：`Doc-engine：LaTeX → DOCX 纯 Rust 核心 + Flutter 全平台转换工具完整技术实现方案（V1）.md`
> 编制日期：2026-06-14
> 文档状态：待评审（细化方案）

---

## 0. 文档目的

本文档在 V1 方案基础上，对架构、模块边界、接口契约、数据结构、错误模型、测试与发布策略进行**工程化细化**，为后续 14 周双轨开发提供可直接落地的实现指南。V1 范围矩阵、Semantic AST 的长期资产定位以及 14 周双轨里程碑在本文中保持不变，本文重点**补全 V1 未明确的接口/契约/算法/质量门禁**。

---

## 1. 范围与非目标（继承并细化 V1）

### 1.1 V1 范围矩阵（继承）

| 类别 | 内容 |
|---|---|
| **核心支持** | 元数据（标题 / 作者 / 摘要 / 关键词）、多级标题与正文、嵌套列表（enumerate / itemize）、标准学术表格（含三线表）、图片插入与交叉引用（`\includegraphics` / `\ref`）、行内 / 块级数学公式、多文件嵌套（`\include` / `\input`）、BibLaTeX 解析与末尾 Bibliography、中文字符与 CTeX 字体映射 |
| **明确排除** | TikZ/PGF、Beamer、用户自定义宏展开机制、PDF/PPT 等非 DOCX 写器、LLM 格式自愈、双向同步编辑与协同 |

### 1.2 V1 细化非目标（新增）

- 不实现 LaTeX 排版引擎（即不输出 PDF，**不与 `tectonic` / `xelatex` 在产物维度竞争**）。
- 不试图 100% 覆盖 `amsmath` / `tcolorbox` 等宏包语法；未识别宏统一降级为 `Plain Text` + `Warning Log`。
- V1 不做 GUI 反向编辑（从 DOCX 改回 LaTeX）。
- V1 不提供云端账户体系与计费；`server` 仅做无状态转换代理。

### 1.3 V1 成功标准（新增，可量化）

| 指标 | 目标 |
|---|---|
| 端到端成功率（IEEE / Springer / arXiv 样例集 ≥ 50 篇） | ≥ 95% |
| 中文字符渲染正确率（含 CTeX 字体映射样例 ≥ 20 篇） | ≥ 98% |
| 公式可编辑率（OMML 而非图片化的比例） | ≥ 90% |
| 桌面端 100 页论文 P50 转换耗时 | ≤ 8 s（M2 笔记本基线） |
| WASM 端 ≤ 5 MB 单文件 P50 转换耗时 | ≤ 12 s（含 WASM 冷启动） |
| 样式回归（insta 快照） | 0 例非预期 diff |

---

## 2. 总体架构（继承 + 细化）

### 2.1 三层拓扑

```
┌──────────────────────────────────────────────────────────────────────┐
│  表现层 (Presentation)                                               │
│  • Flutter Desktop (Windows/macOS/Linux)                             │
│  • Flutter Mobile (Android/iOS)                                      │
│  • Flutter Web (PWA)                                                 │
│  • Chrome Extension (Manifest V3)                                   │
│  • CLI (clap v4)                                                     │
└──────────────────────────────────────────────────────────────────────┘
                  │ FFI (flutter_rust_bridge v2)
                  │ WASM (wasm-bindgen)
                  │ HTTP (Axum Multipart)
┌──────────────────────────────────────────────────────────────────────┐
│  桥接层 (Bridge)                                                      │
│  • core: 统一对外门面（FFI / WASM 入口）                              │
│  • wasm: WASM 专用包装（裁剪、内存预算）                              │
│  • server: Axum 异步 Web 转换服务                                     │
└──────────────────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────────────────┐
│  核心逻辑层 (Rust Core)                                               │
│  • latex-reader   : Logos + Rowan 双阶段解析                         │
│  • semantic-ast   : 强类型 Enum 语义块模型（长期资产）                │
│  • docx-writer    : docx-model / docx-serializer / docx-packer       │
│  • bib            : biblatex 解析                                    │
│  • utils          : VFS、图片解码、include 解算、CTeX 字体映射表     │
└──────────────────────────────────────────────────────────────────────┘
```

### 2.2 关键架构原则

1. **Reader / Writer 完全解耦**：所有数据交互只通过 `semantic-ast`。这保证未来可平滑增加 Markdown / HTML / Typst 等新 Writer。
2. **解析容错优先**：Rowan 维护无损语法树（LST），未识别宏降级为 `SyntaxError` 节点，**绝不抛 panic 中断流水线**。
3. **样式集中、字体内联禁止**：所有段落 / 字符 / 表格样式在 `styles.xml` 统一定义；`document.xml` 仅写引用。
4. **图片二进制与 XML 严格分离**：图片经 `utils` 解码、压缩、按 `word/media/*` 命名后由 `docx-packer` 一并打包。
5. **错误模型分层**：解析错误（Warning）、断言错误（Recoverable Error）、致命 IO 错误（Fatal Error）三类，前端按级别分别走 Toast / 红色 Banner / 阻塞对话框。

---

## 3. Monorepo 目录结构（细化）

```
doc-engine/
├── Cargo.toml                       # Workspace：定义 members 与共享依赖版本
├── rust-toolchain.toml              # 锁定 stable-1.82（具体版本评审时定）
├── .rustfmt.toml
├── clippy.toml
├── deny.toml                        # cargo-deny：依赖白名单
├── crates/
│   ├── core/                        # 统一 FFI/WASM 门面
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── api.rs               # 同步 convert / 异步 convert_stream
│   │   │   ├── progress.rs          # 进度事件枚举
│   │   │   └── error.rs             # 跨层错误模型（thiserror）
│   │   └── tests/ffi_smoke.rs
│   ├── latex-reader/
│   │   ├── src/
│   │   │   ├── lexer.rs             # Logos dfa
│   │   │   ├── parser.rs            # Rowan 语法树构建
│   │   │   ├── include_resolver.rs  # Pass-1 拓扑构建
│   │   │   └── recovery.rs          # 错误恢复策略
│   │   └── tests/fixtures/
│   ├── semantic-ast/
│   │   ├── src/
│   │   │   ├── lib.rs               # Document / Block / TextRun 等
│   │   │   ├── span.rs              # Range<usize> + 位置换算
│   │   │   └── visit.rs             # Visitor trait
│   │   └── tests/serde_roundtrip.rs
│   ├── docx-writer/
│   │   ├── src/
│   │   │   ├── model.rs             # OOXML 扁平结构体
│   │   │   ├── styles.rs            # styles.xml 生成器
│   │   │   ├── serializer.rs        # AST → OOXML 流式写入
│   │   │   ├── mathml_to_omml.rs    # 公式映射管道
│   │   │   ├── packer.rs            # zip 打包
│   │   │   └── template.rs          # reference.docx 样式继承
│   │   └── tests/ooxml_validate.rs
│   ├── bib/
│   │   ├── src/
│   │   │   ├── parser.rs
│   │   │   ├── style.rs             # 常见 citation style 渲染
│   │   │   └── resolver.rs          # 引用链接解析
│   │   └── tests/
│   ├── utils/
│   │   ├── src/
│   │   │   ├── vfs.rs               # 虚拟文件系统
│   │   │   ├── image.rs             # 格式探测 + 重压缩
│   │   │   ├── fontmap.rs           # CTeX → Office 字体映射
│   │   │   └── path.rs              # include 路径解算
│   │   └── tests/
│   ├── server/                      # Axum 异步 Web 转换代理
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── routes.rs            # POST /convert, GET /health
│   │   │   ├── queue.rs             # tokio 任务队列 + 并发上限
│   │   │   └── limits.rs            # 大文件分流策略
│   │   └── tests/api.rs
│   └── wasm/                        # 包装层，导出 wasm-bindgen 接口
│       ├── src/lib.rs
│       └── pkg/                     # wasm-pack 输出
├── flutter_app/
│   ├── lib/
│   │   ├── main.dart
│   │   ├── app.dart                 # Material 3 / 路由
│   │   ├── bridge/                  # 调用生成的 FFI
│   │   ├── features/
│   │   │   ├── workspace/           # 中央看板
│   │   │   ├── options/             # 高级选项侧边栏
│   │   │   ├── progress/            # 状态总线
│   │   │   └── logs/                # 实时日志抽屉
│   │   ├── state/                   # Riverpod providers
│   │   └── theme/                   # M3 主题
│   ├── rust/                        # flutter_rust_bridge 生成的 Dart 绑定
│   ├── assets/
│   │   ├── templates/               # reference.docx（IEEE / Springer / 自定义）
│   │   └── fonts/                   # CTeX 字体映射兜底
│   └── test/
├── extension/                       # Chrome MV3 扩展
│   ├── manifest.json
│   ├── background.js                # Service Worker
│   ├── popup/
│   │   ├── popup.html
│   │   └── popup.dart → 编译为 JS
│   └── content/                     # 上下文菜单注入
├── tests/                           # 端到端 + insta 快照
│   ├── ieee_fixtures/
│   ├── springer_fixtures/
│   ├── arxiv_fixtures/
│   ├── ctex_fixtures/
│   └── snapshots/                   # insta 期望输出
├── scripts/
│   ├── link_cursor_skills.sh
│   └── ci_*.sh
└── docs/
    ├── architecture.md
    ├── api_contracts.md
    ├── error_codes.md
    └── tasks_v2.0_20260614.md
```

---

## 4. 核心模块细化设计

### 4.1 latex-reader

#### 4.1.1 Token 与词法

- **引擎**：`logos` 的 DFA 模式，覆盖：
  - 控制序列：`\\[A-Za-z@*]+`（含 `\` 后接字母或 `@`）
  - 分组符：`{` `}` `[` `]`
  - 数学定界符：`$` `$$` `\( ` `\)` `\[` `\]` `\begin{equation}` …
  - 注释：`%` 至行尾
  - 空白 / 换行（含 `\par` / `\\` / `\newline`）
  - 字符串字面量

#### 4.1.2 双阶段解析

- **Pass 1 (Pre-processor / Include Resolver)**
  1. 扫描 `\include{...}` / `\input{...}` / `\subfile{...}` 指令。
  2. 通过 `utils::vfs` 在以下位置按优先级查找：
     - 当前 `.tex` 所在目录
     - `\graphicspath{}` 声明路径
     - `TEXINPUTS` 环境变量路径
  3. 构造**有向无环图（DAG）**；遇环立即报错（`IncludeCycle`）。
  4. 拼接为**单一连续 Token 流**，保留各 token 原始 `SourceId` 便于 span 还原。

- **Pass 2 (Syntax Builder)**
  - 选用 `rowan` 作为 CST（Concrete Syntax Tree）存储。
  - 节点类型（`SyntaxKind`）至少包含：
    - `Root`, `Command`, `Group`, `Env{begin,end}`, `Text`, `MathInline`, `MathDisplay`, `Comment`, `Whitespace`, `Error`
  - 错误恢复策略：
    1. 遇到未识别宏 → 不消费，标记当前 token 为 `Error` 节点。
    2. 若下一 token 是 `{` 则跳过整组。
    3. 若未匹配 `\begin` 的 `\end` → 跳过至下一 `\begin` 或文件末尾。
    4. **绝不**因单点错误中断整个文件解析。

#### 4.1.3 AST 降级（Reader → semantic-ast）

- 由 `semantic-ast` 提供 `Lowering` trait，将 Rowan CST 降级为 `Document`。
- 降级规则：
  - `\section{...}` / `\subsection{...}` → `Block::Heading`
  - 段落（连续非空文本节点）→ `Block::Paragraph`
  - `\begin{itemize}...\end{itemize}` → `Block::List { is_ordered: false }`
  - `\begin{tabular}...\end{tabular}` → `Block::Table`
  - `\includegraphics{...}` → `Block::Figure`
  - 数学段（`$...$` / `\[...\]` / `equation` 环境）→ `Block::Equation`
- 每个 `Block` / `TextRun` 必须携带 `span: Range<usize>`，定位回原始拼接流。

### 4.2 semantic-ast

#### 4.2.1 核心枚举（V1 锁定）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub metadata: MetaData,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetaData {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub abstract_text: Option<String>,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Block {
    Heading { level: u8, text: String, span: Span },
    Paragraph { runs: Vec<TextRun>, span: Span },
    List { is_ordered: bool, items: Vec<Vec<Block>> },
    Table { rows: Vec<TableRow>, caption: Option<String>, span: Span },
    Figure { path: String, caption: Option<String>, scale: f32, span: Span },
    Equation { mathml: String, is_block: bool, span: Span },
    Bibliography { entries: Vec<BibEntry> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRun {
    pub text: String,
    pub style: TextStyle,   // Bold / Italic / Code / MathInline
    pub span: Span,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TextStyle { Plain, Bold, Italic, BoldItalic, Code, MathInline }
```

- **禁止** `Box<dyn Any>` / 类型擦除；所有变体强类型。
- `Serialize` / `Deserialize` 用 `serde` 派生，**长期资产可作为 MCP / Agent 接口暴露**。

#### 4.2.2 Span

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Span {
    pub start: u32,  // 字节偏移（拼接流）
    pub end: u32,
    pub source: SourceId,
}
```

`SourceId` 为 include 拓扑中的文件标识，便于前端跳转。

### 4.3 docx-writer

#### 4.3.1 三层流水线

| 层 | 职责 | 关键依赖 |
|---|---|---|
| `docx-model` | 严格遵循 OOXML Schema 的扁平结构体（`pPr`, `rPr`, `tblPr`, `sectPr`…） | `quick-xml` 序列化辅助宏 |
| `docx-serializer` | AST → OOXML 元素流式写入 `document.xml` / `styles.xml` / `document.xml.rels` / `numbering.xml` | `quick-xml::Writer` |
| `docx-packer` | 将 XML 树 + `word/media/*` 资源打包为 `.docx` | `zip` |

#### 4.3.2 样式集中策略

- 内置样式 ID（V1 锁定命名）：
  - `Title`, `Subtitle`, `Heading1`–`Heading6`
  - `BodyText`, `BodyTextIndent`
  - `ListBullet`, `ListNumber`
  - `TableHeader`, `TableBody`, `TableThreeLine`
  - `FigureCaption`, `EquationBlock`, `EquationInline`
  - `BibliographyEntry`, `BibHeading`
- 任何 `document.xml` 内出现的 `rPr` / `pPr` **只能引用**上述样式 ID；如需扩展，由 `styles.rs` 集中注册。

#### 4.3.3 reference.docx 模板继承

- 算法（M7-M8 实现）：
  1. 用 `zip` 读取用户上传的 `reference.docx`。
  2. 抽取其 `styles.xml` 中的样式定义，合并到 V1 默认样式表（**用户样式优先**）。
  3. 重写 `numbering.xml`、字体表 `fontTable.xml`、主题 `theme/theme1.xml` 引用关系。
  4. 输出新 `.docx`，结构与原模板同构。

#### 4.3.4 数学公式管道

```
LaTeX Math ──latex2mathml──▶ MathML (UTF-8 XML)
                                    │
                                    │ docx-writer::mathml_to_omml
                                    ▼
                          OMML (m:oMath / m:oMathPara)
                                    │
                                    ▼
                写入 <m:oMathPara>…</m:oMathPara> 至 document.xml
```

- 节点映射矩阵（V1 必覆盖）：
  - `mi`, `mn`, `mo`, `msup`, `msub`, `msubsup`, `mfrac`, `msqrt`, `mrow`, `mtable`
  - 文本节点 → `m:t`
  - 未覆盖节点 → 降级为 `m:r` + `m:t` 纯文本并打 Warning 日志。

### 4.4 bib

- 解析 `.bib` 文件（`@article` / `@inproceedings` / `@book` / `@misc` …）。
- 解析 `\cite{key1,key2}` 与 `\bibliography{file.bib}` 关联。
- 渲染样式 V1 内置两种：
  1. **Numeric**：方括号编号 `[1] [2]`，顺序按首次引用。
  2. **Author-Year**：`(Smith, 2020)` 形式。
- 样式通过 `core::api` 的 `ConvertOptions { bib_style: BibStyle }` 传入。

### 4.5 utils

- `vfs`：以 `BTreeMap<PathBuf, Vec<u8>>` 为核心；支持 `mount_dir` 注入真实目录（用于 include 解析）。
- `image`：通过 `image` crate 解码 → 探测格式（PNG / JPEG / PDF / EPS → 降级提示）→ 重采样（DPI ≤ 300）→ 重压缩（PNG deflate level 6 / JPEG quality 85）。
- `fontmap`：CTeX 字体 → Office 字体内置映射表（V1 默认表）：
  | LaTeX | Office |
  |---|---|
  | `\songti` / SimSun | 宋体 |
  | `\heiti` / SimHei | 黑体 |
  | `\fangsong` / FangSong | 仿宋 |
  | `\kaishu` / KaiTi | 楷体 |
- `path`：include 路径解算（含 Windows / Unix 路径分隔归一化）。

---

## 5. 桥接与多端集成

### 5.1 flutter_rust_bridge v2（桌面 / 移动）

- 由 `flutter_app/rust` 维护生成代码；CI 中 `flutter_rust_bridge_codegen` 自动重新生成。
- 公开 API（C 接口符号）：
  ```rust
  #[frb]
  pub fn convert_sync(input: ConvertRequest) -> Result<ConvertResult, DocError>;

  #[frb(streaming)]
  pub fn convert_stream(req: ConvertRequest) -> impl Stream<Item = ProgressEvent>;
  ```
- `ConvertRequest { source: String, options: ConvertOptions, attachments: Vec<Bytes> }`
- `ProgressEvent { phase: Phase, ratio: f32, message: String }`

### 5.2 wasm-bindgen（Web / 扩展）

- `crates/wasm` 暴露：
  ```ts
  export function convert(req: ConvertRequest): Promise<ConvertResult>;
  export function convertStream(req: ConvertRequest): AsyncIterable<ProgressEvent>;
  ```
- 内存预算：≤ 256 MB 堆（V1 硬上限），超过则拒绝并提示走云端或客户端 App。

### 5.3 Axum server（云端 / 大文件降级）

- 路由：
  - `POST /api/v1/convert`：multipart 上传 `.tex` + 资源 zip。
  - `GET /api/v1/health`：健康检查。
  - `GET /api/v1/version`：版本号。
- 限制：
  - 单文件 ≤ 50 MB
  - 并发任务上限 `num_cpus * 2`
  - 超限 → 429 + 提示跳转本地 App。

---

## 6. 前端交互细化

### 6.1 Flutter 桌面 / 移动（Material 3）

- **工作台（中央看板）**
  - 接受单文件 `.tex` 或包含 `.bib` / 图片的工程 `.zip`。
  - 拖入后展示骨架屏 + 文件名 / 大小 / 解析状态。
- **高级选项侧边栏**
  - 模板下拉：IEEE / Springer / 自定义上传 reference.docx。
  - 字体映射配置：CTeX → Office 字体的可视化映射矩阵。
  - 引用样式：Numeric / Author-Year。
- **进度总线**
  - 监听 `convert_stream` 推送，渲染 `[1/4] 解析…` 等阶段标签 + 进度条。
- **日志抽屉**
  - 按级别（Info / Warn / Error）过滤，可一键复制。

### 6.2 Chrome 扩展（Manifest V3）

- **Popup**：360 px 宽悬浮窗，文件上传 + 最近 10 条历史。
- **Content Script**：在 Overleaf / arXiv 页面注入右键菜单 `使用 Doc-engine 转换`。
- **Service Worker**：监听菜单点击 → 调用 WASM → 复制 OOXML 富文本到剪贴板。
- **分流**：
  - < 5 MB → 本地 WASM。
  - ≥ 5 MB → 弹气泡提示下载桌面 App 或跳转 PWA。

### 6.3 Flutter Web（PWA）

- `manifest.json` + `service_worker.js` 双文件完整 PWA 配置。
- WASM 离线缓存：`CacheStorage` 预存 `doc_engine_wasm_bg.wasm`。
- IndexedDB 维护 50 条历史记录：文件元数据 + DOCX Blob。

### 6.4 CLI

```bash
doc-engine convert <INPUT> [OUTPUT] \
  --template <TEMPLATE.docx> \
  --bib-style numeric|author-year \
  --verbose
```

退出码：

| 码 | 含义 |
|---|---|
| 0 | 成功 |
| 1 | 解析 / 断言失败 |
| 2 | IO / 路径 / 模板缺失 |
| 3 | 参数错误 |

---

## 7. 错误模型与日志规范

### 7.1 错误分层

| 层级 | 类型 | 表现 |
|---|---|---|
| `Warning` | 未识别宏降级、字体未映射 | 日志抽屉一条警告；UI 继续 |
| `Recoverable` | 模板样式合并冲突、公式节点降级 | 红色 Toast；产物仍生成 |
| `Fatal` | IO 失败、include 循环、栈溢出 | 阻塞对话框；不写产物 |

### 7.2 错误码

- 错误码文档：`docs/error_codes.md`（后续单独维护）。
- 命名规范：`EXX-YYYY`（如 `E01-0001` 表示 include 循环）。

### 7.3 日志格式

```
[timestamp] [level] [module] [span?] message
2026-06-14T08:30:11.123Z WARN  latex-reader span=[1240..1280] \
  "Macro \\tikz is not supported in V1. Fallback to plain text."
```

---

## 8. 测试与质量保障

### 8.1 单元测试

- `cargo test --workspace`：每个 crate 内部 `#[cfg(test)]` 覆盖核心路径。
- 覆盖率门槛：核心 crate ≥ 80%。

### 8.2 集成 / 端到端

- 夹具目录：
  - `tests/ieee_fixtures/`（≥ 10 篇）
  - `tests/springer_fixtures/`（≥ 5 篇）
  - `tests/arxiv_fixtures/`（≥ 20 篇）
  - `tests/ctex_fixtures/`（≥ 20 篇，含 CTeX 字体映射）
- 每篇夹具配套 `.docx` 期望产物，关键 XML 走 `insta` 快照断言。

### 8.3 样式回归

- `insta` 快照：`document.xml` / `styles.xml` 关键节点。
- 任何非预期 diff 阻断 CI。

### 8.4 性能基准

- `criterion` 基准：解析、公式转换、ZIP 打包三项。
- 端到端 benchmark：固定样例集，记录 P50 / P95。

### 8.5 CI/CD

- GitHub Actions（或 Gitea Actions）：
  - PR：`cargo fmt` → `cargo clippy -- -D warnings` → `cargo test` → `insta review`。
  - main：上述 + 桌面打包（Windows / macOS / Linux）→ PWA 部署 → 扩展打包。

---

## 9. 性能预算

| 路径 | 指标 | 目标 |
|---|---|---|
| Reader | 100 页 LaTeX 解析 P95 | ≤ 3 s |
| AST Lowering | 100 页 P95 | ≤ 1 s |
| MathML→OMML | 100 个公式 P95 | ≤ 1 s |
| OOXML 序列化 | 100 页 P95 | ≤ 1.5 s |
| ZIP 打包 | 100 页 P95 | ≤ 1.5 s |
| **端到端** | 100 页 P50 | **≤ 8 s** |
| WASM 端到端 | ≤ 5 MB P50 | ≤ 12 s |

---

## 10. 安全与隐私

- 桌面 / 移动 / 离线 PWA：文件**不离开本机**。
- 云端服务：上传文件 1 小时后自动清除；HTTPS 强制；无任何日志留存原始内容（仅记哈希 + 大小）。
- 扩展：剪贴板写入需用户明确点击；不主动读取页面内容（仅响应用户主动选择）。

---

## 11. 风险登记（新增）

| ID | 风险 | 等级 | 缓解措施 |
|---|---|---|---|
| R-01 | Rowan 错误恢复在极端嵌套宏下误吞正文 | 高 | M2 引入模糊测试（`proptest`）覆盖嵌套组合 |
| R-02 | OMML 节点映射表覆盖不全导致公式失真 | 中 | M6 收集 ≥ 200 公式样例回归 |
| R-03 | reference.docx 样式合并冲突导致 Word 拒绝打开 | 中 | M8 增加 OOXML Schema 校验 + Word 自动化冒烟（Office.js / LibreOffice） |
| R-04 | WASM 内存超限导致浏览器崩溃 | 高 | 入口处硬性预检 + 5 MB 分流 |
| R-05 | 中文字体映射在 macOS / Linux 上表现不一致 | 中 | M6 跨平台字体探测；缺失时降级到内置兜底字体 |
| R-06 | Chrome MV3 Service Worker 闲置后被回收，扩展失效 | 中 | `chrome.alarms` 周期唤醒 + 关键状态写 `chrome.storage.session` |
| R-07 | 14 周时间表过紧 | 中 | 预留 M13 缓冲周；任何里程碑落后 > 1 周触发评审 |

---

## 12. 与 V1 的差异（变更日志）

| 项 | V1 描述 | V2.0 细化 |
|---|---|---|
| 错误模型 | 仅有"绝不中断"理念 | 引入 Warning / Recoverable / Fatal 三层 |
| 公式覆盖 | 列举代表节点 | 明确最小节点集与降级策略 |
| 模板继承 | "无损融合" | 给出四步算法 |
| 测试 | insta 快照 | 增加夹具规模、覆盖率门槛、criterion 基准 |
| 安全隐私 | 未提及 | 桌面 / PWA 本地化；云端 1h 清除 |
| 风险 | 未提及 | 7 项风险登记与缓解措施 |
| 命名 / 版本 | 无 | 引入 `error_codes.md` 文档契约 |

---

## 13. 后续 V1.x 演进方向（明确不做，仅记录）

- 真实 PDF / PPT Writer。
- AI 格式自愈（LLM Agent）。
- 双向同步编辑。
- 自定义宏展开（用户脚本沙箱）。
- 多语言 RTL（阿拉伯语 / 希伯来语）。
- 协同编辑 / CRDT。

---

> 本文档为 V2.0 细化版，配套任务清单见同目录下 `tasks_v2.0_20260614.md`。
