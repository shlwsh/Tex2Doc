# CI 与 Git 钩子

> 本节描述 CI 工作流、Git 钩子、本地提交工作流。

---

## 1. GitHub Actions CI

### 1.1 工作流文件

`.github/workflows/ci.yml`：

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -D warnings

jobs:
  rust:
    name: Rust · ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: >-
            .
            crates/core crates/utils crates/semantic-ast
            crates/latex-reader crates/docx-writer crates/bib

      - name: cargo fmt
        run: cargo fmt --all -- --check

      - name: cargo clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

      - name: cargo test
        run: cargo test --workspace --all-targets

      - name: End-to-end artifact
        if: matrix.os == 'ubuntu-latest'
        run: |
          mkdir -p artifacts
          cp crates/core/tests/output/hello.docx artifacts/hello.docx 2>/dev/null || true
      - uses: actions/upload-artifact@v4
        if: matrix.os == 'ubuntu-latest'
        with:
          name: hello-docx
          path: artifacts/hello.docx
          if-no-files-found: ignore
```

### 1.2 触发条件

* **push 到 main 分支**
* **PR 合并到 main 分支**

### 1.3 矩阵

* **Ubuntu latest**（默认）
* **Windows latest**
* **macOS latest**

`fail-fast: false`：一个平台失败不阻塞其他平台。

### 1.4 步骤详解

#### checkout

```yaml
- uses: actions/checkout@v4
```

#### Rust toolchain

```yaml
- name: Install Rust
  uses: dtolnay/rust-toolchain@stable
  with:
    components: rustfmt, clippy
```

* `dtolnay/rust-toolchain@stable`：自动安装 stable + 额外组件。
* `rust-src` 已由 `rust-toolchain.toml` 提供。

#### Cargo 缓存

```yaml
- name: Cache cargo
  uses: Swatinem/rust-cache@v2
  with:
    workspaces: >-
      .
      crates/core crates/utils crates/semantic-ast
      crates/latex-reader crates/docx-writer crates/bib
```

* `Swatinem/rust-cache@v2`：智能 cargo 缓存（基于 Cargo.toml / Cargo.lock 哈希）。
* `workspaces`：要缓存的 workspace 路径。

#### fmt + clippy + test

```yaml
- run: cargo fmt --all -- --check
- run: cargo clippy --workspace --all-targets -- -D warnings
- run: cargo test --workspace --all-targets
```

* `RUSTFLAGS=-D warnings`：所有 warning 视为 error。
* `clippy -D warnings`：clippy 警告也视为 error。

#### 端到端 artifact

```yaml
- name: End-to-end artifact
  if: matrix.os == 'ubuntu-latest'
  run: |
    mkdir -p artifacts
    cp crates/core/tests/output/hello.docx artifacts/hello.docx 2>/dev/null || true
- uses: actions/upload-artifact@v4
  if: matrix.os == 'ubuntu-latest'
  with:
    name: hello-docx
    path: artifacts/hello.docx
    if-no-files-found: ignore
```

* Ubuntu 平台生成 `hello.docx`，上传为 artifact。
* 提供 PR review 视觉参考。

### 1.5 触发与产物

* **触发**：`git push` 到 main / PR 到 main。
* **运行时长**：~5-10 分钟（首次）/ ~2-3 分钟（缓存命中）。
* **产物**：`hello-docx` artifact（Ubuntu 平台）。

### 1.6 自托管 runner（可选）

```yaml
jobs:
  rust:
    runs-on: [self-hosted, linux, x64]
```

需要在仓库 Settings → Actions → Runners → New self-hosted runner 配置。

---

## 2. 本地提交工作流

### 2.1 启用 post-commit 钩子

```powershell
# 仓库根
.\scripts\install_commit_push_hook.ps1
```

行为：
* 设置 `git config core.hooksPath .githooks`。
* `.githooks/post-commit` 在 `git commit` 后自动 push。

或手动：

```bash
git config core.hooksPath .githooks
```

### 2.2 `post-commit` 钩子逻辑

`.githooks/post-commit`：

```sh
#!/bin/sh
set -e
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
if [ -z "$BRANCH" ] || [ "$BRANCH" = "HEAD" ]; then
    exit 0
fi
if ! git rev-parse --abbrev-ref "$BRANCH@{u}" >/dev/null 2>&1; then
    echo "[post-commit] 首次推送，设置 upstream origin/$BRANCH"
    git push --set-upstream origin "$BRANCH" || echo "[post-commit] 推送失败（commit 已保留）"
else
    echo "[post-commit] 自动推送 origin/$BRANCH"
    git push origin "$BRANCH" || echo "[post-commit] 推送失败（commit 已保留）"
fi
```

* 仅在 `git commit` 之后触发。
* 失败不阻断 commit 本体。
* 首次推送自动 `--set-upstream`。

### 2.3 卸载

```powershell
.\scripts\install_commit_push_hook.ps1 -Uninstall
# 或：
git config --unset core.hooksPath
```

### 2.4 一站式 commit + push 脚本

`scripts/commit_push.ps1`：

```powershell
.\scripts\commit_push.ps1 -Message "fix: 修复 xxx"
.\scripts\commit_push.ps1 -Message "feat: 新增 xxx" -Scope latex-reader
.\scripts\commit_push.ps1 -Message "feat: 重大变更" -ForcePush   # 谨慎！
```

行为：
1. 检查 git 仓库。
2. 检查工作区是否有变更。
3. 解析 -Message（第一行作标题）+ 可选 -Scope + -Body。
4. `git add -A`。
5. `git commit --no-verify -m $commitMsg`。
6. `git push origin <branch>`（首次自动 `--set-upstream`）。

参数：
* `-Message`：必填，commit 消息。
* `-Scope`：可选，commit 标题 scope（`feat(scope): ...`）。
* `-Body`：可选，多行用 `;` 分隔。
* `-NoPush`：仅本地，不 push。
* `-ForcePush`：强制 push（谨慎使用，可能覆盖远端历史）。

### 2.5 Conventional Commits 风格

支持的 scope 前缀：

* `feat` —— 新功能
* `fix` —— bug 修复
* `docs` —— 仅文档
* `style` —— 代码风格
* `refactor` —— 重构
* `test` —— 测试
* `chore` —— 杂项
* `build` —— 构建系统
* `ci` —— CI
* `perf` —— 性能

---

## 3. mygit 工具

### 3.1 简介

`.agent/skills/mygit/` 提供三种 mygit 脚本：

* `scripts/mygit.sh`（Bash）
* `scripts/mygit.ps1`（PowerShell）
* `scripts/mygit.py`（Python）

### 3.2 用法

```bash
# 智能提交（AI 辅助）
./scripts/mygit.sh "fix: 修复空白"
# 或
python scripts/mygit.py "fix: 修复空白"
```

* 自动生成 commit message（如有 AI 凭据）。
* 自动 add / commit / push。
* 与 `commit_push.ps1` 配合。

### 3.3 与 `commit_push.ps1` 关系

* `mygit` 是项目无关的通用工具（来自 `.agent/skills/mygit/`）。
* `commit_push.ps1` 是 Tex2Doc 项目专用，行为契约更严格。
* 推荐项目内用 `commit_push.ps1`。

---

## 4. GitNexus 集成

### 4.1 配置

* `AGENTS.md` / `CLAUDE.md` 由 `gitnexus start/end` 注释包裹。
* 内容包含：
  * 索引说明（2419 符号 / 5035 关系 / 203 执行流）。
  * Always Do / Never Do 规则。
  * 资源 / CLI 速查表。

### 4.2 强制规则

* **改函数前**：跑 `impact({target: "symbolName", direction: "upstream"})`。
* **commit 前**：跑 `detect_changes()` 验证范围。
* **HIGH/CRITICAL 风险警告**：必须告知用户。

### 4.3 重新分析

```bash
node .gitnexus/run.cjs analyze
# 或：npx gitnexus analyze
```

### 4.4 文档

* `AGENTS.md` / `CLAUDE.md` 中的 CLI 速查表。
* `.claude/skills/gitnexus/` 全套技能。

---

## 5. 本地 lint 流程

### 5.1 提交前

```bash
# 自动 fmt
cargo fmt --all

# 自动 clippy 修复
cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# 验证
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

### 5.2 提交脚本（`commit_push.ps1` 集成）

可在 `commit_push.ps1` 中加 lint 步骤（V2 计划）。

---

## 6. 持续集成的扩展

### 6.1 WASM 构建

```yaml
# .github/workflows/wasm.yml
name: WASM Build

on:
  push: { branches: [main] }

jobs:
  wasm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: wasm32-unknown-unknown }
      - run: cargo install wasm-pack
      - run: npm run build:wasm
      - uses: actions/upload-artifact@v4
        with:
          name: doc-engine-wasm
          path: flutter_app/wasm/pkg/
```

### 6.2 Flutter Web PWA 构建

```yaml
# .github/workflows/web.yml
name: Web Build

on:
  push: { branches: [main] }

jobs:
  web:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: subosito/flutter-action@v2
        with:
          channel: stable
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: wasm32-unknown-unknown }
      - run: cargo install wasm-pack
      - run: npm run build:wasm
      - run: npm run build:web
      - uses: actions/upload-artifact@v4
        with:
          name: doc-engine-web
          path: flutter_app/build/web/
```

### 6.3 桌面端构建

```yaml
# .github/workflows/desktop.yml
name: Desktop Build

on:
  push: { tags: ['v*'] }

jobs:
  windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: subosito/flutter-action@v2
        with: { channel: stable }
      - run: npm run build:desktop
      - uses: actions/upload-artifact@v4
        with:
          name: doc-engine-windows
          path: flutter_app/build/windows/x64/runner/Release/
```

### 6.4 E2E（Playwright）

```yaml
# .github/workflows/e2e.yml
name: E2E

on:
  push: { branches: [main] }

jobs:
  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - run: npm ci
      - run: npx playwright install --with-deps chromium
      - run: npm run build:wasm
      - run: npm run build:web
      - run: node scripts/build_paper3_zip.mjs
      - run: node scripts/e2e_paper3.mjs
      - uses: actions/upload-artifact@v4
        with:
          name: e2e-report
          path: examples/paper3/output/
```

### 6.5 发布

```yaml
# .github/workflows/release.yml
name: Release

on:
  push: { tags: ['v*'] }

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release --workspace
      - name: Create release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            target/release/doc-server
            target/release/libdoc_native.so
            target/release/doc_native.dll
            target/release/libdoc_native.dylib
            target/wasm32-unknown-unknown/release/doc_engine_bg.wasm
            flutter_app/build/web/
            extension/
```

---

## 7. 安全扫描

### 7.1 cargo-deny（依赖审计）

```bash
# 安装
cargo install --locked cargo-deny

# 运行
cargo deny check
```

`deny.toml` 已配置。

### 7.2 cargo-audit（漏洞）

```bash
cargo install --locked cargo-audit
cargo audit
```

* 检查 RustSec 数据库已知漏洞。

### 7.3 Trivy（容器扫描）

```bash
trivy image doc-engine/server:0.1.0
```

### 7.4 CodeQL（GitHub 原生）

`.github/workflows/codeql.yml`：

```yaml
name: CodeQL
on:
  push: { branches: [main] }
  pull_request: { branches: [main] }
jobs:
  codeql:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: github/codeql-action/init@v3
        with: { languages: rust }
      - uses: github/codeql-action/analyze@v3
```

---

## 8. 故障排查

### 8.1 CI 失败

* 看 GitHub Actions 日志。
* 复现：`cargo test --workspace --all-targets` 本地。
* 缓存问题：用 `Swatinem/rust-cache` 的 `verbose: true` 排查。

### 8.2 post-commit 推送失败

* 检查 remote：`git remote -v`。
* 检查权限：SSH key 或 PAT。
* 钩子失败不阻断 commit 本体。

### 8.3 GitNexus 索引过期

```bash
node .gitnexus/run.cjs analyze
```

### 8.4 mygit / commit_push 脚本问题

* 用 `-NoPush` 跳过 push。
* 手动 `git push` 重试。

---

## 9. 进一步阅读

* [01-rust-build.md](./01-rust-build.md) — Rust 构建
* [02-flutter-build.md](./02-flutter-build.md) — Flutter 构建
* [04-server-deploy.md](./04-server-deploy.md) — 服务端部署
