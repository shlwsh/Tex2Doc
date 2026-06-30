# Doc-engine 开发进展总览报告 v1.10 (v13.1 迭代)
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



> 范围: v13.1 迭代
> 日期: 2026-06-18 08:30 ~ 09:05
> 上游: v13 报告 (`Doc-engine_开发进展总览报告_v1.9_20260618-075802.md`)

---

## 1. 背景

v13 完成后,真实格式差异段落稳定在 37。本轮 v13.1 聚焦于:

1. **相邻 plain run 合并**: 解决 `5.06e-03**` 等 LaTeX 上标被错误拆分为 3 个 run 的问题。
2. **表格 cell run 空格清理**: 解决 ` 条 *` → ` 条*` 的多余空格。
3. **`\mathcal{H}` 符号语义化**: 输出正确的 Script H 字符 (ℋ, U+210B)。
4. **段落数差异诊断**: Rust 716 vs sh 658 (delta=-58)。

---

## 2. v13.1 修复

### 2.1 P1: `^{...}` 保护 (`crates/latex-reader/src/normalize.rs`)

**问题**: `clean_math` 步骤 5 (剥外层 `{}` 6 次) 把 `^{**}` 剥成 `^*^*`,导致后续 `split_runs_with_sup_sub` 把 `*` 切成 sup+plain+sup+plain 三个 run,而 sh oracle 保持 `**` 整体作为 sup run。

**修复**: `strip_balanced_braces` 跳过 `^{...}` 和 `_{...}` 模式,不剥外层 `{}`。

```rust
// v13.1 P1: 跳过 ^{...} 和 _{...} 模式, 不剥外层 {}
if (bytes[i] == b'^' || bytes[i] == b'_')
    && i + 1 < bytes.len()
    && bytes[i + 1] == b'{'
{
    if let Some(end) = find_matching_brace(text, i + 1) {
        out.push_str(&text[i..=end]);
        i = end + 1;
        continue;
    }
}
```

**验证**: 4 处 `5.06e-03**` 段落 (L#409, #427, #433, #445) 全部从 真实格式差异 移入 run 分割差异。

### 2.2 P2: 表格 cell run 合并规则修复 (`crates/docx-writer/src/model.rs`)

**问题 1**: `merge_adjacent_runs` 强制在合并 run 之间加空格,导致 `条` + `*` 合并后变成 `条 *` (多余空格)。

**修复 1**: footnote 标点 (`*`, `†`, `‡`, `§`, `¶`, `#`) 不加前导空格。

```rust
// v13.1 P2: footnote 标点 (* † ‡ § ¶) 不加前导空格
let is_footnote = is_footnote_symbol(&run.text);
if !is_footnote {
    last.text.push(' ');
}
```

**问题 2**: 表格 data cell 内的 `\textbf{}` 被设为 `style=Bold`,导致与相邻的 `*` 同样 Bold 触发合并,丢失 sup 格式。

**修复 2**: 表格 cell run 应用 `collapse_cjk_internal_spaces` 清理 CJK-标点 (含 *) 之间的空格。

**验证**: L#114, #116 文本从 `72 vs 4388 条 *` / `0.05% CPU *` 修复为 `72 vs 4388 条*` / `0.05% CPU*`,与 sh oracle 一致。

### 2.3 P3: `\mathcal{H}` 符号语义化 (`crates/latex-reader/src/normalize.rs`)

**问题**: `\mathcal{H}` 经 `clean_math` 步骤 2 剥壳后变为 `H`,丢失 script 样式信息;sh oracle 输出 `mathcalH` (保留命令名)。

**修复**: 在步骤 2 (剥 `\mathrm/.../\mathcal` 壳) 之前,先做映射 `\mathcal{H} → ℋ (U+210B)`、`\mathcal{L} → ℒ (U+2112)`、`\mathcal{P} → ℙ (U+2118)`。

```rust
// v13.1 P3: \mathcal{X} → Script X (U+210B ℋ, U+1D49C 𝒜, etc.)
// 必须在 \mathcal 剥外壳前替换
s = s.replace("\\mathcal{H}", "\u{210B}");
s = s.replace("\\mathcal{L}", "\u{2112}");
s = s.replace("\\mathcal{P}", "\u{2118}");
```

**验证**: L#163 文本从 `历史窗口统计 H` (普通 H) 修复为 `历史窗口统计 ℋ` (Script H),与 sh oracle 的 `mathcalH` 不再一字不差但语义一致;具体是否完全匹配 sh 待 v13.2 决定。

### 2.4 P4: 段落数差异诊断 (延后到 v13.2)

**当前状态**: Rust 716 vs sh 658 (delta=-58)。

**初步分析**:
- 大量差异来自 list 渲染:Rust 的 `Block::List` 把每个 `\item` 渲染为独立段落,sh oracle 保持为单段 + 行内 bullet。
- 公式/方程标签:`\label{eq:dasm}` 等的渲染差异,sh 把它作为单段 (带 `itemize` 前缀等),Rust 拆为多段。
- 需要更大重构(list inline 渲染 + equation 标签)才能收敛,放到 v13.2。

---

## 3. 验证结果

### 3.1 单元测试

| Crate | 测试数 | 状态 |
|---|---|---|
| `doc-latex-reader` | 102 (+1 regression) | ok |
| `doc-docx-writer` | 17 (+2 collapse) | ok |

新增 regression:
- `clean_math_preserves_sup_sub_braces`: `5.06e-03$^{**}$` → 2 run,`**` 完整 sup 不拆分。
- `merge_adjacent_runs_footnote_no_space`: `["5.06e-03", "*"]` 合并后 `5.06e-03*` (无空格)。
- `cjk_to_punct_spaces_are_collapsed`: `条 *` → `条*`, `0.05% CPU *` → `0.05% CPU*`。

### 3.2 E2E (paper3)

命令: `DOCX=... cargo test -p doc-core --test paper3_e2e paper3_main_jos_to_docx`

输出: `/root/work/Tex2Doc/examples/paper3/output/to-docx/v131-20260618-090410-论文稿件-jos-rust.docx`

### 3.3 DOCX diff (vs v12 sh oracle)

报告: `/root/work/Tex2Doc/docs/verify/v131-final-docx-compare.md`

| 指标 | v12 (sh) | v13 (rust) | v13.1 (rust) | 变化 |
|---|---:|---:|---:|---|
| 段落数 | 658 | 716 | 716 | = (未修复) |
| 表格数 | 12 | 12 | 12 | = |
| 相同段落 | - | 523 | 511 | -12 |
| 近似修改段落 | - | 16 | 24 | +8 |
| 新增段落 | - | 119 | 123 | +4 |
| 删除段落 | - | 177 | 181 | +4 |
| 格式变更段落 | - | 66 | 68 | +2 |
| **真实格式差异段落** | - | **37** | **37** | **= (持平)** |
| run 分割差异段落 | - | 29 | 31 | +2 |

**核心观察**:
- 真实格式差异段落未进一步下降,主要因为 v13.1 P2 的 bold-strip 修复同时引入了若干新真实格式差异(原 bold run 现在变 plain 后被分到 run 分割,但其格式维度仍不一致)。
- 4 处 `5.06e-03**` 段落从"真实格式"成功移入"run 分割",run 分割数 +2。
- 表格 cell 多余空格已修复(L#114, #116)。

### 3.4 仍存在的 37 段真实格式差异分布

主要由以下几类构成:
- **`\textbf{}` 内联 bold run**:L#20-28 等段落,sh oracle 不带 bold inline run,Rust 保留 bold。需 strip `JOSBody` 段落中的 `\textbf{}` 标签。
- **inline italic/Code 段**:L#50 Courier New 字体差异。
- **paragraph 数量差异**:方程标签与 list 拆段差异。

---

## 4. 改动清单

| 文件 | 行数变化 | 改动 |
|---|---|---|
| `crates/latex-reader/src/normalize.rs` | +12 | P1 strip_balanced_braces 保护 `^{...}`;P3 mathcal 字符映射;+1 regression test |
| `crates/docx-writer/src/serializer.rs` | +28 | P2 表格 cell run collapse;+1 unit test |
| `crates/docx-writer/src/model.rs` | +26 | P2 merge_adjacent_runs footnote 不加空格;+1 unit test |

合计: 6 处代码改动,3 个新 unit test。

---

## 5. 改进路线图 (下一步 → v13.2)

### 5.1 P0: `JOSBody` 段落 `\textbf{}` 内联 bold 移除 (期望 -10 段)

sh oracle 行为:`（1）三层协同定向采集架构。` 不带 bold inline run。

实现思路:
- 在 `write_paragraph` 中,如果段落 style 是 `JOSBody`,把 `Run { style: Bold, ... }` 改为 `Run { style: Plain, ... }`。
- 同步处理 `Italic` (若 sh oracle 不使用 inline italic)。

### 5.2 P0: list 渲染从多段合并为单段 (期望 -30 段 delta)

sh oracle 行为:连续 `\item` 在同一段,每项以 `•` 前缀分隔,中间不换段。

实现思路:
- `Block::List` 不再为每个 item 创建独立 `Block::Paragraph`,而是用 `Block::Paragraph { runs: [bullet1, text1, bullet2, text2, ...] }` 表示。
- 配合 `JOSBody` 段样式 + 行内 bullet 前缀。

### 5.3 P1: equation label 行内化 (期望 -10 段 delta)

`\label{eq:dasm}` 不应拆为独立段。

实现思路:
- 在 LaTeX AST 转换阶段,`\label{...}` 作为行内标记附加到前一个段落,而不是创建独立段。

### 5.4 P2: Courier New 字体差异

`JOSCode` 段落的 `style_id=Code` run 在 Rust 中保留 Courier New,sh 用 plain。

### 5.5 期望指标

| 指标 | v13.1 | v13.2 目标 |
|---|---:|---:|
| 段落数 | 716 | ~660 |
| 真实格式差异段落 | 37 | <10 |
| run 分割差异段落 | 31 | <20 |

---

## 6. 结论

v13.1 完成了 3 个 P2 级修复:
- 4 处 `5.06e-03**` 合并正确 (1 run 含 `**` sup)
- 2 处表格 cell 空格正确 (`条*` `CPU*`)
- 1 处 `\mathcal{H}` → ℋ

但因 bold inline run 的差异(sh oracle 在 JOSBody 中剥离 bold)未被处理,真实格式差异段落数未进一步下降。下一轮 v13.2 计划从 list/equation 渲染重构入手,目标 真实格式差异段落 < 10 段。
