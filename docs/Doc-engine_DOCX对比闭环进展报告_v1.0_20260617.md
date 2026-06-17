# Doc-engine DOCX 对比闭环进展报告 v1.0

日期：2026-06-17

## 1. 本轮目标

面向 `examples/paper3/latex/main-jos.tex`，在 Rust 引擎与 `scripts/build_docx.sh`/sh 产物之间建立可重复的 `.docx` 内容与格式对比闭环，并用差异结果反向修正转换引擎。

对比样本：

- Rust 产物：`examples/paper3/output/main-jos-rust.docx`
- sh 产物：`examples/paper3/output/to-docx/v4-论文稿件-jos-sh-20260617-213016.docx`
- DOCX diff Markdown：`examples/paper3/output/main-jos-rust-vs-sh.docx-diff.md`
- DOCX diff JSON：`examples/paper3/output/main-jos-rust-vs-sh.docx-diff.json`

## 2. 已完成实现

### 2.1 DOCX 内容与格式对比器

新增 `doc-quality::docx_diff`，实现双轨对比：

- 语义轨：解析 `word/document.xml`，抽取段落、段落样式、run 格式、表格数、drawing 数、media 文件数。
- 内容轨：对段落归一化文本执行 LCS 对齐，输出插入、删除、相同段落统计。
- 格式轨：对相同文本段落比较 paragraph style、drawing 标志、run 样式签名。
- OOXML 轨：规范化 `document.xml` 与 `styles.xml`，剥离 `rsid*` 噪音后输出稳定 hash。

新增 CLI：

```bash
cargo run -p doc-engine -- docx-diff \
  --left examples/paper3/output/main-jos-rust.docx \
  --right examples/paper3/output/to-docx/v4-论文稿件-jos-sh-20260617-213016.docx \
  --format md \
  --out examples/paper3/output/main-jos-rust-vs-sh.docx-diff.md
```

支持 `--format md/json`、`--max-diffs`、`--no-xml-hash`。

### 2.2 根据差异反向修复的首项优化

DOCX diff 显示大量正文段落差异来自引用编号格式：

- Rust 原输出：`[1,2,3]`
- sh 输出：`[1-3]`

已修正 `crates/latex-reader/src/lower.rs::strip_inline` 中 `\cite{...}` 的编号输出逻辑，复用已有 `crate::normalize::compress_numbers`，将连续引用压缩为范围。

## 3. 当前对比指标

引用压缩前：

- 段落数：Rust 716，sh 658，Delta -58
- 表格数：12 vs 12
- 图片 drawing 数：10 vs 10
- media 文件数：10 vs 10
- 相同段落：463
- 新增段落：195
- 删除段落：253
- 格式变更段落：120

引用压缩后：

- 段落数：Rust 716，sh 658，Delta -58
- 表格数：12 vs 12
- 图片 drawing 数：10 vs 10
- media 文件数：10 vs 10
- 相同段落：470
- 新增段落：188
- 删除段落：246
- 格式变更段落：120

结论：引用压缩已减少 14 条段落级内容差异；表格、图片、media 数量与 sh 产物保持一致。

继续完善后（新增 modified 折叠 + 中文单位 CJK 内部空格归一）：

- 段落数：Rust 716，sh 658，Delta -58
- 表格数：12 vs 12
- 图片 drawing 数：10 vs 10
- media 文件数：10 vs 10
- 相同段落：471
- 近似修改段落：31
- 新增段落：156
- 删除段落：214
- 格式变更段落：120

结论：diff 报告已能把高相似度 delete/insert 折叠为 `modified`，前置单位 `山西 太原` 与 `山西太原` 的真实输出差异已消除，后续可优先处理英文引用分行、公式编号格式和算法块拆分问题。

## 4. 验证结果

已通过：

```bash
cargo fmt --all --check
cargo test -p doc-quality docx_diff -- --nocapture
cargo check -p doc-engine
cargo test -p doc-latex-reader lower_cite -- --nocapture
cargo test -p doc-latex-reader lower_inline_math_and_cite_together -- --nocapture
./scripts/paper3_regression.sh
```

`paper3_regression.sh` 最新结果：

- `passed=True`
- `tables=12`
- `images=10`
- `refs=76`
- `ratio=0.908`

## 5. 主要残差

1. 前置信息存在少量文本归一化差异：
   - `山西 太原` vs `山西太原`
   - 英文引用格式换行位置不同
   - 摘要段落仍有不可见或公式/符号归一化差异

2. 列表与算法块映射差异较大：
   - Rust 中 `JOSCode=82`，sh 中 `JOSCode=4`
   - 算法行在 Rust 中拆分更细，并存在底层公式 fallback 残留
   - 部分 `enumerate/itemize` 在 sh 中落为 `JOSBody` 编号文本，Rust 中仍为 `ListNumber/ListBullet`

3. 文末参考文献与作者简介仍有差异：
   - Rust `JOSReference=76`，sh `JOSReference=78`
   - 中文参考和作者简介区域仍需按 sh 的段落样式与正文保真度继续收敛

4. run 格式差异仍较多：
   - 当前格式差异段落为 120
   - 主要来自标题/关键词/引用标签的 bold run、Heading run style、正文中 `\textbf` 保留策略差异

## 6. 下一步规划

1. 改进 DOCX diff 的“近似匹配”能力：
   - 将 delete+insert 的相邻段落合并为 `modified`
   - 增加字符相似度，降低因单个空格或公式编号造成的误判
   - 输出 top-N 差异按影响面排序

2. 继续反向修转换引擎：
   - 前置信息：单位地址空格归一、英文引用格式按 sh 分行
   - 算法块：减少 `JOSCode` 过度拆行，清理公式 fallback 残留
   - 列表：为 JOS 论文正文列表提供“编号文本 + JOSBody”模式
   - 文末：作者简介与中文参考文献统一到 `JOSReference`/`JOSReferenceHeading`

3. 将 DOCX diff 纳入回归脚本：
   - `paper3_regression.sh` 后自动生成 `main-jos-rust-vs-sh.docx-diff.{md,json}`
   - 设置阶段性阈值：表格/图片必须相等，相同段落数单调提升，格式差异数单调下降
