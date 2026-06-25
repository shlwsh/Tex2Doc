# Doc-engine DOCX 对比闭环进展报告 v1.1
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



日期：2026-06-17

## 1. 本轮目标

根据 Rust 引擎与 `scripts/build_docx.sh` 的双版本 DOCX 差异，继续反向修正 LaTeX → DOCX 转换映射，并输出带版本号、时间戳、引擎标志的双版本文稿与可核验中间文件。

本轮同步样本：

- Rust 产物：`examples/paper3/output/to-docx/v6-论文稿件-jos-rust-20260617-225340.docx`
- sh 产物：`examples/paper3/output/to-docx/v6-论文稿件-jos-sh-20260617-225340.docx`
- v6 核验证据目录：`docs/verify/`
- v6 逐项对比表：`docs/verify/v6-20260617-225340-逐项对比表.md`

## 2. 已完成工作

### 2.1 双版本重新转换输出

Rust 路径：

```bash
./scripts/paper3_regression.sh
```

结果：

- `passed=True`
- `tables=12`
- `images=10`
- `refs=76`
- `ratio=0.909`

sh 路径：

```bash
./scripts/build_docx.sh 6
```

结果：

- `passed=True`
- `tables=12`
- `images=10`
- `refs=76`
- `ratio=0.912`

两份 DOCX 均通过 ZIP 容器完整性检查，并包含 `[Content_Types].xml`、`word/document.xml`、`word/styles.xml`。

### 2.2 中间文件证据包

已按版本号与时间戳输出：

- TeX AST：`docs/verify/v6-20260617-225340-tex-ast.md`
- TeX AST JSON：`docs/verify/v6-20260617-225340-tex-ast.json`
- TeX 正文抽取：`docs/verify/v6-20260617-225340-tex-body.md`
- TeX 结构摘要：`docs/verify/v6-20260617-225340-tex-syntax-summary.md`
- Rust DOCX 正文抽取：`docs/verify/v6-20260617-225340-rust-docx-body.md`
- Rust DOCX 结构抽取：`docs/verify/v6-20260617-225340-rust-docx-syntax.md`
- sh DOCX 正文抽取：`docs/verify/v6-20260617-225340-sh-docx-body.md`
- sh DOCX 结构抽取：`docs/verify/v6-20260617-225340-sh-docx-syntax.md`
- DOCX 差异报告：`docs/verify/v6-20260617-225340-docx-compare.md`
- DOCX 差异 JSON：`docs/verify/v6-20260617-225340-docx-compare.json`
- 逐项对比表：`docs/verify/v6-20260617-225340-逐项对比表.md`

## 3. 本轮反向修正

根据 v5 对比中多处 `式 1` vs `式 (1)` 的正文差异，修正 `crates/latex-reader/src/lower.rs::collect_label_map` 的 equation/align/gather 标签映射：

- 原映射：`\ref{eq:x}` → `1`
- 新映射：`\ref{eq:x}` → `(1)`
- 正文效果：`式~\ref{eq:x}` → `式 (1)`

GitNexus 影响分析结果为 `CRITICAL`，影响转换主链路；本轮仅做窄范围语义映射调整，未改解析流程。

已通过相关单测：

```bash
cargo test -p doc-latex-reader lower_ref_replaces_labels_from_collect_pass
cargo test -p doc-latex-reader lower_inline_math_and_cite_together
cargo test -p doc-latex-reader lower_cite
```

## 4. 对比指标

| 指标 | v5 | v6 | 变化 |
|---|---:|---:|---:|
| 段落 Delta | -58 | -58 | 0 |
| 表格 Delta | 0 | 0 | 0 |
| Drawing Delta | 0 | 0 | 0 |
| Media Delta | 0 | 0 | 0 |
| 相同段落 | 471 | 473 | +2 |
| 修改段落 | 31 | 31 | 0 |
| 插入段落 | 156 | 154 | -2 |
| 删除段落 | 214 | 212 | -2 |
| 格式差异段落 | 120 | 200 | 统计口径变更后需继续拆解 |
| document.xml hash | 不一致 | 不一致 | 未达标 |
| styles.xml hash | 不一致 | 不一致 | 未达标 |

结论：公式引用括号映射已进入 Rust 新产物，并使段落级内容差异小幅收敛；表格、图片、媒体数量继续保持一致。整体仍未达标。

## 5. 当前未达标项

1. 段落边界仍未对齐：
   - Rust 段落数多于 sh。
   - 主要集中在英文引用、算法块、列表、参考文献与作者简介区域。

2. 格式映射仍未完全一致：
   - 段落样式和 run 样式差异仍高。
   - 标题、关键词、引用标签、粗体、上下标与编号样式需要继续细化。

3. 样式表与底层 OOXML 仍不一致：
   - `document.xml` 规范化 hash 不一致。
   - `styles.xml` 规范化 hash 不一致。

## 6. 下一步规划

1. 优先处理英文引用段落边界：
   - 对比 sh 的英文作者/基金/引用区域换行规则。
   - 在 Rust front matter 或 bibliography 映射中实现稳定分段。

2. 处理算法块和列表模式：
   - 减少 Rust `JOSCode` 过度拆行。
   - 将 JOS 正文列表向 sh 的“编号文本 + JOSBody”模式收敛。

3. 继续完善格式映射：
   - 将标题、引用编号、关键词、中文参考文献和作者简介区域的 run 样式纳入映射表。
   - 以 `docs/verify/v6-20260617-225340-逐项对比表.md` 为基线逐项回归。

4. 自动化闭环：
   - 将 TeX AST、DOCX 正文/结构抽取、DOCX diff 和逐项对比表纳入固定脚本。
   - 阈值要求先设为：表格/图片/媒体必须一致；相同段落单调上升；插入/删除/修改段落单调下降。
