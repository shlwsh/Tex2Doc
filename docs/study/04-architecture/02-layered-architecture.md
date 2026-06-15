# 分层与依赖关系详解

> 本节从「依赖图」「代码层职责」「扩展点」三方面详细描述 Tex2Doc 的内部组织。

---

## 1. 物理依赖（`Cargo.toml` 实际声明）

| crate | 直接依赖 |
|-------|----------|
| `doc-utils` | thiserror, serde, image |
| `doc-semantic-ast` | serde, serde_json |
| `doc-bib` | doc-semantic-ast, serde |
| `doc-mathml` | doc-semantic-ast, quick-xml, thiserror |
| `doc-latex-reader` | doc-utils, doc-semantic-ast, logos, rowan, thiserror, serde |
| `doc-docx-writer` | doc-utils, doc-semantic-ast, doc-mathml, quick-xml, zip, thiserror, serde, image, base64 |
| `doc-core` | doc-utils, doc-semantic-ast, doc-latex-reader, doc-docx-writer, doc-bib, thiserror, serde, serde_json, zip |
| `doc-wasm` | doc-core, wasm-bindgen, serde, serde-wasm-bindgen, serde_json, thiserror, js-sys, zip, console_error_panic_hook（可选） |
| `doc-native` | doc-core, serde, serde_json, thiserror |
| `doc-server` | doc-core, axum, tower, tower-http, tokio, tokio-util, memchr, http, bytes, serde, serde_json, thiserror, tracing, tracing-subscriber, mime, async-trait |

> 间接依赖未列出（如 axum 依赖 tokio 等）。

---

## 2. 逻辑分层

### 2.1 L0 — 基础类型层（`doc-utils` / `doc-semantic-ast`）

**职责**：
* 通用错误类型 `DocError` / `CoreError`。
* 虚拟文件系统 `VirtualFs`。
* 路径解析 `PathResolver` / `parse_graphics_path`。
* 图片探测 `SupportedFormat` / `ImageMeta` / `ImageAssets`。
* 字体探测 `FontMap` / `FontDetector` / `FontProbe`。
* 语义块模型 `Document` / `Block` / `TextRun` / `BibEntry`。
* 源位置 `Span` / `SourceId`。
* Visitor 模式 `Visitor` / `BlockCounter`。

**约束**：
* **叶子节点**，不依赖任何内部 crate。
* `#![forbid(unsafe_code)]`（除 `doc-native` 单独声明 `allow`）。
* 所有公开类型 `Serialize` / `Deserialize`，方便 FFI 边界与快照测试。

### 2.2 L1 — 业务逻辑层

#### `doc-latex-reader`
* **职责**：LaTeX 解析、宏展开、include 拓扑、语义降级。
* **公开 API**：
  * `include::IncludeGraph::build` / `topo_order` / `join`
  * `lexer::TokKind`（Logos 词法枚举）
  * `parser::parse` / `Parse`
  * `lower::lower_to_document` / `lower_with_macros` / `lower_with_macros_and_numbering`
  * `expand::expand_macros` / `expand_macros_in` / `MacroMap`
  * `green::SyntaxKind` / `Lang`（Rowan）
* **依赖**：L0（utils, semantic-ast）+ 第三方（logos, rowan）

#### `doc-mathml`
* **职责**：LaTeX 数学子集 → `MathExpr` → MathML / OMML。
* **公开 API**：
  * `parse_latex_math` / `to_mathml` / `to_omml` / `MathExpr`
* **依赖**：L0（semantic-ast）+ 第三方（quick-xml）

#### `doc-bib`
* **职责**：BibLaTeX 解析。
* **公开 API**：
  * `parse` / `parse_raw` / `BibRawEntry`
* **依赖**：L0（semantic-ast）

#### `doc-docx-writer`
* **职责**：OOXML 序列化与 ZIP 打包。
* **公开 API**：
  * `pack` / `pack_with_template` / `pack_with_assets`
  * `serialize_document`
  * `write_styles` / `apply_font_probes`
  * `parse_template` / `merge_styles` / `TemplateStyles`
  * `STYLE_*` 常量
* **依赖**：L0（utils, semantic-ast）+ L1（mathml）+ 第三方（quick-xml, zip, image, base64）

### 2.3 L2 — 门面层（`doc-core`）

* **职责**：FFI / WASM / HTTP 边界统一的对外接口。
* **公开 API**：
  * `convert_sync` / `convert_dir` / `convert_zip` / `convert_stream`
  * `ConvertOptions` / `BibStyle` / `Attachment`
  * `ConvertResult` / `ProgressEvent` / `ProgressPhase`
  * `CoreError`（Io/Parse/Serialize/Unsupported）
* **依赖**：L1 全集 + L0（utils, semantic-ast）
* **关键约束**：
  * **唯一**被 `doc-wasm` / `doc-native` / `doc-server` 直接依赖的业务 crate。
  * 错误类型 `serde::Serialize` + `serde::Deserialize`（FFI 透传）。
  * `forbid(unsafe_code)`。

### 2.4 L3 — 入口层

#### `doc-wasm`
* **职责**：WASM 边界。
* **公开 API**（`#[wasm_bindgen]`）：
  * `convert_zip(zip_bytes, main_tex_path, options_js)` → `{ docx, docx_len, warnings }`
  * `convert_zip_to_docx(zip_bytes, main_tex_path, options_js)` → `Uint8Array`
  * `version()` → `String`
* **依赖**：L2（doc-core）+ wasm-bindgen
* **关键特性**：
  * `serde_wasm_bindgen` 转换 `ConvertResult` ↔ `JsValue`。
  * 错误以字符串抛出（`WasmError::message` getter）。
  * `console_error_panic_hook` 可选 feature。

#### `doc-native`
* **职责**：原生 FFI 边界（dart:ffi 桥接）。
* **公开 API**（`#[no_mangle] pub unsafe extern "C"`）：
  * `doc_engine_version() -> *const c_char`
  * `doc_engine_last_error() -> *const c_char`
  * `doc_engine_free(*mut u8)`
  * `doc_engine_convert_zip(...) -> c_int`（0 成功 / 1 失败）
* **依赖**：L2（doc-core）+ serde
* **关键约束**：
  * 允许 `unsafe_op_in_unsafe_fn`（FFI 必须）。
  * C 堆分配用 `libc::malloc` / `memcpy` / `libc::free`。
  * 错误写入 thread-local `LAST_ERROR`。
  * 返回 docx + warnings 两块独立内存，Dart 端必须 `doc_engine_free` 各一次。

#### `doc-server`
* **职责**：HTTP 服务端（Axum）。
* **公开 API**（HTTP 路由）：
  * `GET /api/v1/health`
  * `GET /api/v1/version`
  * `POST /api/v1/convert`（multipart: file + main_tex）
* **依赖**：L2（doc-core）+ axum + tower + tower-http + tokio
* **关键约束**：
  * 请求体 50 MiB 上限（`tower_http::RequestBodyLimitLayer` + `axum::body::to_bytes(_, MAX_BODY)`）。
  * docx 头/大小验证（`PK\x03\x04` + ≥ 4 KiB）。
  * 错误状态码映射（400/422/500）。

---

## 3. 跨层调用关系

### 3.1 调用方向（自上而下）

```
doc-wasm / doc-native / doc-server
         │
         ▼
      doc-core
         │
         ├─────────────────┐
         ▼                 ▼
   doc-latex-reader   doc-docx-writer
         │                 │
         │           ┌─────┴─────┐
         ▼           ▼           ▼
   doc-utils  doc-semantic-ast  doc-mathml
                       │
                       ▼
                 doc-bib（间接）
```

### 3.2 反向调用

* **无**。所有调用严格自上而下；下层不依赖上层。
* 唯一例外：`doc-docx-writer` 同时依赖 `doc-utils` 和 `doc-semantic-ast`（并列依赖，不反向）。

### 3.3 共享数据流

```
┌─────────────────────────────────────────────────────────────┐
│                  跨层共享的类型                                │
├─────────────────────────────────────────────────────────────┤
│  doc-semantic_ast::Document  ◄── doc-latex-reader 产出       │
│        │                                                    │
│        ├──► doc-core 透传                                   │
│        ├──► doc-docx-writer 消费                            │
│        └──► doc-bib::BibEntry（嵌入 Block::Bibliography）     │
│                                                             │
│  doc-utils::VirtualFs ◄── doc-core 装载                     │
│        │                                                    │
│        ├──► doc-latex_reader::IncludeGraph 使用            │
│        ├──► doc-core::convert_dir 写入                      │
│        └──► doc-core::convert_zip 写入                      │
│                                                             │
│  doc_utils::ImageAssets ◄── doc-core 扫描 VFS 填充         │
│        │                                                    │
│        └──► doc-docx-writer::pack_with_assets 消费          │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. 模块内部组织（以 `doc-latex-reader` 为例）

### 4.1 文件分工

| 文件 | 行数 | 职责 |
|------|------|------|
| `lib.rs` | 23 | 模块声明 + 公共 re-export |
| `lexer.rs` | 103 | Logos 词法 |
| `green.rs` | 78 | Rowan 节点类型 |
| `parser.rs` | 133 | Pass-2 语法树构建 |
| `lower.rs` | 1900+ | Pass-3 语义降级 |
| `include.rs` | 288 | Pass-1 拓扑 + 拼接 |
| `expand.rs` | 259 | 宏展开 |

### 4.2 调用链

```
parse_tex_to_doc(text)  ──  lib.rs::re-export
   └─► IncludeGraph::build(vfs, main)         ← include.rs
   └─► graph.join(vfs)                        ← include.rs
   └─► parse_tex(joined.text)                 ← parser.rs
         └─► Logos lexer (TokKind)            ← lexer.rs
         └─► Rowan GreenNodeBuilder           ← green.rs
   └─► lower_to_document(&parse, &joined)     ← lower.rs
         ├─► expand_macros_in(text, macros)   ← expand.rs
         ├─► strip_preamble(text)
         └─► 主循环：scan_environment / try_top_level_*
```

### 4.3 内部模块依赖

```
include ──► utils, semantic_ast
expand  ──► (无内部依赖)
lexer   ──► green（TokKind::into_syntax）
green   ──► (无内部依赖)
parser  ──► lexer, green
lower   ──► parser, include, expand, utils, semantic_ast
```

---

## 5. 命名约定

### 5.1 Crate 命名

* `doc-utils` / `doc-semantic-ast` / `doc-latex-reader` / `doc-docx-writer` / `doc-bib` / `doc-mathml`：业务 crate，发布名带 `doc-` 前缀。
* `doc-core`：门面 crate。
* `doc-wasm` / `doc-native` / `doc-server`：入口 crate（cdylib / cdylib / bin）。

### 5.2 Cargo.toml 声明

```toml
[workspace.dependencies]
doc-utils        = { path = "crates/utils" }
doc-semantic-ast = { path = "crates/semantic-ast" }
doc-latex-reader = { path = "crates/latex-reader" }
doc-docx-writer  = { path = "crates/docx-writer" }
doc-bib          = { path = "crates/bib" }
doc-mathml       = { path = "crates/mathml" }
```

* 业务 crate 内部依赖：使用 `path = ...` + `workspace = true`（共享版本号）。
* 入口 crate 内部依赖：仅依赖 `doc-core`（不直接依赖其他业务 crate）。

### 5.3 Rust 类型命名

* 错误类型：`XxxError`（`DocError` / `CoreError` / `ServerError` / `DocxWriteError` / `TemplateError` / `IncludeError`）。
* 结果类型：`XxxResult<T>` = `Result<T, XxxError>`。
* Builder/Builder-style：函数式 builder（如 `FontDetector::with_fallback`）。
* 序列化：所有跨 FFI / 跨平台类型 `#[derive(Serialize, Deserialize)]`。
* `RawFallback`：表示「无法解析但保留原文」的语义块。

### 5.4 模块命名

* 单一公开入口：`lib.rs` 内 `pub mod` 显式声明 + 顶层 re-export 公共 API。
* 内部模块（`pub(crate)`）不导出。
* 单元测试：写在 `#[cfg(test)] mod tests` 内联。
* 集成测试：写在 `tests/*.rs`。

---

## 6. 错误处理

### 6.1 错误传播链

```
doc_utils::DocError (底层)
   ├─► From<std::io::Error>
   ├─► From<IncludeError> via InvalidPath
   └─► From<image::ImageError> via ImageDecode

doc_core::CoreError (门面层，serde 友好)
   ├─► From<doc_utils::DocError>
   ├─► From<std::io::Error>
   └─► 4 变体：Io / Parse / Serialize / Unsupported

doc_server::ServerError (HTTP 边界)
   ├─► From<doc_core::CoreError> (#[from])
   ├─► 3 变体：Io / MissingField / Core
   └─► IntoResponse：400/422/500 + JSON body
```

### 6.2 错误降级（**绝不 panic**）

* **词法层**：Logos `Error` token 降级为 `SyntaxKind::Error`。
* **语法层**：未闭合 group/env 自动补 `finish_node`。
* **降级层**：未匹配内容进入 `Block::RawFallback`。
* **公式层**：嵌套超 `MAX_EXPR_DEPTH` 截断为 `Raw`。
* **VFS 层**：缺失路径返回 `DocError::VfsMissing`。
* **图片层**：格式不支持返回 `DocError::Unsupported`。

### 6.3 错误聚合

* `CoreError::Serialize(String)`：所有 `doc-docx-writer` 错误包装。
* `doc-core::convert_zip` 失败时调用方拿到 `Result<ConvertResult, CoreError>`。
* HTTP 错误通过 JSON body 返回 `{ "error": "code", "message": "..." }`。

---

## 7. 测试组织

### 7.1 单元测试（内联）

* 每个模块底部 `#[cfg(test)] mod tests`。
* 50+ 测试覆盖各模块关键路径。

### 7.2 集成测试（`tests/`）

| 位置 | 数量 | 覆盖 |
|------|------|------|
| `crates/core/tests/end_to_end.rs` | 1+ | 最小 docx 健全性 |
| `crates/core/tests/ieee_fixtures.rs` | 1+ | IEEE 模板 |
| `crates/core/tests/paper3_e2e.rs` | 1+ | 真实工程 8 KB LaTeX |
| `crates/latex-reader/tests/insta_snapshots.rs` | 1+ | 段落/列表/表格快照 |
| `crates/latex-reader/tests/proptest.rs` | 1+ | 属性测试 |
| `crates/utils/tests/proptest.rs` | 1+ | VFS 属性测试 |
| `crates/server/tests/api.rs` | 1+ | HTTP API（reqwest） |

### 7.3 端到端测试

* `scripts/verify_paper3.mjs`：内容断言。
* `scripts/e2e_paper3.mjs`：Playwright + Flutter Web。
* `scripts/e2e_server.mjs`：HTTP curl。
* `scripts/e2e_extension.mjs`：Playwright + Chrome 扩展。
* `flutter_app/bin/native_smoke.dart`：桌面端 FFI 冒烟。

### 7.4 快照测试（`insta`）

* `crates/latex-reader/tests/snapshots/` 下：
  * `insta_snapshots__list_doc.snap`
  * `insta_snapshots__simple_doc.snap`
  * `insta_snapshots__table_doc.snap`

---

## 8. 进一步阅读

* [01-end-to-end-pipeline.md](./01-end-to-end-pipeline.md) — 端到端流水线
* [03-frontend-bridges.md](./03-frontend-bridges.md) — 三种前端集成模式
* [05-key-tech/](../05-key-tech/) — 各模块深入解析
