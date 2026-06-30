# Semantic TeX Engine Collector 输出模型开发报告（20260620-192816）
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



## 1. 本轮目标

完成 T10 Semantic Collector trait 内置初版：

- 在 `doc-compiler-engine` 内定义 collector 层，而不是直接让 backend 绑定 `doc-latex-reader` lowering 细节。
- 新增统一中间输出 `CollectedDocument`。
- 将规则降级链路封装为 `RuleBasedCollector`。
- 记录 runtime sidecar 元数据，并在 paper3 报告中输出 sidecar 数量。
- 保持旧 `doc-core` / Rust rule DOCX 转换路径不受影响。

## 2. 代码变更

### 2.1 Collector 抽象

文件：

```text
crates/compiler-engine/src/lib.rs
```

新增：

```rust
SemanticCollector
SemanticCollectorInput
RuleBasedCollector
CollectedDocument
BuildSidecar
```

`CollectedDocument` 当前包含：

```text
document
standard_document
image_assets
events
layout
diagnostics
sidecars
```

旧名：

```rust
SemanticBackendArtifact
```

暂时作为 `CollectedDocument` 的类型别名保留，降低后续迁移成本。

### 2.2 RuleBasedCollector

原 `RuleBasedBackend.collect` 中的核心流程已迁入 `RuleBasedCollector`：

```text
VirtualFs
  -> IncludeGraph::build / join
  -> parse_tex
  -> lower_semantic_document
  -> collect_image_assets_from_vfs
  -> StandardDocument::from_legacy_document
  -> CollectedDocument
```

`RuleBasedBackend` 现在只负责适配 `SemanticBackend` trait。

### 2.3 Runtime sidecar

`XeLaTeXHookBackend` 和 `LuaTeXNodeBackend` 采集 runtime semantic events 后，会把：

```text
__docx_semantic_events.jsonl
```

记录为：

```rust
BuildSidecar {
    kind: "semantic-events-jsonl",
    path: Some("__docx_semantic_events.jsonl"),
    description: ...
}
```

`CompileReport` 新增：

```text
sidecar_count
```

paper3 example 和验证脚本新增输出：

```text
sidecars: 1
```

## 3. 影响分析

GitNexus impact 结果：

- `SemanticBackendArtifact`：LOW，0 impacted。
- `SemanticBackend`：LOW，4 个直接实现者。
- `SemanticBackend.collect`：LOW，4 个直接实现方法。
- `RuleBasedBackend.collect`：LOW。
- `XeLaTeXHookBackend.collect`：LOW。
- `LuaTeXNodeBackend.collect`：LOW。
- `CompileReport` struct / impl：LOW。
- `paper3_to_docx.rs::run`：LOW。

处理策略：

- 不修改 `doc-core`。
- 不修改旧 `doc-docx-writer` 默认行为。
- 先在 `doc-compiler-engine` 内落地 trait 和输出模型，暂不拆 `crates/semantic-collector`。
- 以类型别名保留 `SemanticBackendArtifact`，避免一次性破坏外部调用面。

## 4. 验证结果

已执行：

```bash
cargo fmt -p doc-compiler-engine
cargo test -p doc-compiler-engine collector -- --nocapture
cargo test -p doc-compiler-engine sidecar -- --nocapture
cargo test -p doc-compiler-engine
cargo test -p doc-core
bash scripts/compare_paper3_dual_engines.sh 15
bash scripts/build_paper3_three_docx.sh 15
```

结果：

```text
doc-compiler-engine collector: 1 passed
doc-compiler-engine sidecar: 1 passed
doc-compiler-engine: 21 passed, 1 ignored
doc-core: 5 passed
paper3 dual engines: generated
paper3 three-docx: generated
```

已知 warning：

- `doc-latex-reader` 仍有既存 unused/dead_code warning。
- `doc-docx-writer` 仍有既存 unused helper warning。
- `cargo fmt` 仍提示当前 rustfmt 为 stable，无法启用项目中配置的 nightly-only format options。

这些 warning 与本轮 collector 输出模型无关。

## 5. paper3 最新输出

双引擎对比：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-192735-dual-engines-comparison-report.md
```

摘要：

| engine | docx | bytes | media | paragraphs | tables | drawings | text chars |
|---|---|---:|---:|---:|---:|---:|---:|
| rust-rule | `v15-论文稿件-jos-20260620-192735-dual-engines-rust-rule.docx` | 3,055,363 | 10 | 653 | 12 | 20 | 41,963 |
| semantic-engine auto | `v15-论文稿件-jos-20260620-192735-dual-engines-semantic-engine-auto.docx` | 3,057,574 | 10 | 653 | 12 | 20 | 42,535 |

三路径验证：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-192754-three-docx-report.md
```

摘要：

| path | docx | bytes | media |
|---|---|---:|---:|
| sh | `v15-论文稿件-jos-sh-20260620-192754.docx` | 3,079,377 | 10 |
| rust-rule | `v15-论文稿件-jos-20260620-192754-rust-rule.docx` | 3,055,363 | 10 |
| semantic-engine | `v15-论文稿件-jos-20260620-192754-semantic-engine-xelatex_hook.docx` | 3,057,574 | 10 |

semantic-engine 报告：

```text
compatibility-score: 76
compatibility-unsupported: 0
compatibility-warnings: 2
compatibility-custom-macros: 46
sidecars: 1
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

- 内置 `SemanticCollector` trait。
- 内置 `RuleBasedCollector`。
- `CollectedDocument` 统一输出模型。
- runtime sidecar 元数据记录。
- paper3 报告输出 sidecar 数量。

暂未支持：

- 独立 `crates/semantic-collector`。
- collector 插件注册表。
- runtime sidecar 文件落盘保留和调试导出。
- LuaTeX node tree 的 layout 坐标、box 尺寸、行/页聚类。
- XDV parser 接入。

## 7. 后续建议

下一步进入 T11：

```text
XDV parser 原型
```

先新增最小 `crates/xdv-parser`，锁定 XDV/DVI 指令结构和 fixture 单测，再考虑将布局信息合并到 `LayoutGraph`。
