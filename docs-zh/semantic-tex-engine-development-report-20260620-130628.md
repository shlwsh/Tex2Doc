# Semantic TeX Engine 双引擎对比脚本开发报告（20260620-130628）

## 1. 本轮目标

本轮完成 T2：为 paper3 增加旧 Rust 规则引擎与新 Semantic Engine 的独立对比脚本。该脚本只负责验证与报告，不改变任一路径的实现。

## 2. 新增脚本

新增：

```bash
scripts/compare_paper3_dual_engines.sh
```

用法：

```bash
bash scripts/compare_paper3_dual_engines.sh 15
```

环境变量：

```text
SEMANTIC_BACKEND=auto|xelatex-hook|luatex-node|rule-based
STRICT_SEMANTIC=1|0
KEY_PHRASES='phrase1|phrase2|...'
```

默认行为：

- 重新打包 `examples/paper3/latex` 和 `examples/paper3/figures` 为临时 upload zip。
- 旧 Rust 路径调用 `target/release/doc-engine convert`。
- 新语义路径调用 `cargo run -p doc-compiler-engine --example paper3_to_docx`。
- 默认 `SEMANTIC_BACKEND=auto`，paper3 上自动选择 `xelatex-hook`。
- 输出目录固定为 `examples/paper3/output/to-docx`。

## 3. 报告能力

脚本会使用 Python 标准库直接读取 DOCX zip 和 `word/document.xml`，输出：

- DOCX 文件大小。
- zip part 数。
- `word/media/*` 数量。
- paragraph/table/drawing 数量。
- document.xml 文本字符数。
- semantic backend 选择报告。
- 关键短语命中表。
- rust-rule 与 semantic-engine 的 document.xml 文本摘要。
- 两份纯文本文件。
- unified diff 文件。

## 4. paper3 验证结果

执行命令：

```bash
bash scripts/compare_paper3_dual_engines.sh 15
```

输出报告：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-130548-dual-engines-comparison-report.md
```

DOCX 产物：

| engine | 文件 | 大小 | media | paragraphs | tables | drawings | text chars |
|---|---|---:|---:|---:|---:|---:|---:|
| rust-rule | `v15-论文稿件-jos-20260620-130548-dual-engines-rust-rule.docx` | 3,055,363 bytes | 10 | 653 | 12 | 20 | 41,963 |
| semantic-engine auto | `v15-论文稿件-jos-20260620-130548-dual-engines-semantic-engine-auto.docx` | 3,055,688 bytes | 10 | 653 | 12 | 20 | 42,744 |

Semantic backend 报告：

```text
backend-requested: auto
backend-selected: xelatex-hook
backend-reason: Auto selected XeLaTeXHookBackend: detected ctex class/package, xeCJK package; found /usr/bin/xelatex; xelatex-hook available: found /usr/bin/xelatex
```

关键短语验证：

| phrase | rust-rule | semantic-engine |
|---|---:|---:|
| 基于动态关注清单 | 3 | 3 |
| 微服务日志 | 3 | 3 |
| Dynamic Attention List | 1 | 1 |
| DASM | 28 | 28 |
| Loki | 35 | 35 |
| DSB-Lite | 13 | 13 |
| 系统总体设计 | 2 | 2 |
| 实验与分析 | 2 | 2 |

文本差异：

```text
diff_file: v15-论文稿件-jos-20260620-130548-dual-engines-document-text.diff
hunks: 1
changed_lines: 110
```

## 5. 验证结论

- 两条路径独立执行，旧 Rust 路径仍走 `doc-engine convert`，新语义路径走 `doc-compiler-engine` example。
- 两条路径输出的 media、paragraph、table、drawing 数一致。
- 关键短语均在两条路径中命中。
- 差异报告已落地，后续可以用于 ProfileSpec、引用图、公式和表格增强的回归对照。

## 6. 下一步

建议进入 T3：实现 `ProfileSpec`，把 JOS、中文学术、医学期刊规则内聚到新语义引擎 profile 层，并继续保持 `doc-core` 旧路径独立。
