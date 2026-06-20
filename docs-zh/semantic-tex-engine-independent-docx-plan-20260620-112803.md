# Semantic TeX Engine 独立 DOCX 转换路径方案（20260620-112803）

> 状态：待审核
>
> 本方案只调整设计与任务清单，不要求立即开发。审核通过前，不改动现有 Rust 版本 DOCX 转换引擎。
>
> 后续补充方案：[Semantic TeX Engine 双后端语义采集方案（20260620-115348）](./semantic-tex-engine-dual-backend-design-20260620-115348.md)

## 1. 方案目标

在当前项目中保留两条互不影响的 DOCX 转换路径：

```text
路径 A：现有 Rust 转换引擎
  doc-core
    -> doc-latex-reader
    -> doc-semantic-ast::Document
    -> doc-docx-writer
    -> DOCX

路径 B：新 Semantic TeX Engine
  doc-compiler-engine
    -> independent semantic collector facade
    -> DocumentGraph / StandardDocument
    -> semantic DOCX renderer path
    -> DOCX
```

核心原则：

- 不迁移、不替换、不破坏现有 `doc-core`。
- 新语义引擎另起炉灶，作为第二条可对照路径存在。
- 可以复用或拷贝现有 reader/writer/utility 组件，但不能让新路径的行为改动影响旧路径。
- 两条路径都能对同一 paper3 输入生成 DOCX，便于做最终效果对比。
- 审核通过后再开发；本方案阶段只输出设计与任务计划。

## 2. 明确不做的事

本方案不做以下工作：

- 不把 `doc-core::convert_sync` 改为调用 `SemanticTexEngine::compile_source_to_docx`。
- 不把 `doc-core::convert_zip` 改为调用 `SemanticTexEngine::compile_zip_to_docx`。
- 不把 `doc-core::convert_dir` 改为调用 `SemanticTexEngine::compile_dir_to_docx`。
- 不让 `doc-engine convert` 默认改用新语义引擎。
- 不删除或重构现有 `doc-core` 内部的解析、图片、页眉页脚补偿逻辑。

旧路径如果需要维护，只做独立 bugfix；新路径如果需要借鉴旧路径，优先通过复制、适配器或只读复用来避免行为耦合。

## 3. 双路径边界

### 3.1 路径 A：现有 Rust 转换引擎

保留职责：

- 继续服务现有 WASM、Native、Server、`doc-engine convert/build` 路径。
- 继续使用当前 `ConvertOptions`、`ConvertResult`、`CoreError` API。
- 继续保持已有 paper3 e2e、server API、wasm/native 兼容。

约束：

- 新语义引擎开发不得修改 `doc-core` 的公开行为。
- 需要共享代码时，先评估是否会改变旧路径输出。
- 旧路径测试作为回归基线，而不是迁移目标。

### 3.2 路径 B：新 Semantic TeX Engine

保留/新增职责：

- `doc-compiler-engine` 作为新路径入口。
- 独立支持 source/dir/zip/VFS 到 DOCX。
- 输出 `CompileReport`、`DocumentGraph`、`StandardDocument`，便于诊断。
- 建立独立 CLI、脚本和输出目录。
- 以 paper3 为首个高保真对照样例。

当前已有入口：

```rust
SemanticTexEngine::compile_source_to_docx
SemanticTexEngine::compile_dir_to_docx
SemanticTexEngine::compile_zip_to_docx
SemanticTexEngine::compile_vfs_to_graph
SemanticTexEngine::compile_vfs_to_docx
```

当前已有 paper3 脚本：

```bash
bash scripts/build_paper3_compiler_engine_docx.sh
```

输出目录：

```text
examples/paper3/output/to-docx
```

## 4. 代码复用策略

### 4.1 允许直接复用的组件

这些组件可被旧路径和新路径共同依赖，因为它们天然是底层库：

| 组件 | 复用方式 | 说明 |
|---|---|---|
| `doc-utils::VirtualFs` | 直接依赖 | 输入挂载、路径解析、资源读取 |
| `doc-semantic-ast` | 直接依赖 | 共享语义模型，但新增字段需保持兼容 |
| `doc-latex-reader` | 短期直接依赖 | 新路径当前 rule-based collector 可复用 |
| `doc-mathml` | 直接依赖 | 新路径公式 OMML 输出优先使用 |
| `doc-docx-writer` | 短期直接依赖 | 新路径可先复用 packer，后续拆 semantic renderer |

### 4.2 建议复制或适配的逻辑

这些逻辑可以参考旧路径，但不应直接搬动旧路径行为：

| 旧逻辑 | 新路径处理方式 |
|---|---|
| `doc-core` 的 JOS 页眉页脚补偿 | 复制为 `doc-compiler-engine` profile 规则 |
| `doc-core` 的 PDF 图片转 PNG | 保留新路径独立实现或下沉到 `doc-utils` 后双路径显式接入 |
| `doc-core` 的 bibliography fallback | 新路径独立保留，并逐步结构化为 `ReferenceGraph` |
| `doc-docx-writer` 的 JOS 样式细节 | 新路径先复用，后续抽为 profile-to-style 映射 |

### 4.3 禁止的耦合

- 禁止通过修改 `doc-core` 来“顺便”完成新引擎能力。
- 禁止让 `doc-core` 依赖 `doc-compiler-engine`。
- 禁止让旧路径默认调用新路径。
- 禁止以旧路径测试通过作为新路径高保真完成证明。

## 5. 推荐目录与命名

现状保留：

```text
crates/compiler-engine
```

建议新增时使用清晰前缀：

```text
crates/semantic-collector
crates/xdv-parser
crates/compatibility-analyzer
```

推荐 CLI/脚本命名：

```text
scripts/build_paper3_semantic_docx.sh
scripts/compare_paper3_dual_engines.sh
```

推荐输出命名：

```text
examples/paper3/output/to-docx/
  <version>-论文稿件-jos-<timestamp>-rust-engine.docx
  <version>-论文稿件-jos-<timestamp>-semantic-engine.docx
  <version>-论文稿件-jos-<timestamp>-engine-compare.md
```

现有 `*-compiler-engine.docx` 可继续保留；新命名只用于后续更明确地区分路径。

## 6. 新路径任务清单

### P0 审核方案与冻结边界

状态：待审核

目标：

- 确认新旧路径独立并存。
- 确认不迁移 `doc-core`。
- 确认后续开发只在新路径或新增 crate 中进行。

验收：

- 本文档通过审核。
- `docs-zh/semantic-tex-engine-progress-and-task-plan.md` 不再把 `doc-core` 迁移作为下一步。

### P1 新语义引擎 profile 规则内聚

状态：待开发

目标：

- 在 `doc-compiler-engine` 内实现独立 profile 规则。
- 把 JOS 默认页面、页眉、页脚、caption 命名、基础字体策略纳入新路径。
- 不依赖 `doc-core` 的 profile 补偿逻辑。

建议实现：

```rust
pub struct ProfileSpec {
    pub id: &'static str,
    pub page_setup: Option<PageSetup>,
    pub default_header_policy: HeaderPolicy,
    pub caption_policy: CaptionPolicy,
}
```

验证：

```bash
cargo test -p doc-compiler-engine profile
```

### P2 新语义引擎 paper3 独立 E2E

状态：待开发

目标：

- 为 `doc-compiler-engine` 增加 paper3 E2E 测试。
- 断言 DOCX zip 结构、media 数量、关键中文短语、阶段报告。
- 输出仍写到 `examples/paper3/output/to-docx`，但命名显式带 `semantic-engine`。

验证：

```bash
cargo test -p doc-compiler-engine --test paper3_semantic_e2e
bash scripts/build_paper3_compiler_engine_docx.sh
```

### P3 双路径对比脚本

状态：待开发

目标：

- 新增对比脚本，同时生成旧 Rust 引擎 DOCX 与新 Semantic Engine DOCX。
- 输出结构差异、media 数量、document.xml 文本摘要、文件大小。
- 不改变两条路径各自实现。

建议脚本：

```bash
scripts/compare_paper3_dual_engines.sh
```

验证：

```bash
bash scripts/compare_paper3_dual_engines.sh
```

### P4 ReferenceGraph 独立实现

状态：待开发

目标：

- 在新路径中新增显式 `ReferenceGraph`。
- 结构化 `label/ref/eqref/autoref/cite`。
- 对未解析引用输出 diagnostics。

约束：

- 不修改 `doc-core` 的引用处理。
- 可复用 `doc-latex-reader` 现有 label/ref 识别逻辑。

验证：

```bash
cargo test -p doc-compiler-engine reference
```

### P5 公式 OMML 接入新路径

状态：待开发

目标：

- 新路径优先把 `Block::Equation` 渲染为 OMML。
- 旧路径保持当前公式输出行为不变。

建议：

- 在 `doc-compiler-engine` 中增加公式预渲染层，或新增 semantic renderer。
- 避免直接改变 `doc-docx-writer` 的默认 `write_equation` 行为。
- 如必须修改 `doc-docx-writer`，需提供 feature 或新 API，让旧路径仍走原行为。

验证：

```bash
cargo test -p doc-mathml
cargo test -p doc-compiler-engine equation
```

### P6 新语义 DOCX renderer 边界

状态：待开发

目标：

- 为新路径建立 renderer 边界，避免长期直接复用旧路径 packer 行为。
- 支持从 `DocumentGraph` 渲染 DOCX。

候选方案：

```rust
pub trait SemanticRenderer {
    fn render_docx(&self, graph: &DocumentGraph, options: &RenderOptions) -> Result<Vec<u8>>;
}
```

短期可以内部调用 `doc-docx-writer`，但对外暴露为新路径 renderer。

### P7 兼容性分析器

状态：待开发

目标：

- 新增 `compatibility-analyzer`。
- 为新路径转换前提供支持度评分。
- 输出 unsupported/warnings，不影响旧路径。

验证：

```bash
cargo test -p doc-compatibility-analyzer
```

### P8 Semantic Collector trait

状态：待开发

目标：

- 把当前 rule-based collector 从 `SemanticTexEngine` 中抽成独立 collector。
- 后续接入 LuaHook/XDV/AI fallback 时不影响旧路径。

### P9 XDV parser 与 LuaHook 原型

状态：待开发

目标：

- 新增 XDV parser 原型。
- 设计 LuaHook collector 输出协议。
- 只作为新路径增强，不参与旧路径。

## 7. 审核后第一批开发建议

审核通过后建议按以下顺序执行：

```text
P1 profile 规则内聚
P2 paper3 semantic E2E
P3 双路径对比脚本
P4 ReferenceGraph
P5 公式 OMML 新路径接入
```

不建议第一步做 LuaHook/XDV，因为当前最需要先建立“独立新路径 + 可对比验证”的工程边界。

## 8. 验证策略

### 8.1 不影响旧路径的验证

每次新路径开发后，至少运行：

```bash
cargo test -p doc-core
```

目标不是让旧路径使用新能力，而是证明旧路径未被破坏。

### 8.2 新路径验证

至少运行：

```bash
cargo test -p doc-compiler-engine
bash scripts/build_paper3_compiler_engine_docx.sh
```

### 8.3 双路径对比验证

待 P3 完成后运行：

```bash
bash scripts/compare_paper3_dual_engines.sh
```

输出应包含：

- 两个 DOCX 文件路径。
- 文件大小。
- `word/media/*` 数量。
- `word/document.xml` 文本摘要。
- 关键短语命中。
- 差异报告路径。

## 9. 当前状态

当前已具备：

- `doc-compiler-engine` facade。
- paper3 compiler-engine 生成脚本。
- V2 质量闭环相关 crate。
- 本独立路径审核方案。

当前待审核：

- 是否接受新旧引擎长期双路径并存。
- 是否接受新路径通过复用/复制旧组件快速落地。
- 是否接受后续开发从 P1/P2/P3 开始，而不是迁移 `doc-core`。
- 是否接受新 Semantic Engine 内部进一步采用 XeLaTeXHookBackend + LuaTeXNodeBackend 双后端路线。
