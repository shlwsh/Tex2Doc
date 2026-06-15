# 第四章 · 技术架构

> 本章从三个层次描述 Tex2Doc 的技术架构：**端到端数据流**、**分层与依赖关系**、**三种前端集成模式**。读完应能在脑中画出一张完整的系统图。

---

## 1. 整体概览

```
┌──────────────────────────────────────────────────────────────────────────┐
│                          Tex2Doc 整体架构                                │
│                                                                          │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐        │
│  │   Flutter App    │  │ Chrome MV3 Ext   │  │   doc-server     │  三种前端
│  │  (Web/Desktop)   │  │     (Popup)      │  │    (HTTP)        │        │
│  └────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘        │
│           │                     │                     │                  │
│           │ WASM                │ WASM                │ multipart        │
│           │ FFI                 │                     │                  │
│  ┌────────▼─────────┐  ┌────────▼─────────┐  ┌────────▼─────────┐        │
│  │   doc-wasm       │  │   doc-wasm       │  │   doc-server     │ 绑定层
│  │   doc-native     │  │                  │  │                  │        │
│  └────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘        │
│           │                     │                     │                  │
│           └─────────────────────┼─────────────────────┘                  │
│                                 │                                        │
│                    ┌────────────▼────────────┐                           │
│                    │       doc-core          │  统一门面                  │
│                    │   (FFI/WASM 唯一入口)   │                           │
│                    └────────────┬────────────┘                           │
│                                 │                                        │
│                    ┌────────────▼────────────┐                           │
│                    │  转换管道（5 段流水线）  │  核心实现                  │
│                    │  1. Include 拓扑        │                           │
│                    │  2. Logos 词法          │                           │
│                    │  3. Rowan 语法树        │                           │
│                    │  4. 语义降级            │                           │
│                    │  5. OOXML 序列化        │                           │
│                    └────────────┬────────────┘                           │
│                                 │                                        │
│        ┌────────────┬───────────┼────────────┬─────────────┐            │
│        │            │           │            │             │  基础库     │
│  ┌─────▼────┐ ┌─────▼────┐ ┌────▼────┐ ┌────▼────┐  ┌─────▼─────┐       │
│  │doc-utils │ │doc-      │ │doc-mathml│ │doc-bib  │  │   doc-    │       │
│  │(VFS/字体)│ │latex-    │ │(OMML)   │ │(BibLaTeX)│ │  semantic │       │
│  │          │ │reader    │ │         │ │         │  │   -ast    │       │
│  └──────────┘ └──────────┘ └─────────┘ └─────────┘  └───────────┘       │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## 2. 端到端数据流（五段流水线）

### 2.1 全景

```
┌─────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌────────┐    ┌────────┐
│ LaTeX   │    │ 虚拟     │    │ Logos    │    │  Rowan   │    │ semantic│    │ OOXML  │
│ 源码    │───▶│ 文件系统 │───▶│ 词法     │───▶│ 语法树   │───▶│  AST    │───▶│ 包     │
│ (zip)   │    │  (VFS)   │    │ (tokens) │    │  (CST)   │    │         │    │ (.docx)│
└─────────┘    └──────────┘    └──────────┘    └──────────┘    └────────┘    └────────┘
   Pass-1: include 拓扑 & 拼接     Pass-2: 朴素 Rowan 构建       Pass-3: 降级
```

### 2.2 详细阶段

#### 阶段 1：Include 拓扑 & 拼接（`doc-latex-reader::include`）

* **入口**：`doc_latex_reader::IncludeGraph::build(vfs, main_path)`。
* **算法**：
  1. 词法扫描主 `.tex`，找 `\include` / `\input` / `\subfile` / `\graphicspath`。
  2. 通过 `doc_utils::PathResolver` 在 VFS 中按优先级（`base_dir` + `graphicspath` + `/`）查找目标。
  3. 递归构建 DAG；遇环报错 `IncludeError::Cycle`。
  4. 收集 `SourceId`（按文件 index 分配）。
* **拓扑排序**：Kahn 算法；出度为 0 入队；遇不可达节点报 `Cycle`。
* **拼接**：`IncludeGraph::join(vfs)` → `JoinedStream { text, source_map }`，按拓扑序把每文件内容拼接为单流文本，每个字符带 `SourceId`。

#### 阶段 2：Logos 词法（`doc-latex-reader::lexer`）

* **库**：`logos = 0.14`（零拷贝正则 DFA）。
* **词法元素**：
  * `Command`：`\\[A-Za-z@]+`
  * `LBrace` / `RBrace` / `LBracket` / `RBracket`
  * `Dollar` / `DollarDollar`（数学定界符）
  * `Comment`：`%[^\n]*`
  * `Whitespace`：`[ \t]+`
  * `Newline`：`\r?\n`
  * `LineBreak`：`\\\\`
  * `Par`：`\par`（关键字 token）
  * `Error`：兜底
* **到 SyntaxKind 映射**：`TokKind::into_syntax()`。

#### 阶段 3：Rowan 语法树（`doc-latex-reader::parser`）

* **库**：`rowan = 0.15`（增量式红绿树）。
* **算法**：朴素无文法硬编码。
  1. 启动 `GreenNodeBuilder`。
  2. 遍历 token 流：
     * `LBrace` → 启动 `Group` 节点
     * `RBrace` → 结束 `Group`（若已开）
     * `Command` 且为 `\begin` → 启动 `Env` 节点
     * `Command` 且为 `\end` → 结束 `Env`
     * 其它 → 平铺为叶子
  3. **错误恢复**：未闭合的 Group/Env 自动补 `finish_node`；不 panic。
* **输出**：`Parse { green: GreenNode, root: SyntaxNode, source: String }`。

#### 阶段 4：语义降级（`doc-latex-reader::lower`）

* **核心算法**：M3 完整版 ~1700 行。
* **流水线**：
  1. **宏展开**：`expand_macros_in(text, macros)`，先把 `outer` 收集的 `\newcommand` 用于 `inner` 段（如 rjabstract）。
  2. **preamble 剥离**：`strip_preamble` 找 `\begin{document}` 位置，截其后内容。
  3. **逐字符扫描**（字节级，遇到 CJK 多字节字符走 `chars().next()` + `len_utf8()`）：
     * 跳过 ASCII 空白 / 注释。
     * **环境优先**：`scan_environment` 找 `\begin{name}...\end{name}` 整段扣出，做专项降级。
     * **顶层段命令**：`try_top_level_command` 识别 `\section` / `\subsection` / `\subsubsection` / `\paragraph` / `\caption`。
     * **顶层 metadata 命令**：`try_top_level_metadata_command` 一次性吞掉 50+ 装饰命令（`\rjtitle` / `\hypersetup` / `\usepackage` / `\documentclass` 等）。
     * **段落 buffer**：非空行累计；空行 / 段命令 / 新环境 / EOF 触发 `flush_paragraph`。
     * **段落内联清洗**：`strip_inline` 处理 `\textbf` / `\textit` / `\texttt` / `\emph` / `\cite` / `\ref` / `\href` / `\url` / `\rowcolor` / `\multicolumn` / 嵌套 tabular。
     * **inline math 抽出**：$…$ 整段抽出为 `Block::Equation { is_block: false }`。
  4. **自动编号**：`NumberingState` 跟踪 heading 1.1.1 / figure / table 编号。
  5. **citation 编号**：`\cite{key}` → 全局 `[n]`（首次出现分配序号）。
  6. **错误降级**：未匹配内容进入 `Block::RawFallback`；绝不 panic。
* **关键环境支持**：
  * `itemize` / `enumerate` / `description` → `Block::List`
  * `tabular` / `tabular*` / `array` → `Block::Table`（含 `\rowcolor` / `\multicolumn` / 嵌套 tabular）
  * `figure` / `figure*` / `table` / `table*` → `Block::Figure` 或 `Block::Table`（带 caption / number）
  * `equation` / `equation*` / `align` / `align*` / `gather` / `gather*` → `Block::Equation { is_block: true }`
  * `document` → 递归降级 body，折叠首个非空块
  * `flushleft` / `flushright` / `center` / `quote` / `quotation` / `verbatim` / rj 类容器 → 递归降级为 `Block::Paragraph`
  * 未知环境 → `Block::RawFallback`

#### 阶段 5：OOXML 序列化（`doc-docx-writer`）

* **入口**：`doc_docx_writer::pack(doc) / pack_with_template / pack_with_assets`。
* **步骤**：
  1. `serialize_document` 用 `quick_xml::Writer` 写出 `word/document.xml`：
     * `Block::Heading` → `<w:p>` + `w:pStyle w:val="HeadingN"` + 编号 + 文本
     * `Block::Paragraph` → `<w:p>` + `w:pStyle w:val="BodyText"` + 多 run
     * `Block::List` → 多段（每项一个 `<w:p>` + 编号/项目符号）
     * `Block::Table` → `<w:tbl>` + 边框 + `<w:tr>` + `<w:tc>` + colspan / bg_color
     * `Block::Figure` → `<w:drawing>` + `<wp:inline>` + `<pic:pic>` + base64 `<w:binData>`（或占位 `[图片：path]`）
     * `Block::Equation` → `<w:p>` + 调 `doc_mathml::parse_latex_math` + `to_omml` 嵌入 `<m:oMath>`
     * `Block::Bibliography` → "参考文献" 标题 + `[key] title (year)` 列表
     * `Block::RawFallback` → 原文段落
  2. `write_styles` 生成默认 `word/styles.xml`（9 个样式）。
  3. （可选）`merge_styles` 从 `reference.docx` 合并缺失样式。
  4. `zip::ZipWriter` 打包：`[Content_Types].xml` / `_rels/.rels` / `word/_rels/document.xml.rels` / `word/document.xml` / `word/styles.xml`。
  5. `Deflated` 压缩。

### 2.3 数据流时序图

```
Zip 字节流 (50 MiB max)
   │
   ▼
[doc-core::convert_zip]
   │
   ├─► 解压到 VirtualFs
   │     │
   │     ├─► IncludeGraph::build (Pass-1: 拓扑)
   │     │     │
   │     │     ├─► 扫描 \input/\include/\graphicspath
   │     │     ├─► PathResolver 查找
   │     │     └─► Kahn 拓扑
   │     │
   │     ├─► IncludeGraph::join (拼接单流)
   │     │
   │     ├─► Logos 词法
   │     │
   │     ├─► Rowan 解析
   │     │
   │     ├─► 宏展开
   │     ├─► preamble 剥离
   │     ├─► 环境优先
   │     ├─► 段命令处理
   │     ├─► 段落内联清洗
   │     ├─► inline math 抽出
   │     └─► 错误降级
   │
   ├─► Document (semantic AST)
   │
   ├─► doc-docx-writer::pack
   │     │
   │     ├─► serialize_document
   │     ├─► write_styles
   │     ├─► (可选) merge_styles
   │     └─► zip 打包
   │
   └─► docx 字节流
```

---

## 3. 分层与依赖关系

### 3.1 分层

```
┌─────────────────────────────────────────────────────────────┐
│ L1: 入口层（doc-wasm / doc-native / doc-server）            │
│     - 暴露 FFI / WASM / HTTP 边界                            │
└─────────────────────────────────┬───────────────────────────┘
                                  │
┌─────────────────────────────────▼───────────────────────────┐
│ L2: 门面层（doc-core）                                       │
│     - 统一对外 4 个转换入口                                  │
│     - 错误模型 / 选项 / 结果类型                              │
└─────────────────────────────────┬───────────────────────────┘
                                  │
┌─────────────────────────────────▼───────────────────────────┐
│ L3: 业务逻辑层（doc-latex-reader + doc-docx-writer）         │
│     - 解析 → 降级 → 序列化                                   │
└────────────┬────────────────────────┬───────────────────────┘
             │                        │
┌────────────▼────────────┐  ┌─────────▼────────────┐  ┌──────▼─────┐
│ L4: 基础库层             │  │ L4: 基础库层         │  │ L4: 基础库  │
│   doc-semantic-ast      │  │   doc-mathml         │  │  doc-bib   │
│   doc-utils (VFS)       │  │   (公式管道)         │  │ (BibLaTeX) │
└─────────────────────────┘  └──────────────────────┘  └────────────┘
```

### 3.2 依赖图（实际 `Cargo.toml`）

* `doc-core` 依赖：`doc-utils` / `doc-semantic-ast` / `doc-latex-reader` / `doc-docx-writer` / `doc-bib`
* `doc-latex-reader` 依赖：`doc-utils` / `doc-semantic-ast` + `logos` + `rowan`
* `doc-docx-writer` 依赖：`doc-utils` / `doc-semantic-ast` / `doc-mathml` + `quick-xml` + `zip` + `image` + `base64`
* `doc-mathml` 依赖：`doc-semantic-ast`（间接） + `quick-xml`
* `doc-bib` 依赖：`doc-semantic-ast`（直接）
* `doc-utils` 依赖：`thiserror` + `serde` + `image`（**叶子节点，无内部依赖**）
* `doc-wasm` 依赖：`doc-core` + `wasm-bindgen` + `serde-wasm-bindgen` + `js-sys` + `zip`
* `doc-native` 依赖：`doc-core` + `serde` + `serde_json` + `thiserror`
* `doc-server` 依赖：`doc-core` + `axum` + `tower` + `tower-http` + `tokio` + `reqwest`（dev）

### 3.3 隔离原则

* **业务 crate 不允许依赖入口 crate**（`doc-latex-reader` 不依赖 `doc-wasm`）。
* **基础 crate 永远是叶子节点**（`doc-utils` / `doc-semantic-ast` / `doc-bib` / `doc-mathml` 不依赖任何内部 crate）。
* **`doc-core` 是唯一聚合点**：任何新增入口都通过它。

---

## 4. 三种前端集成模式

### 4.1 模式 A：WASM（Web / Chrome 扩展）

```
┌──────────────────┐
│ Flutter Web /    │  Dart ↔ JS interop
│  Chrome popup    │  Uint8List ↔ JSUint8Array
└────────┬─────────┘
         │ call window.docEngine.convert_zip_to_docx(zipBytes, mainTex, "")
         │
┌────────▼─────────┐
│  doc_engine.js   │  wasm-bindgen 生成的 ESM
│  (ESM wrapper)   │
└────────┬─────────┘
         │ init({ wasmBinary }) → WebAssembly.instantiate
         │
┌────────▼─────────┐
│ doc_engine_bg.   │  WASM 字节流（~3.5 MB）
│      wasm        │
└────────┬─────────┘
         │ #[wasm_bindgen] fn convert_zip(...)
         │
┌────────▼─────────┐
│   doc-wasm       │  serde_wasm_bindgen ↔ JsValue
│  (Rust crate)    │  doc-core::convert_zip
└────────┬─────────┘
         │ Docx 字节流
         │
┌────────▼─────────┐
│   doc-core       │
└──────────────────┘
```

**关键特性**：
* **零拷贝字节传递**：`Uint8List.toJS` / `JSUint8Array.toDart` 不复制。
* **异步加载**：`await import('./wasm/doc_engine.js')` + `fetch` + `arrayBuffer`。
* **错误传播**：`WasmError` → `JsValue::from_str(message)`。

**使用位置**：
* `flutter_app/lib/wasm_bridge.dart`：Flutter Web PWA。
* `extension/popup/popup.js`：Chrome MV3 popup。

### 4.2 模式 B：Native FFI（桌面端）

```
┌──────────────────┐
│ Flutter Desktop  │  Dart
│   (Windows/      │  dart:ffi
│    macOS/Linux)  │  pkg_ffi.calloc
└────────┬─────────┘
         │ call DynamicLibrary.open("doc_engine")
         │      .lookupFunction<...>("doc_engine_convert_zip")
         │
┌────────▼─────────┐
│  doc_engine.dll  │  Rust cdylib
│  doc_engine.dylib│
│  doc_engine.so   │
└────────┬─────────┘
         │ extern "C" fn doc_engine_convert_zip(...)
         │
┌────────▼─────────┐
│   doc-native     │  FFI 边界：
│  (Rust crate)    │  - malloc / memcpy
│                  │  - thread-local LAST_ERROR
│                  │  - doc-core::convert_zip
└────────┬─────────┘
         │
┌────────▼─────────┐
│   doc-core       │
└──────────────────┘
```

**关键特性**：
* **直接 cdylib**：`cdylib` crate-type；Dart 端 `DynamicLibrary.open` 加载。
* **C 堆内存**：`libc::malloc` 分配 docx + warnings；Dart 端 `asTypedList` 读出；用 `doc_engine_free` 释放。
* **环境变量覆盖**：`DOC_ENGINE_LIB` 允许指定 `.dll` / `.dylib` / `.so` 路径。
* **自动 cargo 调用**：`flutter_app/windows/CMakeLists.txt` 在 CMake build 时自动 `cargo build -p doc-native` 并拷贝 DLL。

**使用位置**：
* `flutter_app/lib/native_bridge.dart`：Flutter 桌面 UI。
* `flutter_app/bin/native_smoke.dart`：冒烟脚本。

### 4.3 模式 C：HTTP（服务端 / 跨域集成）

```
┌──────────────────┐
│ curl / Frontend  │  HTTP multipart
│   / Backend      │  (file + main_tex)
└────────┬─────────┘
         │ POST /api/v1/convert
         │
┌────────▼─────────┐
│  doc-server      │  Axum + tokio
│  (Rust binary)   │  tower-http limit
└────────┬─────────┘
         │ doc_core::convert_zip
         │
┌────────▼─────────┐
│   doc-core       │
└──────────────────┘
```

**关键特性**：
* **多部分上传**：`memchr::memmem::find` 自定义解析（不依赖 `axum::extract::Multipart`）。
* **请求体限制**：`tower_http::RequestBodyLimitLayer::new(50 * 1024 * 1024)`。
* **docx 验证**：`PK\x03\x04` 头 + ≥ 4 KiB。
* **MIME**：`application/vnd.openxmlformats-officedocument.wordprocessingml.document`。
* **错误映射**：`ServerError → StatusCode`（400/422/500）。

**使用位置**：
* `crates/server/`：HTTP 服务。
* `scripts/e2e_server.mjs`：端到端验证。

---

## 5. 跨平台一致性保证

### 5.1 唯一核心原则

> **所有功能差异必须在 `doc-core`（或更底层）实现；三种入口层只做格式转换。**

* `doc-wasm` 不持有解析逻辑。
* `doc-native` 不持有序列化逻辑。
* `doc-server` 不持有 VFS 逻辑。

### 5.2 CI 三平台矩阵

`.github/workflows/ci.yml` 强制 `cargo test --workspace --all-targets` 在 Ubuntu / Windows / macOS 三平台通过。

### 5.3 关键漂移点

* **行尾**（CRLF vs LF）：`lower.rs` 的 `bytes[p] != b'\n'` 检查仅识别 LF；CRLF 文本中的 `\r` 走 `chars().next()` 透传。
* **路径分隔符**：`VFS` 统一用 POSIX `/`（`vfs.rs::normalize_path` + `path.rs::normalize`）。
* **大小写敏感**：VFS 路径查找**不**做大小写折叠（保留源文件大小写）。

---

## 6. 性能特征

### 6.1 时间复杂度

| 阶段 | 复杂度 | 备注 |
|------|--------|------|
| Include 拓扑 | O(N + E) | N = 文件数，E = include 边数 |
| Logos 词法 | O(L) | L = 源长度，零拷贝 |
| Rowan 构建 | O(T) | T = token 数 |
| Macro 展开 | O(L × M) | M = 宏表大小（实际很小） |
| 降级 | O(L) | 一次扫描 |
| 序列化 | O(B) | B = Block 数 |
| ZIP 打包 | O(D) | D = docx 字节数 |

### 6.2 实测

* `examples/paper3/latex/main-jos.tex` (8 KB) + 6 include + BibTeX：Rust 转换 ~800 ms（WASM 略慢，约 1.2x）。
* WASM 产物：`doc_engine_bg.wasm` ~3.5 MB（dev build）。
* 桌面端 cdylib：`doc_native.dll` ~ 几 MB。

### 6.3 内存峰值

* 全部 docx 在内存中组装（不流式）。
* ZIP 压缩 `Deflated` 级别（平衡）。
* WASM 限制 4 GB（浏览器）。

---

## 7. 安全边界

### 7.1 FFI 边界（doc-native）

* 入参非空检查（`zip_ptr.is_null() || main_tex_ptr.is_null() || ...`）。
* 长度非零检查。
* UTF-8 校验（主文件路径）。
* `malloc` 失败兜底。
* `out_*_ptr` 写回前清零。
* `unsafe` 块显式注释 Safety 契约。

### 7.2 WASM 边界（doc-wasm）

* zip 字节直接信任（视为可信源）。
* 主文件路径不验证（仅用作 include 索引）。
* 错误以字符串形式抛出（无堆信息泄漏）。

### 7.3 HTTP 边界（doc-server）

* `tower_http::limit::RequestBodyLimitLayer` 限制 50 MiB。
* docx 头/大小验证（PK 头 + ≥ 4 KiB）。
* `Content-Disposition` 含 `sanitize()` 处理文件名。
* `Content-Type` 显式设为 docx MIME。

### 7.4 解析器兜底

* Logos `Error` token 不 panic。
* Rowan 未闭合 group/env 自动补。
* 降级未匹配进入 `RawFallback`。
* 公式嵌套深度 `MAX_EXPR_DEPTH = 100` 截断。
* VFS 路径 `..` 拒绝（zip 内路径安全检查）。

---

## 8. 扩展性

### 8.1 新增 LaTeX 命令
* 在 `doc-latex-reader::lower::strip_inline` / `try_top_level_command` / `try_top_level_metadata_command` 加分支。
* 在 `crates/latex-reader/tests/` 加单元测试。
* 端到端：用 `examples/paper3` 验证不挂。

### 8.2 新增 Writer 格式（Markdown / HTML / Typst）
* `doc-semantic-ast` 已与格式无关。
* 新建 `crates/md-writer` / `crates/html-writer` 即可。
* `doc-core` 加 `convert_to_markdown` 入口。

### 8.3 新增前端（CLI / Tauri / Electron）
* CLI：`crates/cli/` 占位已建，添加 `clap` 入口即可。
* Tauri：复用 `doc-wasm`。
* Electron：复用 `doc-wasm` + Node.js native addon 可选。

---

## 9. 进一步阅读

* [05-key-tech/](../05-key-tech/) — 每个模块的深入技术解析
* [06-user-guide/](../06-user-guide/) — 各类使用方式
* [07-deployment/](../07-deployment/) — 各类构建/部署
