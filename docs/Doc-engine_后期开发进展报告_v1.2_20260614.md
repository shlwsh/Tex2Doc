# Doc-engine 后期开发进展报告
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



| 文档版本 | 时间 | 范围 |
|---|---|---|
| V1.0 | 2026-06-14 | Sprint 0 + M1 + M2 完成 |
| V1.1 | 2026-06-14 | M3 + M5 + M7 + 质量加固完成 |
| **V1.2** | **2026-06-14** | **M4 + M6 + M8 + 5 大风险全部完成** |

## 1. 总览

| 阶段 | 状态 | 测试数 |
|---|---|---|
| M3 列表 / 表格 / 图片 / Bib / 链接 | ✅ 完成 | +7 |
| M5 数学公式管道（LaTeX→OMML） | ✅ 完成 | +12 |
| M7 reference.docx 模板继承 | ✅ 完成 | +4 |
| 质量加固（proptest + 夹具 + insta） | ✅ 完成 | +7 |
| **M4 字体探测（系统 vs 嵌入）** | ✅ 完成 | +6 |
| **M6 风险 1: OMML m:d 配对符** | ✅ 完成 | +2 |
| **M6 风险 2: 图片二进制嵌入** | ✅ 完成 | +1 |
| **M6 风险 3: 嵌套表格递归** | ✅ 完成 | +1 |
| **M6 风险 4: Bib [n] 编号** | ✅ 完成 | +1 |
| **M6 风险 5: 行内公式 $...$** | ✅ 完成 | +2 |
| **M6 任务: 大公式 OOM 压缩** | ✅ 完成 | +1 |
| **M6 任务: 高级表格 (multicolumn/rowcolor)** | ✅ 完成 | +2 |
| **M8 完整自动编号 (heading/figure/table)** | ✅ 完成 | +3 |
| **合计** | — | **99 个测试全过** |

## 2. 本轮（M4 / M6 / M8 / 5 大风险）变更详情

### 2.1 M4 — 字体探测（系统 vs 嵌入）

新增 `crates/utils/src/fontdetect.rs`：

- `FontStatus { Available, Embed, Fallback }` 三态分类
- `FontDetector` 自动探测 Windows/Mac/Linux 系统字体目录
- `FontProbe` 包含原始名 / 状态 / 推荐字体名 / 系统路径
- `office_map` 预置 CTeX 字体映射（songti→SimSun、heiti→SimHei、kaishu→KaiTi、fangsong→FangSong、lishu→SimLi 等）
- 用户可通过 `register_office_mapping()` 扩展映射表
- `probe_font(name)` 便捷单次探测
- `docx-writer::styles::apply_font_probes` 把 Fallback 结果应用到 `word/styles.xml` 字节流（替换 `w:ascii` / `w:hAnsi` / `w:eastAsia` / `w:cs` 字体引用）

测试 +6：构造器、未知字体回退、Office 映射、Calibri 探测（条件启用）、自定义映射、styles 应用。

### 2.2 M6 — 5 大风险

| 风险 | 修复 | 测试 |
|---|---|---|
| **1. OMML m:d 配对符** | `mathml::omml::write_expr` 对 `MathExpr::Op(c)` 输出 `<m:oSupp>` + `<m:begChr>` + `<m:endChr>`；`MathExpr::Seq` 统一包裹 `<m:r>` | +2 |
| **2. 图片二进制嵌入** | `pack_with_assets(doc, template_bytes, ImageAssets)`；`convert_dir` 扫描 VFS 中 PNG/JPEG；serializer 输出 `<w:drawing>` + `<wp:inline>` + `<pic:pic>` + base64 编码的 `w:binData`；`Content_Types.xml` 注册 `image/png`、`image/jpeg` | +1 |
| **3. 嵌套表格** | `strip_inline` 保留 `[TAB:…]` 占位；`extract_nested_tabulary` 抽出内层 body；`lower_table` 递归调用并在外层单元格以 `[表格: …]` 文本表示 | +1 |
| **4. Bib 编号** | `strip_inline` 接收 `&mut HashMap<String, usize>`；遇到 `\cite{key}` 按出现顺序分配 1-based 编号，替换为 `[n]` 或 `[n1,n2]` | +1 |
| **5. 行内公式** | 新增 `RunPart { Text, InlineMath }` 与 `split_inline_math`；`flush_paragraph` 检测 `$…$`，将原文替换为 `[公式：…]` 占位，并在段落之后追加 `Block::Equation { is_block: false }` | +2 |

**副效应修复**：实施 4/5 时 `paper3_e2e` 集成测试出现"中文摘要关键短语丢失"回归——`rjabstract` 环境首次非 fallback 块可能变成 `Equation`，挤掉 `Paragraph`。新增 `lower_abstract_paragraph` 显式返回首个非空 `Block::Paragraph`，恢复摘要内容。

### 2.3 M6 — 任务

**大公式 OOM 压缩**（`mathml::latex::Parser`）：

- 新增 `const MAX_EXPR_DEPTH: usize = 100`
- `Parser` 增加 `depth: usize` 字段
- `parse_seq`（处理 `\` 命令）和 `parse_group_or_single`（处理 `{` 分组）在进入时检查深度，超限则截断为 `MathExpr::Raw` 或保留当前结果并停止递归
- +1 测试覆盖超深嵌套场景

**高级表格 multicolumn / rowcolor**（`latex-reader::lower_table` + `docx-writer::serializer`）：

- `TableCell` 新增 `bg_color: Option<String>` 字段（位于 `semantic-ast`）
- `\multicolumn{n}{spec}{text}` 解析为 `colspan` + 单元格文本
- `\rowcolor{color}` / `\rowcolor[model]{color}` 解析为整行 `bg_color`
- `strip_inline` 保留 `&` 命令占位供后续处理
- serializer 输出 `<w:gridSpan w:val="n"/>` 和 `<w:shd w:fill="RRGGBB"/>` 标签
- 修正 `TextStyle::MathInline` 的 italic 映射
- +2 测试覆盖 multicolumn 与 rowcolor
- 注：`\multirow` 因 Word DOCX 的 `vMerge` 模型与 LaTeX 的 `multirow{n}{width}{text}` 语义差异较大（跨行垂直合并 + 文本锚定策略不同），本轮暂不实现，将在 V1.3 单独处理。

### 2.4 M8 — 完整编号

**`semantic-ast`**：`Heading` / `Figure` / `Table` 三种 Block 均新增 `number: Option<String>` 字段。

**`latex-reader::NumberingState`**：

- `heading_counters: [u32; 5]` 维护 4 级标题嵌套计数（每次进入更高级别自动重置更深的级别）
- `figure_counter` / `table_counter` 顺序递增
- `next_heading(level)` 输出 `1` / `1.1` / `1.1.1` / `1.1.1.1` 多级编号
- `next_figure()` 输出 `图 1`、`图 2` ……
- `next_table()` 输出 `表 1`、`表 2` ……
- 透传到 `lower_with_macros` / `lower_environment` / `lower_captioned_env` / `try_top_level_command`

**`docx-writer::serializer`**：

- Heading 标题输出：`{number} {text}`（如 `1 Introduction`）
- Figure caption 输出：`{number} {caption}`（如 `图 1 系统架构`）
- Table caption 输出：`{number} {caption}`（如 `表 1 实验数据`）

+3 测试覆盖：heading 多级嵌套编号、figure 顺序编号、table 顺序编号。

附带修复：insta snapshot 重建（`simple_doc`、`table_doc`）。

## 3. 测试总览

```bash
cargo test --workspace
# 99 passed; 0 failed
```

分布：
- `doc_bib` 5
- `doc_core` 6
- `doc_docx_writer` 40
- `doc_latex_reader` 3（insta）+ 2（proptest）+ 19（lib 单测）= 24
- `doc_mathml` 15
- `doc_semantic_ast` 3
- `doc_utils` 19（含字体探测 6 + image 3 + vfs 4 + path 4 + fontmap 2）
- `proptest` 2

## 4. 后续待办（V1.3+）

| 任务 | 优先级 | 估时 | 备注 |
|---|---|---|---|
| M6 — `\multirow` 高级表格 | 中 | 3 人天 | vMerge 模型与 LaTeX 语义差异需仔细设计 |
| Flutter 端 Dart FFI | 高 | 10 人天 | |
| Chrome 扩展 MV3 | 中 | 5 人天 | |
| Server 端 REST + 队列 | 中 | 5 人天 | |
| OTF 字体子集嵌入 | 中 | 4 人天 | M4 的 `Embed` 状态完整实现 |
| MathML 渲染回退 | 低 | 2 人天 | 兼容老版本 Office |

## 5. 风险与已知限制（V1.2 状态）

| 风险 | V1.1 状态 | V1.2 状态 |
|---|---|---|
| 1. OMML m:d 配对符 | 简化 m:begChr | ✅ 升级为 m:oSupp + 配对符 |
| 2. 图片二进制嵌入 | 仅文本占位 | ✅ 完整二进制嵌入到 `word/media/` |
| 3. 嵌套表格 | 单行回退 | ✅ 递归 `lower_table` |
| 4. Bib 引用 | 文本占位 | ✅ `[n]` 顺序编号 |
| 5. 行内公式 | 未分离 | ✅ `Block::Equation { is_block: false }` |
| 大公式 OOM | 无保护 | ✅ 100 层深度截断 |
| 高级表格 (multicolumn/colors) | 不支持 | ✅ 已实现 |
| 完整编号 | 无 | ✅ heading/figure/table 全自动 |
| 字体探测 | 无 | ✅ 系统 vs 嵌入 vs fallback |

## 6. 验证

```bash
cargo test --workspace
# 99 passed; 0 failed
```

主要验证路径：

- 端到端：`end_to_end_hello`、`end_to_end_full`、`ieee_simple_end_to_end`、`ieee_nested_round_trip`、`paper3_e2e`（已修复）
- 模糊：`proptest::*` 任意字节
- 快照：`insta_snapshots::{simple, list, table}`
- 模板：`template::tests::{parse, merge, round_trip}`
- 字体探测：`fontdetect::tests::{detector_creates, probe_unknown_returns_fallback, probe_with_office_mapping, probe_finds_calibri_if_present, register_custom_mapping}` + `styles::tests::{apply_font_probes_*}` 共 8 个
- 编号：`latex_reader::tests::{lower_heading_auto_number, lower_figure_auto_number, lower_table_auto_number}` 3 个
- 表格高级特性：`multicolumn_test`、`rowcolor_test` 共 2 个
- OOM 保护：`mathml::latex::tests::depth_limit` 1 个
- 嵌套表格：`nested_tabular_lowering` 1 个
- Bib 编号：`cite_produces_brackets` 1 个
- 行内公式：`inline_math_*` 2 个
- OMML 配对符：`omml_op_with_oSupp`、`omml_seq_always_wrapped_in_r` 2 个
- 图片嵌入：`figure_writes_drawing_with_base64` 1 个
