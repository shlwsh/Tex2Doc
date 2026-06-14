# 第二章 · Rust 技术栈

> Tex2Doc 的核心转换逻辑完全用 Rust 实现，10 个 crate 构成一个 Cargo Workspace。本节列出所有依赖、技术约束与构建配置。

---

## 1.1 工具链要求

`rust-toolchain.toml`：

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-src"]
profile = "minimal"
```

| 工具 | 最低版本 | 实际使用 | 备注 |
|------|----------|----------|------|
| rustc | 1.82 | stable | `rust-version = "1.82"` |
| cargo | 随 stable | 1.82+ | workspace `resolver = "2"` |
| rustfmt | stable | 100 列 / 4 空格 tab | `.rustfmt.toml` 强制 |
| clippy | stable | `-D warnings` | CI 强约束 |

---

## 1.2 Cargo Workspace 结构

`Cargo.toml`（顶层）：

```toml
[workspace]
resolver = "2"
members = [
    "crates/core",         # doc-core：FFI/WASM 统一门面
    "crates/utils",        # doc-utils：VFS、路径、图片、字体
    "crates/semantic-ast", # doc-semantic-ast：核心语义块模型
    "crates/latex-reader", # doc-latex-reader：Logos + Rowan 解析
    "crates/docx-writer",  # doc-docx-writer：OOXML 序列化 + ZIP 打包
    "crates/bib",          # doc-bib：BibLaTeX 解析
    "crates/mathml",       # doc-mathml：LaTeX → MathML / OMML
    "crates/wasm",         # doc-wasm：WASM 桥接（cdylib）
    "crates/native",       # doc-native：原生 cdylib（dart:ffi 桥接）
    "crates/server",       # doc-server：Axum HTTP 服务
]
exclude = [
    "crates/cli",          # （占位目录，未来 CLI 工具）
]
```

---

## 1.3 内部 crate 互依赖矩阵

```
                ┌─────────────────────────────────────────┐
                │             doc-core (FFI 门面)          │
                └─┬───────────┬──────────┬─────────────┬───┘
                  │           │          │             │
       ┌──────────▼─┐  ┌──────▼────┐ ┌───▼────┐  ┌─────▼─────┐
       │ doc-utils  │  │ doc-      │ │ doc-   │  │ doc-bib   │
       │ (VFS/路径/ │  │ semantic- │ │ latex- │  │ (BibTeX)  │
       │  图片/字体)│  │ ast       │ │ reader │  └───────────┘
       └────┬───────┘  └────▲──────┘ └───┬────┘
            │               │            │
            │      ┌────────┴────────────┘
            │      │
            │  ┌───▼─────────┐     ┌─────────────────┐
            │  │ doc-mathml  │     │ doc-docx-writer │
            │  │ (LaTeX→     │◄────┤ (OOXML 序列化)  │
            │  │  MathML)    │     └─────────────────┘
            │  └─────────────┘              ▲
            │                              │
            │         ┌────────────────────┘
            │         │
   ┌────────▼─────────▼────────┐
   │      doc-wasm             │  ← WASM 入口（cdylib）
   │      doc-native           │  ← FFI 入口（cdylib）
   │      doc-server           │  ← HTTP 入口
   └───────────────────────────┘
```

* `doc-core` 是**唯一**对外门面；其它 crate 不允许被 `doc-wasm` / `doc-native` / `doc-server` 直接依赖（除 `doc-semantic-ast` 通过 `doc-utils` 间接使用）。
* `doc-docx-writer` 是**唯一**写入 OOXML 的 crate；公式依赖 `doc-mathml` 提供 OMML。
* `doc-utils` 是**唯一**处理 VFS / 路径 / 图片 / 字体的 crate；不可被绕过。

---

## 1.4 关键第三方依赖

| crate | 版本 | 用途 | 出现位置 |
|-------|------|------|----------|
| `logos` | 0.14 | 零拷贝正则词法 | `doc-latex-reader` |
| `rowan` | 0.15 | 增量语法树（CST） | `doc-latex-reader` |
| `quick-xml` | 0.36 (features=`serialize`) | OOXML / MathML / OMML 写出 | `doc-docx-writer` / `doc-mathml` |
| `zip` | 2.2 (default-features=false, features=`deflate`) | docx / 读 zip | `doc-docx-writer` / `doc-core` / `doc-wasm` |
| `image` | 0.25 (default-features=false, features=`png,jpeg`) | PNG/JPEG 探测 | `doc-utils` / `doc-docx-writer` |
| `base64` | 0.22 | 图片内联 | `doc-docx-writer` |
| `thiserror` | 1.0 | 错误派生 | 全部 crate |
| `anyhow` | 1.0 | 顶层错误装箱 | `Cargo.toml`（锁版本，未深度使用） |
| `serde` | 1.0 (features=`derive`) | AST 序列化 | `doc-semantic-ast` / `doc-core` |
| `serde_json` | 1.0 | FFI 边界 | `doc-core` / `doc-wasm` / `doc-server` |
| `wasm-bindgen` | 0.2 | JS 互操作 | `doc-wasm` |
| `serde-wasm-bindgen` | 0.6 | serde ↔ wasm-bindgen | `doc-wasm` |
| `js-sys` | 0.3 | JS 全局对象 | `doc-wasm` |
| `axum` | 0.7 (features=`multipart,macros`) | HTTP 框架 | `doc-server` |
| `tower` | 0.5 | 服务中间件 | `doc-server` |
| `tower-http` | 0.6 (features=`limit,trace`) | body 限制 / trace | `doc-server` |
| `tokio` | 1.40 (features=`macros,rt-multi-thread,signal`) | 异步运行时 | `doc-server` |
| `tracing` / `tracing-subscriber` | 0.1 / 0.3 | 日志 | `doc-server` |
| `mime` | 0.3 | docx MIME | `doc-server` |
| `clap` | 4.5 (features=`derive`) | CLI 参数（锁版本，V2 落地） | `Cargo.toml` |
| `insta` | 1.40 (features=`yaml,ron`) | 快照测试 | `doc-latex-reader` |
| `proptest` | 1.4 | 属性测试 | `doc-latex-reader` / `doc-utils` |
| `criterion` | 0.5 (features=`html_reports`) | 基准测试（锁版本） | `Cargo.toml` |

---

## 1.5 编译配置

### Release profile

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "symbols"
```

* `lto = "thin"` 平衡编译时间与产物大小。
* `codegen-units = 1` 提升跨 crate 内联概率。
* `strip = "symbols"` 减小 WASM / 二进制体积。

### Dev profile

```toml
[profile.dev]
opt-level = 0
debug = true
incremental = true
```

### Workspace lints

```toml
[workspace.lints.rust]
unsafe-op-in-unsafe-fn = "allow"
```

仅在 `doc-native`（dart:ffi 桥接 crate）需要此 lint，其余 crate 均 `#![forbid(unsafe_code)]`。

---

## 1.6 静态分析

### rustfmt

`.rustfmt.toml`：

```toml
edition = "2021"
max_width = 100
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "default"
reorder_imports = true
reorder_modules = true
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
format_strings = true
format_macro_matchers = true
trailing_comma = "Vertical"
trailing_semicolon = true
```

### clippy

`clippy.toml`：仅一行注释（项目级策略）。

CI 强制：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

---

## 1.7 依赖审计

`deny.toml`（`cargo-deny`）：

* **允许的许可证**：`MIT` / `Apache-2.0` / `Apache-2.0 WITH LLVM-exception` / `BSD-2-Clause` / `BSD-3-Clause` / `ISC` / `Unicode-DFS-2016` / `Unicode-3.0` / `CC0-1.0` / `MPL-2.0` / `Zlib`。
* **重复版本**：`warn`。
* **通配符依赖**：`deny`（必须显式锁版本）。
* **未知 registry**：`deny`（仅 `crates.io`）。
* **未知 git 依赖**：`warn`。
* **漏洞数据库**：`version = 2`（cargo-deny 0.16+ 格式）。
* **yanked**：`warn`。

> 本项目未在 CI 中强制运行 `cargo deny`，但已配置；建议接入 nightly 审计。

---

## 1.8 锁文件策略

* `Cargo.lock` 已 **入仓**（用于应用 crate 锁定可重现构建）。
* 工作区 `publish = false`（内部 crate 不发布到 crates.io）。
* `anyhow` / `clap` / `tokio` 等暂未直接使用的依赖也锁版本，便于 V2 增量启用。

---

## 1.9 构建命令速查

```bash
# 整个 workspace
cargo build --workspace

# 单 crate
cargo build -p doc-core
cargo build -p doc-wasm
cargo build -p doc-native
cargo build -p doc-server

# Release
cargo build --workspace --release

# 带 test
cargo test --workspace --all-targets

# 跑指定集成测试
cargo test -p doc-core --test paper3_e2e -- --nocapture

# 强制 clippy 零警告
cargo clippy --workspace --all-targets -- -D warnings

# 格式化检查
cargo fmt --all -- --check
```

---

## 1.10 跨平台支持矩阵

| 平台 | 核心 | WASM | Native cdylib | HTTP Server | Flutter Desktop |
|------|:----:|:----:|:------------:|:-----------:|:---------------:|
| Linux x86_64 | ✅ | ✅ | ✅ | ✅ | ✅ |
| Windows x64 | ✅ | ✅ | ✅ | ✅ | ✅ |
| macOS x64 / aarch64 | ✅ | ✅ | ✅ | ✅ | ✅ |
| Web (Chrome/Firefox/Safari) | — | ✅ | — | — | ✅ (Flutter Web) |

> WASM 目标：`wasm32-unknown-unknown`。
> Native cdylib 目标：iOS / Android 未在 V1 范围（待 V2 Flutter mobile 启用）。
