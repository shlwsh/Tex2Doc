# Doc-engine V1 任务清单（基于 14 周双轨里程碑）
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



> 版本：V2.0 配套
> 编制日期：2026-06-14
> 关联方案：`Doc-engine_LaTeX-to-DOCX_技术方案_v2.0_20260614.md`
> 范围：V1 全部交付（Rust 核心 + Flutter 多端 + Chrome 扩展 + CLI + 云端服务）

---

## 0. 阅读说明

- **轨道**：每条任务标注 `[R]` Rust 核心 / `[F]` Flutter 端 / `[E]` Chrome 扩展 / `[S]` 服务端 / `[X]` 跨轨（CI / 文档 / 测试）。
- **里程碑**：M1–M14，与方案 §6.1 一一对应。
- **依赖**：列出主要前置任务 ID；可并行项标注 `可并行`。
- **估时**：单位为「人日」（1 人日 = 8 小时），仅供排期参考。
- **DoD（Definition of Done）**：每条任务给出可验收的产物与质量门禁。
- **状态**：`⬜` 未开始 / `🟡` 进行中 / `✅` 已完成 / `❌` 阻塞（评审后补）。

---

## 1. 全局工程准备（M0，预 Sprint 启动前）

| ID | 任务 | 轨道 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|---|
| X-001 | 建立 Monorepo：`Cargo.toml` workspace、`.gitignore`、`README.md`（中文） | [X] | 0.5d | — | 仓库 `cargo build` 通过；目录与方案 §3 一致 | ⬜ |
| X-002 | 配置 `rust-toolchain.toml`、`rustfmt`、`clippy`、`deny.toml` | [X] | 0.5d | X-001 | `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo deny check` 全通过 | ⬜ |
| X-003 | GitHub/Gitea Actions 流水线：`fmt` → `clippy` → `test` → `insta review` | [X] | 1d | X-002 | PR 触发流水线全绿 | ⬜ |
| X-004 | 建立 `docs/` 索引：技术方案、API 契约草案、错误码草案 | [X] | 0.5d | X-001 | 三份骨架文档可被引用 | ⬜ |
| X-005 | 选型与版本锁定：`logos`、`rowan`、`quick-xml`、`zip`、`biblatex`、`image`、`clap`、`axum`、`flutter_rust_bridge`、`flutter` | [X] | 0.5d | X-001 | `Cargo.toml` 与 `pubspec.yaml` 锁定主版本 | ⬜ |

**M0 小计**：3 人日

---

## 2. M1–M2：核心骨架 + 最小端到端

### 2.1 Rust 核心侧

| ID | 任务 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|
| R-001 | `crates/utils/vfs`：BTreeMap 虚拟文件系统 + `mount_dir` | 1d | X-001 | 单元测试：mount/lookup/missing 三种场景 | ⬜ |
| R-002 | `crates/utils/path`：include 路径解算（含 `\graphicspath`） | 1d | R-001 | 含 Windows/Unix 路径归一化测试 | ⬜ |
| R-003 | `crates/latex-reader/lexer`：Logos 词法（控制序列 / 分组 / 数学定界 / 注释） | 1.5d | X-001 | 单测覆盖 ≥ 20 个 token 模式 | ⬜ |
| R-004 | `crates/latex-reader/include_resolver`：Pass-1 拓扑构建 + 环检测 | 1.5d | R-001 | 含循环 include 报错样例 | ⬜ |
| R-005 | `crates/latex-reader/parser`：Rowan 语法树 + 错误恢复 | 3d | R-003, R-004 | 输入坏文件不 panic；产出 LST | ⬜ |
| R-006 | `crates/semantic-ast`：定义 `Document/Block/TextRun/MetaData/Span` | 1d | — | 派生 `Serialize/Deserialize`；serde 回环测试通过 | ⬜ |
| R-007 | `crates/latex-reader → semantic-ast` 降级：`\section`、段落 | 1.5d | R-005, R-006 | 输入 `hello.tex` 输出 `Document` 包含 Heading+Paragraph | ⬜ |
| R-008 | `crates/docx-writer/model`：OOXML 扁平结构体（pPr/rPr/tblPr） | 1.5d | — | 编译通过 + 基础 newtype 测试 | ⬜ |
| R-009 | `crates/docx-writer/serializer` + `packer`：最小 Heading + Paragraph → docx | 2d | R-007, R-008 | LibreOffice / Word 能打开；含标题/段落 | ⬜ |
| R-010 | `crates/core/api`：暴露 `convert_sync` 同步入口 | 0.5d | R-009 | FFI 单元 smoke 测试通过 | ⬜ |

### 2.2 Flutter 端

| ID | 任务 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|
| F-001 | `flutter_app` 工程初始化：Material 3、空骨架、依赖锁定 | 1d | X-001 | 桌面三端空跑 | ⬜ |
| F-002 | `flutter_rust_bridge_codegen` 接入 | 0.5d | R-010, F-001 | `flutter_app/rust/` 生成可用 | ⬜ |
| F-003 | CI 多平台打包脚本（Windows/macOS/Linux） | 1d | F-001 | 触发可产出空安装包 | ⬜ |

**M1–M2 小计**：16 人日（R: 14.5d + F: 2.5d）

---

## 3. M3–M4：列表 / 表格 / 图片 + 工作台 UI

### 3.1 Rust 核心侧

| ID | 任务 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|
| R-011 | `bib` crate：`@article`/`@inproceedings`/`@book`/`@misc` 解析 | 2d | R-006 | 20 条样例 .bib 解析回环 | ⬜ |
| R-012 | `bib/style` Numeric / Author-Year 两种样式 | 1d | R-011 | 单元测试覆盖两样式 | ⬜ |
| R-013 | `\cite{...}` 关联 → `Block::Bibliography` 渲染 | 1d | R-012 | 样例含 5 条引用 → 文末 BIB 条目 | ⬜ |
| R-014 | `latex-reader` 降级 `itemize`/`enumerate` → `Block::List` | 1d | R-007 | 三层嵌套列表测试 | ⬜ |
| R-015 | `latex-reader` 降级 `tabular` → `Block::Table` | 1.5d | R-007 | 含 `&`/`\\`/`\hline` 测试 | ⬜ |
| R-016 | `latex-reader` 降级 `\includegraphics` → `Block::Figure` | 0.5d | R-007 | 含路径解析测试 | ⬜ |
| R-017 | `utils/image`：解码 + 重采样 + 重压缩 | 1.5d | R-001 | PNG/JPEG 双向回环 + DPI 测试 | ⬜ |
| R-018 | `docx-writer` 列表 / 表格 / 图片 → OOXML | 2.5d | R-009, R-014..R-016 | `numbering.xml`、表格 `tblPr` 正确 | ⬜ |

### 3.2 Flutter 端

| ID | 任务 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|
| F-004 | 工作台中央看板：拖拽 + 骨架屏 | 2d | F-001 | 桌面三端拖入 .tex 即解析 | ⬜ |
| F-005 | Riverpod 异步状态总线骨架 | 1d | F-001 | `ConversionState` Provider 完成 | ⬜ |
| F-006 | 工程 `.zip` 拖入支持（vfs 注入） | 1d | R-001, F-004 | 拖入压缩包自动解包并列出 | ⬜ |

**M3–M4 小计**：14 人日（R: 11d + F: 4d，可并行）

---

## 4. M5–M6：数学公式 + CTeX + 进度总线

### 4.1 Rust 核心侧

| ID | 任务 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|
| R-019 | `latex2mathml` 接入与公式 token 边界识别 | 1.5d | R-005 | 行内/块级公式各 10 例 | ⬜ |
| R-020 | `docx-writer/mathml_to_omml`：最小节点集映射 | 3d | R-019 | Word 中双击公式可编辑 | ⬜ |
| R-021 | OMML 节点降级策略 + 警告日志 | 1d | R-020 | 未覆盖节点打 WARN 不中断 | ⬜ |
| R-022 | `utils/fontmap` CTeX → Office 字体映射表 + 加载 | 1d | — | 默认映射 8 项；可扩展 | ⬜ |
| R-023 | 中文段落渲染走 `fontmap` 注入 styles.xml | 1d | R-022, R-018 | 含 CTeX 字体样例 1 篇 | ⬜ |
| R-024 | `core/api::convert_stream`（带 Phase 事件） | 1d | R-009 | 4 个 Phase 事件 | ⬜ |

### 4.2 Flutter 端

| ID | 任务 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|
| F-007 | 进度总线对接 `convert_stream` | 1d | R-024, F-005 | 进度条 + 阶段标签实时刷新 | ⬜ |
| F-008 | 高级选项侧边栏（模板 / 字体 / 引用样式） | 2d | F-005 | 三类配置可调 + 持久化 | ⬜ |
| F-009 | 模板下拉：IEEE / Springer / 自定义上传 | 1d | F-008 | 自定义 reference.docx 注入 core | ⬜ |

**M5–M6 小计**：12.5 人日（R: 8.5d + F: 4d，可并行）

---

## 5. M7–M8：模板继承 + 日志抽屉

### 5.1 Rust 核心侧

| ID | 任务 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|
| R-025 | `docx-writer/template`：reference.docx 抽取 + 合并算法 | 3d | R-018 | 用户模板样式覆盖默认 | ⬜ |
| R-026 | OOXML Schema 基础校验（自定义） | 1d | R-025 | 缺标签时打 WARN | ⬜ |
| R-027 | 公式 OMML 节点集扩展（M6 已覆盖的补全） | 1d | R-020 | +5 个常见节点 | ⬜ |

### 5.2 Flutter 端

| ID | 任务 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|
| F-010 | 日志抽屉：级别过滤 + 复制 | 1.5d | R-024, F-005 | 三级过滤可即时刷新 | ⬜ |

**M7–M8 小计**：6.5 人日（R: 5d + F: 1.5d，可并行）

---

## 6. M9–M10：多文件 + CLI 联调

### 6.1 Rust 核心侧

| ID | 任务 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|
| R-028 | include 拓扑完整算法（DAG 缓存 + 重复检测） | 1.5d | R-004 | 大文章 5 文件嵌套 | ⬜ |
| R-029 | `.zip` 工程多资源解析 | 1.5d | R-017, R-001 | 10 文件项目端到端 | ⬜ |
| R-030 | `crates/server` Axum 框架 + 路由 + 队列 | 2d | R-024 | 同步调用可达 | ⬜ |
| R-031 | `crates/cli` clap v4 派生 + 退出码规范 | 1d | R-009 | 退出码 0/1/2/3 | ⬜ |

### 6.2 Flutter 端

| ID | 任务 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|
| F-011 | CLI 跨平台打包与冒烟（Win/Linux） | 1d | R-031 | 两平台 `convert --help` 一致 | ⬜ |
| F-012 | 桌面端联调：模板切换 + 字体映射 UI 反馈 | 1d | F-008, F-009 | 切换后立即看到 docx 差异 | ⬜ |

**M9–M10 小计**：8 人日（R: 6d + F: 2d，可并行）

---

## 7. M11–M12：WASM 裁剪 + PWA + 扩展

### 7.1 Rust 核心侧

| ID | 任务 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|
| R-032 | `crates/wasm` 包装 + 内存预算（≤ 256MB） | 1.5d | R-024 | `wasm-pack build` 通过 | ⬜ |
| R-033 | WASM 端公式降级开关（小尺寸） | 0.5d | R-021 | 5MB 文件不超内存 | ⬜ |

### 7.2 Flutter / Web / 扩展侧

| ID | 任务 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|
| F-013 | PWA：`manifest.json` + `service_worker.js` | 1d | — | Lighthouse PWA 体检 ≥ 90 | ⬜ |
| F-014 | PWA：IndexedDB 历史（≤ 50 条） | 1d | F-013 | 历史可查可重下 | ⬜ |
| F-015 | PWA：WASM 离线缓存 | 0.5d | R-032, F-013 | 断网仍可转 ≤ 5MB 文件 | ⬜ |
| E-001 | Chrome 扩展 Manifest V3 骨架 | 1d | — | Popup 360px 弹出 | ⬜ |
| E-002 | Content Script 上下文菜单注入 | 1d | E-001 | Overleaf 页面右键可见 | ⬜ |
| E-003 | Service Worker 调 WASM + 剪贴板 OOXML | 1.5d | R-032, E-001 | Ctrl+V 粘到 Word 公式可编辑 | ⬜ |
| E-004 | 大小分流：> 5MB 弹气泡跳 App/PWA | 0.5d | E-001 | 拦截逻辑可观察 | ⬜ |

**M11–M12 小计**：9.5 人日（R: 2d + F: 4.5d + E: 4d，可并行）

---

## 8. M13–M14：云端 + 全平台发布

### 8.1 Rust / 服务端

| ID | 任务 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|
| S-001 | 大文件降级通道（> 50MB → 队列 + 限流） | 1.5d | R-030 | 429 行为正确 | ⬜ |
| S-002 | 云端产物 1 小时自动清理 | 0.5d | S-001 | 定时任务有日志 | ⬜ |
| S-003 | 部署文档 + Docker 镜像 | 1d | S-001 | `docker compose up` 可跑 | ⬜ |

### 8.2 全平台冒烟

| ID | 任务 | 估时 | 前置 | DoD | 状态 |
|---|---|---|---|---|---|
| X-006 | 桌面三端冒烟：Windows/macOS/Linux | 1.5d | F-003, F-011 | 端到端 1 篇 IEEE 论文 | ⬜ |
| X-007 | 移动两端冒烟：Android/iOS | 1d | F-007 | 端到端 1 篇短文 | ⬜ |
| X-008 | 扩展冒烟：Overleaf / arXiv | 1d | E-003 | 公式复制成功 | ⬜ |
| X-009 | PWA 冒烟：在线 + 离线 | 0.5d | F-015 | Lighthouse PWA ≥ 90 | ⬜ |
| X-010 | 静态签名 + 安装包发布（GitHub/Gitea Release） | 0.5d | X-006..X-009 | 4 个产物（3 桌面 + 1 PWA）就绪 | ⬜ |

**M13–M14 小计**：8.5 人日（S: 3d + X: 5.5d）

---

## 9. 质量与里程碑门槛

每里程碑结束必须达成（否则不进入下一里程碑）：

- `cargo test` 全绿。
- `cargo clippy -- -D warnings` 全绿。
- 覆盖核心 crate 行覆盖 ≥ 80%。
- 当里程碑相关 insta 快照零非预期 diff。
- 当里程碑 demo 在评审会上通过。

---

## 10. 资源与排期参考

- 假设 3 人小队（R 主力 1.5 + F 主力 1 + 全栈 0.5）。
- 单里程碑 2 周（10 工作日）≈ 30 人日可用。
- 各里程碑预算（R+F+E+S+X）：3 / 16 / 14 / 12.5 / 6.5 / 8 / 9.5 / 8.5 ≈ 78 人日 ≈ 2.6 倍单里程碑。
- 结论：3 人小队 14 周**临界**；建议 M13 缓冲周保留至少 5 人日，应对 R-01/R-04 高风险。

---

## 11. 任务总览（按 ID 排序）

```
X-001 X-002 X-003 X-004 X-005
R-001 R-002 R-003 R-004 R-005 R-006 R-007 R-008 R-009 R-010
F-001 F-002 F-003
R-011 R-012 R-013 R-014 R-015 R-016 R-017 R-018
F-004 F-005 F-006
R-019 R-020 R-021 R-022 R-023 R-024
F-007 F-008 F-009
R-025 R-026 R-027 F-010
R-028 R-029 R-030 R-031 F-011 F-012
R-032 R-033 F-013 F-014 F-015
E-001 E-002 E-003 E-004
S-001 S-002 S-003
X-006 X-007 X-008 X-009 X-010
```

**任务总数**：55 条（R: 33 / F: 15 / E: 4 / S: 3 / X: 10，可按需裁剪）

---

## 12. 后续 V1.x 待办（不在本任务清单内，仅记录）

- LLM 格式自愈（V1.1 评估）
- Markdown / HTML Writer（V1.1）
- 协同编辑（V2.0 评估）

---

> 本任务清单与 `Doc-engine_LaTeX-to-DOCX_技术方案_v2.0_20260614.md` 配套使用，所有估时与里程碑为 V1 基线，应根据实际人力与风险在 Sprint 0 评审中调整。
