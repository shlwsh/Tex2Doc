# 第七章 · Rust 核心构建
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



> 本节描述 Rust 核心 crate 的完整构建流程：从依赖准备到 release 优化。

---

## 1. 环境要求

### 1.1 工具链

* **Rust 1.82+**（项目固定 `stable` channel）
* **cargo**（随 Rust）
* **rustfmt + clippy + rust-src**（`rust-toolchain.toml` 自动安装）

### 1.2 平台特定

| 平台 | 额外依赖 |
|------|----------|
| Linux | `build-essential` / `pkg-config` / `libssl-dev`（部分依赖） |
| Windows | Visual Studio Build Tools 2022（C++ 工作负载） |
| macOS | Xcode Command Line Tools |

### 1.3 工具链固定

`rust-toolchain.toml`：

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-src"]
profile = "minimal"
```

---

## 2. 依赖安装

### 2.1 第一次拉取

```bash
git clone <repo>
cd Tex2Doc
```

### 2.2 启用 Git 钩子（推荐）

```bash
# PowerShell
.\scripts\install_commit_push_hook.ps1

# Bash
git config core.hooksPath .githooks
```

> 启用后，`git commit` 会自动 push 到 origin（详见 `scripts/install_commit_push_hook.ps1`）。

### 2.3 锁定依赖

```bash
# Cargo.lock 已入仓，无需 cargo generate-lockfile
# 但若有 Cargo.toml 变更，建议：
cargo update --workspace
```

---

## 3. 编译

### 3.1 整个 workspace

```bash
cargo build --workspace
# 产物：target/debug/ 下各 crate 的 binary / rlib
```

### 3.2 Release

```bash
cargo build --workspace --release
# 产物：target/release/
```

release profile（`Cargo.toml`）：

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "symbols"
```

* `opt-level = 3`：最大优化。
* `lto = "thin"`：跨 crate 优化（平衡编译时间）。
* `codegen-units = 1`：最大化内联。
* `strip = "symbols"`：减小 binary 体积。

### 3.3 单 crate

```bash
# doc-core（FFI 门面）
cargo build -p doc-core

# doc-compiler-engine（Semantic TeX Engine facade）
cargo build -p doc-compiler-engine

# doc-engine CLI（V2 PDF/质量闭环）
cargo build -p doc-engine

# doc-wasm（需 wasm32 target）
cargo build -p doc-wasm --target wasm32-unknown-unknown

# doc-native（cdylib）
cargo build -p doc-native

# doc-server（二进制）
cargo build -p doc-server

# V2 PDF 质量闭环 crate
cargo build -p doc-tex-facade
cargo build -p doc-docx-pdf
cargo build -p doc-quality
```

### 3.4 包含测试代码

```bash
cargo build --workspace --all-targets
```

### 3.5 编译时间优化

#### 使用 sccache

```bash
# 安装
cargo install sccache

# 配置（~/.cargo/config.toml）
[build]
rustc-wrapper = "sccache"
```

#### 使用 mold / lld（Linux）

```bash
sudo apt install mold clang lld
# ~/.cargo/config.toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

#### 使用 lld（Windows）

```bash
cargo install -f cargo-binutils
rustup component add llvm-tools-preview
```

---

## 4. 测试

### 4.1 全部测试

```bash
cargo test --workspace --all-targets
```

### 4.2 单 crate

```bash
cargo test -p doc-latex-reader
cargo test -p doc-utils
cargo test -p doc-compiler-engine
cargo test -p doc-docx-writer
```

### 4.3 集成测试

```bash
cargo test -p doc-core --test paper3_e2e -- --nocapture
cargo test -p doc-server --test api
bash scripts/build_paper3_compiler_engine_docx.sh
```

### 4.4 显示 println 输出

```bash
cargo test --workspace -- --nocapture
```

### 4.5 文档测试

```bash
cargo test --doc --workspace
```

---

## 5. 静态分析

### 5.1 格式检查

```bash
cargo fmt --all -- --check
# 修复：
cargo fmt --all
```

### 5.2 Clippy（强约束）

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

* `-D warnings`：所有 warning 视为 error。
* CI 强制。

### 5.3 依赖审计

```bash
# 安装 cargo-deny
cargo install --locked cargo-deny

# 审计
cargo deny check
```

`deny.toml` 已配置：
* 允许 MIT / Apache-2.0 / BSD 等。
* 通配符依赖禁止。
* 未知 registry 禁止。

---

## 6. 性能基准

### 6.1 Criterion（V2 路线）

Cargo.toml 已锁 `criterion = 0.5`。当前未用。

```rust
// benches/parse_tex.rs
use criterion::{criterion_group, criterion_main, Criterion};
use doc_latex_reader::parse_tex;

fn bench_parse(c: &mut Criterion) {
    let src = include_str!("../examples/paper3/latex/main-jos.tex");
    c.bench_function("parse_tex", |b| b.iter(|| parse_tex(src)));
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
```

### 6.2 实测

| 操作 | 耗时（Windows 11 / i7-12700H） |
|------|-------------------------------|
| `cargo build --workspace`（首次） | ~3 min |
| `cargo build -p doc-core`（增量） | < 5 s |
| `cargo test -p doc-core --test paper3_e2e` | ~3 s |
| `cargo test -p doc-compiler-engine` | < 1 s（增量） |
| paper3 compiler-engine 转换 | 生成约 3.0 MB DOCX，含 250 blocks / 10 image assets |

---

## 7. 产物清单

| 路径 | 用途 |
|------|------|
| `target/debug/doc_server`（Linux / macOS）<br>`target/debug/doc-server.exe`（Windows） | doc-server 二进制 |
| `target/debug/doc-engine`（Linux / macOS）<br>`target/debug/doc-engine.exe`（Windows） | V2 CLI 二进制 |
| `target/debug/examples/paper3_to_docx` | compiler-engine paper3 示例二进制 |
| `target/debug/libdoc_native.so`（Linux）<br>`target/debug/libdoc_native.dylib`（macOS）<br>`target/debug/doc_native.dll`（Windows） | doc-native cdylib |
| `target/wasm32-unknown-unknown/debug/doc_engine.wasm` | doc-wasm 字节流 |

---

## 8. 跨平台构建

### 8.1 GitHub Actions（CI 已用）

`.github/workflows/ci.yml`：

```yaml
strategy:
  fail-fast: false
  matrix:
    os: [ubuntu-latest, windows-latest, macos-latest]
```

### 8.2 cross（本地）

```bash
# 安装
cargo install cross

# Linux
cross build --target x86_64-unknown-linux-gnu --release -p doc-server
# Windows
cross build --target x86_64-pc-windows-gnu --release -p doc-server
# macOS（需 macOS 主机）
cross build --target x86_64-apple-darwin --release -p doc-server
```

### 8.3 zigbuild（无 glibc 依赖）

```bash
rustup target add x86_64-unknown-linux-musl
cargo install cargo-zigbuild
cargo zigbuild --release --target x86_64-unknown-linux-musl -p doc-server
```

* 产物可在任何 Linux（无 glibc 版本要求）运行。

---

## 9. Docker 镜像（仅 Rust 部分）

```dockerfile
# multi-stage
FROM rust:1.82-slim AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY . .
RUN cargo build --release -p doc-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/doc-server /usr/local/bin/
ENV DOC_SERVER_ADDR=0.0.0.0:2624
EXPOSE 2624
CMD ["/usr/local/bin/doc-server"]
```

```bash
docker build -t doc-engine/server:0.1.0 -f Dockerfile.server .
```

---

## 10. 故障排查

### 10.1 编译错误

| 错误 | 解决 |
|------|------|
| `linker 'cc' not found` | Linux：装 `gcc` / `build-essential` |
| `link.exe not found` | Windows：装 Visual Studio Build Tools |
| `cannot find -lffi` | Linux：装 `libffi-dev` |
| `failed to run custom build command for openssl-sys` | Linux：装 `libssl-dev`；macOS：`brew install openssl` |
| `error: linker not found` | 安装平台 linker |

### 10.2 测试失败

```bash
# 看具体错误
cargo test -p doc-core --test paper3_e2e -- --nocapture

# 单测试
cargo test -p doc-latex-reader lower_inline_math -- --nocapture
```

### 10.3 性能问题

* 用 `cargo build --release` 而非 dev。
* 检查 `lto = "thin"` 是否生效。
* 用 `cargo flamegraph`（需 `cargo-flamegraph`）生成火焰图。

### 10.4 编译时间过长

* 用 sccache。
* 用 mold / lld。
* 用 `cargo check` 替代 `cargo build`（不做 codegen）。

---

## 11. 进一步阅读

* [02-flutter-build.md](./02-flutter-build.md) — Flutter 构建
* [03-wasm-publish.md](./03-wasm-publish.md) — WASM 产物发布
* [04-server-deploy.md](./04-server-deploy.md) — 服务端部署
* [06-ci-and-hooks.md](./06-ci-and-hooks.md) — CI 与钩子
