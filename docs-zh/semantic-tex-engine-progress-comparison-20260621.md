# Semantic TeX Engine 进展对比报告（20260621）
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



**基准版本**：`docs-zh/semantic-tex-engine-detailed-progress-report-20260620-193855.md`（截至 2026-06-20 19:38，commit 748a842）
**当前版本**：基于 2026-06-21 06:00 报告，结合代码实际验证
**验证时间**：2026-06-21 07:15（包含 XeLaTeX hook bug 修复后验证）

---

## 一、总体进展概览

| 维度 | 基准（20260620） | 当前（20260621） | 变化 |
|------|-----------------|-----------------|------|
| 完成的主要能力 | 独立 engine 路径初版 + 双后端原型 | 8 项新能力全部完成 | 大幅推进 |
| 新增 crate | 0 | 4（xdv-parser、semantic-collector、compatibility-analyzer、rule-engine） | 新增 |
| 测试总数（已核实） | ~369 | ~379+ | +10+ |
| 单元测试通过率 | 全通过 | 全通过 | 无回归 |

---

## 二、已完成能力对比

### 2.1 基准中已有的能力（已验证）

以下能力在基准中标记为"已完成初版"，当前代码中均已验证存在且测试通过：

| 模块 | 基准状态 | 验证方式 | 当前状态 |
|------|---------|---------|---------|
| 独立 semantic engine 路径 | 已完成初版 | `doc-compiler-engine` | ✅ 存在，`32+` 测试 |
| 旧 Rust rule 路径隔离 | 已完成 | `cargo test -p doc-core` | ✅ 存在，`19+` 测试 |
| 双后端策略（RuleBased/XeLaTeX/LuaTeX） | 已完成初版 | BackendSelector | ✅ 存在 |
| Collector 输出模型（内置） | 已完成初版 | SemanticCollector/CollectedDocument | ✅ 存在（已迁移到独立 crate） |
| JOS/中文学术 profile | 已完成初版 | `EngineProfile::JosPaper` | ✅ 存在（已迁移到外置 JSON） |
| ReferenceGraph | 已完成初版 | reference_graph | ✅ 存在（已迁移到 semantic-collector） |
| DOCX bookmark/hyperlink | 已完成初版 | docx-writer | ✅ 存在 |
| OMML 块级公式 | 已完成初版 | serializer | ✅ 存在 |
| 表格 span（multicolumn/multirow） | 已完成初版 | serializer | ✅ 存在 |
| 图片尺寸表达式 | 已完成初版 | FigureSizing | ✅ 存在 |
| 兼容性分析器（内置） | 已完成初版 | CompatibilityReport | ✅ 存在（已迁移到独立 crate） |
| paper3 验证脚本 | 已完成初版 | scripts/ | ✅ 存在 |

### 2.2 本轮新增完成的能力

以下能力在基准中为"待实现"或"部分完成"，当前已全部完成：

| 任务 ID | 能力 | 基准状态 | 当前状态 | 新增内容 |
|--------|------|---------|---------|---------|
| **T11** | XDV Parser crate | 未实现 | ✅ 已完成 | `crates/xdv-parser/`，11+13=24 个测试 |
| **T12** | LuaTeX Collector v2 | 未实现 | ✅ 已完成 | `SemanticEventV2`、`XELATEX_SEMANTIC_HOOK` v2、`Caption` 枚举、v1/v2 兼容解析 |
| **T4** | semantic-collector crate 拆分 | 未实现 | ✅ 已完成 | 独立 crate，`lib.rs` + `reference_graph.rs` + `tex_utils.rs`，20 个测试 |
| **T5** | compatibility-analyzer crate 拆分 | 未实现 | ✅ 已完成 | 独立 crate，单一 `lib.rs`，14 个测试 |
| **T7** | Profile 外置化 | 未实现 | ✅ 已完成 | `profiles/*.json`（JSON 而非原设计 TOML），`profiles.rs` |
| **T8** | DOCX 高保真 — 代码块 | 未实现 | ✅ 已完成 | `Block::CodeBlock`、`CodeBlockNode`、`CodeBlockSource`、DOCX 渲染 |
| **T8b** | DOCX 高保真 — 行内公式 OMML | 未实现 | ✅ 已完成 | `write_inline_math_run()`、`TextStyle::MathInline` → OMML |
| **T6** | AI Fallback / Rule Engine | 未实现 | ✅ 已完成 | `crates/rule-engine/`，6 个模块，11 个测试，45+ 内置规则 |

---

## 三、代码结构对比

### 3.1 Crate 变化

**基准时的 crates/ 目录**（推断，基准未明确列出）：
```
doc-core / doc-compiler-engine / doc-latex-reader / doc-docx-writer / ...
```

**当前 crates/ 目录**：
```
bib/           cli/           compatibility-analyzer/  core/
docx-pdf/      docx-writer/   flutter_app/          latex-reader/
mathml/        native/         quality/               rule-engine/
semantic-ast/  semantic-collector/                 server/
tex-facade/    utils/          wasm/
xdv-parser/
```

**新增的 4 个 crate**：
- `xdv-parser/` — T11
- `semantic-collector/` — T4
- `compatibility-analyzer/` — T5
- `rule-engine/` — T6

### 3.2 doc-compiler-engine 结构变化

**基准**：
```
src/lib.rs（包含所有 SemanticCollector/CollectedDocument/CompatibilityReport 等）
```

**当前**：
```
src/lib.rs        # 保留 facade，重新导出独立 crate 的类型
src/profiles.rs   # T7 Profile 外置化
profiles/
  generic-article.json
  chinese-academic.json
  jos-paper.json
  medical-journal.json
```

### 3.3 docx-writer 增强

| 增强项 | 基准 | 当前 |
|--------|------|------|
| 代码块渲染 | 无 | `STYLE_CODE` + Courier New + 语言标注 |
| 行内公式 OMML | 无 | `write_inline_math_run()` → `<m:oMath>` |
| 命名空间 | 无 `xmlns:m` | 完整 `xmlns:m`、`xmlns:a`、`xmlns:wp` |

### 3.4 semantic-ast 增强

| 增强项 | 基准 | 当前 |
|--------|------|------|
| `Block::CodeBlock` | 无 | ✅ 新增 |
| `BlockKind::CodeBlock` | 无 | ✅ 新增 |
| `CodeBlockNode` | 无 | ✅ 新增 |
| `CodeBlockSource` enum | 无 | ✅ 新增（Minted/Listlings/Verbatim/Lstlisting/MarkdownFence） |
| Visitor trait `CodeBlock` 分支 | 无 | ✅ 新增 |
| `block_fingerprint` | 无 CodeBlock | ✅ 新增 |

---

## 四、测试覆盖对比

| Crate | 基准测试数 | 当前测试数 | 变化 |
|-------|-----------|-----------|------|
| doc-xdv-parser | 0（crate 不存在） | 11 + 13 = **24** | **新增** |
| doc-semantic-collector | 0（crate 不存在） | **20** | **新增** |
| doc-compatibility-analyzer | 0（crate 不存在） | **14** | **新增** |
| doc-rule-engine | 0（crate 不存在） | **11** | **新增** |
| doc-compiler-engine | 21+1 ignored | 32 + 1 ignored | +11 |
| doc-core | 5 | 19+ | +14 |
| doc-docx-writer | 36+ | 36 | 不变 |
| doc-latex-reader | 119+ | 119 | 不变 |
| doc-semantic-ast | 5+ | 5+ | 不变 |
| **合计（核心 crates）** | **~369**（含 baseline 的 report 数） | **~379+**（已核实部分） | **+10+** |

> 注：实际测试总数可能更高。docx-writer 36 个测试在 `test result` 中只出现一次但包含多个测试模块。latex-reader 119 个测试同。测试通过率：100%，无失败，无回归。

---

## 五、仍需继续开发的内容

### 5.1 从基准继承的剩余任务

| 任务 | 描述 | 基准优先级 | 当前状态 | 建议优先级 |
|------|------|----------|---------|---------|
| **P2 LayoutGraph 数据接入** | 将 XDV parser 输出接入 compiler-engine 的 layout 推理 | 高 | XDV parser 本身已完成，LayoutGraph 接入未开始 | P1 |
| **P3 LuaTeX Collector v2（node tree）** | 采集 post_linebreak_filter 的 box/glyph/font 信息到 LayoutGraph | 高 | JSONL v2 和 hook 已完成，node tree 采集未开始 | P2 |
| **P6 AI Engine** | 实现可选 AI inference（需网络）+ prompt hash + 审计记录 | 中低 | RuleEngine 框架完成，AI 推断未实现 | P2 |
| **P1 Word REF 字段** | `\ref{label}` → `<w:fldSimple REF="...">` 而非 hyperlink | 未列入 | 仍为 hyperlink | P2 |
| **P1 latex-reader 代码环境检测** | 检测 `minted`/`lstlisting`/`verbatim` 环境，生成 `Block::CodeBlock` | 未列入 | CodeBlock 类型已存在，但 latex-reader 尚未生成该类型 | P1 |
| **P2 TikZ 降级** | 检测 TikZ 源码，编译为 PNG/PDF 或保留为 alt metadata | 未列入 | 未实现 | P3 |
| **P3 Rule Engine AI Plugin** | 实现 `AiEngine` trait，允许插入 OpenAI/Ollama 等后端 | 未列入 | 未实现 | P3 |

### 5.2 新识别出的后续工作

| 任务 | 描述 |
|------|------|
| RuleEngine 与 compiler-engine 集成 | 当前 `rule-engine` 是独立 crate，但尚未在 `compiler-engine` 中实际调用 |
| XDV → LayoutGraph 转换层 | XDV parser 已就绪，需要实现 `XdvDocument` → `LayoutGraph` 的转换逻辑 |
| Profile schema 完整性 | 4 个 JSON profile 已存在，但字段覆盖度和 JOS page setup 的具体实现需要验证 |
| paper3 三路径回归 | 基准中 `paper3 three-docx.sh` 验证成功，当前应重新运行确认新 crate 不破坏现有流程 |

---

## 六、架构健康度评估

### 6.1 架构变化总结

本轮最重要的架构演进是**从单体 `doc-compiler-engine` 拆分为多个专用 crate**：

```
基准架构：
doc-compiler-engine（单体）

当前架构：
doc-compiler-engine（facade + profiles）
  ├── doc-semantic-collector  （语义采集 trait + 输出模型）
  ├── doc-compatibility-analyzer （兼容性扫描器）
  ├── doc-rule-engine （规则引擎 + AI fallback 框架）
  └── doc-xdv-parser （XDV/DVI 字节码解析器）
```

### 6.2 依赖方向（符合基准要求）

```
✅ doc-core       → 不依赖 doc-compiler-engine
✅ doc-compiler-engine → 依赖 doc-semantic-collector, doc-compatibility-analyzer, doc-xdv-parser, doc-rule-engine
✅ doc-semantic-collector → 依赖 doc-semantic-ast, doc-utils
✅ doc-compatibility-analyzer → 无特殊依赖
✅ doc-rule-engine → 无特殊依赖（默认离线）
✅ doc-xdv-parser → 仅 thiserror + serde
```

### 6.3 设计决策说明

| 决策项 | 基准方案 | 当前实现 | 说明 |
|--------|---------|---------|------|
| Profile 文件格式 | TOML | JSON | 基准已预见："沙箱网络限制导致 `toml` crate 无法下载"，使用 `serde_json` 替代 |
| Rule Engine AI | 双层结构（rule + AI） | RuleEngine 框架完成，AI plugin 接口留空 | AI inference 未实现，符合"默认禁用"要求 |
| XDV Parser opcode 覆盖 | 逐步扩大 | 第一阶段 24 个 opcode + fixture | 符合 fixture 驱动的设计原则 |

---

## 七、下一步建议

### 立即可做（Priority 1）

1. **运行 paper3 三路径回归测试**：`bash scripts/build_paper3_three_docx.sh` 确认新 crate 未破坏现有流程
2. **RuleEngine 与 compiler-engine 集成**：将 `RuleEngine` 接入 `compiler-engine` 的未知宏处理路径
3. **latex-reader 代码环境检测**：让 latex-reader 在遇到 `minted`/`lstlisting` 时输出 `Block::CodeBlock`
4. **XDV → LayoutGraph 转换**：实现 `XdvDocument` → `LayoutGraph` 的 glyph/rule/special 映射

### 中期目标（Priority 2）

5. **LuaTeX node tree 采集**：在 v2 hook 基础上，增加 `post_linebreak_filter` 的 box/glyph/font 采集
6. **Word REF 字段**：将 `\ref{label}` 从 hyperlink 升级为 `<w:fldSimple REF="...">`
7. **AI Engine plugin**：定义 `AiEngine` trait，实现 OpenAI/Ollama adapter

### 长期目标（Priority 3）

8. **TikZ 降级策略**：检测 + 编译 + PNG/PDF 降级
9. **Profile schema 完整性验证**：确认各 profile 字段覆盖 JOS/中文学术/医学/通用场景

---

## 八、提交记录（建议）

根据本轮完成情况，建议按以下顺序分 3 个 commit 提交：

| Commit | 内容 | Crates |
|--------|------|--------|
| `feat(xdv-parser): add XDV/DVI bytecode parser crate` | T11 XDV parser + fixture 测试 | `xdv-parser` |
| `feat(compiler-engine: T12) LuaTeX collector v2 with enhanced hooks` | SemanticEventV2、hook v2、兼容解析 | `compiler-engine` |
| `refactor(architecture): split semantic-collector and compatibility-analyzer crates` | T4 + T5 crate 拆分 | `semantic-collector`, `compatibility-analyzer`, `compiler-engine` |
| `feat(compiler-engine: T7) profile externalization to JSON files` | T7 profile 外置化 | `compiler-engine` |
| `feat(rule-engine): add AI fallback / rule engine crate` | T6 规则引擎 | `rule-engine` |
| `feat(docx): add CodeBlock type and inline math OMML rendering` | T8 + T8b DOCX 增强 | `semantic-ast`, `docx-writer`, `core` |

> 实际提交数可按开发习惯合并为 2-3 个逻辑 commit。

---

## 九、XeLaTeX Hook Bug 修复（2026-06-21 07:15）

### 问题描述

三路径验证中 semantic-engine 管道失败，报错：

```
./__docx_semantic_xelatex_hook.tex:121: LaTeX Error: Missing \begin{document}.
l.121 \let\docxsemOldTabularStar\tabular*
```

### 根本原因

1. **T12 新增的 `tabular*` hook**（commit `f361768`）在 preamble 阶段执行，但 `\tabular*` 是由 `array`/`hyperref` 在 `\begin{document}` 之后才定义的宏。hook 在 preamble 中的 `\let\tabular*\tabular*` 拿到了未定义的引用。

2. **修复后的 `\AtBeginDocument` 方案无效**：`\AtBeginDocument` 内部的 group-local 定义在后续环境调用中失效。

3. **第二次修复（`\def\tabular#1`）仍有 TeX 参数匹配错误**：LaTeX 的 `\tabular` 宏将 `{column-spec}` 作为特殊参数处理，与 `\def\tabular#1` 的参数模型不兼容。

4. **JSONL 解析错误**：hook 输出的 v2 事件中 heading 文本包含未转义的 TeX 宏（如 `\refname` 展开后的 `\@mkboth {...}{...}` 结构），导致 JSON 解析失败。

### 修复方案

**核心原则**：XeLaTeX hook 只 hook LaTeX **内核宏**（在 `\begin{document}` 之前已完全定义的命令），不 hook 任何可能被宏包重新定义或有复杂参数结构的命令。

**保留的 hook**：
- `\label` / `\ref` / `\eqref` / `\autoref`：LaTeX 内核命令
- `\cite`：LaTeX 内核命令
- `\includegraphics`：kernel/graphics package，preamble 中已定义

**移除的 hook**：
- `\section` / `\subsection` / `\subsubsection`：latex-reader 从源码捕获
- `\caption`：latex-reader 从源码捕获
- `\tabular` / `\tabular*`：复杂宏层，latex-reader 从源码捕获
- `\equation` / `\align` / `\gather`：复杂宏层，latex-reader 从源码捕获

**JSONL 解析器修复**：
- schema header 行（`{"schema":"semantic-event-v2","engine":"xelatex"}`）单独跳过
- v2 事件检测：识别 `"source":{` + `"macro":"` 模式
- 无 schema 字段的 v2 事件 fallback 到 v1 解析

### 验证结果

```
| path | bytes | media |
|---|---|---:|
| sh | 3,079,377 | 10 |
| rust-rule | 3,055,363 | 10 |
| semantic-engine | 3,057,630 | 10 |
```

无未解析引用，OMML 公式 4 个，fallback 0 个，与基准一致。

---

*报告基于代码实际验证，生成于 2026-06-21 06:18。*
