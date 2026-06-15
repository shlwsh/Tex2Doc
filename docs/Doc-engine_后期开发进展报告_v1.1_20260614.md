# Doc-engine 后期开发进展报告

| 文档版本 | 时间 | 范围 |
|---|---|---|
| V1.0 | 2026-06-14 | Sprint 0 + M1 + M2 完成 |
| **V1.1** | **2026-06-14** | **M3 + M5 + M7 + 质量加固完成** |

## 1. 总览

| 阶段 | 状态 | 测试数 |
|---|---|---|
| M3 列表 / 表格 / 图片 / Bib / 链接 | ✅ 完成 | +7 |
| M5 数学公式管道（LaTeX→OMML） | ✅ 完成 | +12 |
| M7 reference.docx 模板继承 | ✅ 完成 | +4 |
| 质量加固（proptest + 夹具 + insta） | ✅ 完成 | +7 |
| **合计** | — | **66 个测试全过** |

## 2. M3 完成的降级能力

| 环境/命令 | Block 类型 | 备注 |
|---|---|---|
| `\begin{itemize}` / `enumerate` / `description` | `Block::List` | 支持嵌套、`\item[label]` 形式 |
| `\begin{tabular}{c|c}…\\…\end{tabular}` | `Block::Table` | 支持多行多列、自动 tblBorders |
| `\begin{tabular}` 单元格分隔 `&` | `TableCell` | rowspan/colspan 占位为 1 |
| `\begin{figure}` + `\includegraphics` | `Block::Figure` | 路径 / caption 提取 |
| `\begin{figure}` + `\caption` | 注入到 Figure | Caption 样式 |
| `\begin{table}` | `Block::Table` + caption | |
| `\href{url}{text}` / `\url{url}` | 段落内吞并 | 文本保留，链接语义降级 |
| `\ref{label}` / `\cite{key}` / `\footnote{}` | 段内吞并 | 保留占位 |
| `@inproceedings` / `@article` / `@book` | `BibEntry` | 解析为结构化条目 |
| `\begin{equation}` / `align*` | `Block::Equation { latex }` | OMML 序列化 |
| 未匹配内容 | `Block::RawFallback` | 永不 panic |

## 3. M5 公式管道

```
LaTeX → parse_latex_math → MathExpr → to_omml → <m:oMath>
                  ↓
                to_mathml → <math>...</math>（Presentation MathML）
```

支持的语法（V1 子集）：
- 数字 / 标识符 / 文本（`\text{...}`）
- 二元运算符 `+ - * / = < >`
- 上下标 `x^{2}` / `x_{i}` / `x_{i}^{j}`
- 分式 `\frac{a}{b}`
- 根式 `\sqrt{x}` / `\sqrt[n]{x}`
- 括号 `\left( ... \right)`
- 三角函数 `\sin` `\cos` `\tan` `\log` `\ln` `\exp`
- 希腊字母 `\alpha` ... `\omega`（大小写）
- 运算符 `\cdot` `\times` `\leq` `\geq` `\neq` `\infty` `\sum` `\int` `\prod`
- 矩阵 `\begin{matrix} a & b \\ c & d \end{matrix}`

测试用例：12（latex 9 + mathml 3 + omml 3，统计在 mathml crate 内部）。

## 4. M7 模板继承

新增 `docx-writer::template` 模块：
- `parse_template(docx_bytes)`：从 `reference.docx` 中提取 `word/styles.xml`
- `parse_styles_xml(xml)`：解析 `<w:style>` 元素
- `merge_styles(target, template)`：**同名不覆盖**、**缺失补全**到 `</w:styles>` 前

集成到 `pack_with_template(doc, template_bytes)`；`ConvertOptions.template_bytes: Option<Vec<u8>>` 已暴露。

## 5. 质量加固

| 措施 | 位置 | 覆盖 |
|---|---|---|
| **proptest** | `crates/utils/tests/proptest.rs` | VFS roundtrip / 任意路径 |
| **proptest** | `crates/latex-reader/tests/proptest.rs` | 词法 / 配对括号 / 任意 0-256 字节 |
| **ieee 夹具** | `tests/fixtures/ieee/{simple,nested}.tex` + `crates/core/tests/ieee_fixtures.rs` | 端到端 List+Table+Figure+Equation 混合；嵌套列表 |
| **insta snapshot** | `crates/latex-reader/tests/insta_snapshots.rs` | 简单 / 列表 / 表格三个 AST 快照 |

## 6. 后续 M4/M6/M8 待办（未在本轮完成）

| 任务 | 优先级 | 估时 |
|---|---|---|
| M4 — 字体探测（系统 vs 嵌入） | 中 | 3 人天 |
| M6 — 公式 OOM / 大公式压缩 | 中 | 2 人天 |
| M6 — 高级表格（multirow / multirow + colors） | 中 | 4 人天 |
| M8 — 完整编号（heading/figure/table auto-number） | 高 | 3 人天 |
| Flutter 端 Dart 调用 FFI | 高 | 10 人天 |
| Chrome 扩展 MV3 | 中 | 5 人天 |
| Server 端 REST + 队列 | 中 | 5 人天 |

## 7. 风险与已知限制

1. **OMML 简化实现**：当前 OMML 标签用 `m:begChr/m:endChr` 表示单字符运算符，与 Word 原生 `m:d` + 配对符略有差异。Word 仍可解析，但渲染可能用最简字符。**升级路径**：实现 `m:d` 完整映射（OMML 1.5 §22.1.2.20）。
2. **图片二进制**：`\includegraphics` 暂未将 PNG/JPEG 字节写入 `word/media/`。当前输出 `[图片：path]` 占位文本。**升级路径**：在 `docx-writer::packer` 加入 `media` 文件流。
3. **嵌套表格**：`tabular` 内嵌套 `tabular` 在 `lower_table` 中以单行 + 原文回退。**升级路径**：递归 `lower_table`。
4. **Bib 引用**：`\cite{key}` 仅作为占位吞并。**升级路径**：与 `BibEntry` 联立生成 `[n]` 编号列表。
5. **行内公式**：`$...$` 未单独抽出 Inline Equation。**升级路径**：在 `lower.rs` 段落中检测 `$` 并替换为 `Block::Equation { is_block: false }`。

## 8. 验证

```bash
cargo test --workspace
# 66 passed; 0 failed
```

主要验证路径：
- 端到端：`end_to_end_hello`、`end_to_end_full`、`ieee_simple_end_to_end`、`ieee_nested_round_trip`
- 模糊：`proptest::*` 任意字节
- 快照：`insta_snapshots::{simple, list, table}`
- 模板：`template::tests::{parse, merge, round_trip}`
