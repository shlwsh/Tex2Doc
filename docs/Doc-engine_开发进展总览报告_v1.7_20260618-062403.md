# Doc-engine 开发进展总览报告
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



> 版本：v1.7  
> 时间戳：20260618-062403  
> 生成时间：2026-06-18 06:24:03 CST  
> 目标文档：`examples/paper3/latex/main-jos.tex`  
> 最新闭环版本：v11-20260617-233749  
> 参考文档：`docs/Doc-engine_完善设计实现方案_v2.0_20260617.md`、`docs/Doc-engine_DOCX对比闭环进展报告_v1.6_20260617.md`

## 1. 本次总目标

本次工作的总目标是建设 Rust 版高保真学术文档转换引擎，首期聚焦 LaTeX 到 DOCX，要求能够针对 `examples/paper3/latex/main-jos.tex` 这类软件学报论文模板，稳定输出可审阅、可对比、可持续迭代的 DOCX 文稿。

目标不只是“生成一个能打开的 DOCX”，而是形成可工程化演进的转换体系：

1. 以 TeX/LaTeX 语法事实标准和 OOXML/OPC 目标规范作为解析与渲染基线。
2. 建立标准文档 AST，完整承载正文、结构、格式意图、编号、引用、公式、表格、图片、算法、参考文献和来源元数据。
3. 建立从标准 AST 到 DOCX WordprocessingML 的映射管理机制，支持模板 Profile、样式映射、编号映射、媒体关系映射和公式映射。
4. 使用 `scripts/build_docx.sh` 的高保真 sh/Python 流程作为当前质量 oracle，与 Rust 引擎输出进行双版本对比。
5. 将 AST 抽取、DOCX 内容抽取、DOCX 格式语法抽取、逐项差异表、质量报告全部纳入闭环，依据差异反向修复转换引擎。

当前判定：目标体系已经建立，双版本生成和核验链路已经跑通；但转换结果尚未完全通过，主要差距集中在段落边界、列表环境、run 格式签名和样式 XML 等细节。

## 2. 开发实现思路

### 2.1 总体架构思路

采用编译器式转换架构，而不是直接从 TeX 字符串拼接 DOCX：

```text
.tex/.bib/TeX artifacts
        ↓
LaTeX Frontend
        ↓
Standard Document AST
        ↓
Mapping Registry
        ↓
DOCX Render Tree
        ↓
OOXML Writer
        ↓
Quality Gate / Diff Loop
```

核心思想：

- LaTeX 侧以语法解析、宏展开、TeX 侧产物、模板 Profile 共同补齐语义。
- 中间层形成项目专用标准文档 AST，并输出 Markdown/JSON，方便人工核验。
- DOCX 侧以 OOXML/OPC 为目标标准，生成 `word/document.xml`、`styles.xml`、`numbering.xml`、relationships、media 等部件。
- 转换质量不靠主观观察，而靠 Rust 输出和 sh oracle 输出的内容/格式逐项对比。

### 2.2 标准 AST 与元数据策略

当前设计要求强化转换引擎的语法树，使其成为“标准文档 AST”，而不是临时解析结构。AST 至少需要保存：

| 类别 | 内容 |
|---|---|
| 结构节点 | heading、paragraph、list、table、figure、algorithm、equation、reference |
| 行内节点 | text、bold、italic、superscript、subscript、citation、reference、math、code |
| 格式意图 | profile style、semantic role、font hint、paragraph hint、layout hint |
| 编号信息 | section number、figure/table/equation number、list numbering、citation number |
| 来源信息 | source span、source file、rule ids、macro expansion trace |
| 质量元数据 | fallback marker、raw LaTeX residue、mapping rule id、confidence |

AST 输出要求：

- JSON：供程序化比对、回归测试和映射调试。
- Markdown：供人工审查语法树、正文抽取和格式意图。

v11 已输出：

- `docs/verify/v11-20260617-233749-tex-ast.json`
- `docs/verify/v11-20260617-233749-tex-ast.md`
- `docs/verify/v11-20260617-233749-tex-body.md`
- `docs/verify/v11-20260617-233749-tex-syntax-summary.md`

### 2.3 映射管理思路

转换引擎需要单独设计 Mapping Registry，将标准 AST 映射到 DOCX 目标结构。映射不应散落在 writer 代码中。

建议分层：

| 映射层 | 示例 |
|---|---|
| 结构映射 | `Heading(level=1)` -> `w:p + JOSHeading1` |
| 行内映射 | `Citation([1-6])` -> superscript run |
| 编号映射 | `Figure(label=fig:x)` -> `图 1` 与 cross reference |
| 样式映射 | `JOSBody` -> `w:pStyle`、字体、字号、缩进、行距 |
| 关系映射 | `Image(path)` -> `word/media/*` + relationship id |
| 公式映射 | LaTeX math -> normalized text / MathML / OMML |
| 模板映射 | JOS front matter、页眉页脚、双栏、参考文献格式 |

映射规则的存储可采用两阶段：

1. 短期使用专用配置文件，例如 JSON/YAML/TOML，便于版本管理、diff 和审阅。
2. 中长期在规则规模扩大后引入本地数据库，用于保存标准 AST 元数据、映射规则版本、转换结果指标、差异历史和质量趋势。

当前阶段建议优先文件化，避免过早引入数据库复杂度；但设计上保留 `rule_id`、`profile_id`、`version`、`effective_from` 等字段，以便后续平滑迁移。

## 3. 关键里程碑

| 里程碑 | 状态 | 说明 |
|---|---|---|
| sh 高保真转换通路移植 | 已完成 | `scripts/build_docx.sh` 可生成带 sh 标志、版本号和时间戳的 DOCX，并输出校验报告 |
| Rust 主转换链路打通 | 已完成 | `paper3_regression.sh` 可生成 Rust DOCX、AST/render/verify/traceability 等产物 |
| 双版本 DOCX 命名规范 | 已完成 | Rust 文件名含 `rust`，sh 文件名含 `sh`，均含版本号和时间戳 |
| AST Markdown/JSON 输出 | 已完成 | `doc-engine ast-dump` 已输出 v11 AST MD/JSON |
| DOCX 内容/格式差异输出 | 已完成 | `doc-engine docx-diff` 已输出 v11 MD/JSON 差异 |
| 逐项对比表输出 | 已完成 | 已生成 `docs/verify/v11-20260617-233749-逐项对比表.md` |
| 数学函数空格修复 | 已完成 | 修复 `O(Nlog N)`、`O(Mlog K)` 等问题 |
| DOCX 包结构校验 | 已完成 | v11 Rust/sh DOCX 均通过 ZIP 完整性和关键 OOXML 部件检查 |
| 完全对齐 sh oracle | 未完成 | 仍有段落边界、列表、run 格式、styles XML 差异 |

## 4. 最新进展摘要

### 4.1 最新输出文件

| 类型 | 文件 |
|---|---|
| Rust DOCX | `examples/paper3/output/to-docx/v11-论文稿件-jos-rust-20260617-233749.docx` |
| sh DOCX | `examples/paper3/output/to-docx/v11-论文稿件-jos-sh-20260617-233749.docx` |
| 最新闭环报告 | `docs/Doc-engine_DOCX对比闭环进展报告_v1.6_20260617.md` |
| 本总览报告 | `docs/Doc-engine_开发进展总览报告_v1.7_20260618-062403.md` |

### 4.2 v11 质量指标

| 指标 | 当前值 | 说明 |
|---|---:|---|
| `paragraph_delta` | -58 | Rust/sh 段落结构仍未对齐 |
| `table_delta` | 0 | 表格数量已对齐 |
| `drawing_delta` | 0 | drawing 数量已对齐 |
| `media_delta` | 0 | 媒体文件数量已对齐 |
| `equal_paragraphs` | 521 | 相同段落数 |
| `modified_paragraphs` | 12 | 修改段落数 |
| `inserted_paragraphs` | 125 | sh 侧新增段落差异数 |
| `deleted_paragraphs` | 183 | Rust 侧删除段落差异数 |
| `format_changed_paragraphs` | 220 | 格式差异达到当前输出上限 |
| `document_xml_equal` | false | `document.xml` 尚未等价 |
| `styles_xml_equal` | false | `styles.xml` 尚未等价 |

### 4.3 v10 到 v11 的变化

| 指标 | v10 | v11 | 变化 |
|---|---:|---:|---:|
| `equal_paragraphs` | 520 | 521 | +1 |
| `modified_paragraphs` | 12 | 12 | 0 |
| `inserted_paragraphs` | 126 | 125 | -1 |
| `deleted_paragraphs` | 184 | 183 | -1 |
| `paragraph_delta` | -58 | -58 | 0 |

本轮最明确的质量收益是数学函数文本输出收敛。v11 核验中未再发现：

- `O(Nlog`
- `O(Mlog`
- `O(N+Mlog`

对应输出已变为：

- `O(N log N)`
- `O(M log K)`
- `O(M log M)`
- `O(N+M log M)`

## 5. 质量管控方式

### 5.1 双版本对照质量门禁

当前质量门禁采用 Rust 引擎输出与 sh oracle 输出并行生成、并行抽取、逐项比对的方式。

```text
Rust DOCX
    ↓
docx-diff / OOXML parse
    ↓
内容差异 + 格式差异 + XML 差异
    ↑
sh DOCX oracle
```

对比维度包括：

- 段落正文文本。
- 段落样式。
- run 格式签名。
- 表格数量。
- drawing 数量。
- media 数量。
- `document.xml` 规范化差异。
- `styles.xml` 规范化差异。

### 5.2 AST 到 DOCX 的核验链

每轮必须同时保留源侧和目标侧材料：

1. TeX AST JSON/Markdown。
2. TeX 正文抽取。
3. TeX 语法摘要。
4. Rust DOCX 正文抽取。
5. Rust DOCX 格式语法摘要。
6. sh DOCX 正文抽取。
7. sh DOCX 格式语法摘要。
8. DOCX diff JSON/Markdown。
9. 逐项对比表。

这保证可以从差异反查到：

```text
DOCX 差异
  -> Mapping Rule
  -> Standard AST Node
  -> TeX Source Span
  -> 修复转换算法
```

### 5.3 测试与回归策略

当前已执行并通过的验证项：

| 验证项 | 结果 |
|---|---|
| `cargo test -p doc-latex-reader clean_math_function_keeps_space_after_variable` | 通过 |
| `cargo test -p doc-latex-reader clean_math_common_greek_and_fonts` | 通过 |
| `cargo test -p doc-latex-reader latex_to_text_math_function_subscript` | 通过 |
| `cargo fmt --all --check` | 通过 |
| `./scripts/paper3_regression.sh` | 通过 |
| `./scripts/build_docx.sh 11` | 通过 |
| v11 Rust/sh DOCX ZIP 完整性检查 | 通过 |
| v11 Rust/sh DOCX 关键 OOXML 部件检查 | 通过 |

### 5.4 GitNexus 影响分析

按项目 `AGENTS.md` 要求，修改符号前必须执行影响分析。本轮修改涉及：

| 符号 | 风险 | 影响范围 |
|---|---|---|
| `clean_math` | CRITICAL | 14 个符号，10 条流程，2 个模块 |
| `strip_math_command_names` | CRITICAL | 6 个符号，10 条流程，2 个模块 |

处理方式：

- 已在修改前识别 CRITICAL 风险并向用户说明。
- 修改限定在数学函数名前导空格这一局部规则。
- 用新增单测、相关历史单测和 paper3 回归共同验证。

## 6. 文档输出要求

后续每轮转换与修复必须遵守统一文档输出规则。

### 6.1 文件命名

所有核心输出文件必须包含：

- 版本号，例如 `v11`、`v12`。
- 时间戳，例如 `20260617-233749`。
- 生成来源标识，例如 `rust` 或 `sh`。
- 文件用途标识，例如 `tex-ast`、`docx-compare`、`逐项对比表`、`开发进展报告`。

推荐命名：

```text
examples/paper3/output/to-docx/v12-论文稿件-jos-rust-<timestamp>.docx
examples/paper3/output/to-docx/v12-论文稿件-jos-sh-<timestamp>.docx
docs/verify/v12-<timestamp>-tex-ast.md
docs/verify/v12-<timestamp>-tex-ast.json
docs/verify/v12-<timestamp>-docx-compare.md
docs/verify/v12-<timestamp>-docx-compare.json
docs/verify/v12-<timestamp>-逐项对比表.md
docs/Doc-engine_开发进展总览报告_v<report-version>_<timestamp>.md
```

### 6.2 报告内容

每份阶段性进展报告至少包含：

1. 本次总目标。
2. 当前版本和时间戳。
3. 输入文档和输出文件。
4. 开发实现思路。
5. 已完成里程碑。
6. 本轮代码或算法变更。
7. 质量管控方式。
8. 验证命令与结果。
9. 对比指标表。
10. 剩余问题。
11. 下一步规划。

### 6.3 中间文件保留

每轮 `docs/verify` 下的中间文件必须保留，不能只保留最终 DOCX。原因是当前质量提升依赖差异反推算法，必须能复盘每一轮：

- TeX 原始语法如何进入 AST。
- AST 如何映射到 DOCX 段落、run、样式和关系。
- Rust 输出与 sh oracle 在正文、格式、XML 层分别差在哪里。
- 哪些差异已经消除，哪些差异仍然存在。

## 7. 当前风险与问题

1. 段落边界仍未统一：Rust 非空段落数与 sh 非空段落数存在差异，`paragraph_delta=-58`。
2. 列表环境映射仍需收敛：`itemize`/`enumerate` 在部分位置表现为段落合并或样式不一致。
3. run 格式差异较多：bold、superscript、样式 run 分割与 sh oracle 不一致，`format_changed_paragraphs=220` 达到输出上限。
4. `styles.xml` 未等价：样式定义还没有完全对齐 JOS sh oracle。
5. TeX/LaTeX 宏语言的动态行为复杂，不能只依靠静态 parser，仍需 TeX artifact 和 Profile 共同补齐。
6. 当前规则仍以文件化配置和代码规则为主，尚未建立长期规则版本库和质量趋势库。

## 8. 下一步规划

### 8.1 v12 优先任务

1. 处理列表环境：统一 AST 中 `List`/`ListItem` 到 DOCX numbering 或 paragraph style 的映射。
2. 修复 `itemize` 文本痕迹：确保环境名不会进入正文。
3. 建立段落规范化规则：对 Rust/sh 的段落边界差异进行分类，区分真实内容差异和换行/列表造成的结构差异。
4. 建立 run 规范化规则：合并连续相同格式 run，降低无语义格式差异。
5. 输出 v12 双版本 DOCX 与 `docs/verify/v12-<timestamp>-*` 全量材料。

### 8.2 中期任务

1. 将 sh/Python 高保真流程中的 JOS front matter、图片、算法、表格、参考文献规则系统化回迁到 Rust。
2. 建立 Mapping Registry 的配置文件版本，至少覆盖 JOSHeading、JOSBody、JOSAbstract、JOSKeywords、JOSReference、ListBullet、FigureCaption、TableCaption。
3. 扩展 DOCX diff：把“run 分割差异”和“真实格式差异”分开统计。
4. 将 OOXML 清洗规范化固定下来，过滤 rsid 等保存噪音，提升 XML diff 的可解释性。

### 8.3 长期任务

1. 构建可复用的学术文档标准 AST，支持 LaTeX、Markdown、HTML 等输入向 DOCX/HTML/Markdown 输出扩展。
2. 引入规则元数据存储机制，先文件化，后续可迁移到本地数据库。
3. 建立多模板 Profile：JOS、计算机学报、中文信息学报等。
4. 建立质量趋势看板：记录每轮 `equal/modified/inserted/deleted/format_changed` 指标变化。
5. 将文档转换质量门禁纳入 CI，要求关键指标不回退。

## 9. 当前状态判定

当前项目已经从“能生成 DOCX”推进到“可持续质量闭环”的阶段：

- 双版本生成已跑通。
- AST 与 DOCX 差异核验已跑通。
- 关键中间材料已按版本和时间戳归档。
- 已通过差异反推完成一次有效算法修复。

但尚未达到最终目标。下一阶段的重点应从单点文本修复转向结构级收敛，优先处理列表、段落边界和 run 格式规范化，使 Rust 引擎输出逐步接近 sh oracle。
