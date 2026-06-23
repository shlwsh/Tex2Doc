# Semantic TeX Engine 开发进展报告

**生成时间**：2026-06-21 06:00
**报告范围**：基于 2026-06-20 设计文档的增量开发

---

## 一、本轮完成情况概览

| 任务 | ID | 状态 | 备注 |
|------|----|------|------|
| XDV Parser crate | T11 | ✅ 完成 | 13 个测试 |
| LuaTeX Collector v2 | T12 | ✅ 完成 | 24 个测试 |
| semantic-collector crate 拆分 | T4 | ✅ 完成 | 20 个测试 |
| compatibility-analyzer crate 拆分 | T5 | ✅ 完成 | 14 个测试 |
| Profile 外置化（JSON） | T7 | ✅ 完成 | TOML 格式改为 JSON |
| DOCX 高保真增强 — 代码块 | T8 | ✅ 完成 | `CodeBlock` 类型 + DOCX 渲染 |
| DOCX 高保真增强 — 行内公式 OMML | T8b | ✅ 完成 | `TextStyle::MathInline` → OMML |
| AI Fallback / Rule Engine | T6 | ✅ 完成 | 11 个测试 |
| **合计** | | **8/8** | **369 个测试全部通过** |

---

## 二、详细实现说明

### 2.1 T11：XDV Parser crate（已完成）

**新增 crate**：`crates/xdv-parser/`

**模块结构**：
```
crates/xdv-parser/src/
├── lib.rs              # 入口、错误类型
├── parser.rs           # XdvParser 核心 + ByteReader
├── opcodes.rs          # XDV/DVI opcode 枚举
├── fixtures.rs         # 测试夹具辅助
└── tests/             # 集成测试
```

**核心能力**：
- `ByteReader`：字节流读取，支持 peek/read_u8/read_i8/read_u32
- `XdvParser::new()`：从 `&[u8]` 构造解析器
- `parse_stream()`：解析 XDV stream，提取 `SetChar`、`PutRule`、`Set`、`Push`、`Pop`、`Bop`、`Eop`、`XXX`、`Xxx` 等操作
- `XdvPage`：单页容器（glyph + rule + special 事件序列）
- `XdvDocument`：多页文档（`Vec<XdvPage>` + `comments: Vec<String>`）

**测试**：13 个单测 + fixture 解析，覆盖 opcode 覆盖完整性。

---

### 2.2 T12：LuaTeX Collector v2（已完成）

**变更范围**：`crates/compiler-engine/src/lib.rs`

**关键增强**：

1. **JSONL Schema v2**：`SemanticEventV2` 新增字段：
   - `label` / `number`（编号）
   - `io`（算法 I/O 元数据：`KwIn`/`KwOut`）
   - `tags`（关键词标签数组）
   - `Caption` 枚举变体（独立 caption 事件）

2. **XeLaTeX Hook 更新**：`XELATEX_SEMANTIC_HOOK` v2
   - 补全 `\section*`、算法环境、`\caption` 处理
   - 使用 `\DetokenizeInto` 避免 TeX 展开问题

3. **v1/v2 兼容解析**：`parse_semantic_events_jsonl()` 自动检测 JSONL 行格式，降级处理混合文件

**测试**：24 个单测（JSONL 解析、事件序列化、版本检测）。

---

### 2.3 T4：semantic-collector crate 拆分（已完成）

**新增 crate**：`crates/semantic-collector/`

**模块结构**：
```
crates/semantic-collector/src/
├── lib.rs              # 主入口 + 核心类型 re-export
├── reference_graph.rs  # 交叉引用图
└── tex_utils.rs       # TeX 工具函数
```

**移出的类型**（从 `compiler-engine/src/lib.rs`）：
- `SemanticEvent` / `SemanticEventV2` / `EventSource` / `SourceSpan`
- `LayoutNode` / `LayoutGraph`
- `SemanticBackendKind` / `BackendSelectionReport` / `BackendAvailability`
- `BuildSidecar` / `CollectorDiagnostic` / `CollectorDiagnosticSeverity`
- `CollectedDocument` / `SemanticBackendArtifact`
- `CollectorError`
- `ReferenceGraph` 及相关类型（reference_graph.rs）
- TeX 工具函数（tex_utils.rs）

**测试**：20 个单测（JSON 序列化、引用图构建、v1/v2 解析）。

---

### 2.4 T5：compatibility-analyzer crate 拆分（已完成）

**新增 crate**：`crates/compatibility-analyzer/`

**模块结构**：
```
crates/compatibility-analyzer/src/
└── lib.rs              # 单一模块
```

**核心类型**：
- `CompatibilityReport` / `CompatibilityIssue`
- `CompatibilityRules`（可配置阈值）
- `CompatibilityAnalyzer`
- `ProfileKind`（generic-article / chinese-academic / jos-paper / medical-journal）

**移出的函数**（从 `compiler-engine/src/lib.rs`）：
- `analyze_compatibility` + `add_compatibility_issue` + `apply_compatibility_score`
- `is_tex_source_path` + `count_custom_macro_definitions`
- `contains_tex_environment`
- `strip_tex_comments` + `is_escaped` + `scan_tex_commands`
- `tex_command_matches` + `skip_tex_space_and_options`
- `find_matching_bracket` + `split_tex_name_list`

**测试**：14 个单测（包检测、宏计数、环境识别、分数计算）。

---

### 2.5 T7：Profile 外置化（已完成）

**新增目录**：`crates/compiler-engine/profiles/`
**新增文件**：`crates/compiler-engine/src/profiles.rs`

**Profile 文件（JSON 格式）**：
| 文件 | ID | 描述 |
|------|-----|------|
| `generic-article.json` | generic-article | 通用学术论文 |
| `chinese-academic.json` | chinese-academic | 中文期刊 |
| `jos-paper.json` | jos-paper | Journal of Software 格式 |
| `medical-journal.json` | medical-journal | 医学期刊 |

> **注**：原设计为 TOML 格式，但沙箱网络限制导致 `toml` crate 无法下载，改为使用 `serde_json`（已有工作区依赖）解析 JSON。

**主文件** `profiles.rs` 提供：
- `load_profile(id)`：加载内置或外部 profile
- `load_from_file(path)`：从文件系统加载
- `builtin_json(id)`：获取内置 JSON 字符串
- `ProfileLoadError` / `resolve_profile_path` 等工具

---

### 2.6 T8：DOCX 高保真增强 — 代码块（已完成）

**修改文件**：

1. **`crates/semantic-ast/src/lib.rs`**：`Block` enum 新增 `CodeBlock` 变体
   ```rust
   CodeBlock {
       language: Option<String>,
       code: String,
       span: Span,
   }
   ```

2. **`crates/semantic-ast/src/standard.rs`**：
   - `BlockKind` 新增 `CodeBlock(CodeBlockNode)` 变体
   - 新增 `CodeBlockNode` struct：`language`、`code`、`source: CodeBlockSource`
   - 新增 `CodeBlockSource` enum：`Minted`、`Listlings`、`Verbatim`、`Lstlisting`、`MarkdownFence`
   - `BlockNode::from_legacy` 增加 `CodeBlock` 分支
   - `kind_name()` 增加 `"code_block"`
   - `to_markdown` 增加代码块渲染

3. **`crates/docx-writer/src/serializer.rs`**：
   - `Block::CodeBlock` 分支：用 `STYLE_CODE` + Courier New 渲染
   - 新增 `STYLE_COMMENT` 常量
   - 新增 `CodeBlock` 语言标注段落

4. **`crates/docx-writer/src/styles.rs`**：新增 `STYLE_COMMENT`

5. **`crates/docx-writer/src/docx_render.rs`**：Caption 提取增加 `CodeBlock`

6. **`crates/semantic-ast/src/visit.rs`**：Visitor trait 增加 `CodeBlock` 分支

7. **`crates/core/src/convert.rs`**：`block_fingerprint` 增加 `CodeBlock`

---

### 2.7 T8b：DOCX 高保真增强 — 行内公式 OMML（已完成）

**修改文件**：`crates/docx-writer/src/serializer.rs`

**新增功能**：

1. **`write_inline_math_run()`** 函数：将 `TextStyle::MathInline` 渲染为 OMML：
   ```xml
   <w:r><m:oMath><m:oMathPara>
     <m:r><m:t>LaTeX内容</m:t></m:r>
   </m:oMathPara></m:oMath></w:r>
   ```

2. **`write_paragraph_with_opts()`** 改造：遍历 runs 时检测 `TextStyle::MathInline`，调用 `write_inline_math_run()` 而非普通 `write_run()`

3. **命名空间增强**：
   - `w:document` root 增加 `xmlns:m`、`xmlns:a`、`xmlns:wp`
   - `wp:inline` drawing 增加 `xmlns:m`

4. **`TextStyle::MathInline` 斜体处理**：`from_text_run()` 将 `MathInline` 映射为 `italic: true`（作为降级 fallback）

**行为说明**：
- 行内公式 `$E=mc^2$` → Word 中为可编辑 OMML 公式对象
- 块级公式 `$$...$$` → 仍然使用 JOSCode 纯文本居中（不降级）

---

### 2.8 T6：AI Fallback / Rule Engine（已完成）

**新增 crate**：`crates/rule-engine/`

**模块结构**：
```
crates/rule-engine/src/
├── lib.rs              # 主入口
├── audit.rs            # AuditCache / AuditRecord
├── builtin_rules.rs    # 内置宏规则集
├── registry.rs         # RuleRegistry
├── rule_output.rs      # RuleOutput enum
└── rule_engine.rs     # RuleEngine 核心
```

**核心类型**：

| 类型 | 描述 |
|------|------|
| `RuleOutput` | 规则输出语义：`Heading`、`Paragraph`、`InlineText`、`Ignore`、`Table`、`Figure`、`Verbatim` |
| `MacroRule` | 单条宏规则：`id`、`name`、`arity`、`output`、`description` |
| `RuleRegistry` | 规则仓库：注册/查找/JSON 导入导出 |
| `AuditRecord` | 审计记录：`macro_name`、`decision`、`confidence`、`source`、`accepted`、`location` |
| `AuditCache` | 会话审计缓存：record/serialize/deserialize |
| `DecisionSource` | 决策来源：`Fallback`、`Builtin`、`Loaded`、`AI`、`UserOverride` |
| `RuleEngine` | 规则引擎：`process_unknown()` 入口 |
| `RuleEngineConfig` | 配置：`enable_ai`、`ai_min_confidence`、`warn_on_unknown` |

**内置规则集**（45+ 条）：
- 字体命令：`textbf`、`textit`、`texttt`、`emph`、`mathbf`、`mathrm`、`mathsf`、`mathtt`
- 字号命令：`tiny`、`small`、`large`、`Large`、`LARGE`、`huge`、`Huge`
- 间距命令：`hspace`、`vspace`、`quad`、`qquad`、`smallskip`、`mediumskip`、`bigskip`
- 断行断页：`newline`、`newpage`、`pagebreak`、`clearpage`
- 装饰命令：`underline`、`sout`、`xout`、`uwave`、`CJKfamily`
- 章节命令：`paragraph`（level 4）、`subparagraph`（level 5）

**设计要点**：
- AI fallback **默认禁用**（`enable_ai: false`）
- 未知宏 → warning + verbatim fallback + 审计记录
- 审计记录可 JSON 序列化导出供用户审查
- 支持从外部 JSON 文件加载自定义规则

**测试**：11 个单测（规则注册/查找/JSON 往返/审计导出/配置切换）。

---

## 三、Workspace 结构

```
crates/
├── xdv-parser/           # T11 XDV/DVI bytecode parser
├── semantic-collector/   # T4 语义采集类型和 trait
├── compatibility-analyzer/ # T5 LaTeX 兼容性分析
├── rule-engine/          # T6 AI fallback / 规则引擎
├── compiler-engine/      # T7 profiles.rs + 主逻辑
├── semantic-ast/         # T8 CodeBlock 类型
├── docx-writer/         # T8/T8b DOCX 渲染（代码块 + 行内公式 OMML）
├── latex-reader/         # LaTeX → AST
├── doc-core/            # 转换入口
├── bib/                 # BibTeX 处理
├── mathml/              # MathML 处理
└── ...
```

---

## 四、测试覆盖

| Crate | 测试数 |
|-------|--------|
| doc-xdv-parser | 13 |
| doc-semantic-collector | 20 |
| doc-compatibility-analyzer | 14 |
| doc-rule-engine | 11 |
| doc-latex-reader | 119+ |
| doc-docx-writer | 36+ |
| doc-semantic-ast | 5+ |
| doc-compiler-engine | 32+ |
| doc-core | 19+ |
| 其他 | 100+ |
| **总计** | **369** |

所有 369 个测试全部通过，无回归。

---

## 五、待审核 / 待开发内容

### 待审核（本轮产出）

1. **doc-rule-engine crate** — T6 AI Fallback / Rule Engine 完整实现
2. **CodeBlock 类型** — `Block` + `BlockKind` 新变体及 DOCX 渲染
3. **行内公式 OMML** — `TextStyle::MathInline` → Word OMML
4. **docx-writer 命名空间** — `xmlns:m`、`xmlns:a`、`xmlns:wp` 完整声明

### 后续开发建议

| 优先级 | 任务 | 描述 |
|--------|------|------|
| P1 | **T6 AI Engine** | 实现可选 AI inference（需网络）+ prompt hash + 审计记录 |
| P1 | **latex-reader 代码环境检测** | 检测 `minted`/`lstlisting`/`verbatim` 环境，生成 `Block::CodeBlock` |
| P2 | **Word REF 字段** | `\ref{label}` → `<w:fldSimple REF="...">` 而非 hyperlink |
| P2 | **TikZ 降级** | 检测 TikZ 源码，编译为 PNG/PDF 或保留为 alt metadata |
| P2 | **LayoutGraph 接入** | 将 XDV parser 输出接入 compiler-engine 的 layout 推理 |
| P3 | **Rule Engine AI Plugin** | 实现 `AiEngine` trait，允许插入 OpenAI/Ollama 等后端 |

---

## 六、提交建议

建议按以下顺序分 3 个 commit 提交：

### Commit 1: T11 + T12
```
feat(xdv-parser): add XDV/DVI bytecode parser crate
feat(compiler-engine: T12) LuaTeX collector v2 with enhanced hooks
```

### Commit 2: T4 + T5 + T7
```
refactor(architecture): split semantic-collector and compatibility-analyzer crates
feat(compiler-engine: T7) profile externalization to JSON files
```

### Commit 3: T6 + T8 + T8b
```
feat(rule-engine): add AI fallback / rule engine crate
feat(docx): add CodeBlock type and inline math OMML rendering
```

---

*报告自动生成，基于 2026-06-21 06:00 的 workspace 状态。*
