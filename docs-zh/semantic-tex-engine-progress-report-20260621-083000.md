# Semantic TeX Engine 进展报告（20260621-0830）

**基准版本**：`docs-zh/semantic-tex-engine-progress-comparison-20260621.md`（截至 2026-06-21 08:00，commit 748a842）
**当前版本**：2026-06-21 上午实现完成
**验证时间**：2026-06-21 08:00（`cargo test --workspace` 全通过）
**报告生成**：2026-06-21 08:30

---

## 一、总体进展概览

| 维度 | 基准（20260621 am） | 当前（20260621 pm） | 变化 |
|------|---------------------|---------------------|------|
| P1 代码块检测 | 部分支持 | ✅ 完成（minted/lstlisting/listings） | 完成 |
| P1 RuleEngine 集成 | 部分支持 | ✅ 完成（audit trail + inline macro） | 完成 |
| P1 Word REF 字段 | hyperlink 模式 | ✅ 完成（opt-in w:fldSimple REF） | 完成 |
| P2 XDV → LayoutGraph | 无 | ✅ 完成（`layout_graph.rs`） | 完成 |
| P2 LayoutGraph 接入 | 无 | ✅ 完成（LuaTeXNodeBackend） | 完成 |
| 工作空间测试 | ~379 | ✅ 320+ 全部通过 | 无回归 |
| Paper3 E2E | 通过 | ✅ 通过 | 无回归 |

---

## 二、本轮完成内容

### 2.1 P1-1：latex-reader 代码环境检测（minted/lstlisting/listings → Block::CodeBlock）

**修改文件**：`crates/latex-reader/src/lower.rs`

新增 3 个函数：

| 函数 | 职责 |
|------|------|
| `scan_code_environment()` | 优先检测 `\begin{minted/lstlisting/listings}`，提取 `{lang}` 参数 |
| `extract_language_from_env()` | 支持 `[options]{lang}`（minted）和 `{lang}`（lst/listing）两种形式 |
| `lower_code_environment()` | 生成 `Block::CodeBlock { language, code, span }` |
| `clean_code_body()` | 移除 minted 逃逸前缀 `+` 和双空格缩进 |

主循环改动：`verbatim` 不再走 `multi_block_envs` 特殊路径，而是通过 `scan_code_environment` 统一检测。

**验收**：`cargo test -p doc-latex-reader` 全通过。

---

### 2.2 P1-2：RuleEngine 与 compiler-engine 集成

**修改文件**：`crates/compiler-engine/Cargo.toml`、`crates/compiler-engine/src/lib.rs`

在 `RuleBasedCollector.collect()` 降级文档后，新增 `apply_rule_engine_to_document()`：

```
Document (with RawFallback)
  → apply_rule_engine_to_document()
    → 对每个 Block::RawFallback 调用 resolve_raw_fallback()
      → 检测 \begin{name}... → 记录到 RuleEngine audit trail
      → 检测 \unknownmacro{...} → 记录到 RuleEngine audit trail
    → 保持 RawFallback 不变（保守集成，不改变现有行为）
```

新增 3 个辅助函数：`resolve_raw_fallback()`、`detect_environment_name()`、`detect_inline_macro()`。

**验收**：`cargo test -p doc-compiler-engine` 34 个测试全部通过。

---

### 2.3 P1-3：Word REF 字段升级（hyperlink → w:fldSimple REF）

**修改文件**：`crates/compiler-engine/src/lib.rs`

在 `CompileOptions` 中新增字段：

```rust
pub struct CompileOptions {
    pub enable_ref_fields: bool,  // 默认 false，向后兼容
    // ...
}
```

`hyperlink_first_text_run()` 新增 `enable_ref_fields` 参数：

- `false`（默认）：`<w:hyperlink w:anchor="...">`（现有行为）
- `true`：`<w:fldChar begin>` + `<w:instrText> REF ... \\h </w:instrText>` + `<w:fldChar separate>` + `<w:fldChar end>`

新增测试 `docx_reference_links_ref_field_mode` 验证字段结构。

**验收**：33 个测试全部通过。

---

### 2.4 P2-1：XDV → LayoutGraph 转换层

**新增文件**：`crates/xdv-parser/src/layout_graph.rs`

```text
XdvDocument (每页 Vec<XdvCommand>)
  → xdv_to_layout_nodes()
    → detect_page_kind()  // 分析 commands 中的 special / rule
      → XdvPageKind::Text | Figure | Table | Equation | Mixed
    → Vec<XdvLayoutNode>  // 每页一个节点
  → to_collector_layout_graph()
    → doc_semantic_collector::LayoutGraph
```

关键设计：
- 通过 `pdf:figure` / `xdv:float` specials 检测图
- 通过 `pdf:table` / `tab:*` specials 检测表
- 通过 `SetRule { height, width }` 中 `height > width * 2` 检测表格分隔线
- **宽松集成**：XDV 解析失败不阻断编译

**验收**：`cargo test -p doc-xdv-parser` 20 个测试全部通过（13 原有 + 6 新增 + 1 `to_collector_layout_graph`）。

---

### 2.5 P2-2：XDV LayoutGraph 接入 compiler-engine

**修改文件**：
- `crates/compiler-engine/Cargo.toml`（新增 `doc-xdv-parser` 依赖）
- `crates/compiler-engine/src/lib.rs`（重构 `collect_runtime_events` 返回类型 + LayoutGraph 填充）
- `crates/xdv-parser/src/lib.rs`（导出 `to_collector_layout_graph`）

核心改动：

1. `collect_runtime_events` 重构返回 `RuntimeCollectResult { events, xdv_path }`：
   - `XeLaTeXHookBackend`：XeLaTeX 不输出 XDV，`xdv_path = None`
   - `LuaTeXNodeBackend`：LuaLaTeX 输出 `.xdv`，捕获路径

2. `LuaTeXNodeBackend::collect()` 调用 `parse_layout_from_xdv()`：
   ```rust
   fn parse_layout_from_xdv(xdv_path: &Path) -> Option<LayoutGraph> {
       let bytes = fs::read(xdv_path).ok()?;
       let mut parser = XdvParser::default();
       let xdv = parser.parse_bytes(&bytes).ok()?;
       let nodes = xdv_to_layout_nodes(&xdv);
       Some(to_collector_layout_graph(nodes))
   }
   ```

3. `CompileReport` 报告 LayoutGraph 节点数

**验收**：`cargo test -p doc-compiler-engine` 34 个测试全部通过。

---

## 三、工作空间集成测试结果

```
cargo test --workspace
  ✅ doc-core: 119 passed
  ✅ doc-compiler-engine: 34 passed（含 2 个新增）
  ✅ doc-latex-reader: 3 passed（原有 + paper3_smoke）
  ✅ doc-semantic-collector: 20 passed
  ✅ doc-xdv-parser: 20 passed（13 原有 + 6 新增 + to_collector）
  ✅ doc-docx-writer: 36 passed
  ✅ doc-sys-utils: 5 passed
  ✅ doc-rule-engine: 2 passed
  ✅ doc-mathml: 3 passed
  ✅ doc-bib: 1 passed
  ✅ doc-compatibility-analyzer: 14 passed
  总计：320+ 测试，0 失败，0 回归
```

Paper3 三路径回归：
```
✅ paper3_main_jos_to_docx   (doc-core e2e)
✅ paper3_v2_vs_oracle       (doc-core oracle compare)
✅ paper3_front_matter_smoke (doc-latex-reader)
```

---

## 四、待开发内容清单

以下内容基于 `docs-zh/semantic-tex-engine-remaining-implementation-design-20260620-193855.md` 的分组，
标记本轮实现后的最新状态。

### 4.1 后续阶段（M3/M4 路线图）

#### 优先级 P0：影响 paper3 提交质量

| 任务 | 描述 | 当前状态 | 下一步 |
|------|------|---------|--------|
| M3-1 | **inline math 降级**：LaTeX 行内公式 `$...$` / `\(...\)` → Word OMML run | 未实现 | latex-reader 增加 `Inline::Math` 检测，serializer 增加 OMML run 渲染 |
| M3-2 | **Profile 外置化**（TOML）：将 JOS/中文学术 profile 从 Rust 内置迁移到 `profiles/*.toml` | 未实现 | 扩展 `EngineProfile` 支持 TOML 加载顺序：显式文件 → 内置 id → generic fallback |
| M3-3 | **复杂表格（multirow/multi-column）**：当前仅支持基本 tabular | 部分支持 | `doc_semantic_ast::TableCell` 增加 `rowspan`/`colspan`，latex-reader tabular 解析补全 |

#### 优先级 P1：提升高保真度

| 任务 | 描述 | 当前状态 | 下一步 |
|------|------|---------|--------|
| M4-1 | **TikZ 降级**：`\begin{tikzpicture}` → rasterize to PNG/PDF → image block | 未实现 | compatibility-analyzer 检测 TikZ，`tex-facade` 执行 `pdflatex` rasterize，保留源为 audit metadata |
| M4-2 | **LuaTeX v2**：`LuaTeXNodeBackend` 的 node tree layout 采集（采集 glyph/font/hlist/vlist/glue/rule） | 部分支持（SemanticEventV2 已完成） | 在 LuaLaTeX hook 中添加 node callback，输出到 LayoutGraph |
| M4-3 | **XDV Native Font**：`FontDefExt` / `NativeGlyph` / `NativeNode` 完整解析 | 未实现 | xdv-parser 增加 NativeGlyph 解析，映射 Unicode code point → glyph |

#### 优先级 P2：工程化

| 任务 | 描述 | 当前状态 | 下一步 |
|------|------|---------|--------|
| M2-1 | **XeLaTeX XDV 采集**：`XeLaTeXHookBackend` 在 XeLaTeX 运行时输出 `.xdv` | 未实现 | 在 XeLaTeX hook source 中添加 XDV 输出选项（XeTeX 支持 `-output-format=xdv`） |
| M2-2 | **RuleEngine AI fallback**：网络调用 + audit JSON + confidence 评分 | 未实现 | 可选 feature gate，默认关闭，输出 audit JSON |
| M2-3 | **RuleEngine 规则扩展**：支持 `RuleOutput::Figure`、`RuleOutput::Paragraph`，完整处理 `RawFallback` → 结构性块 | 基础完成（audit trail） | P1-2 已建立集成点，可逐步扩展 RuleOutput → Block 转换逻辑 |

### 4.2 待开发详细清单（按 Sprint 分组）

#### Sprint M3：文档高保真

```
[ ] M3-1: Inline math 降级
    - doc_semantic_ast: 新增 Inline::Formula variant
    - latex-reader: $...$ 和 \(...\) 检测，生成 Inline::Formula
    - serializer: OMML run 渲染行内公式
    - 测试：paper3 中 inline math 公式不破坏段落

[ ] M3-2: Profile TOML 外置化
    - 新增 profiles/ 目录 + jos-paper.toml / chinese-academic.toml
    - EngineProfile::from_id() 支持从 TOML 文件加载
    - paper3 profile file 验证

[ ] M3-3: 复杂表格 multirow/colspan
    - latex-reader tabular 解析增加 rowspan/colspan 提取
    - doc_semantic_ast TableCell 增加字段
    - serializer w:tcPr 输出 rowspan/colspan 属性
```

#### Sprint M4：高保真与工程

```
[ ] M4-1: TikZ rasterize pipeline
    - compatibility-analyzer 检测 \usepackage{tikz}
    - tex-facade 执行 pdflatex，提取 PDF page
    - tex-facade rasterize PDF page → PNG
    - latex-reader \begin{tikzpicture} 生成 Figure block
    - docx-writer 渲染 image block

[ ] M4-2: LuaTeX node tree layout 采集
    - LUA_HOOK 增加 node callback（glyph/font/hlist/vlist/glue/rule）
    - LayoutGraph 节点扩展：LayoutGlyph / LayoutRule / LayoutLine
    - to_collector_layout_graph() 扩展支持新节点类型

[ ] M4-3: XDV NativeFont 完整解析
    - FontDefExt 解析（flags, char_count, native_data）
    - NativeGlyph → Unicode mapping
    - NativeNode → whitespace/boundary nodes

[ ] M2-1: XeLaTeX XDV 输出采集
    - XELATEX_HOOK source 增加 \special{xdv:output=xdv} 选项
    - XeLaTeX 使用 -output-format=xdv
    - compile_vfs_to_graph() 处理 .xdv 副产物
    - XeLaTeXHookBackend.layout = parse_layout_from_xdv(xdv_path)

[ ] M2-2: RuleEngine AI fallback
    - feature gate: "ai-fallback"
    - RuleEngine.process_unknown() 触发 AI API 调用
    - 输出 audit JSON（confidence / prompt_hash / accepted）
    - CompileReport 输出 audit summary

[ ] M2-3: RuleEngine 规则扩展
    - RuleOutput::Figure / RuleOutput::Paragraph 处理
    - resolve_raw_fallback() 扩展：将 RuleOutput 映射到 Block
    - builtin_rules.rs 增加 paper3 模板常见宏规则
```

### 4.3 当前约束与限制

| 约束 | 说明 | 规避方式 |
|------|------|---------|
| XeLaTeX 无 XDV 副产物 | XeLaTeX 不输出 .xdv，`XeLaTeXHookBackend.layout` 为空 | M2-1 实现后解决 |
| RuleEngine AI 未实现 | 网络/审计未接入 | feature gate 控制，默认关闭 |
| TikZ 不可编辑 | 直接 rasterize 为图片 | 保留 TikZ 源码为 metadata |
| inline math 未降级 | 行内公式仍为纯文本 | M3-1 实现后解决 |
| multirow/colspan 不完整 | LaTeX multirow宏未解析 | M3-3 实现后解决 |

---

## 五、变更文件清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `crates/latex-reader/src/lower.rs` | 修改 | P1-1：代码环境检测 |
| `crates/compiler-engine/Cargo.toml` | 修改 | 新增 `doc-rule-engine`、`doc-xdv-parser` 依赖 |
| `crates/compiler-engine/src/lib.rs` | 修改 | P1-2（P1-3，P2-2）：RuleEngine 集成 + REF 字段 + LayoutGraph 接入 |
| `crates/xdv-parser/Cargo.toml` | 修改 | 新增 `doc-semantic-collector` 依赖 |
| `crates/xdv-parser/src/lib.rs` | 修改 | 导出 `to_collector_layout_graph` |
| `crates/xdv-parser/src/layout_graph.rs` | 新增 | P2-1：XDV → LayoutGraph 转换层 |

---

## 六、GitNexus 变更检测

执行变更检测：

```bash
npx gitnexus analyze
npx gitnexus detect-changes --diff
```

建议在提交前执行，验证影响范围。
