# Doc-engine DOCX 对比闭环进展报告 v1.5

日期：2026-06-17

## 1. 本轮目标

延续 v9 对比闭环，针对 Rust DOCX 可见文本中的 XML 双重转义差异，反向修正 DOCX writer，并重新生成带版本号、时间戳和引擎标志的双版本 DOCX。

本轮同步样本：

- Rust 产物：`examples/paper3/output/to-docx/v10-论文稿件-jos-rust-20260617-232900.docx`
- sh 产物：`examples/paper3/output/to-docx/v10-论文稿件-jos-sh-20260617-232900.docx`
- v10 逐项对比表：`docs/verify/v10-20260617-232900-逐项对比表.md`
- v10 DOCX diff：`docs/verify/v10-20260617-232900-docx-compare.md`

## 2. 本轮反向修正

v9 差异中，Rust 输出存在可见文本 `p&lt;0.001`，而 sh 输出为 `p<0.001`。进一步检查发现：

- 标准 AST 与 render dump 中均为正确文本 `p<0.001`。
- Rust DOCX 的 `word/document.xml` 中为 `p&amp;lt;0.001`。
- sh DOCX 的 `word/document.xml` 中为标准 OOXML 转义 `p&lt;0.001`。

根因是 `crates/docx-writer/src/serializer.rs::write_run` 在写入 `BytesText` 前先手工调用 `xml_escape(&run.text)`，而 quick-xml 对 `BytesText` 会再次执行 XML 文本转义，导致 `<` 被写成 `&amp;lt;`。

修正内容：

- `write_run` 中移除对 run text 的预转义。
- 保留 quick-xml 对文本节点执行标准 XML 转义。
- 新增回归测试 `text_nodes_are_not_double_escaped`，验证 `p<0.001 & x>0` 在 XML 中写为 `p&lt;0.001 &amp; x&gt;0`，且不出现 `&amp;lt;` / `&amp;gt;`。

GitNexus 影响分析结果为 `CRITICAL`：

- 直接调用：1 个
- 影响流程：8 条
- 影响范围包括正文、公式、算法、表格、front matter 和 DOCX 打包流程

因此本轮只修改文本节点转义职责边界，没有改段落切分、样式映射、数学转换或表格布局。

## 3. 验证结果

已通过：

```bash
cargo test -p doc-docx-writer text_nodes_are_not_double_escaped
cargo test -p doc-docx-writer paragraph_with_inline_citation_keeps_body_style
cargo fmt --all --check
./scripts/paper3_regression.sh
./scripts/build_docx.sh 10
```

Rust 端到端校验：

- `passed=True`
- `tables=12`
- `images=10`
- `refs=76`
- `ratio=0.906`

sh 校验：

- `passed=True`
- `tables=12`
- `images=10`
- `refs=76`
- `ratio=0.912`

两份 v10 DOCX 均通过 ZIP/OOXML 核心部件检查，并包含 `[Content_Types].xml`、`word/document.xml`、`word/styles.xml`。Rust v10 的 `word/document.xml` 已确认不再包含 `p&amp;lt;`。

## 4. v9 与 v10 对比指标

| 指标 | v9 | v10 | 变化 |
|---|---:|---:|---:|
| 段落 Delta | -58 | -58 | 0 |
| 表格 Delta | 0 | 0 | 0 |
| Drawing Delta | 0 | 0 | 0 |
| Media Delta | 0 | 0 | 0 |
| 相同段落 | 514 | 520 | +6 |
| 修改段落 | 16 | 12 | -4 |
| 插入段落 | 128 | 126 | -2 |
| 删除段落 | 186 | 184 | -2 |
| document.xml hash | 不一致 | 不一致 | 未达标 |
| styles.xml hash | 不一致 | 不一致 | 未达标 |

说明：v9 与 v10 的格式差异报告均使用 `--max-diffs 220` 生成，格式差异条数仍达到报告上限，暂不作为质量收敛指标。

## 5. v10 中间文件

已输出到 `docs/verify`：

- `v10-20260617-232900-tex-ast.md`
- `v10-20260617-232900-tex-ast.json`
- `v10-20260617-232900-tex-body.md`
- `v10-20260617-232900-tex-syntax-summary.md`
- `v10-20260617-232900-rust-docx-body.md`
- `v10-20260617-232900-rust-docx-syntax.md`
- `v10-20260617-232900-sh-docx-body.md`
- `v10-20260617-232900-sh-docx-syntax.md`
- `v10-20260617-232900-docx-compare.md`
- `v10-20260617-232900-docx-compare.json`
- `v10-20260617-232900-逐项对比表.md`

## 6. 当前未达标项

| 项目 | 当前状态 | 下一步处理方向 |
|---|---|---|
| 段落数量 | Rust 716，sh 658，delta=-58 | 对齐英文引用、列表、算法块、公式附近的段落边界 |
| 修改段落 | 12 | 优先处理 `≈` vs `approx`、`vs.` vs `vs.\`、数学 fallback 文本化 |
| 插入/删除段落 | 126/184 | 将列表和算法环境按 sh 参考实现归并到相同块边界 |
| `document.xml` hash | 不一致 | 内容、段落边界和 run 样式收敛后再做 OOXML 规范化级别对齐 |
| `styles.xml` hash | 不一致 | 建立 Rust writer 与 sh 模板样式映射表 |

## 7. 下一步规划

1. 修正数学文本化差异：
   - 当前仍有 `d≈378` vs `dapprox378`、`γ/δ` vs `gamma/delta`、`H` vs `mathcalH` 等差异。
   - 需要判断哪些应进入 OMML，哪些应按 sh 参考输出做文本 fallback。

2. 修正英文引用格式分段：
   - Rust 与 sh 对英文引用格式的换段位置仍不同。
   - 需要建立 front matter 专用换段规则。

3. 修正列表与算法块结构：
   - 工作流程、复杂度分析和算法伪代码仍是段落数量差异的主要来源。
   - 需要把 `itemize/enumerate/algorithm` 的 AST 降低规则继续对齐 sh 输出。

4. 样式映射治理：
   - 表格文本、标题、caption、参考文献 run style 仍有大量格式差异。
   - 需要建立可配置样式映射表，并把视觉等价差异从必须修复差异中分离。
