# Doc-engine DOCX 对比闭环进展报告 v1.6
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



生成时间：2026-06-18 06:13:10 CST  
本轮版本：v11-20260617-233749  
目标文档：`examples/paper3/latex/main-jos.tex`

## 1. 本轮结论

本轮已重新生成 Rust 引擎与 sh 流程双版本 DOCX，并同步输出 AST、DOCX 正文、DOCX 格式语法与逐项对比表到 `docs/verify`。

v11 重点修复数学文本兜底转换中的函数名空格问题：`\log` 等数学函数命令在紧跟变量或闭合符号后出现时，Rust 输出现在保留为 `O(N log N)`、`O(M log K)`、`O(N+M log M)`，不再退化为 `O(Nlog N)`、`O(Mlog K)`、`O(N+Mlog M)`。

当前尚未达到“完全通过”。v11 相比 v10 已有小幅收敛，但仍存在段落拆分/合并差异与 run 格式签名差异。

## 2. 生成物

### 2.1 DOCX 双版本

| 类型 | 文件 |
|---|---|
| Rust 引擎 | `examples/paper3/output/to-docx/v11-论文稿件-jos-rust-20260617-233749.docx` |
| sh 流程 | `examples/paper3/output/to-docx/v11-论文稿件-jos-sh-20260617-233749.docx` |
| sh 校验报告 | `examples/paper3/output/to-docx/v11-论文稿件-jos-sh-20260617-233749-docx校验报告.md` |
| sh 校验 JSON | `examples/paper3/output/to-docx/v11-论文稿件-jos-sh-20260617-233749-docx校验报告.json` |

### 2.2 核验材料

| 类型 | 文件 |
|---|---|
| TeX AST Markdown | `docs/verify/v11-20260617-233749-tex-ast.md` |
| TeX AST JSON | `docs/verify/v11-20260617-233749-tex-ast.json` |
| TeX 正文抽取 | `docs/verify/v11-20260617-233749-tex-body.md` |
| TeX 语法摘要 | `docs/verify/v11-20260617-233749-tex-syntax-summary.md` |
| Rust DOCX 正文抽取 | `docs/verify/v11-20260617-233749-rust-docx-body.md` |
| Rust DOCX 格式语法 | `docs/verify/v11-20260617-233749-rust-docx-syntax.md` |
| sh DOCX 正文抽取 | `docs/verify/v11-20260617-233749-sh-docx-body.md` |
| sh DOCX 格式语法 | `docs/verify/v11-20260617-233749-sh-docx-syntax.md` |
| DOCX 差异 Markdown | `docs/verify/v11-20260617-233749-docx-compare.md` |
| DOCX 差异 JSON | `docs/verify/v11-20260617-233749-docx-compare.json` |
| 逐项对比表 | `docs/verify/v11-20260617-233749-逐项对比表.md` |

## 3. 代码修复

修复文件：`crates/latex-reader/src/normalize.rs`

修复点：

- `strip_math_command_names` 在保留数学函数命令名时，新增前导空格判定。
- 新增 `math_function_needs_leading_space`：当前输出末尾为 ASCII 字母、数字或闭合符号 `)`、`]`、`}` 时，在 `log`、`sin`、`cos` 等保留函数名前补一个空格。
- 新增测试 `clean_math_function_keeps_space_after_variable`，覆盖 `O(N\log N)`、`O(N+M\log M)`、`O(M\log K)`。

GitNexus 影响分析结果：

| 符号 | 风险 | 直接调用者 | 影响范围 |
|---|---|---:|---|
| `clean_math` | CRITICAL | 4 | 14 个符号，10 条流程，2 个模块 |
| `strip_math_command_names` | CRITICAL | 1 | 6 个符号，10 条流程，2 个模块 |

风险处理：本轮只调整数学函数名的局部空格归一策略，并通过定向单测、相关历史单测和 paper3 回归验证控制影响面。

## 4. 验证结果

### 4.1 命令验证

| 验证项 | 结果 |
|---|---|
| `cargo test -p doc-latex-reader clean_math_function_keeps_space_after_variable` | 通过 |
| `cargo test -p doc-latex-reader clean_math_common_greek_and_fonts` | 通过 |
| `cargo test -p doc-latex-reader latex_to_text_math_function_subscript` | 通过 |
| `cargo fmt --all --check` | 通过 |
| `./scripts/paper3_regression.sh` | 通过，`passed=True tables=12 images=10 refs=76 ratio=0.906` |
| `./scripts/build_docx.sh 11` | 通过，`passed=True tables=12 images=10 refs=76 ratio=0.912` |
| v11 DOCX ZIP/OOXML 包结构检查 | Rust/sh 均通过 |

### 4.2 v10/v11 指标对比

| 指标 | v10 | v11 | 变化 |
|---|---:|---:|---:|
| `paragraph_delta` | -58 | -58 | 0 |
| `table_delta` | 0 | 0 | 0 |
| `drawing_delta` | 0 | 0 | 0 |
| `media_delta` | 0 | 0 | 0 |
| `equal_paragraphs` | 520 | 521 | +1 |
| `modified_paragraphs` | 12 | 12 | 0 |
| `inserted_paragraphs` | 126 | 125 | -1 |
| `deleted_paragraphs` | 184 | 183 | -1 |
| `format_changed_paragraphs` | 220 | 220 | 0 |
| `document_xml_equal` | false | false | 未通过 |
| `styles_xml_equal` | false | false | 未通过 |

### 4.3 关键文本回归

`docs/verify/v11-20260617-233749-*.md` 中未检出以下错误形态：

- `O(Nlog`
- `O(Mlog`
- `O(N+Mlog`

对应文本在 TeX AST、Rust DOCX 与 sh DOCX 抽取结果中均显示为：

- `O(N log N)`
- `O(M log K)`
- `O(M log M)`
- `O(N+M log M)`

## 5. 剩余主要差异

1. 段落拆分/合并仍不一致：Rust 抽取非空段落 702 个，sh 抽取非空段落 648 个，DOCX diff 汇总仍为 `paragraph_delta=-58`。
2. 列表环境仍存在结构差异：部分 Rust 输出为 `ListBullet` 独立段落，而 sh 流程存在被并入正文段落并带 `itemize` 文本痕迹的情况。后续要以 TeX AST 的 `List`/`ListItem` 标准节点为基线，统一两侧映射判定。
3. 格式 run 签名仍有大量差异：标题、关键词、复杂度段落等位置存在 bold/run 切分不一致。当前 `format_changed_paragraphs` 达到 diff 输出上限 220，需要继续缩小。
4. `document.xml` 与 `styles.xml` 仍不等价：虽然表格、图片、媒体数量已经对齐，但底层 XML 的段落结构、run 分割、样式定义仍有差异。

## 6. 下一步规划

1. 以 `docs/verify/v11-20260617-233749-逐项对比表.md` 为输入，优先处理列表环境差异：统一 `itemize`/`enumerate` 的 AST 标准节点、段落边界、bullet/numbering 映射。
2. 建立 run 合并/拆分规范：对连续相同样式 run 做规范化合并，避免无语义差异导致格式签名不一致。
3. 对标题、作者、关键词、摘要等 JOS 前置区建立专用映射表，减少 bold 与段落样式的重复表达。
4. 扩展 DOCX diff 的格式差异分类，把 run 差异拆为“真实格式变更”和“仅 run 切分差异”。
5. 继续执行闭环：修复映射算法后重新生成 v12 双版本 DOCX，输出同名规则的 `docs/verify/v12-<timestamp>-*` 材料，并要求 `equal_paragraphs` 继续增加、`inserted/deleted/format_changed` 持续下降。
