# Doc-engine DOCX 对比闭环进展报告 v1.2

日期：2026-06-17

## 1. 本轮目标

延续 v6 对比闭环，针对 `docs/verify/v6-20260617-225340-逐项对比表.md` 中暴露的稳定正文差异，反向修正 Rust 转换引擎，并重新生成带版本号、时间戳和引擎标志的双版本 DOCX。

本轮同步样本：

- Rust 产物：`examples/paper3/output/to-docx/v7-论文稿件-jos-rust-20260617-230015.docx`
- sh 产物：`examples/paper3/output/to-docx/v7-论文稿件-jos-sh-20260617-230015.docx`
- v7 逐项对比表：`docs/verify/v7-20260617-230015-逐项对比表.md`
- v7 DOCX diff：`docs/verify/v7-20260617-230015-docx-compare.md`

## 2. 本轮反向修正

v6 差异中多处出现 Rust 输出单位格式错误：

- Rust：`180,s`、`400,ms`、`30,s`
- sh：`180 s`、`400 ms`、`30 s`

根因是 `crates/latex-reader/src/lower.rs::strip_inline` 在进入 `normalize::latex_to_text` 前只保护了 `\%`、`\$`、`\&`、`\#`、`\_`、`\{`、`\}` 等单字符转义，没有保护 LaTeX 窄空格 `\,`。后续命令兜底会丢掉反斜杠并留下逗号，导致 `180\,s` 变成 `180,s`。

修正内容：

- 将 `\,` 加入 `strip_inline` 单字符转义白名单。
- 保留为 `\,` 交给 `normalize::latex_to_text`，由其既有规则转为普通空格。
- 新增回归测试 `lower_thin_space_unit_does_not_leave_comma`。

GitNexus 影响分析结果为 `CRITICAL`：

- 直接调用：3 个
- 影响流程：6 条
- 涉及主转换入口、列表、表格、caption、`convert_zip`

因此本轮仅做窄范围白名单扩展，未改段落、表格、列表或数学解析流程。

## 3. 验证结果

已通过：

```bash
cargo test -p doc-latex-reader lower_thin_space_unit_does_not_leave_comma
cargo test -p doc-latex-reader lower_ref_replaces_labels_from_collect_pass
cargo test -p doc-latex-reader lower_inline_math_and_cite_together
./scripts/paper3_regression.sh
./scripts/build_docx.sh 7
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

两份 v7 DOCX 均通过 ZIP 容器完整性检查，并包含 `[Content_Types].xml`、`word/document.xml`、`word/styles.xml`。

## 4. v6 与 v7 对比指标

| 指标 | v6 | v7 | 变化 |
|---|---:|---:|---:|
| 段落 Delta | -58 | -58 | 0 |
| 表格 Delta | 0 | 0 | 0 |
| Drawing Delta | 0 | 0 | 0 |
| Media Delta | 0 | 0 | 0 |
| 相同段落 | 473 | 497 | +24 |
| 修改段落 | 31 | 31 | 0 |
| 插入段落 | 154 | 130 | -24 |
| 删除段落 | 212 | 188 | -24 |
| 格式差异段落 | 200 | 200 | 0 |
| document.xml hash | 不一致 | 不一致 | 未达标 |
| styles.xml hash | 不一致 | 不一致 | 未达标 |

结论：`\,` 单位空格修正显著降低了段落级内容差异。表格、图片、媒体数量继续保持一致，但整体尚未达标。

## 5. v7 中间文件

已输出到 `docs/verify`：

- `v7-20260617-230015-tex-ast.md`
- `v7-20260617-230015-tex-ast.json`
- `v7-20260617-230015-tex-body.md`
- `v7-20260617-230015-tex-syntax-summary.md`
- `v7-20260617-230015-rust-docx-body.md`
- `v7-20260617-230015-rust-docx-syntax.md`
- `v7-20260617-230015-sh-docx-body.md`
- `v7-20260617-230015-sh-docx-syntax.md`
- `v7-20260617-230015-docx-compare.md`
- `v7-20260617-230015-docx-compare.json`
- `v7-20260617-230015-逐项对比表.md`

## 6. 下一步规划

1. 修正英文引用格式分段：
   - 当前 Rust 将英文引用正文与 URL 拆成两段。
   - sh 将英文引用正文在 `for` 后换段，并把 URL 接在第二段。

2. 修正引用编号映射残差：
   - 当前仍存在 `OTel 尾部采样[1]` vs `[18]`、`eBPF 日志[1-2]` vs `[38,54]` 等差异。
   - 需要检查表格/list/caption 内引用是否没有使用 `.bbl` 编号映射。

3. 修正列表与算法块结构：
   - Rust 的列表项与算法行仍拆分过细。
   - sh 更偏向编号文本合并到正文段落或单个算法代码块。

4. 修正格式映射：
   - 当前 `format_changed_paragraphs=200`，需要继续拆解段落样式、run 样式、上下标、bold/italic 和标题样式。

5. 自动化证据包：
   - 将当前手动生成的 TeX AST、DOCX 正文/结构、DOCX diff、逐项对比表固化为脚本，减少人工重复并避免表格字段误写。
