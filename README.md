# Doc-engine

> LaTeX → DOCX 纯 Rust 核心 + Flutter 全平台转换工具（V1）。

本项目面向学术论文的高保真、本地化格式转换需求，提供：

- **零重型 TeX 依赖**：纯 Rust 实现的 LaTeX 解析与 OOXML 序列化。
- **多端覆盖**：Flutter 桌面 / 移动 / Web PWA、Chrome MV3 扩展、CLI、云端服务。
- **长期资产**：`semantic-ast` 强类型 Enum 语义块模型，可平滑对接 Markdown / HTML / Typst Writer 与 MCP Agent。

## 仓库结构

```
doc-engine/
├── Cargo.toml              # Workspace 顶层
├── crates/
│   ├── core/               # FFI/WASM 统一门面
│   ├── utils/              # 虚拟文件系统 / 路径 / 图片 / 字体映射
│   ├── semantic-ast/       # 核心语义块模型（长期资产）
│   ├── latex-reader/       # Logos + Rowan 双阶段解析
│   ├── mathml/             # LaTeX 数学 → MathML / OMML
│   ├── docx-writer/        # OOXML 序列化 / ZIP 打包 / 模板继承
│   └── bib/                # BibLaTeX 解析
├── flutter_app/            # Flutter 多端工程（待接入）
├── extension/              # Chrome MV3 扩展（待接入）
├── tests/                  # 端到端夹具与 insta 快照
├── scripts/                # 本地脚本
└── docs/                   # 设计与方案文档
```

## 当前里程碑

**Sprint 0 + M1–M2 + M3 + M5 + M7** —— 仓库骨架、CI 基础设施、核心 crate 端到端、列表/表格/图片/Bib/链接/公式管道、reference.docx 模板继承、proptest/insta/夹具质量加固。

详细规划见：

- `docs/Doc-engine_LaTeX-to-DOCX_技术方案_v2.0_20260614.md`
- `docs/Doc-engine_LaTeX-to-DOCX_任务清单_v2.0_20260614.md`
- `docs/Doc-engine_后期开发进展报告_v1.1_20260614.md`

## 构建与测试

```bash
# 编译
cargo build --workspace

# 测试
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Git 工作流

提交代码统一走 `scripts/commit_push.ps1`（自动 add → commit → push）：

```powershell
.\scripts\commit_push.ps1 -Message "fix: 修复 xxx"
.\scripts\commit_push.ps1 -Message "feat: 新增 xxx" -Scope latex-reader   # 自动加 scope
```

仓库已配置 `post-commit` hook（`.githooks/post-commit`），任何 `git commit`（包括 Cursor / VSCode 提交）都会自动 push。

新 clone 后初始化 hook：
```powershell
.\scripts\install_commit_push_hook.ps1
```
或等效命令：`git config core.hooksPath .githooks`

卸载：`.\scripts\install_commit_push_hook.ps1 -Uninstall`

## 许可证

MIT OR Apache-2.0
