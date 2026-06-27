# Semantic TeX Engine ReferenceGraph 开发报告（20260620-132709）
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



## 1. 本轮目标

本轮完成 T4 初版：在新 `doc-compiler-engine` 路径中建立 `ReferenceGraph`，结构化 `label/ref/eqref/autoref/cite`，并为后续 DOCX bookmark/hyperlink 做准备。旧 `doc-core` 路径不接入该图，继续保持独立。

## 2. 代码实现

`crates/compiler-engine/src/lib.rs` 新增：

```rust
ReferenceGraph
ReferenceLabel
CrossReference
CitationReference
UnresolvedReference
ReferenceSource
ReferenceOrigin
ReferenceTargetKind
```

`DocumentGraph` 新增：

```rust
reference_graph: ReferenceGraph
```

`CompileReport` 新增统计字段：

```rust
reference_label_count
reference_edge_count
citation_count
unresolved_reference_count
```

## 3. 采集方式

当前 ReferenceGraph 由两部分合并而来：

1. VFS TeX 源轻量扫描。
2. XeLaTeX/LuaTeX runtime semantic events。

源码扫描支持：

```text
\label{...}
\ref{...}
\eqref{...}
\autoref{...}
\cite{...}
\citep{...}
\citet{...}
\citealp{...}
\citealt{...}
\citeauthor{...}
\citeyear{...}
```

扫描器会剥离未转义 `%` 注释，避免注释中的伪 label/ref/cite 进入图。

## 4. 解析与 diagnostics

当前行为：

- label 按 key 去重。
- label kind 根据 key 前缀推断，例如 `fig:`、`tab:`、`eq:`、`alg:`、`sec:`。
- source scan 会为 figure/table/equation/algorithm/theorem/proposition/heading 分配初步编号。
- cross reference 会解析到 target kind 和 rendered number。
- 未解析引用会进入 `ReferenceGraph.unresolved_references`。
- 未解析引用会进入 `CompileReport.diagnostics`，code 为 `unresolved_reference`。

该逻辑不改变旧文本级引用替换结果，只提供额外结构化图。

## 5. paper3 验证

执行：

```bash
bash scripts/compare_paper3_dual_engines.sh 15
bash scripts/build_paper3_three_docx.sh 15
```

最新双引擎报告：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-132544-dual-engines-comparison-report.md
```

paper3 ReferenceGraph 统计：

```text
reference-labels: 35
reference-edges: 46
citations: 36
unresolved-references: 0
```

同时确认：

```text
backend-selected: xelatex-hook
profile-id: jos-paper
profile-page-setup: jos-paper3
```

## 6. 测试

已执行：

```bash
cargo test -p doc-compiler-engine reference
cargo test -p doc-compiler-engine
cargo test -p doc-compiler-engine luatex_runtime_collects_semantic_events -- --ignored --nocapture
cargo test -p doc-latex-reader ref
cargo test -p doc-core
```

结果：

```text
doc-compiler-engine reference: 2 passed
doc-compiler-engine: 14 passed, 1 ignored
doc-compiler-engine luatex ignored integration: 1 passed
doc-latex-reader ref: 9 passed
doc-core: 5 passed
```

## 7. 当前边界

- ReferenceGraph 已结构化引用关系，但尚未驱动 DOCX bookmark/hyperlink。
- source scan 的编号是轻量初版，用于 graph 可审计和 fallback；最终高保真编号仍应与 renderer/profile 编号统一。
- 未解析引用目前只产生 diagnostics，不会中断转换。

## 8. 下一步

建议进入 T5：基于 `ReferenceGraph` 输出 DOCX bookmark 和内部 hyperlink，并保持无法解析引用的纯文本 fallback。
