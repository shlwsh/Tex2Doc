# Tex2Doc (Doc-engine) 安装与配置指南
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



本指南将帮助您在本地环境中快速配置、编译并运行 Tex2Doc 项目。

## 环境依赖清单

由于本项目为多端架构（Rust 核心 + Flutter 界面 + WebAssembly + Node.js 脚本），需提前安装以下基础工具链：

1. **Rust 工具链**
   - 核心解析与转换库由纯 Rust 编写。
   - 要求版本：`1.82` (通过项目中的 `rust-toolchain.toml` 自动管理)。
   - 包含组件：`rustfmt`, `clippy`, `rust-src`。

2. **Node.js 与 npm**
   - 用于自动化测试、Playwright E2E 验证及依赖构建脚本执行。

3. **Flutter SDK**
   - 用于构建 Web、Desktop (Windows) 等跨端用户界面（位于 `flutter_app/` 目录）。

4. **wasm-pack**
   - 用于将 Rust 核心库编译为 WebAssembly (`.wasm`)，供 Web 及 Chrome 扩展端使用。

---

## 1. 基础环境安装

### 1.1 安装 Rust
如果尚未安装 Rust，请运行以下命令：
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 1.2 安装 Node.js
推荐使用 LTS 版本的 Node.js（>= 18.x）。
```bash
# 可通过 nvm 或其他包管理工具安装
nvm install --lts
nvm use --lts
```

### 1.3 安装 Flutter
请参考官方文档下载并配置 Flutter：[Flutter 安装指南](https://docs.flutter.dev/get-started/install)。
确保将其加入系统 `PATH` 环境变量中，并运行检查确认无误：
```bash
flutter doctor
```

### 1.4 安装 wasm-pack
用于 WebAssembly 目标构建：
```bash
cargo install wasm-pack
```

---

## 2. 项目配置与初始化

### 2.1 克隆仓库
```bash
git clone https://github.com/shlwsh/Tex2Doc.git
cd Tex2Doc
```

### 2.2 安装 Node 依赖与 E2E 测试环境
项目根目录下包含依赖构建配置，并引入了 Playwright 用于端到端 (E2E) 测试。执行以下命令：
```bash
npm install
npx playwright install --with-deps
```

### 2.3 初始化 Git Hook (强烈推荐)
本项目约定代码提交通过脚本完成（自动包含 add → commit → push）。建议安装项目定制的 `post-commit` 钩子，以保障协同流程：
```powershell
# Windows 环境推荐方式
.\scripts\install_commit_push_hook.ps1
```
*(或者跨平台等效命令手动配置：`git config core.hooksPath .githooks`)*

---

## 3. 编译与运行指南

### 3.1 编译 Rust 核心工程
用于直接测试解析核心、生成文档或使用 CLI 工具：
```bash
# 构建整个 Workspace 的所有 crate
cargo build --workspace

# 运行所有单元测试与快照测试 (insta)
cargo test --workspace

# 代码格式化与规范检查 (Lint)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

### 3.2 构建多端产物 (通过 npm scripts)

本项目在 `package.json` 中配置了统一的构建指令：

- **WASM 模块构建**（供 Flutter Web 及扩展使用）：
  ```bash
  npm run build:wasm
  ```
- **Web 端构建**（基于 Flutter Web）：
  ```bash
  npm run build:web
  ```
- **Windows 桌面端构建**（基于 Flutter Windows）：
  ```bash
  npm run build:windows
  ```
- **一键全端快捷构建**：
  ```bash
  npm run build:all      # 构建 WASM + Web
  npm run build:desktop  # 构建 Native + Windows
  ```

### 3.3 运行验证测试 (E2E)
项目配置了完备的 Playwright 端到端测试，用于严格验证最终生成的 `.docx` 和其它产物：
```bash
# 运行完整 E2E 验证工作流
npm run verify:e2e
```

---

## 4. 常见问题排查 (FAQ)

1. **Rust 版本报错 / 找不到 1.82**
   执行 `rustup update`。项目内配置了 `rust-toolchain.toml`，在拉取代码后 cargo 会自动切换并下载对应版本（stable/1.82）的工具链。
2. **wasm-pack 构建失败**
   请确认已为当前 Rust 工具链添加了 wasm 目标：
   ```bash
   rustup target add wasm32-unknown-unknown
   ```
3. **Flutter Web 报错找不到包**
   在编译 Flutter 项目前，请确保已经先执行过 `npm run build:wasm`。该操作会将 Rust 编译好的 WASM 产物注入到 `flutter_app/wasm/pkg` 目录下供前端调用。
