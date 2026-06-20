# Semantic TeX Engine 表格 span 渲染开发报告（20260620-182504）

## 1. 本轮目标

在不影响旧 Rust DOCX 转换路径入口的前提下，补齐语义路径与共享底层组件中的表格跨列/基础跨行能力：

- `\multicolumn{n}{spec}{text}` 保持语义化 `colspan`，并修正 DOCX 单元格宽度。
- 基础 `\multirow{n}{width}{text}` 降级为 `rowspan=n`。
- 后续空占位单元格降级为 `rowspan=0`，对应 DOCX `vMerge continue`。
- DOCX 输出 `w:gridSpan` 与 `w:vMerge restart/continue`。

本轮没有修改 `doc-core::convert_sync`、`convert_zip`、`convert_dir` 的入口，也没有让旧路径依赖 `doc-compiler-engine`。

## 2. 代码变更

### 2.1 LaTeX 降级

文件：

```text
crates/latex-reader/src/lower.rs
```

变更：

- `lower_table` 增加 `active_rowspans` 状态表，用于跨行单元格的后续行占位。
- `\multicolumn` 分支现在会推进逻辑列索引，避免后续 `multirow` 占位列定位错误。
- 新增 `parse_multirow`，支持基础形式：

```latex
\multirow{2}{*}{Merged}
```

- 首行生成：

```text
TableCell { colspan: 1, rowspan: 2, ... }
```

- 后续空占位生成：

```text
TableCell { colspan: 1, rowspan: 0, runs: [] }
```

### 2.2 DOCX 输出

文件：

```text
crates/docx-writer/src/serializer.rs
```

变更：

- 新增 `table_row_logical_columns`，按 `colspan` 求逻辑列数，而不是简单使用 `row.cells.len()`。
- 跨列单元格的 `w:tcW` 按 `colspan` 放大。
- 已有 `gridSpan` 输出继续保留。
- 新增 `vMerge` 输出：

```xml
<w:vMerge w:val="restart"/>
<w:vMerge w:val="continue"/>
```

### 2.3 单元测试

新增/覆盖：

- `lower_multirow`：验证 `\multirow{2}{*}{Merged}` 降级为首行 `rowspan=2` 与后续空占位 `rowspan=0`。
- `table_colspan_and_rowspan_emit_grid_span_and_vmerge`：验证 DOCX XML 中存在 `gridSpan`、跨列宽度、`vMerge restart`、`vMerge continue`。

## 3. 影响分析

根据 GitNexus impact 结果，本轮涉及共享底层函数，风险级别为 CRITICAL：

- `lower_table`：影响 `lower_environment`、`lower_captioned_env`，并通过 `doc-latex-reader` 进入旧规则路径和新语义路径。
- `write_table`：影响 `serialize_document` 与所有 DOCX 打包入口。
- `TableCell`：风险高，但本轮未修改结构，只复用已有 `colspan` / `rowspan` 字段。

处理策略：

- 不改 AST 结构，避免扩大兼容面。
- 不改 `doc-core` 入口。
- 用表格单测、`doc-core` E2E、`doc-compiler-engine` E2E 和 paper3 三路径脚本覆盖回归。

## 4. 验证结果

已执行：

```bash
cargo test -p doc-latex-reader multi -- --nocapture
cargo test -p doc-latex-reader table
cargo test -p doc-docx-writer table_colspan_and_rowspan_emit_grid_span_and_vmerge -- --nocapture
cargo test -p doc-docx-writer table
cargo test -p doc-core
cargo test -p doc-compiler-engine
bash scripts/compare_paper3_dual_engines.sh 15
bash scripts/build_paper3_three_docx.sh 15
```

结果：

```text
doc-latex-reader multi: 7 passed
doc-latex-reader table: 7 unit tests plus snapshot_table passed
doc-docx-writer table_colspan_and_rowspan_emit_grid_span_and_vmerge: 1 passed
doc-docx-writer table: 6 passed
doc-core: 5 passed
doc-compiler-engine: 17 passed, 1 ignored
paper3 dual engines: generated
paper3 three-docx: generated
```

paper3 最新三路径输出：

| path | docx | bytes | media |
|---|---|---:|---:|
| sh | `v15-论文稿件-jos-sh-20260620-182407.docx` | 3,079,377 | 10 |
| rust-rule | `v15-论文稿件-jos-20260620-182406-rust-rule.docx` | 3,055,363 | 10 |
| semantic-engine | `v15-论文稿件-jos-20260620-182406-semantic-engine-xelatex_hook.docx` | 3,057,541 | 10 |

paper3 双引擎对比摘要：

| engine | paragraphs | tables | drawings | text chars |
|---|---:|---:|---:|---:|
| rust-rule | 653 | 12 | 20 | 41,963 |
| semantic-engine auto | 653 | 12 | 20 | 42,535 |

semantic-engine 报告：

```text
backend-selected: xelatex-hook
reference-labels: 35
reference-edges: 46
citations: 36
unresolved-references: 0
bookmarks: 25
hyperlinks: 35
omml-equations: 4
omml-equation-fallbacks: 0
profile-id: jos-paper
profile-page-setup: jos-paper3
```

## 5. 当前边界

已支持：

- 基础 `\multicolumn`。
- 基础正整数 `\multirow{n}{width}{text}`。
- DOCX `gridSpan`。
- DOCX `vMerge restart/continue`。
- 按逻辑列数推断表格网格宽度。

暂未支持：

- 负数或复杂表达式形式的 `multirow`。
- `\multirow` 与 `\multicolumn` 深度组合冲突消解。
- 表格内多段落结构的完整保真。
- 表格内脚注的 Word footnote 结构化输出。

## 6. 下一步

按进展计划进入 T8：

```text
图片尺寸表达式
```

优先处理：

- `\includegraphics[width=.8\textwidth]`
- `\includegraphics[width=0.5\linewidth]`
- `\includegraphics[scale=...]`
- 无法计算时保持当前默认尺寸 fallback。
