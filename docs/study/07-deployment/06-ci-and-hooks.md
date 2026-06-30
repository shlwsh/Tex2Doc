# CI 与 Git 钩子
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



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
        # macOS runners are temporarily disabled because they can remain queued
        # long enough to block PR merges and production deployment. Re-enable
        # macos-13 once runner availability is stable.
        os: [ubuntu-latest, windows-latest]
    steps:
      - name: Configure git line endings
        run: git config --global core.autocrlf false
      - uses: actions/checkout@v5
        with:
          fetch-depth: 1

      - name: Install Linux native dependencies
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libfontconfig1-dev \
            libxkbcommon-dev \
            libwayland-dev \
            libx11-xcb-dev \
            libxcb1-dev \
            libxcb-render0-dev \
            libxcb-shape0-dev \
            libxcb-xfixes0-dev \
            libegl1-mesa-dev \
            libgl1-mesa-dev \
            libudev-dev \
            libinput-dev \
            postgresql \
            postgresql-contrib

      - name: Start PostgreSQL test database
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo systemctl start postgresql
          sudo -u postgres psql -c "ALTER USER postgres PASSWORD 'postgres';"
          sudo -u postgres createdb -O postgres docdb || true

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: .

      - name: cargo fmt
        run: cargo fmt --all -- --check

      - name: cargo clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

      - name: cargo test
        run: |
          cargo test --workspace --all-targets --exclude doc-server -- --test-threads=1
          cargo test -p doc-server --lib --bin doc-server -- --test-threads=1

      - name: doc-server API integration tests
        if: matrix.os == 'ubuntu-latest'
        env:
          DATABASE_URL: postgres://postgres:postgres@127.0.0.1:5432/docdb
        run: cargo test -p doc-server --test api -- --test-threads=1

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

  flutter:
    name: Flutter web/client checks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 1

      - name: Install Flutter
        uses: subosito/flutter-action@v2
        with:
          channel: stable
          cache: true

      - name: Flutter pub get
        working-directory: flutter_app
        run: flutter pub get

      - name: Flutter analyze
        working-directory: flutter_app
        run: flutter analyze

      - name: Flutter test
        working-directory: flutter_app
        run: flutter test
```

### 1.2 触发条件

* **push 到 main 分支**
* **PR 合并到 main 分支**

### 1.3 矩阵与 Job 拆分

1. **`rust` 检查 Job**：
   - 包含 **Ubuntu latest** 和 **Windows latest** 双系统矩阵。
   - 为了确保 GitHub Actions 在高峰期不因 macOS runner 排队造成阻塞，**macOS 平台目前已临时下线**。
2. **`flutter` 检查 Job**：
   - 在 Ubuntu 环境中运行，执行客户端 `pub get`、代码分析 `analyze` 和自动化单元测试 `test`。

### 1.4 步骤详解

#### Linux 原生依赖及数据库
在 Ubuntu 容器上首先使用 `apt-get` 补全图形渲染（字体探测及 Web 渲染）所需的动态库，并安装 PostgreSQL 及常用工具组件。

#### 测试库自动初始化与测试串行化
- 每次 CI 运行，会通过 systemd 自动拉起本地 PostgreSQL 并创建名称为 `docdb` 的测试数据库。
- 为了避免高并发环境下测试库发生死锁，`cargo test` 全程开启 **`--test-threads=1`** 以单线程串行模式运行。
- 在 `DATABASE_URL` 设置的环境下，特地隔离运行 `doc-server` 专有的 `api` 集成测试。

#### Cargo 缓存与 checkout 优化
- 缓存只针对根目录的工作区进行缓存。
- 使用 `actions/checkout@v5` 并通过 `fetch-depth: 1` 显著提升拉取速度。

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

### 6.6 生产服务器自动部署 (deploy-production.yml)

工作流 `.github/workflows/deploy-production.yml` 用于将服务与静态资源一键部署至腾讯云生产服务器：

* **触发方式**：通过 `workflow_dispatch` 手动触发，或在代码推送/合并到 `main` 分支时自动触发运行。
* **两个阶段**：
  - **`build`**：
    1. 构建 Linux 平台的 `doc-server` 生产二进制包。
    2. 分别构建针对 `/`、`/user/`、`/admin/` 三种 base-href 路由的 Flutter Web 多端产品代码。
    3. 合并打包为包含服务器程序及三端静态文件的 `tex2doc-production.tar.gz` 归档。
  - **`deploy`**：
    1. 下载编译产物归档。
    2. 基于 GitHub Secrets 中的私钥和目标服务器配置自动装载并建立 SSH 连接。
    3. SCP 传输安装包，自动在远程服务器创建带时间戳的 `releases` 归档子目录并解压，最后利用软链接 `current` 进行指向切换。
    4. 执行 `systemctl restart tex2doc-server` 和 `systemctl reload nginx` 进行平滑发布。

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
