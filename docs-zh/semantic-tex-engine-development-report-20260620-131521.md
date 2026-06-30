# Semantic TeX Engine ProfileSpec 开发报告（20260620-131521）
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



## 1. 本轮目标

本轮完成 T3 初版：把 `EngineProfile` 从简单枚举扩展为可查询的 profile 规则表，并将 JOS 页面设置接入新 `doc-compiler-engine` 路径。旧 `doc-core` / `doc-engine convert` 路径保持独立。

## 2. 代码实现

`crates/compiler-engine/src/lib.rs` 新增：

```rust
ProfileSpec
PageSetupProfile
FontPolicySpec
CaptionPolicySpec
CitationPolicySpec
ProfileSpecReport
```

`EngineProfile` 新增：

```rust
EngineProfile::spec()
```

当前内置 profile：

| profile | 默认页面 | 文档类 | caption | 引用 |
|---|---|---|---|---|
| `generic-article` | default | `article/report/book` | `Figure/Table/Equation` | `numeric/plain` |
| `chinese-academic` | A4 | `ctexart/ctexrep/ctexbook/article` | `图/表/式` | `numeric-compressed/gbt7714-like` |
| `jos-paper` | `jos-paper3` | `rjthesis/ctexart` | `图/表/式` | `numeric-super-compressed/unsrt` |
| `medical-journal` | A4 | `article/elsarticle/wlscirep` | `Fig./Table/Equation` | `numeric/vancouver-like` |

## 3. 渲染接入

`CompileOptions` 新增：

```rust
effective_page_setup()
```

行为：

- 如果调用方显式传入 `page_setup`，继续使用显式覆盖。
- 如果未传入，则使用 `EngineProfile::spec().default_page_setup()`。
- `JosPaper` 会返回 `PageSetup::jos_paper3()`。
- `GenericArticle` 保持 writer 默认页面。

`CompileReport` 新增：

```rust
profile_spec: ProfileSpecReport
```

用于输出本次编译使用的 profile 规则摘要。

## 4. paper3 example 调整

`crates/compiler-engine/examples/paper3_to_docx.rs` 已去掉：

```rust
page_setup: Some(PageSetup::jos_paper3())
```

现在由：

```rust
profile: EngineProfile::JosPaper
```

间接提供 JOS 页面设置。

example 输出新增：

```text
profile-id: jos-paper
profile-page-setup: jos-paper3
```

## 5. 验证结果

已执行：

```bash
cargo fmt -p doc-compiler-engine
cargo test -p doc-compiler-engine profile
cargo test -p doc-compiler-engine
cargo test -p doc-core
bash -n scripts/compare_paper3_dual_engines.sh
bash -n scripts/build_paper3_three_docx.sh
bash scripts/compare_paper3_dual_engines.sh 15
```

结果：

```text
doc-compiler-engine profile: 2 passed
doc-compiler-engine: 12 passed, 1 ignored
doc-core: 5 passed
paper3 dual engines: generated comparison report and text diff
```

paper3 最新对比报告：

```text
examples/paper3/output/to-docx/v15-论文稿件-jos-20260620-131459-dual-engines-comparison-report.md
```

semantic log 摘要：

```text
backend-requested: auto
backend-selected: xelatex-hook
profile-id: jos-paper
profile-page-setup: jos-paper3
```

## 6. 当前边界

- Profile 规则仍为 Rust 内置表，尚未外置到 YAML/TOML。
- `ProfileSpec` 当前主要驱动页面设置和报告输出；字体、caption、引用策略已建模，但尚未全面驱动 DOCX renderer。
- `doc-core` 旧路径未接入 `ProfileSpec`，仍保持独立稳定基线。

## 7. 下一步

建议进入 T4：实现 `ReferenceGraph`，结构化 label/ref/cite，并为后续 DOCX bookmark/hyperlink 做准备。
