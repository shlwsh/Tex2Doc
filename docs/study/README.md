# Tex2Doc 项目说明文档（Study 索引）
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



> **项目代号**：Doc-engine / Tex2Doc
> **当前版本**：V2.1（V1.3 纯 Rust 转换主线 + V2 PDF 质量闭环 + Semantic TeX Engine facade + PostgreSQL 持久化）
> **文档目标**：为新加入项目的工程师 / 架构师 / 维护者提供**自下而上**的完整学习入口

本目录汇总了 Tex2Doc 项目的所有学习材料。建议按以下顺序阅读：

---

## 阅读路径

| 阶段 | 章节 | 目标 |
|------|------|------|
| ① 概览 | [01-overview/01-features.md](./01-overview/01-features.md) | 5 分钟了解 Tex2Doc 是什么、能做什么 |
| ① 概览 | [01-overview/02-quick-tour.md](./01-overview/02-quick-tour.md) | 10 分钟跑通最小演示链路 |
| ② 技术栈 | [02-tech-stack/01-rust-stack.md](./02-tech-stack/01-rust-stack.md) | 核心 Rust 工具链与依赖矩阵 |
| ② 技术栈 | [02-tech-stack/02-flutter-dart-stack.md](./02-tech-stack/02-flutter-dart-stack.md) | Flutter / Dart / FFI 工具链 |
| ② 技术栈 | [02-tech-stack/03-web-extension-stack.md](./02-tech-stack/03-web-extension-stack.md) | Chrome MV3 / Node.js 端到端栈 |
| ③ 工程目录 | [03-project-structure/01-top-level.md](./03-project-structure/01-top-level.md) | 仓库根目录结构总览 |
| ③ 工程目录 | [03-project-structure/02-rust-crates.md](./03-project-structure/02-rust-crates.md) | crates/ 内 19 个 crate 及 apps/ 内 2 个 app 详尽说明 |
| ③ 工程目录 | [03-project-structure/03-flutter-app.md](./03-project-structure/03-flutter-app.md) | `flutter_app/` 多端工程目录 |
| ③ 工程目录 | [03-project-structure/04-extension-scripts-tests.md](./03-project-structure/04-extension-scripts-tests.md) | 扩展、脚本、测试、夹具目录 |
| ④ 架构 | [04-architecture/01-end-to-end-pipeline.md](./04-architecture/01-end-to-end-pipeline.md) | 端到端数据流：LaTeX → DOCX |
| ④ 架构 | [04-architecture/02-layered-architecture.md](./04-architecture/02-layered-architecture.md) | 分层与依赖关系 |
| ④ 架构 | [04-architecture/03-frontend-bridges.md](./04-architecture/03-frontend-bridges.md) | 三种前端如何对接 Rust 核心 |
| ⑤ 关键技术 | [05-key-tech/01-include-topology.md](./05-key-tech/01-include-topology.md) | 多文件 LaTeX 拓扑与拼接 |
| ⑤ 关键技术 | [05-key-tech/02-lexer-and-cst.md](./05-key-tech/02-lexer-and-cst.md) | Logos 词法 + Rowan 语法树 |
| ⑤ 关键技术 | [05-key-tech/03-semantic-lowering.md](./05-key-tech/03-semantic-lowering.md) | CST → 语义 AST 降级 |
| ⑤ 关键技术 | [05-key-tech/04-docx-serialization.md](./05-key-tech/04-docx-serialization.md) | 语义 AST → OOXML 序列化 |
| ⑤ 关键技术 | [05-key-tech/05-math-pipeline.md](./05-key-tech/05-math-pipeline.md) | LaTeX 公式 → OMML 数学 |
| ⑤ 关键技术 | [05-key-tech/06-vfs-and-fonts.md](./05-key-tech/06-vfs-and-fonts.md) | VFS 抽象与字体探测 |
| ⑥ 使用说明 | [06-user-guide/01-cli-and-script.md](./06-user-guide/01-cli-and-script.md) | 命令行 / 脚本使用（含统一 CLI `doc-engine` 12 个子命令） |
| ⑥ 使用说明 | [06-user-guide/02-pwa-web.md](./06-user-guide/02-pwa-web.md) | Flutter Web PWA 使用 |
| ⑥ 使用说明 | [06-user-guide/03-desktop.md](./06-user-guide/03-desktop.md) | Flutter Desktop 桌面端使用 |
| ⑥ 使用说明 | [06-user-guide/04-chrome-extension.md](./06-user-guide/04-chrome-extension.md) | Chrome 扩展使用 |
| ⑥ 使用说明 | [06-user-guide/05-http-server.md](./06-user-guide/05-http-server.md) | HTTP 服务端使用 |
| ⑦ 部署手册 | [07-deployment/01-rust-build.md](./07-deployment/01-rust-build.md) | Rust 核心构建 |
| ⑦ 部署手册 | [07-deployment/02-flutter-build.md](./07-deployment/02-flutter-build.md) | Flutter 多端构建 |
| ⑦ 部署手册 | [07-deployment/03-wasm-publish.md](./07-deployment/03-wasm-publish.md) | WASM 包发布 |
| ⑦ 部署手册 | [07-deployment/04-server-deploy.md](./07-deployment/04-server-deploy.md) | 服务端部署与自动 CD |
| ⑦ 部署手册 | [07-deployment/05-extension-pack.md](./07-deployment/05-extension-pack.md) | Chrome 扩展打包 |
| ⑦ 部署手册 | [07-deployment/06-ci-and-hooks.md](./07-deployment/06-ci-and-hooks.md) | CI 与 Git 钩子 |
| ⑧ 演进路线 | [08-pdf-pipeline/README.md](./08-pdf-pipeline/README.md) | V2 PDF 流水线草案：docx→PDF 同步输出 + TeX oracle 质量对比 |
| ⑧ 演进路线 | [08-pdf-pipeline/01-pipeline-overview.md](./08-pdf-pipeline/01-pipeline-overview.md) | V2 端到端总览、产物命名、CI 流程 |
| ⑧ 演进路线 | [08-pdf-pipeline/02-tex-facade.md](./08-pdf-pipeline/02-tex-facade.md) | `crates/tex-facade` 设计：xelatex / tectonic / latexmk 可插拔封装 |
| ⑧ 演进路线 | [08-pdf-pipeline/03-docx-to-pdf.md](./08-pdf-pipeline/03-docx-to-pdf.md) | `crates/docx-pdf` 设计：LibreOffice headless 二次转换 |
| ⑧ 演进路线 | [08-pdf-pipeline/04-quality-comparison.md](./08-pdf-pipeline/04-quality-comparison.md) | `crates/quality` 三层质量对比：结构 + 文本 + 视觉 |
| ⑧ 演进路线 | [08-pdf-pipeline/05-implementation-roadmap.md](./08-pdf-pipeline/05-implementation-roadmap.md) | M1–M5 实施路线图、风险、回滚 |
| ⑧ 演进路线 | [08-pdf-pipeline/07-progress-2026-06-20.md](./08-pdf-pipeline/07-progress-2026-06-20.md) | 最新实现快照：V2 CLI、compiler-engine、paper3 to-docx |

---

## 文档总览

### 第一章 · 项目概览（[01-overview/](./01-overview/））
* **产品定位**：LaTeX/CTeX → DOCX 纯 Rust 核心 + Semantic TeX Engine facade + Flutter 全平台转换工具
* **目标用户**：需要把 LaTeX 论文、报告、模板高保真转换为 Word 文档的学术/工程作者
* **关键差异化**：语义优先转换、中文学术论文/JOS 模板高保真、可选 TeX oracle 质量闭环、本地化离线运行、多端覆盖（桌面/Web/扩展/CLI/服务端）

### 第二章 · 技术栈（[02-tech-stack/](./02-tech-stack/））
* Rust 1.82+ 稳定工具链 + Cargo Workspace
* Flutter 3.12+ / Dart 3 + FFI 桥接
* Chrome MV3 Service Worker + Content Script
* Node.js + Playwright 端到端测试栈
* Node.js + fflate 验证脚本

### 第三章 · 工程目录（[03-project-structure/](./03-project-structure/））
* 仓库根：工作区配置、CI、钩子、夹具
* `crates/`：19 个 crate（包含 `core` / `compiler-engine` / `utils` / `semantic-ast` / `latex-reader` / `mathml` / `docx-writer` / `bib` / `wasm` / `native` / `tex-facade` / `docx-pdf` / `quality` / `cli` / `xdv-parser` / `semantic-collector` / `compatibility-analyzer` / `rule-engine` / `commercial-api-client`）
* `apps/`：2 个 app（包含基于 PostgreSQL 的 Rust API 服务 `rust-service` (即 `doc-server`) 和桌面端 Slint APP `slint-user`）
* `flutter_app/`：多端 Dart 工程（Web/Windows/macOS/Linux）
* `extension/`：Chrome MV3 扩展（popup + background + content）
* `tests/`、`examples/`、`scripts/`、`docs/`、`flutter_app/wasm/`、`flutter_app/windows/`

### 第四章 · 技术架构（[04-architecture/](./04-architecture/））
* 六段主链路：VFS/Include 拓扑 → Logos/Rowan 解析 → Semantic Collector → Semantic AST / StandardDocument → Document Graph → DOCX Renderer
* 三种前端集成模式：WASM（Web）、FFI（Desktop）、HTTP（Server）
* 两层门面：`doc-core` 保持 FFI/WASM/HTTP 兼容，`doc-compiler-engine` 承载新一代语义编译器 facade

### 第五章 · 关键技术（[05-key-tech/](./05-key-tech/））
* 深入解析每个 crate 的设计原理、数据结构、关键算法
* 适合需要修改核心逻辑、二次开发、性能调优的工程师

### 第六章 · 使用说明（[06-user-guide/](./06-user-guide/））
* 六种使用方式：CLI/PWA/Desktop/Extension/Server/Compiler Engine 脚本
* 每种方式含：环境要求、构建步骤、典型操作流程

### 第七章 · 部署手册（[07-deployment/](./07-deployment/））
* Rust 核心、Flutter 多端、WASM 产物、HTTP 服务、扩展包的完整构建/打包/发布
* CI 双平台矩阵（Ubuntu / Windows），macOS 临时从必过矩阵移除以避免队列堆积
* 增加了腾讯云生产环境通过 GitHub Actions 实现的一键自动部署与回滚机制
* Git 钩子与提交工作流

### 第八章 · 演进路线 V2 · PDF 流水线（[08-pdf-pipeline/](./08-pdf-pipeline/））
* V1 → V2 演进：新增 docx→PDF 同步生成、TeX oracle 质量对比、CLI 串联构建
* 已落地 crate：`tex-facade`（可插拔 TeX 封装）、`docx-pdf`（LibreOffice roundtrip）、`quality`（结构+文本+视觉三层）、`cli`（统一命令入口）
* 新增语义编译 facade：`doc-compiler-engine`，对齐 Semantic TeX Engine 方向，支持 source/dir/zip/VFS → DOCX
* 数据库持久化：在 `doc-server` 服务中集成了 `sqlx` 与 PostgreSQL，用于管理充值、计费和用户记录。
* **当前状态**：核心实现已落地；最新快照见 [08-pdf-pipeline/07-progress-2026-06-20.md](./08-pdf-pipeline/07-progress-2026-06-20.md)

---

## 配套原始文档

`docs/` 目录下另有以下已存在的工程文档，建议交叉参考：

* `Doc-engine_LaTeX-to-DOCX_技术方案_v2.0_20260614.md` — 项目技术方案（V1 总览）
* `Doc-engine_LaTeX-to-DOCX_任务清单_v2.0_20260614.md` — 任务清单与里程碑
* `Doc-engine_后期开发进展报告_v1.1~v1.3_20260614.md` — 后期开发报告
* `Doc-engine_任务清单完成度补丁_v1.0_v1.3_20260614.md` — 任务完成度补丁
* `Doc-engine_V1.3_计划与实施归档_20260614.md` — V1.3 计划与归档
* `docs-zh/semantic-tex-engine-docx-implementation-plan.md` — Semantic TeX Engine 最新实现技术方案

---

## 贡献约定

* 提交代码：先按 `AGENTS.md` 跑 GitNexus impact/detect_changes，再使用 git 或 `scripts/commit_push.ps1`
* 文档变更：与代码同步更新到本目录
* 提问反馈：使用本仓库的 issue / PR 流程
