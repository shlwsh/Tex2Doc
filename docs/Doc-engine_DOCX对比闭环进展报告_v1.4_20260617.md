# Doc-engine DOCX 对比闭环进展报告 v1.4
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



日期：2026-06-17

## 1. 本轮目标

延续 v8 对比闭环，针对 `docs/verify/v8-20260617-230924-逐项对比表.md` 中稳定出现的 citation/ref 后空格差异，反向修正 Rust 转换引擎，并重新生成带版本号、时间戳和引擎标志的双版本 DOCX。

本轮同步样本：

- Rust 产物：`examples/paper3/output/to-docx/v9-论文稿件-jos-rust-20260617-231937.docx`
- sh 产物：`examples/paper3/output/to-docx/v9-论文稿件-jos-sh-20260617-231937.docx`
- v9 逐项对比表：`docs/verify/v9-20260617-231937-逐项对比表.md`
- v9 DOCX diff：`docs/verify/v9-20260617-231937-docx-compare.md`

## 2. 本轮反向修正

v8 差异中，多处源文件在 `\cite{...}` 或 `\ref{...}` 后显式保留了空格，但 Rust `strip_inline` 在跳过命令参数后的空白时直接吞掉空格，导致正文与 sh 参考输出不一致：

- v8 Rust：`LogGPT[32]利用`
- v8 sh：`LogGPT[32] 利用`
- v8 Rust：`表 1从`
- v8 sh：`表 1 从`
- v8 Rust：`表 2所示`
- v8 sh：`表 2 所示`

修正内容：

- 在 `crates/latex-reader/src/lower.rs::strip_inline` 中新增 `push_space_if_source_had_visible_gap`。
- `\cite{...}` 和 `\ref{...}` 处理完参数后，如果源文件后续显式出现空白，则保留为一个普通空格。
- 后续字符为中英文标点、闭括号或行尾时不补空格，避免把 `\cite{...},` 改成 `[N] ,`。
- 新增回归测试：
  - `strip_inline_preserves_explicit_space_after_cite`
  - `strip_inline_preserves_explicit_space_after_ref`

GitNexus 影响分析结果为 `CRITICAL`：

- 直接调用：3 个
- 影响流程：6 条
- 影响流程包括 `convert_zip`、主 lower 入口、列表、表格和 caption

因此本轮只改 inline 命令后显式空白保留规则，没有修改段落切分、表格结构、DOCX writer 或样式表。

## 3. 验证结果

已通过：

```bash
cargo test -p doc-latex-reader strip_inline_preserves_explicit_space_after_cite
cargo test -p doc-latex-reader strip_inline_preserves_explicit_space_after_ref
cargo test -p doc-latex-reader lower_cite_uses_external_bbl_order_and_superscript_runs
cargo test -p doc-latex-reader lower_table_cite_uses_external_bbl_order
cargo fmt --all --check
./scripts/paper3_regression.sh
./scripts/build_docx.sh 9
```

Rust 端到端校验：

- `passed=True`
- `tables=12`
- `images=10`
- `refs=76`
- `ratio=0.907`

sh 校验：

- `passed=True`
- `tables=12`
- `images=10`
- `refs=76`
- `ratio=0.912`

两份 v9 DOCX 均通过 ZIP/OOXML 核心部件检查，并包含 `[Content_Types].xml`、`word/document.xml`、`word/styles.xml`。

## 4. v8 与 v9 对比指标

| 指标 | v8 | v9 | 变化 |
|---|---:|---:|---:|
| 段落 Delta | -58 | -58 | 0 |
| 表格 Delta | 0 | 0 | 0 |
| Drawing Delta | 0 | 0 | 0 |
| Media Delta | 0 | 0 | 0 |
| 相同段落 | 502 | 514 | +12 |
| 修改段落 | 28 | 16 | -12 |
| 插入段落 | 128 | 128 | 0 |
| 删除段落 | 186 | 186 | 0 |
| document.xml hash | 不一致 | 不一致 | 未达标 |
| styles.xml hash | 不一致 | 不一致 | 未达标 |

说明：v9 的格式差异报告使用 `--max-diffs 220` 生成，v8 为 200 条上限，因此格式差异条数不直接做趋势判断。内容指标显示本轮修正明确减少了 12 个修改段落。

## 5. v9 中间文件

已输出到 `docs/verify`：

- `v9-20260617-231937-tex-ast.md`
- `v9-20260617-231937-tex-ast.json`
- `v9-20260617-231937-tex-body.md`
- `v9-20260617-231937-tex-syntax-summary.md`
- `v9-20260617-231937-rust-docx-body.md`
- `v9-20260617-231937-rust-docx-syntax.md`
- `v9-20260617-231937-sh-docx-body.md`
- `v9-20260617-231937-sh-docx-syntax.md`
- `v9-20260617-231937-docx-compare.md`
- `v9-20260617-231937-docx-compare.json`
- `v9-20260617-231937-逐项对比表.md`

## 6. 当前未达标项

| 项目 | 当前状态 | 下一步处理方向 |
|---|---|---|
| 段落数量 | Rust 716，sh 658，delta=-58 | 对齐英文引用、列表、算法块、公式附近的段落边界 |
| 修改段落 | 16 | 优先处理 XML 转义、数学符号文本化、摘要/英文摘要 run 差异 |
| 插入/删除段落 | 128/186 | 将列表和算法环境按 sh 参考实现归并到相同块边界 |
| `document.xml` hash | 不一致 | 内容、段落边界和 run 样式收敛后再做 OOXML 规范化级别对齐 |
| `styles.xml` hash | 不一致 | 建立 Rust writer 与 sh 模板样式映射表 |

## 7. 下一步规划

1. 修正英文引用格式分段：
   - 当前 Rust 将英文引用正文和 URL 拆为两段。
   - sh 将英文引用在 `for` 后换段，并把 URL 接入第二段。

2. 修正列表与算法块结构：
   - 当前系统工作流程、复杂度分析和算法伪代码仍是段落数量差异的主要来源。
   - 需要把 `itemize/enumerate/algorithm` 的 AST 降低规则对齐 sh 输出边界。

3. 修正数学文本化：
   - 当前仍有 `H` vs `mathcalH`、`γ/δ` vs `gamma/delta`、`Σ_i` vs `sumi` 等差异。
   - 需要拆分“公式应进 OMML”和“正文 fallback 文本化”两套映射。

4. 修正 XML 转义与特殊字符：
   - 当前 Rust 正文仍保留 `p&lt;0.001`，sh 为 `p<0.001`。
   - 需要在 DOCX 文本抽取对比层和 writer 输出层区分 XML 转义与可见文本。

5. 样式映射治理：
   - 表格文本、标题、caption、参考文献 run style 仍有大量格式差异。
   - 需要建立“视觉等价可忽略”和“必须对齐”的格式差异分类。
