# Semantic TeX Engine OMML 公式接入开发报告（20260620-135937）
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



## 1. 本次目标

本次开发目标是完成 T6 的初版：在新的 `doc-compiler-engine` 语义路径中，把块级 LaTeX 公式转换为 Word 可识别的 OMML。

边界约束：

- 不修改 `doc-docx-writer::write_equation` 的默认行为。
- 不影响旧 `doc-core` 转换路径。
- 旧 Rust 规则引擎继续输出 JOSCode 纯文本公式，作为对照基线。
- 新语义引擎单独在 DOCX 打包后处理 `word/document.xml`。

## 2. 已实现内容

### 2.1 依赖与开关

`doc-compiler-engine` 新增对 `doc-mathml` 的直接依赖。

`CompileOptions` 新增：

```rust
enable_omml_equations: bool
```

默认值为 `true`，仅影响 `doc-compiler-engine` 语义路径。

### 2.2 报告指标

`CompileReport` 新增：

```rust
omml_equation_count: usize
omml_equation_fallback_count: usize
```

`paper3_to_docx` example 与 paper3 验证脚本会输出：

```text
omml-equations: N
omml-equation-fallbacks: N
```

### 2.3 DOCX 后处理流程

新增语义路径 OMML 后处理：

```text
DocumentGraph.document
  -> 收集 Block::Equation { is_block: true }
  -> DOCX bytes unzip
  -> 读取 word/document.xml
  -> 查找 JOSCode 公式段
  -> doc_mathml::parse_latex_math
  -> doc_mathml::to_omml
  -> 替换为 <m:oMath>
  -> 保留公式编号 w:t
  -> rezip DOCX
```

后处理顺序：

```text
DOCX pack
  -> OMML equation postprocess
  -> ReferenceGraph bookmark/hyperlink postprocess
```

这样公式段可以先变为 OMML，再作为 `\eqref` 等引用目标写入 bookmark。

### 2.4 旧路径保持不变

本次没有修改 `doc-docx-writer::write_equation`。已有测试仍验证：

```text
Block::Equation -> JOSCode plain text
```

这保证 `doc-core` 旧路径和新 `doc-compiler-engine` 语义路径继续独立存在。

## 3. 验证结果

已执行：

```bash
cargo fmt -p doc-compiler-engine
cargo check -p doc-compiler-engine
cargo test -p doc-compiler-engine omml -- --nocapture
cargo test -p doc-compiler-engine
cargo test -p doc-mathml
cargo test -p doc-docx-writer block_equation_uses_jos_code_plain_text
cargo test -p doc-core
bash scripts/compare_paper3_dual_engines.sh 15
bash scripts/build_paper3_three_docx.sh 15
```

结果：

```text
doc-compiler-engine omml: 1 passed
doc-compiler-engine: 17 passed, 1 ignored
doc-mathml: 19 passed
doc-docx-writer block equation legacy behavior: 1 passed
doc-core: 5 passed
paper3 dual engines: completed
paper3 three-docx: completed
```

已知 warning 仍为既有 warning：

- `doc-latex-reader` unused/dead_code warning。
- `doc-docx-writer` 公式 helper unused warning。
- rustfmt stable 对 nightly-only 配置项输出 warning。

## 4. paper3 最新产物

三路径验证报告：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-135859-three-docx-report.md
```

三路径 DOCX：

| 路径 | 文件 | 大小 | media |
|---|---|---:|---:|
| sh | `v15-论文稿件-jos-sh-20260620-135900.docx` | 3,079,377 bytes | 10 |
| rust-rule | `v15-论文稿件-jos-20260620-135859-rust-rule.docx` | 3,055,363 bytes | 10 |
| semantic-engine | `v15-论文稿件-jos-20260620-135859-semantic-engine-xelatex_hook.docx` | 3,057,535 bytes | 10 |

语义引擎日志指标：

```text
reference-labels: 35
reference-edges: 46
citations: 36
unresolved-references: 0
bookmarks: 25
hyperlinks: 35
omml-equations: 4
omml-equation-fallbacks: 0
backend-requested: xelatex-hook
backend-selected: xelatex-hook
profile-id: jos-paper
profile-page-setup: jos-paper3
```

双引擎对比报告：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-135835-dual-engines-comparison-report.md
```

## 5. 当前限制

- 当前仅处理块级 `Block::Equation { is_block: true }`。
- inline math 尚未转为 OMML。
- 复杂公式仍受 `doc-mathml` parser 子集限制，可能降级为 raw/text 结构。
- 公式段匹配依赖当前 writer 输出的 `JOSCode` 段落顺序。
- Word 字段型公式编号/交叉引用尚未实现，当前编号仍是普通文本。

## 6. 下一步

建议进入 T7：表格增强。

优先事项：

1. 盘点 `tabular`、`array`、`booktabs`、`multicolumn`、`multirow` 当前 AST 表达。
2. 先实现 `multicolumn -> w:gridSpan` 的最小闭环。
3. 保持旧路径可测，必要时继续把增强限定在语义路径。
4. 用 paper3 三路径脚本回归验证。
