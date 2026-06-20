# Semantic TeX Engine 开发进展报告（20260620-124347）

## 1. 本轮目标

本轮围绕新语义引擎路径继续开发，不改变现有 `doc-core` / `doc-engine convert` 旧 Rust DOCX 路径。目标包括：

- 在 `doc-compiler-engine` 内落地 RuleBased、XeLaTeXHook、LuaTeXNode 三类 backend。
- 同步实现 LuaTeX 语义解析能力。
- 提供 paper3 一键验证脚本，生成 sh、rust-rule、semantic-engine 三份 DOCX。
- 输出验证报告，便于对比三条路径的实际产物。

## 2. 代码实现

### 2.1 Backend 抽象

`crates/compiler-engine/src/lib.rs` 新增：

```rust
SemanticBackendKind
BackendSelectionReport
SemanticBackend
RuleBasedBackend
XeLaTeXHookBackend
LuaTeXNodeBackend
SemanticEvent
LayoutGraph
```

`CompileOptions` 新增：

```rust
semantic_backend: SemanticBackendKind
allow_backend_fallback: bool
```

`CompileReport` 新增：

```rust
backend: BackendSelectionReport
semantic_event_count: usize
layout_node_count: usize
```

### 2.2 XeLaTeXHookBackend

当前 `XeLaTeXHookBackend` 已实现最小可运行链路：

```text
VFS materialize
  -> 注入 hook tex
  -> xelatex 编译
  -> 读取 __docx_semantic_events.jsonl
  -> parse_semantic_events_jsonl
  -> 合并到 DocumentGraph.semantic_events
```

paper3 使用 `xeCJK` / CTeX 生态，适合该 backend。验证中已严格选中：

```text
backend-requested: xelatex-hook
backend-selected: xelatex-hook
backend-reason: xelatex-hook available: found /usr/bin/xelatex
```

### 2.3 LuaTeXNodeBackend

当前 `LuaTeXNodeBackend` 已实现最小可运行链路：

```text
VFS materialize
  -> 注入 Lua collector
  -> lualatex 编译
  -> macro hook 输出结构事件
  -> post_linebreak_filter 输出段落事件
  -> 读取 __docx_semantic_events.jsonl
  -> parse_semantic_events_jsonl
  -> 合并到 DocumentGraph.semantic_events
```

已支持的 sidecar 事件：

```text
heading
paragraph
label
reference
citation
figure
equation
```

LuaTeX runtime 会把 `TEXMFVAR` / `TEXMFCACHE` 指向临时目录，避免依赖或污染用户 HOME 下的 TeX Live cache。

限制：

- paper3 模板加载 `xeCJK`，该包强制要求 XeTeX；因此 `luatex-node` 在 paper3 上会按设计 fallback。
- 当前 LuaTeX node tree 只用于段落文本事件；尚未输出真实 layout 坐标、box 尺寸、页码和行聚类。

## 3. 新增脚本

新增：

```bash
scripts/build_paper3_three_docx.sh
```

用途：

```bash
bash scripts/build_paper3_three_docx.sh 15
```

默认输出目录：

```text
examples/paper3/output/to-docx
```

三条路径：

| 路径 | 实现 |
|---|---|
| sh | `scripts/build_docx.sh`，即现有 Python/sh JOS 产线 |
| rust-rule | `target/release/doc-engine convert`，即现有 Rust 规则路径 |
| semantic-engine | `doc-compiler-engine` example，默认 `--semantic-backend xelatex-hook --no-backend-fallback` |

脚本默认把临时 paper3 zip 写入 `examples/paper3/output/to-docx`，不会改写已跟踪的 `examples/paper3/upload.zip`。

## 4. paper3 验证结果

执行命令：

```bash
bash scripts/build_paper3_three_docx.sh 15
```

生成报告：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-124347-three-docx-report.md
```

三份 DOCX：

| 路径 | 文件 | 大小 | media |
|---|---|---:|---:|
| sh | `v15-论文稿件-jos-sh-20260620-124347.docx` | 3,079,377 bytes | 10 |
| rust-rule | `v15-论文稿件-jos-20260620-124347-rust-rule.docx` | 3,055,363 bytes | 10 |
| semantic-engine | `v15-论文稿件-jos-20260620-124347-semantic-engine-xelatex_hook.docx` | 3,055,688 bytes | 10 |

## 5. 测试与验证

已执行：

```bash
cargo fmt -p doc-compiler-engine
cargo test -p doc-compiler-engine
cargo test -p doc-compiler-engine luatex_runtime_collects_semantic_events -- --ignored --nocapture
cargo test -p doc-core
bash -n scripts/build_paper3_three_docx.sh
bash scripts/build_paper3_three_docx.sh 15
bash scripts/compare_paper3_semantic_backends.sh
```

验证结论：

- `doc-compiler-engine` 默认测试通过。
- LuaTeX ignored 集成测试在本机真实 `lualatex` 下通过，并断言采集到 heading、label、ref、cite、equation、paragraph 事件。
- `doc-core` 测试通过，旧 Rust 路径未迁移、未替换。
- paper3 三路径脚本已生成 sh、rust-rule、semantic-engine 三份 DOCX。

## 6. 当前风险与后续任务

当前仍需继续推进：

- `Auto` backend selector 仍保守选择 `RuleBasedBackend`，尚未根据 `ctex` / `xeCJK` / `fontspec` / runtime 可用性自动切换。
- `LuaTeXNodeBackend` 对 `xeCJK` 模板不能直接替代 XeLaTeX；paper3 已验证为按设计 fallback。
- `SemanticEvent` 尚未上升为独立 `semantic-collector` crate 的稳定协议。
- `LayoutGraph` 只有结构预留，LuaTeX/XDV layout 事件尚未实际落地。
- DOCX renderer 仍使用现有文档图；runtime sidecar 事件当前用于报告和后续增强，还没有全面驱动 DOCX 渲染。
