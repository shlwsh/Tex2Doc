# Semantic TeX Engine Auto Selector 开发进展报告（20260620-125915）

## 1. 本轮目标

本轮继续推进双后端语义采集方案，重点补齐 `SemanticBackendKind::Auto` 的真实选择能力。实现要求仍然是新语义引擎另起炉灶，不改变现有 `doc-core` / `doc-engine convert` Rust 规则转换路径。

## 2. 已实现内容

### 2.1 Auto backend selector

`doc-compiler-engine` 已新增模板特征扫描与运行时可用性判断：

```text
VirtualFs
  -> 扫描 .tex/.sty/.cls/.ltx
  -> TemplateSignals
  -> RuntimeAvailabilitySnapshot
  -> AutoBackendSelection
```

当前选择规则：

| 条件 | 选择 |
|---|---|
| 检测到 `ctex` / `xeCJK` / `fontspec` / XeTeX 字体命令，且 `xelatex` 可用 | `XeLaTeXHookBackend` |
| 检测到 XeTeX-only 特征，但 `xelatex` 不可用 | `RuleBasedBackend` |
| 检测到 LuaTeX 特征，且 `lualatex` 可用 | `LuaTeXNodeBackend` |
| 通用 LaTeX，且 `lualatex` 可用 | `LuaTeXNodeBackend` |
| 通用 LaTeX，`lualatex` 不可用但 `xelatex` 可用 | `XeLaTeXHookBackend` |
| 无可用 runtime | `RuleBasedBackend` |

paper3 是 `ctex` / `xeCJK` 模板，因此 Auto 会选择 `xelatex-hook`，不会误切到 LuaTeX。

### 2.2 fallback 语义修正

`BackendSelectionReport::fallback_from` 现在记录真实失败或不可用的 backend，而不是简单记录用户请求值。

示例：

```text
requested: auto
selected: rule-based
fallback_from: luatex-node
```

这能区分：

- 用户显式请求 `luatex-node` 后失败。
- `auto` 选择了 `luatex-node` 后失败。
- `auto` 直接选择了 `rule-based`。

### 2.3 测试覆盖

新增 selector 单元测试：

```text
auto_selector_prefers_xelatex_for_xecjk_templates
auto_selector_prefers_luatex_for_generic_templates
auto_selector_keeps_xecjk_templates_off_luatex_without_xelatex
```

原有 source/zip 编译测试显式使用 `RuleBasedBackend`，避免测试结果依赖本机是否安装 TeX runtime。

## 3. paper3 验证

三路径验证命令：

```bash
bash scripts/build_paper3_three_docx.sh 15
```

输出报告：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-125728-three-docx-report.md
```

DOCX 产物：

| 路径 | 文件 | 大小 | media |
|---|---|---:|---:|
| sh | `v15-论文稿件-jos-sh-20260620-125728.docx` | 3,079,377 bytes | 10 |
| rust-rule | `v15-论文稿件-jos-20260620-125728-rust-rule.docx` | 3,055,363 bytes | 10 |
| semantic-engine | `v15-论文稿件-jos-20260620-125728-semantic-engine-xelatex_hook.docx` | 3,055,688 bytes | 10 |

语义后端对比命令：

```bash
bash scripts/compare_paper3_semantic_backends.sh
```

输出报告：

```text
examples/paper3/output/to-docx/semantic-backends-20260620-125747-report.md
```

结果：

| requested | selected | fallback_from | 文件 | 大小 | media |
|---|---|---|---|---:|---:|
| auto | xelatex-hook |  | `paper3-20260620-125747-auto.docx` | 3,055,688 bytes | 10 |
| rule-based | rule-based |  | `paper3-20260620-125747-rule_based.docx` | 3,055,688 bytes | 10 |
| xelatex-hook | xelatex-hook |  | `paper3-20260620-125747-xelatex_hook.docx` | 3,055,688 bytes | 10 |
| luatex-node | rule-based | luatex-node | `paper3-20260620-125747-luatex_node.docx` | 3,055,688 bytes | 10 |

结论：

- `auto` 已在 paper3 上自动选择 `xelatex-hook`。
- `luatex-node` 作为独立后端仍可显式验证；由于 paper3 依赖 `xeCJK`，失败后按设计 fallback 到 `rule-based`。
- 旧 `rust-rule` 路径仍由 `doc-engine convert` 生成，未被新语义引擎替换。

## 4. 已执行验证

```bash
cargo fmt -p doc-compiler-engine
cargo test -p doc-compiler-engine
cargo test -p doc-compiler-engine luatex_runtime_collects_semantic_events -- --ignored --nocapture
cargo test -p doc-core
bash scripts/build_paper3_three_docx.sh 15
bash scripts/compare_paper3_semantic_backends.sh
```

结果：

```text
doc-compiler-engine: 10 passed, 1 ignored
doc-compiler-engine luatex ignored integration: 1 passed
doc-core: 5 passed
paper3 three-docx: sh/rust-rule/semantic-engine generated
paper3 semantic backend compare: auto/rule-based/xelatex-hook/luatex-node generated
```

## 5. 当前边界

- `Auto` selector 已落地，但只是模板特征启发式，不是完整兼容性分析器。
- `LuaTeXNodeBackend` 已能采集 macro events 与段落事件，但尚未输出页码、坐标、盒模型和行聚类。
- runtime sidecar events 目前进入 `DocumentGraph.semantic_events`，尚未全面驱动 DOCX renderer。
- `doc-core`、`doc-engine convert` 和新 `doc-compiler-engine` 仍是独立路径，便于继续对照验证。

## 6. 下一步

建议进入：

```text
T2 双路径对比脚本与报告
T3 Profile 规则表
```

重点是把当前三路径生成能力扩展成更细的差异报告，并把 JOS、中文学术、医学期刊规则收敛到新语义引擎的 profile 层。
