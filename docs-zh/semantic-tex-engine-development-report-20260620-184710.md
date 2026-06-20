# Semantic TeX Engine 图片尺寸表达式开发报告（20260620-184710）

## 1. 本轮目标

完成 T8 图片尺寸表达式初版：

- 解析 `\includegraphics[width=.8\textwidth,height=...,scale=...]{...}`。
- 在语义 AST / Document Graph 中保存原始 options 和归一化尺寸信息。
- DOCX writer 按页面 profile 把尺寸表达式转换为 EMU。
- 保持无法解析时的旧默认尺寸 fallback。

## 2. 代码变更

### 2.1 语义 AST

文件：

```text
crates/semantic-ast/src/lib.rs
```

新增：

```rust
FigureSizing {
    source_options,
    width_expr,
    height_expr,
    scale_expr,
    normalized_width_ratio,
    normalized_height_ratio,
}
```

`Block::Figure` 新增：

```rust
sizing: Option<FigureSizing>
```

当前支持从 graphicx options 中抽取：

- `width=...`
- `height=...`
- `scale=...`

对 `\textwidth`、`\linewidth`、`\columnwidth`、`\textheight` 等表达式会计算相对比例。

### 2.2 LaTeX 降级

文件：

```text
crates/latex-reader/src/lower.rs
```

变更：

- `extract_includegraphics_and_caption` 改为返回图片 path 和 `FigureSizing`。
- 新增 `find_includegraphics`，读取 `\includegraphics` 的 optional arguments。
- 新增 `find_matching_bracket`，用于安全跳过 `[...]`。
- `Block::Figure.scale` 继续保留，用 `normalized_width_ratio` 作为旧 writer 兼容字段。

### 2.3 Document Graph

文件：

```text
crates/semantic-ast/src/standard.rs
```

变更：

- `FigureNode` 新增 `sizing: Option<FigureSizing>`。
- `LayoutHints.width` 会记录规范化宽度提示，例如：

```text
0.8000\textwidth
```

这让标准文档图中同时保留：

- 原始 `source_options`。
- 解析后的 width/height/scale 表达式。
- 归一化宽度提示。

### 2.4 DOCX 输出

文件：

```text
crates/docx-writer/src/serializer.rs
```

变更：

- 图片输出分支读取 `scale` 和 `sizing`。
- 新增 `calc_image_emu_for_figure`。
- 相对尺寸按 `PageSetup` 的正文宽/高换算为 EMU。
- 支持绝对单位：

```text
in
cm
mm
pt
bp
pc
```

无 sizing 且 `scale=1.0` 时仍走原 `calc_image_emu` 默认策略。

### 2.5 Runtime Hook

文件：

```text
crates/compiler-engine/src/lib.rs
```

变更：

- `XeLaTeXHookBackend` hook 新增 `\includegraphics` 包装。
- sidecar figure event 输出 `path` 和 `width_expr`。
- 与 LuaTeX sidecar 的 figure event 协议保持一致。

## 3. 影响分析

GitNexus impact 结果：

- `extract_includegraphics_and_caption`：CRITICAL，直接影响 `lower_captioned_env`，并传导到 `doc-core`、`doc-compiler-engine`。
- `lower_captioned_env`：CRITICAL，影响 figure/table/algorithm 环境降级。
- `serialize_document`：CRITICAL，影响所有 DOCX 打包入口。
- `calc_image_emu`：CRITICAL，影响图片写出链路。
- `Block` / `FigureNode` / `BlockNode`：GitNexus 图上为 LOW，但 Rust 类型变更会由编译器覆盖所有构造点。
- `XELATEX_SEMANTIC_HOOK`：LOW。

处理策略：

- 保留旧 `doc-core` 入口，不迁移到 `doc-compiler-engine`。
- 无 sizing 的图片继续走旧默认尺寸策略。
- 用 `doc-core` 和 `doc-compiler-engine` E2E 覆盖共享类型与 writer 风险。
- 用 paper3 三路径脚本验证输出结构稳定。

## 4. 验证结果

已执行：

```bash
cargo test -p doc-semantic-ast standard_document_preserves_figure_sizing -- --nocapture
cargo test -p doc-semantic-ast
cargo test -p doc-latex-reader figure -- --nocapture
cargo test -p doc-docx-writer image -- --nocapture
cargo test -p doc-docx-writer figure
cargo test -p doc-core
cargo test -p doc-compiler-engine
bash scripts/compare_paper3_dual_engines.sh 15
bash scripts/build_paper3_three_docx.sh 15
```

结果：

```text
doc-semantic-ast standard figure sizing: 1 passed
doc-semantic-ast: 13 passed
doc-latex-reader figure: 2 passed
doc-docx-writer image: 1 passed
doc-docx-writer figure: 2 passed
doc-core: 5 passed
doc-compiler-engine: 17 passed, 1 ignored
paper3 dual engines: generated
paper3 three-docx: generated
```

paper3 最新三路径输出：

| path | docx | bytes | media |
|---|---|---:|---:|
| sh | `v15-论文稿件-jos-sh-20260620-184633.docx` | 3,079,377 | 10 |
| rust-rule | `v15-论文稿件-jos-20260620-184633-rust-rule.docx` | 3,055,363 | 10 |
| semantic-engine | `v15-论文稿件-jos-20260620-184633-semantic-engine-xelatex_hook.docx` | 3,057,574 | 10 |

paper3 双引擎对比摘要：

| engine | paragraphs | tables | drawings | media | text chars |
|---|---:|---:|---:|---:|---:|
| rust-rule | 653 | 12 | 20 | 10 | 41,963 |
| semantic-engine auto | 653 | 12 | 20 | 10 | 42,535 |

semantic-engine 报告：

```text
backend-selected: xelatex-hook
reference-labels: 35
reference-edges: 46
citations: 36
unresolved-references: 0
bookmarks: 25
hyperlinks: 35
omml-equations: 4
omml-equation-fallbacks: 0
profile-id: jos-paper
profile-page-setup: jos-paper3
```

## 5. 当前边界

已支持：

- `width=.8\textwidth`
- `width=0.5\linewidth`
- `width=0.5\columnwidth`
- `height=.4\textheight`
- `scale=.5`
- `width=5cm`、`width=2in`、`width=120pt` 等绝对单位

暂未支持：

- `keepaspectratio` 的完整 graphicx 行为。
- `trim`、`clip`、`angle` 等图片变换。
- `\dimexpr` 复杂尺寸表达式。
- runtime sidecar event 反向改写 legacy `Document`，当前 DOCX 输出仍主要由规则降级的 `FigureSizing` 驱动。

## 6. 下一步

按进展计划进入 T9：

```text
兼容性分析器
```

优先处理：

- 宏包/文档类扫描。
- 自定义宏扫描。
- TikZ、minted、listings 等风险项识别。
- score、unsupported、warnings 报告结构。
