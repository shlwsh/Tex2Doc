# Semantic TeX Engine 兼容性分析器开发报告（20260620-190143）
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



## 1. 本轮目标

完成 T9 兼容性分析器内置初版：

- 编译前扫描 TeX 工程的文档类、宏包、自定义宏和高风险环境。
- 输出 `score`、`unsupported`、`warnings`。
- 将兼容性结果写入 `CompileReport` 和 paper3 验证报告。
- 保持旧 `doc-core` / Rust rule DOCX 转换路径不受影响。

## 2. 代码变更

### 2.1 编译报告

文件：

```text
crates/compiler-engine/src/lib.rs
```

新增：

```rust
CompatibilityReport
CompatibilityIssue
CompileReport.compatibility
CompileStage::CompatibilityAnalyze
```

当前 `CompatibilityReport` 字段：

```text
score
scanned_files
document_classes
packages
custom_macro_count
unsupported
warnings
```

### 2.2 兼容性扫描

扫描范围：

```text
.tex
.sty
.cls
.ltx
```

已识别：

- `\documentclass`
- `\usepackage`
- `\RequirePackage`
- `\newcommand`
- `\renewcommand`
- `\providecommand`
- `\DeclareRobustCommand`
- `\NewDocumentCommand`
- `\RenewDocumentCommand`
- `\def`
- `tikzpicture`
- `minted`
- `lstlisting`

当前规则：

- `tikz` / `pgf` / `pgfplots` / `circuitikz`：unsupported。
- `pstricks`：unsupported。
- `minted`：unsupported。
- `beamer` / `standalone`：unsupported。
- `listings`：warning。
- `biblatex`：warning。
- `longtable` / `tabularx` / `tabulary`：warning。
- `algorithm2e` / `algorithmicx` / `algpseudocode`：warning。
- profile 外文档类：warning。
- 自定义宏：warning。

评分策略为轻量启发式：

```text
100
- 18 * unsupported_count
- 6 * warnings_count
- min(custom_macro_count * 2, 12)
```

最低为 0。

### 2.3 编译阶段接入

`compile_vfs_to_graph` 在 `SourceMount` 后执行：

```text
CompatibilityAnalyze
```

并将 unsupported / warnings 同步写入 `EngineDiagnostic`：

```text
compatibility_unsupported
compatibility_warning
```

该阶段只读取 VFS 并生成报告，不改变后续语义采集、Document Graph、DOCX 渲染结果。

### 2.4 paper3 示例与脚本

文件：

```text
crates/compiler-engine/examples/paper3_to_docx.rs
scripts/compare_paper3_dual_engines.sh
scripts/build_paper3_three_docx.sh
```

新增输出：

```text
compatibility-score
compatibility-unsupported
compatibility-warnings
compatibility-custom-macros
```

双引擎对比报告和三路径验证报告现在会直接收录 compatibility 摘要。

## 3. 影响分析

GitNexus impact 结果：

- `CompileReport` struct / impl：LOW。
- `compile_vfs_to_graph`：LOW。
- `CompileStage`：LOW。
- `EngineDiagnostic`：LOW。
- `paper3_to_docx.rs::run`：LOW。
- shell 脚本内报告函数未被 GitNexus 索引，影响分析返回 UNKNOWN / 0 impacted；本轮仅扩展报告摘录字段，不改变生成 DOCX 的命令。

处理策略：

- 不修改 `doc-core::convert_sync/convert_zip/convert_dir`。
- 不修改旧 `doc-docx-writer` 默认行为。
- 兼容性分析只接入新 `doc-compiler-engine` 路径。
- 独立 `crates/compatibility-analyzer` 暂不拆分，先稳定数据结构和 paper3 验证输出。

## 4. 验证结果

已执行：

```bash
cargo test -p doc-compiler-engine compatibility -- --nocapture
cargo test -p doc-compiler-engine
bash scripts/compare_paper3_dual_engines.sh 15
bash scripts/build_paper3_three_docx.sh 15
```

结果：

```text
doc-compiler-engine compatibility: 2 passed
doc-compiler-engine: 19 passed, 1 ignored
paper3 dual engines: generated
paper3 three-docx: generated
```

已知 warning：

- `doc-latex-reader` 仍有既存 unused/dead_code warning。
- `doc-docx-writer` 仍有既存 unused helper warning。

这些 warning 与本轮兼容性分析器无关。

## 5. paper3 最新输出

双引擎对比：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-190053-dual-engines-comparison-report.md
```

摘要：

| engine | docx | bytes | media | paragraphs | tables | drawings | text chars |
|---|---|---:|---:|---:|---:|---:|---:|
| rust-rule | `v15-论文稿件-jos-20260620-190053-dual-engines-rust-rule.docx` | 3,055,363 | 10 | 653 | 12 | 20 | 41,963 |
| semantic-engine auto | `v15-论文稿件-jos-20260620-190053-dual-engines-semantic-engine-auto.docx` | 3,057,574 | 10 | 653 | 12 | 20 | 42,535 |

三路径验证：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-190109-three-docx-report.md
```

摘要：

| path | docx | bytes | media |
|---|---|---:|---:|
| sh | `v15-论文稿件-jos-sh-20260620-190110.docx` | 3,079,377 | 10 |
| rust-rule | `v15-论文稿件-jos-20260620-190109-rust-rule.docx` | 3,055,363 | 10 |
| semantic-engine | `v15-论文稿件-jos-20260620-190109-semantic-engine-xelatex_hook.docx` | 3,057,574 | 10 |

semantic-engine 报告：

```text
compatibility-score: 76
compatibility-unsupported: 0
compatibility-warnings: 2
compatibility-custom-macros: 46
reference-labels: 35
reference-edges: 46
citations: 36
unresolved-references: 0
bookmarks: 25
hyperlinks: 35
omml-equations: 4
omml-equation-fallbacks: 0
backend-selected: xelatex-hook
profile-id: jos-paper
profile-page-setup: jos-paper3
```

## 6. 当前边界

已支持：

- 编译前静态兼容性预检。
- 兼容性分数和 diagnostics 输出。
- paper3 验证报告直接展示兼容性摘要。
- TikZ/minted/listings 等常见风险识别。

暂未支持：

- 独立 `crates/compatibility-analyzer`。
- 可配置规则表。
- 基于 profile 的可配置阈值。
- 宏展开后的兼容性分析。
- AI-assisted unknown macro inference。

## 7. 后续建议

下一步进入 T10：

```text
Semantic Collector trait
```

重点是收敛 `RuleBasedBackend`、`XeLaTeXHookBackend`、`LuaTeXNodeBackend` 的公共输出模型，再评估拆分 `crates/semantic-collector`。
