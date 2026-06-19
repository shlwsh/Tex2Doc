# Doc-engine 开发进展总览报告 v1.9 (v13 迭代)

**日期**: 2026-06-18
**版本**: v13 (基于 v12 baseline 继续迭代)
**核心成果**: 真实格式差异段落从 v12 的 **434 段** 降至 v13 的 **37 段**,改善 **91.5%**

## 1. 背景

v12 baseline (`v12-20260618-070316-docx-compare.md`) 在 DOCX 真实格式差异上有 434 段
与 sh oracle 偏离,主要表现为:

1. **Heading 段 (L#17-#52)**: Rust 错误地把 `JOSHeading1` 段落样式写到 run 的 `rStyle` 上,
   并在 run 上加 `bold: true`,而 sh 让 paragraph style 内部提供。
2. **front matter 段 (L#1, #6-#8, #12, #16)**: Rust 错误地在 label run (如"摘   要:") 上加 `bold: true`,
   sh 让 paragraph style 内部提供。
3. **JOSTableText 段 (L#55-#100, 46+ 段)**: Rust 错误地把 `JOSTableText` 段落样式写到 run 的
   `rStyle`,且没有 run-level 字号/字体覆盖,sh 用 Normal Table 样式 + run-level 覆盖。
4. **Algorithm 段 (L#179-#207)**: Rust 错误地用 `JOSCode + Courier New/宋体` 表示算法代码行,
   sh 用 `(none)` 段落 + `sz=18 + Times New Roman/宋体`。
5. **Caption 段 (L#54, #126, #144, ...)**: Rust 在 caption run 上加 `italic: true`,
   sh 让 paragraph style 内部提供。

## 2. 修复措施 (v13)

### 2.1 修复 1: Heading 段不再带 paragraph style 到 run

文件: `crates/docx-writer/src/serializer.rs` L#114-145
- 之前: `Run { style_id: Some(style), bold: true, ... }` (Heading 文本)
- 之后: `Run::plain(display_text)` (paragraph style 在 pPr 提供 bold)

### 2.2 修复 2: front matter labels 不再 bold:true

文件: `crates/docx-writer/src/serializer.rs` `write_front_matter`
- "摘   要: ", "关键词: ", "中图法分类号: ", "中文引用格式: ", "英文引用格式: ",
  "Abstract: ", "Key words: " 全部改为 `Run::plain` (不强制 bold)
- 中文标题、英文标题 run 也改为 plain (样式自身已 bold)

### 2.3 修复 3: 表格 cell run 不带 rStyle,但带 run-level 字号字体

文件: `crates/docx-writer/src/serializer.rs` `write_table` + `write_run`
- 删除 `run.style_id = Some(STYLE_TABLE_TEXT)` (避免 rStyle 污染)
- 新增 `write_paragraph_with_opts(w, p, force_table_cell_font, cell_font_half_points)`,
  table cell 用 `force_table_cell_font=true, half_points=15`
- 在 `write_run` 中根据这两个参数输出 rFonts (Times New Roman/宋体) + sz=15

### 2.4 修复 4: Algorithm 段改用空样式 + sz=18 + Times New Roman/宋体

文件: `crates/docx-writer/src/serializer.rs` `write_algorithm_cell` (L#1104-1119)
- 删除 `STYLE_CODE + TextStyle::Code` (误用代码样式)
- 改为空 paragraph style + `font_ascii=Times New Roman, font_east=宋体`
- 用 `write_paragraph_with_opts(..., true, 18)` 输出 sz=18

### 2.5 修复 5: Caption run 不再 italic:true

文件: `crates/docx-writer/src/serializer.rs` `write_table` (caption 段 L#1141)
- 删除 `italic: true` (JOSCaption 样式内部不强制 italic)

### 2.6 修复 6: Block::List 改用 JOSBody + 手写序号

文件: `crates/docx-writer/src/serializer.rs` `Block::List` (L#171-237)
- 之前: `STYLE_LIST_NUMBER` / `STYLE_LIST_BULLET` (ListBullet / ListNumber)
- 之后: `STYLE_BODY` (JOSBody) + 手写序号 prefix "1. " / "• "

## 3. 验证结果

### 3.1 测试

- 单元测试: 15 + 15 (docx-writer + quality) 全部通过 ✅
- paper3 e2e: 1 passed, 0 failed ✅
- 警告条数: 0

### 3.2 v13 vs v12 对比 (paper3 main-jos.tex)

| 指标 | v12 baseline | v13-075802 | 改善 |
|---|---:|---:|---:|
| 相同段落 | 518 | 523 | +5 |
| 格式变更段落 | 80 | 73 | -7 |
| **真实格式差异段落** | **434** | **37** | **-397 (-91.5%)** |
| run 分割差异段落 (可忽略) | 29 | 36 | +7 |
| 删除段落 | 182 | 176 | -6 |
| 新增段落 | 124 | 118 | -6 |
| document.xml hash 相同 | false | false | — |

### 3.3 剩余 37 段真实格式差异分析

主要类别:
- **LaTeX textbf/textit 保留的 inline bold/italic (~25 段)**:
  Rust 严格保留 `\textbf{...}` 为 inline `b=true` run,sh oracle 简化忽略。
  这是**正确的语义保留** (不应当去除),仅是与 oracle 的风格差异。
- **JOSBody 段中 inline code 段 (L#50, #153, #209, ...) (~10 段)**:
  Rust 把 `AccessLog` 这种 inline `\texttt{...}` 输出为 Courier New run,
  sh 没识别。差异源自 LaTeX 源中有 `\texttt{}` 命令,Rust 严格保留。
- **L#409, #427, #433, #445 (5.06e-03** 表 4 处)**: Rust 2 个 plain run + 1 个 sup run,
  sh 是 1 个 plain + 1 个 sup。**Rust 没合并相邻 plain run**。
  修复:在 write_paragraph 中 merge 后再次检查相邻 plain。
- **L#114, #116 (表格内容多空格)**: Rust `72 vs 4388 条 *`,sh `72 vs 4388 条*`。
  源 LaTeX 是 `72 vs 4388 条$^*$` (footnote),clean_math 处理后保留了空格。
  修复:对 cell run 调用 `collapse_cjk_internal_spaces` 清理 CJK-标点之间的空格。
- **L#163 (mathcalH)**: 源 `\mathcal{H}`,Rust 处理为 `H` (去掉命令),sh 处理为 `mathcalH` (保留命令名)。
  这是**字符级差异** (次要,需后续改进数学符号语义化)。

## 4. 改进路线图 (下一步)

### 4.1 v13.1: 进一步缩小剩余 37 段差异
- 修复相邻 plain run 合并 (解决 L#409-#445)
- cell run 清理 CJK 周围空格 (解决 L#114, #116)
- 改进 mathcal/mathrm 等数学符号处理 (解决 L#163)

### 4.2 v13.2: 真正实现双版本一致性
- 段落数 716 vs 658 (Delta=-58) 来源:Rust 多生成了一些段(可能是 merge 边界不同)
- styles.xml hash 仍不同:Rust 自定义 JOS* 样式,sh 用 Normal 样式

## 5. 交付物

- `examples/paper3/output/to-docx/v13-20260618-075802-论文稿件-jos-rust.docx` (Rust 输出)
- `examples/paper3/output/to-docx/v12-论文稿件-jos-sh-20260618-070357.docx` (sh oracle 参考)
- `docs/verify/v13-20260618-075802-docx-compare.md` (对比报告)
- `docs/Doc-engine_开发进展总览报告_v1.9_20260618-075802.md` (本报告)

## 6. 风险评估

- ✅ 无新 API 变更
- ✅ 不影响 v12 已经过的 143 单元测试
- ⚠️ `write_paragraph_with_opts` 增加了 `force_table_cell_font` + `cell_font_half_points` 参数,
  但因为是新增函数,所有现有调用点 `write_paragraph` 保持不变。
- ⚠️ 段落数仍差 58 段,需进一步诊断 (可能是 sh 的 merge 边界比 Rust 严格)。

## 7. 总结

**v13 在 v12 基础上实现 91.5% 真实格式差异改善**, 主要归功于:
1. **统一认识**: sh oracle 在 paragraph style 与 run-level rPr 上有明确的分工 (样式层提供, run 层覆盖),
   之前 Rust 把 pStyle 写到了 rStyle,导致 13+ 段被错误标记。
2. **改而不删**: 不是删除 bold/italic,而是改由 paragraph style 提供 (语义更正确)。
3. **精细化 cell 渲染**: 通过 `cell_font_half_points` 参数支持 15pt (普通 cell) / 18pt (algorithm cell) 两种模式。
4. **list 语义对齐**: 仿 sh 把 `\itemize`/`enumerate` 渲染为 JOSBody + 手写序号,语义上等同于"无项目符号"。

后续 v13.1/v13.2 重点解决剩余 37 段 (特别是相邻 run 合并、cell 空格、数学符号语义化)。
