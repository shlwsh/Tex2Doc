# Doc-engine DOCX 对比闭环进展报告 v1.3
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



日期：2026-06-17

## 1. 本轮目标

延续 v7 对比闭环，针对 `docs/verify/v7-20260617-230015-逐项对比表.md` 中仍存在的引用编号残差，反向修正 Rust 转换引擎，并重新生成带版本号、时间戳和引擎标志的双版本 DOCX。

本轮同步样本：

- Rust 产物：`examples/paper3/output/to-docx/v8-论文稿件-jos-rust-20260617-230924.docx`
- sh 产物：`examples/paper3/output/to-docx/v8-论文稿件-jos-sh-20260617-230924.docx`
- v8 逐项对比表：`docs/verify/v8-20260617-230924-逐项对比表.md`
- v8 DOCX diff：`docs/verify/v8-20260617-230924-docx-compare.md`

## 2. 本轮反向修正

v7 差异中，表格、列表和 caption 路径仍有引用编号没有复用 `.bbl` 顺序映射：

- Rust：`OTel 尾部采样[1]`
- sh：`OTel 尾部采样[18]`
- Rust：`eBPF 日志[1-2]`
- sh：`eBPF 日志[38,54]`

根因是主正文链路已维护 `cite_numbers`，但 `lower_environment` 下游的表格、列表、caption 和列表项正文转换中，部分路径仍创建或使用局部空映射，导致内部 `\cite{...}` 按局部首次出现顺序重新编号。

修正内容：

- 将 `cite_numbers` 和 `label_map` 显式传递到 `lower_environment`。
- 将引用映射继续传入 `lower_list`、`lower_description_with_label`、`lower_item_body`、`lower_table`、`lower_captioned_env` 和 `normalize_caption`。
- 表格单元格、列表项正文、caption 内文本统一复用主链路 `.bbl` 引用编号映射。
- 新增回归测试 `lower_table_cite_uses_external_bbl_order`，覆盖表格内引用应输出外部编号 `[18]` 而不是局部 `[1]`。

GitNexus 影响分析结果：

| 符号 | 风险 |
|---|---|
| `collect_label_map` | CRITICAL |
| `replace_refs_in_latex` | CRITICAL |
| `strip_inline` | CRITICAL |
| `lower_environment` | HIGH |
| `lower_list` | CRITICAL |
| `lower_item_body` | CRITICAL |
| `lower_table` | CRITICAL |

因此本轮只做映射上下文传递和局部回归测试，不改 DOCX writer、样式表、数学公式渲染或段落合并策略。

## 3. 验证结果

已通过：

```bash
cargo test -p doc-latex-reader lower_table_cite_uses_external_bbl_order
cargo test -p doc-latex-reader lower_thin_space_unit_does_not_leave_comma
cargo test -p doc-latex-reader lower_cite_uses_external_bbl_order_and_superscript_runs
cargo test -p doc-latex-reader lower_table_auto_number
cargo fmt --all --check
./scripts/paper3_regression.sh
./scripts/build_docx.sh 8
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

两份 v8 DOCX 均通过 ZIP/OOXML 核心部件检查，并包含 `[Content_Types].xml`、`word/document.xml`、`word/styles.xml`。

## 4. v7 与 v8 对比指标

| 指标 | v7 | v8 | 变化 |
|---|---:|---:|---:|
| 段落 Delta | -58 | -58 | 0 |
| 表格 Delta | 0 | 0 | 0 |
| Drawing Delta | 0 | 0 | 0 |
| Media Delta | 0 | 0 | 0 |
| 相同段落 | 497 | 502 | +5 |
| 修改段落 | 31 | 28 | -3 |
| 插入段落 | 130 | 128 | -2 |
| 删除段落 | 188 | 186 | -2 |
| 格式差异段落 | 200 | 200 | 0 |
| document.xml hash | 不一致 | 不一致 | 未达标 |
| styles.xml hash | 不一致 | 不一致 | 未达标 |

结论：表格、列表和 caption 内引用编号映射已并入主链路，旧的 `[1]` vs `[18]`、`[1-2]` vs `[38,54]` 类差异已消除。段落相同数继续上升，但段落数量、内容差异、run 级格式差异和规范化 XML hash 仍未达标。

## 5. v8 中间文件

已输出到 `docs/verify`：

- `v8-20260617-230924-tex-ast.md`
- `v8-20260617-230924-tex-ast.json`
- `v8-20260617-230924-tex-body.md`
- `v8-20260617-230924-tex-syntax-summary.md`
- `v8-20260617-230924-rust-docx-body.md`
- `v8-20260617-230924-rust-docx-syntax.md`
- `v8-20260617-230924-sh-docx-body.md`
- `v8-20260617-230924-sh-docx-syntax.md`
- `v8-20260617-230924-docx-compare.md`
- `v8-20260617-230924-docx-compare.json`
- `v8-20260617-230924-逐项对比表.md`

## 6. 当前未达标项

| 项目 | 当前状态 | 下一步处理方向 |
|---|---|---|
| 段落数量 | Rust 716，sh 658，delta=-58 | 对齐英文引用、列表、算法块、公式附近的段落合并策略 |
| 修改段落 | 28 | 优先处理引用后空格、`<` 转义、表号空格、数学符号文本化 |
| 插入/删除段落 | 128/186 | 将列表和算法环境按 sh 参考实现归并到相同段落边界 |
| 格式差异段落 | 200 | 对齐表格文本样式、标题 run 样式、caption italic/bold、上下标 run |
| `document.xml` hash | 不一致 | 在内容和段落边界收敛后再做 OOXML 规范化级别对齐 |
| `styles.xml` hash | 不一致 | 需要建立 Rust writer 与 sh 模板样式映射表 |

## 7. 下一步规划

1. 修正引用后空格规则：
   - 当前仍有 `LogGPT[32]利用` vs `LogGPT[32] 利用` 等差异。
   - 需要在 superscript citation run 与后续中英文正文之间建立语言敏感空格策略。

2. 修正英文引用格式分段：
   - 当前 Rust 与 sh 对英文引用格式的换段位置仍不同。
   - 需要把前置元信息区域纳入专门模板规则，而不是普通段落规则。

3. 修正列表与算法块结构：
   - 当前复杂列表、复杂度说明和算法伪代码仍是段落数量差异的主要来源。
   - 需要把 `itemize/enumerate/algorithm` 的 AST 节点降低到更接近 sh 参考输出的块边界。

4. 修正公式与数学文本化：
   - 当前仍有 `mathcalH`、`gamma`、`delta`、`sum_i` 等文本化差异。
   - 需要增强数学 AST 到 OMML/文本 fallback 的双轨映射。

5. 建立样式映射表：
   - 当前表格单元格存在 `JOSTableText` run style 与 sh 空 style 的差异。
   - 需要把样式差异拆为“视觉等价可忽略”和“必须对齐”两类，减少伪差异。

6. 自动化闭环脚本：
   - 将 TeX AST、DOCX body、DOCX syntax、DOCX diff、逐项对比表和指标趋势合并为单一命令。
   - 后续每轮输出统一前缀：`vN-YYYYMMDD-HHMMSS-*`，并自动写入进展报告草稿。
