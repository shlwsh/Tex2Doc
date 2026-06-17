# Doc-engine 开发任务清单

> 版本：v2.0  
> 日期：2026-06-17  
> 来源方案：`docs/Doc-engine_完善设计实现方案_v2.0_20260617.md`  
> 目标：将设计方案拆解为可开发、可验收、可持续更新的任务清单，并逐步完成实现。

## 状态约定

| 状态 | 含义 |
|---|---|
| TODO | 尚未开始 |
| DOING | 正在实现 |
| DONE | 已完成并有验证证据 |
| BLOCKED | 受外部条件阻塞 |

## Phase 0：高保真基线固化

| ID | 状态 | 任务 | 验收证据 |
|---|---|---|---|
| P0-01 | DONE | 移植 JOS 高保真转换入口 `scripts/build_docx.sh` | `./scripts/build_docx.sh` 可生成 DOCX 与校验报告 |
| P0-02 | DONE | 移植 `scripts/build_jos_docx.py` 与 `scripts/verify_jos_docx.py` | Python 语法检查通过，手动 build/verify 通过 |
| P0-03 | DONE | 引入 JOS 格式定义 JSON | `docs/format/jos_2025_docx_format_definitions.json` 存在 |
| P0-04 | DONE | 为 `scripts/build_docx.sh` 增加输入 hash manifest，避免章节变更后复用旧 PDF/BBL | `examples/paper3/latex/.main-jos.inputs.sha256` 已生成；二次运行命中 manifest 并复用 PDF/BBL |

## Phase 1：标准规则库与 Profile

| ID | 状态 | 任务 | 验收证据 |
|---|---|---|---|
| P1-01 | DONE | 建立 `standards/tex`、`standards/ooxml`、`standards/mappings` 目录 | `standards/tex/*.yaml`、`standards/ooxml/*.yaml`、`standards/mappings/standard-ast-to-docx.yaml` 已存在 |
| P1-02 | DONE | 建立 `profiles/jos-2025` 目录 | `profiles/jos-2025/profile.yaml`、`styles.yaml`、`overrides.yaml` 已存在 |
| P1-03 | DONE | 定义最小 `SyntaxRule`、`MappingRule`、`ProfileOverride` 字段集 | 规则/Profile YAML 已包含 `id`、`source`、`syntax`、`semantic`、`validation`、`style_from_profile`、`quality` 等字段 |
| P1-04 | DONE | 生成 `standards.lock.json` | `scripts/build_standards_lock.py` 生成 `standards.lock.json`，记录 11 个规则/Profile 文件 SHA-256 |
| P1-05 | DONE | 评估 SQLite cache，实现前保持为可选项 | `docs/Doc-engine_标准规则缓存策略_v1.0_20260617.md` 明确首版采用 YAML/JSON + `standards.lock.json` 作为事实源，SQLite 仅保留为可重建派生缓存 |

## Phase 2：标准文档 AST

| ID | 状态 | 任务 | 验收证据 |
|---|---|---|---|
| P2-01 | DONE | 在 `semantic-ast` 新增 `StandardDocument` | `cargo test -p doc-semantic-ast` 通过 |
| P2-02 | DONE | 新增 `SourceBundle`、`BuildArtifacts`、`DocumentMetadata`、`NumberingState`、`ResourceIndex`、`Diagnostic` | `examples/paper3/output/main-jos.ast.json` 可序列化输出 metadata、source、numbering、resources、diagnostics |
| P2-03 | DONE | 新增 `BlockNode`、`InlineNode`、`StyleIntent`、`LayoutHints` | `standard_document_json_roundtrip` 单测通过 |
| P2-04 | DONE | `latex-reader` lowering 输出 `StandardDocument` | 新增 `lower_to_standard_document` / `lower_with_macros_to_standard_document`；`ast-dump` 已切到 reader 层标准 AST API；paper3 AST 含 metadata/front matter、章节、图表、算法、公式、theorem-like、引用相关文本，且 `RawFallback=0` |
| P2-05 | DONE | 保留旧 `Document` 兼容 API | `cargo test -p doc-latex-reader --lib` 67 passed；`cargo check -p doc-engine` 通过 |

## Phase 3：AST Dump

| ID | 状态 | 任务 | 验收证据 |
|---|---|---|---|
| P3-01 | DONE | 新增 CLI 子命令 `ast-dump` | `doc-engine ast-dump --help` 可用，`cargo check -p doc-engine` 通过 |
| P3-02 | DONE | 支持输出 `*.ast.json` | `examples/paper3/output/main-jos.ast.json` 含 10 个 source hash、156 个 block、72 个编号项、10 个资源项、0 个 diagnostic |
| P3-03 | DONE | 支持输出 `*.ast.md` | `examples/paper3/output/main-jos.ast.md` 含 metadata、Source Files、Numbering、Resources、Blocks、Diagnostics |
| P3-04 | DONE | AST dump 标注规则 id | 156 个 AST block 均含 `rule_ids`，覆盖 `latex2e.section`、`graphicx.includegraphics`、`booktabs.table`、`algorithm2e.algorithm`、`latex2e.theorem` 等 TeX/LaTeX 规则 id；OOXML 映射规则在 render dump 中标注 |

## Phase 4：映射注册表与 DOCX Render Tree

| ID | 状态 | 任务 | 验收证据 |
|---|---|---|---|
| P4-01 | DONE | 新增 `MappingRegistry` | `MappingRegistry::for_profile("jos-2025")` 可查找 block/inline/resource mapping |
| P4-02 | DONE | 新增 `DocxRenderTree` | `render_tree_json_roundtrip` 单测通过，支持 Markdown/JSON dump |
| P4-03 | DONE | 新增 CLI 子命令 `render-dump` | `doc-engine render-dump --help` 可用，`cargo check -p doc-engine` 通过 |
| P4-04 | DONE | 将 `StandardDocument` 映射为 DOCX Render Tree | `examples/paper3/output/main-jos.render.json` 含 5 个 package part、156 个 render node、7 个 style、10 个 rel、10 个 media、0 个 diagnostic |

## Phase 5：Rust 主链路 P0/P1 修复

| ID | 状态 | 任务 | 验收证据 |
|---|---|---|---|
| P5-01 | DONE | 修复图片资产传递链 | `cargo test -p doc-core --test paper3_e2e paper3_main_jos_to_docx -- --nocapture` 通过；Rust `convert_dir` 输出 DOCX 含 10 个 `word/media/*` 和 10 个图片 relationship |
| P5-02 | DONE | 移植 algorithm2e parser 到标准 AST | `examples/paper3/output/main-jos.ast.json` 中 `RawFallback=0`，algorithm 节点保留，新增 theorem/proof/proposition 为 `theorem_like` |
| P5-03 | DONE | 将 `AlgorithmNode` 映射为 DOCX 算法表格 | `cargo test -p doc-docx-writer` 通过；paper3 `document.xml` 含算法三列表格、行号列、注释列和缩进 guide 前缀 |
| P5-04 | DONE | 实现 `.bbl` 引用顺序和上标引用 | `lower_to_document_with_cite_map` 支持 `.bbl` cite_map；`cargo test -p doc-latex-reader lower_cite_uses_external_bbl_order_and_superscript_runs` 通过；paper3 DOCX 含 50 个 superscript 引用标记 |
| P5-05 | DONE | 实现两遍 label 编号和 `\ref` 替换 | `cargo test -p doc-latex-reader lower_ref_replaces_labels_from_collect_pass` 通过；paper3 DOCX 中 `fig:`、`tab:`、`alg:`、`eq:`、`thm:`、`prop:`、`\ref`、`\label` 残留计数均为 0 |
| P5-06 | DONE | 改造表格 AST，支持 booktabs/tabular* | `cargo test -p doc-latex-reader lower_table -- --nocapture` 通过；paper3 DOCX 12 个表格不再残留 `[TAB:]`、`@{}`、`extracolsep`、`tabular`、`toprule/midrule/bottomrule` |
| P5-07 | DONE | 修复公式上下标和常见数学符号 | `cargo test -p doc-latex-reader clean_math -- --nocapture`、`cargo test -p doc-latex-reader lower_theorem_like -- --nocapture`、`cargo test -p doc-latex-reader lower_front_matter_abstract_normalizes_math -- --nocapture`、`cargo test -p doc-latex-reader latex_to_text_math_function_subscript -- --nocapture`、`cargo test -p doc-latex-reader parse_algorithm_comment_normalizes_math -- --nocapture` 通过；paper3 DOCX 数学命令与非代码裸 `^`/`_` 残留为 0 |

## Phase 6：质量体系产品化

| ID | 状态 | 任务 | 验收证据 |
|---|---|---|---|
| P6-01 | DONE | 将 `verify_jos_docx.py` JSON schema 固定下来 | `docs/schema/verify_jos_docx_report.schema.json` 和 `scripts/validate_verify_report_schema.py` 已存在；`python3 scripts/validate_verify_report_schema.py examples/paper3/output/main-jos.verify.json` 通过 |
| P6-02 | DONE | `quality` crate 联合 AST dump、render dump、DOCX XML、PDF 文本比对 | `scripts/quality_traceability_report.py` 读取 AST JSON、render JSON、DOCX XML、PDF 文本和 verify JSON，输出 `main-jos.traceability.{md,json}`，报告含 AST rule ids、render mapping ids、DOCX part/media/relationship 和 failed checks |
| P6-03 | DONE | paper3 作为回归样本接入 CI/本地脚本 | `scripts/paper3_regression.sh` 一键生成 DOCX、AST dump、render dump、verify report 和 traceability report；当前质量门通过并返回 0（verify: `passed=True`，refs=56，ratio=0.876） |

## 本轮执行结果

本轮已完成：

1. `P1-01`、`P1-02`、`P1-03`、`P1-04` 的规则/Profile 文件骨架和 lock 文件。
2. `P2-01`、`P2-02`、`P2-03` 的最小标准 AST 类型与 JSON roundtrip 单测。
3. `P3-01`、`P3-02`、`P3-03` 的最小 AST dump 入口、JSON 输出、Markdown 输出。
4. `P3-04`、`P4-01`、`P4-02`、`P4-03`、`P4-04` 的规则 id 标注、映射注册表、DOCX Render Tree 与 render dump。
5. `P0-04` 的输入 hash manifest，防止章节源文件变化后复用过期 PDF/BBL。
6. `P2-04`、`P2-05`、`P5-02`：`latex-reader` 新增标准 AST lowering API，保留旧 `Document` API，同时新增 theorem-like AST；paper3 标准 AST 已降至 `RawFallback=0`。
7. `P5-01`：Rust 主链路 `convert_dir` 已打通图片资产传递；目录转换会收集 LaTeX 根目录和相邻 `figures/` 下的 PNG/JPG/PDF，PDF 渲染缺少 `libpdfium.so` 时不再 panic，并通过 basename/PNG fallback 匹配 `\includegraphics{*.pdf}`。
8. `P5-03`：`docx-writer` 新增算法专用三列表格 writer，保留 `Algorithm N: caption` 标题，并将 I/O、行号、代码、注释列、缩进 guide 前缀写入 `document.xml`；paper3 DOCX 已确认含算法表格。
9. `P5-04`：`latex-reader` 新增 `.bbl` cite_map 降级入口，`doc-core` 在 VFS 主链路读取同名 `.bbl` 并传入；正文 citation 按 BibTeX 编号替换，并作为 `TextStyle::Superscript` 输出到 DOCX。
10. `P5-05`：`latex-reader` 新增两遍 label_map 收集和 `\ref` 替换，支持正文、caption、表格单元格、算法注释/代码、公式环境内的前向/后向引用；paper3 DOCX 已确认不再泄漏 label key 或 `\ref`/`\label` 源码。
11. `P5-06`：`table` 浮动环境会先提取内部 `tabular` / `tabular*` / `array` 主体，再进入表格 AST 降级，避免 `\caption`、`\label`、`\resizebox`、列规格和 booktabs 命令进入单元格文本；paper3 DOCX 表格首行已恢复为真实表头。
12. `P5-07`：`clean_math` 补齐常见希腊字母、数学字体命令和 `max/min` 下标词处理；theorem-like、front matter 摘要和 algorithm 注释均接入数学归一化；paper3 DOCX 已确认不再残留 `\alpha`、`\gamma`、`\lambda`、`\mathrm`、`\mathcal`、`\pm` 等 LaTeX 数学命令，且非代码文本无裸 `^`/`_`。
13. `P1-05`：完成标准规则缓存策略评估，首版不引入 SQLite，将规则/Profile 文件与 `standards.lock.json` 作为事实源；SQLite 仅作为后续可重建派生缓存。
14. `P6-01`、`P6-02`、`P6-03`：新增 verify JSON schema、schema 校验脚本、跨层 traceability 报告脚本和 paper3 一键回归脚本。`scripts/paper3_regression.sh` 已跑通报告生成链路，当前 JOS verify 质量门通过并返回 0。

当前 `cargo test -p doc-latex-reader`、`cargo test -p doc-semantic-ast`、`cargo test -p doc-docx-writer`、`cargo check -p doc-engine`、`cargo test -p doc-core --test paper3_e2e paper3_main_jos_to_docx -- --nocapture`、`cargo test -p doc-latex-reader lower_cite_uses_external_bbl_order_and_superscript_runs`、`cargo test -p doc-latex-reader lower_ref_replaces_labels_from_collect_pass`、`cargo test -p doc-latex-reader lower_table -- --nocapture`、`cargo test -p doc-latex-reader clean_math -- --nocapture`、`cargo test -p doc-latex-reader lower_theorem_like -- --nocapture`、`cargo test -p doc-latex-reader lower_front_matter_abstract_normalizes_math -- --nocapture`、`cargo test -p doc-latex-reader latex_to_text_math_function_subscript -- --nocapture`、`cargo test -p doc-latex-reader parse_algorithm_comment_normalizes_math -- --nocapture`、`cargo fmt --all --check`、`python3 scripts/validate_verify_report_schema.py examples/paper3/output/main-jos.verify.json` 均通过。`scripts/paper3_regression.sh` 可完整生成报告并通过质量门返回 0（verify: `passed=True`，字符比例 0.876，参考文献 56 条）。
