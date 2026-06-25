# Semantic TeX Engine DOCX 引用链接开发报告（20260620-134723）
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



## 1. 本次目标

本次开发目标是在不影响原有 Rust 版本 DOCX 转换引擎的前提下，为新的 `doc-compiler-engine` 语义路径接入 DOCX bookmark 与内部 hyperlink 能力。

边界约束：

- `doc-core` 旧路径保持独立，不依赖 `doc-compiler-engine`。
- `doc-docx-writer` 默认输出保持不变。
- 新能力只在 `doc-compiler-engine` 打包 DOCX 后作为语义后处理启用，便于和 sh、rust-rule、semantic-engine 三路径直接对比。

## 2. 已实现内容

### 2.1 语义路径独立开关

`CompileOptions` 新增：

```rust
enable_reference_links: bool
```

默认值为 `true`，只影响 `SemanticTexEngine::compile_vfs_to_docx`。关闭该选项时，语义路径会回到原来的纯 DOCX 打包结果。

### 2.2 CompileReport 指标

`CompileReport` 新增：

```rust
bookmark_count: usize
hyperlink_count: usize
```

`paper3_to_docx` example 和 paper3 脚本日志会输出：

```text
bookmarks: N
hyperlinks: N
```

### 2.3 DOCX 后处理器

新增 `apply_reference_links_to_docx`，流程为：

```text
DOCX bytes
  -> unzip
  -> read word/document.xml
  -> ReferenceGraph labels -> bookmark plan
  -> pass 1: 给目标段落插入 w:bookmarkStart / w:bookmarkEnd
  -> pass 2: 给已解析引用插入 w:hyperlink w:anchor
  -> rezip DOCX
```

当前使用内部 anchor hyperlink，不需要改写 `document.xml.rels`。

### 2.4 关键防误判处理

本次验证中修复了两个重要问题：

1. 段落扫描不能用宽泛的 `<w:p` 匹配，否则会把 `<w:pgSz>`、`<w:pgMar>` 等页设置标签误判为段落，导致第二阶段 hyperlink 不执行。
2. figure/table 等引用不能用裸数字作为通用匹配条件，否则会误链接到文献引用编号。当前仅匹配 `图1`、`图 1`、`Figure 1`、`表1`、`Table 1` 等带语义前缀的文本。

此外，每个 `CrossReference` 当前最多链接一次，避免同一个目标编号在全文中被重复误链接。

## 3. 验证结果

已执行：

```bash
cargo fmt -p doc-compiler-engine
cargo test -p doc-compiler-engine bookmark -- --nocapture
cargo test -p doc-compiler-engine
cargo test -p doc-core
bash scripts/compare_paper3_dual_engines.sh 15
bash scripts/build_paper3_three_docx.sh 15
```

结果：

```text
doc-compiler-engine bookmark: 1 passed
doc-compiler-engine: 16 passed, 1 ignored
doc-core: 5 passed
paper3 dual engines: completed
paper3 three-docx: completed
```

已知 warning 仍为既有 warning：

- `doc-latex-reader` unused/dead_code warning。
- `doc-docx-writer` 公式相关 unused warning。
- rustfmt stable 对 nightly-only 配置项输出 warning。

## 4. paper3 最新产物

三路径验证报告：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-134613-three-docx-report.md
```

三路径 DOCX：

| 路径 | 文件 | 大小 | media |
|---|---|---:|---:|
| sh | `v15-论文稿件-jos-sh-20260620-134613.docx` | 3,079,377 bytes | 10 |
| rust-rule | `v15-论文稿件-jos-20260620-134613-rust-rule.docx` | 3,055,363 bytes | 10 |
| semantic-engine | `v15-论文稿件-jos-20260620-134613-semantic-engine-xelatex_hook.docx` | 3,056,724 bytes | 10 |

语义引擎日志指标：

```text
reference-labels: 35
reference-edges: 46
citations: 36
unresolved-references: 0
bookmarks: 21
hyperlinks: 30
backend-requested: xelatex-hook
backend-selected: xelatex-hook
profile-id: jos-paper
profile-page-setup: jos-paper3
```

双引擎对比报告：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-134551-dual-engines-comparison-report.md
```

关键短语命中在 rust-rule 与 semantic-engine 两条路径中均为 `ok`。

## 5. 当前限制

- bookmark 目标定位仍以 caption/heading/equation 样式和文本编号为启发式，不是源位置到 DOCX 段落的强映射。
- equation、heading 的 hyperlink 覆盖仍保守，避免裸数字误链接。
- 当前输出是 Word 内部 hyperlink，不是 Word 字段型交叉引用。
- hyperlink 会重写命中的文本 run，复杂 run 样式保持能力仍需后续增强。

## 6. 下一步

建议进入 T6：公式 OMML 端到端接入。

优先事项：

1. 评估 `doc-mathml` 当前 LaTeX math -> OMML 覆盖范围。
2. 决定 OMML 在 `doc-compiler-engine` 预渲染，还是在 `doc-docx-writer` 增加可选接入。
3. 增加块级公式、编号公式和 fallback diagnostics 的测试。
