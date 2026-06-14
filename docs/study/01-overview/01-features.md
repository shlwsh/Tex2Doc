# 第一章 · 项目功能介绍

> 本章为读者提供 5 分钟可读完的产品概览。读完应能向他人讲清楚：**Tex2Doc 是什么、解决什么痛点、覆盖哪些场景、当前边界在哪里**。

---

## 1.1 一句话定义

**Tex2Doc（Doc-engine）** 是一个**纯 Rust 编写核心转换逻辑、Flutter 跨端承载 UI** 的 **LaTeX → DOCX 文档格式转换工具**，可在 Windows / macOS / Linux 桌面、Flutter Web PWA、Chrome MV3 扩展以及 HTTP 服务端多种形态下运行。

---

## 1.2 痛点与定位

| 传统方案 | 痛点 | Tex2Doc 的对应能力 |
|---------|------|---------------------|
| 论文排版用 LaTeX，导师/合作者要 Word 版本 | 手工转格式、丢样式 | 一键导出 .docx，保留章节、列表、表格、图片、公式 |
| pandoc + 复杂 LaTeX 模板 | 公式变图片、表格错乱 | 内置 LaTeX→OMML 公式管道、表格降级为 docx 原生表格 |
| Overleaf / arXiv 分享需要 Word | 在线服务不可控、文件外泄 | 100% 本地化运行，WASM/FFI 不联网 |
| 工程化 LaTeX 项目（含 `\input`、`\graphicspath`） | 普通转换工具无法处理 include 拓扑 | 内置 `IncludeGraph` 模块，支持 `\input` / `\include` / `\graphicspath` |
| 中文 CTeX / rjthesis 等特殊模板 | 大量装饰命令污染段落流 | 顶部 metadata 剥离层 + 宏表展开，干净输出 |
| Web / 桌面 / 浏览器扩展需要不同实现 | 多端重复开发 | 共享同一 Rust 核心，三种绑定（WASM / FFI / HTTP） |

---

## 1.3 核心功能清单

### 1.3.1 解析与降级（Rust 核心）

* **多文件工程解析**：自动构建 `\input` / `\include` / `\subfile` 拓扑（DAG），支持环检测与拓扑排序。
* **路径解析**：兼容 `\graphicspath{...}` 列表、相对 / 绝对路径、自动补 `.tex` 扩展。
* **词法 / 语法**：Logos 词法（命令、括号、注释、数学定界符、空白） → Rowan 语法树（Group / Env / Command / Whitespace / Math / ...）。
* **宏展开**：`\newcommand` / `\providecommand` / `\renewcommand` / `[n]` 参数（V1 简化，无参数替换）支持单 pass 共享宏表。
* **CST → 语义 AST 降级**：
  * 顶层段命令：`\section` / `\subsection` / `\subsubsection` / `\paragraph` / `\caption`（自动编号 1.1.1 / 图 1 / 表 1）。
  * 段落内联清洗：`\textbf` / `\textit` / `\texttt` / `\emph`、纯装饰命令吞掉、字体切换命令保留文本。
  * 数学：行内 `$...$` 抽为独立 `Equation` 块，块级 `$$...$$` / `\[...\]` / `equation` / `align` / `gather` 整段保留。
  * 列表：嵌套 `itemize` / `enumerate` / `description`。
  * 表格：`tabular` / `tabular*` / `array`，支持 `\multicolumn` 与 `\rowcolor`。
  * 引用：`\cite{key}` → 文档内全局 `[n]` 编号；`\ref` / `\label` / `\href` / `\url` / `\footnote` 静默吞掉。
  * 元数据剥离：`\rjtitle` / `\rjauthor` / `\hypersetup` / `\usepackage` / `\documentclass` 等 50+ 命令直接吞掉，不污染正文。
* **错误降级**：所有未匹配内容进入 `Block::RawFallback`；未闭合 group / env 自动补；**绝不 panic**。
* **数学公式管道**：LaTeX 源码 → `MathExpr` 简化 AST → OMML（`<m:oMath>`）。
  * 支持：数字 / 标识符 / 二元运算符 / 上下标 / 分式 / 根式 / `\frac` / `\sqrt[n]` / `\left(...\right)` / `\sin` `\cos` `\tan` / 希腊字母 / `\begin{matrix}` 矩阵。
  * 不支持语法降级为 `<m:mtext>` 文本。
* **图片处理**：PNG / JPEG 探测（`image` crate），`word/media/imageN.png` 内联嵌入，base64 内联；其它格式返回错误。
* **模板继承**：可选 `reference.docx` 字节流解析 `word/styles.xml`，按 `w:styleId` 同名覆盖 / 缺失补全策略合并。

### 1.3.2 序列化（docx-writer）

* 写出最小可工作的 OOXML 包：`[Content_Types].xml` / `_rels/.rels` / `word/_rels/document.xml.rels` / `word/document.xml` / `word/styles.xml`。
* 默认样式表：Title / Heading1-3 / BodyText / ListBullet / ListNumber / Caption / TableHeader。
* 图片：内联 `<w:drawing>` + `wp:inline` + `pic:pic` + `w:binData`（base64）。
* 公式：嵌入 `<m:oMath>` 段（OMML）。
* 引用：作者-年份（`AuthorYear`）与数字（`Numeric`）两种内置 BibTeX 样式。

### 1.3.3 BibLaTeX 解析（`doc-bib`）

* 支持 `@inproceedings` / `@article` / `@book` / `@misc` / `@techreport`。
* 字段：`author` / `title` / `year` / `booktitle` / `journal` / `publisher` / `url`。
* 错误降级：未闭合自动补、非法条目跳过。

### 1.3.4 字体探测（`doc-utils/fontdetect`）

* 自动识别 Windows / macOS / Linux 系统字体目录。
* 内置 CTeX 字体 → Office 字体映射表（SimSun / SimHei / KaiTi / FangSong / SimLi 等）。
* 三态结果：`Available` / `Embed` / `Fallback`。
* 写出 docx 时按探测结果替换 `w:ascii` / `w:hAnsi` / `w:eastAsia` / `w:cs` 属性。

### 1.3.5 端到端形态

| 形态 | 入口 | 适用场景 |
|------|------|----------|
| **CLI / 集成测试** | `cargo test -p doc-core --test paper3_e2e` | CI 验证、本地脚本 |
| **HTTP 服务端** | `cargo run -p doc-server` | 集成到企业内部系统、Web 代理 |
| **WASM 库** | `wasm-pack build crates/wasm` | Flutter Web、Chrome 扩展、第三方 JS |
| **Native cdylib** | `cargo build -p doc-native` | Flutter Desktop（Windows / macOS / Linux） |
| **Flutter App** | `flutter build web/windows/...` | 跨端 PWA 与桌面应用 |
| **Chrome MV3 扩展** | `extension/manifest.json` | 浏览器内联转换、Overleaf / arXiv 集成 |

### 1.3.6 工程基础设施

* **CI**：GitHub Actions 三平台矩阵（Ubuntu / Windows / macOS），fmt + clippy -D warnings + cargo test。
* **Git 钩子**：`.githooks/post-commit` 自动 push。
* **提交脚本**：`scripts/commit_push.ps1`（PowerShell）一站式 add / commit / push。
* **端到端验证**：`scripts/verify_paper3.mjs`（Playwright） + `scripts/e2e_paper3.mjs`（Web PWA） + `bin/native_smoke.dart`（Desktop）。
* **GitNexus 索引**：2419 符号 / 5035 关系 / 203 执行流已索引；建议改代码前跑 `impact` 与 `detect_changes`。

---

## 1.4 典型应用场景

### 场景 1：研究生提交 Word 版本
* 学生本地有 `main-jos.tex` + 多个 `\input` 子文件 + `references.bib`。
* 在 Tex2Doc Web PWA 上传 zip，输入 `main-jos.tex`，下载 docx。
* 整个过程不联网，3 秒内完成。

### 场景 2：Overleaf 在线编辑后导出
* Chrome 扩展监测 Overleaf / arXiv 页面的选中文本。
* 用户右键「使用 Doc-engine 转换」→ 弹出 popup → 选本地 zip → 下载 docx。
* 单文件限制 5 MB；超过时引导用户使用桌面 App。

### 场景 3：企业批量期刊投稿
* 公司内自部署 `doc-server`（HTTP，限 50 MiB / 请求）。
* 投稿系统对接 `POST /api/v1/convert`（multipart: file + main_tex）。
* 后端返回 docx 字节流，自动 `Content-Disposition: attachment`。

### 场景 4：本地 LaTeX 工程
* CLI 调用 `cargo test -p doc-core --test paper3_e2e` 把 `examples/paper3/latex/` 转换到 `output/main-jos.docx`。
* 用于回归测试与 CI 验证。

---

## 1.5 当前版本边界（V1 限制）

> 知道"能做什么"同样重要：以下场景**当前不支持**或降级为占位。

| 能力 | 当前状态 | 备注 |
|------|----------|------|
| 完整 LaTeX 引擎（编译数学宏包、引用解析） | ❌ 不支持 | 纯解析器，不调用 TeX |
| PDF / PostScript / EPS 图片 | ❌ 不支持 | 显式 `Unsupported` 错误 |
| TikZ / pgfplots 绘图 | ❌ 整段 `RawFallback` | 保留原文 |
| `\def` / `\let` / 条件宏 / 嵌套宏定义 | ❌ 不展开 | 宏表简化实现 |
| `\newcommand` 带可选参数 `[def]{body}` 的实参替换 | ❌ 仅做字面替换 | 不识别 `#1` |
| 章节自动交叉引用 (`\ref`) | ⚠️ 静默吞掉 | 不生成 OOXML 域代码 |
| `beamer` 幻灯片 | ❌ 整段 RawFallback | 不在 V1 范围 |
| 多语言 RTL / BiDi | ❌ 不支持 | V1 仅 LTR |
| 复杂宏包（`minted` / `listings` 高亮） | ❌ RawFallback | V2 路线图 |
| Office 字体嵌入字形 | ⚠️ 提示级 | `Embed` 状态下在 docx 中声明，但不嵌入字形文件 |

---

## 1.6 与同类方案的对比

| 维度 | Tex2Doc | pandoc | tectonic | latex2docx 商业工具 |
|------|---------|--------|----------|---------------------|
| 公式类型 | OMML 原生可编辑 | 图片（mathjax → png） | 仅生成 PDF | 图片 |
| 表格 | docx 原生表格 | docx 表格 | 依赖 pandoc | 图片 |
| 多文件 include | ✅ DAG 拓扑 | ✅ 但无 graphicspath | ✅ 调用 TeX | 弱 |
| 离线 | ✅ 全部 | ✅ 全部 | ✅ 全部 | 部分 |
| 多端 | 桌面/Web/扩展/服务 | CLI | CLI | 桌面 |
| Rust 实现 | ✅ 100% 核心 | ❌ Haskell | ❌ Rust+TeX | 闭源 |
| 启动 < 200ms | ✅（WASM 一次性加载 ~3.5MB） | ✅ | ❌ 启动 TeX 引擎 | — |

---

## 1.7 关键成果指标（V1.3 现状）

* **核心代码量**：约 7,400 行 Rust（不含 vendor / target）。
* **测试覆盖**：`cargo test --workspace` 全通过；`paper3_e2e` 跑通 8 千行 LaTeX（包含 6 个 `\input` 子文件 + BibTeX）。
* **转换耗时**：`examples/paper3/latex/main-jos.tex` 在 Windows 11 / i7-12700H 上单次转换 < 800 ms。
* **产物大小**：约 41 KB 输入 zip → 38 KB docx（首四字节 `PK\x03\x04` 验证通过）。
* **内容断言**：5/5 关键中文短语（"微服务架构下" / "网关" / "Grafana Loki" / "石洪雷" / "赵涓涓"）命中。
* **杂质剥离**：21 个 LaTeX 装饰命令（`\hypersetup` / `\rjtitle` / `\PassOptionsToClass` 等）100% 剥离。
* **多平台 CI**：Ubuntu + Windows + macOS 三平台 `cargo test --workspace --all-targets` 全部通过。

---

## 1.8 文档导航

继续阅读 [02-quick-tour.md](./02-quick-tour.md) 跑通最小演示。
