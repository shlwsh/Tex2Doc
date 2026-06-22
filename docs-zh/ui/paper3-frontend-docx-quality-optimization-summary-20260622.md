# paper3 前端 DOCX 转换质量优化小结

**日期**：2026-06-22  
**范围**：`doc-desktop-slint` 前端本地转换路径、`doc-docx-writer` DOCX 写出层、`doc-mathml` 数学解析与 OMML 写出。  
**目标**：修复前端转换 `D:\papers\paper3` 后生成的 DOCX 中数学公式异常、raw LaTeX 泄漏、算法/代码块表格变形等问题，使前端本地转换结果接近 `scripts\build_paper3_compiler_engine_docx.ps1` 的质量。

## 1. 问题现象

用户通过前端选择 `D:\papers\paper3` 并转换后，生成的 `D:\output3.docx` 存在以下质量问题：

1. 数学公式不正确，部分公式显示为 raw LaTeX 或乱码。
2. 算法/代码块使用表格布局，注释列被固定宽度挤压，长注释换行后导致表格变形。
3. 代码缩进和空格在 Word 中可能被折叠，影响伪代码可读性。

## 2. 根因分析

前端界面本身不是主要原因。前端本地转换最终调用共享转换链路，质量差异主要来自 DOCX 写出层和数学解析路径：

1. `JOSBody` 正文归一化会把 `MathInline` run 合并成普通文本 run，导致内联公式没有机会进入 OMML 写出逻辑。
2. 原 `write_inline_math_run` 只是把 raw LaTeX 写进 `<m:t>`，并不是真正的 OMML 结构，Word 打开后公式容易异常。
3. `doc-mathml` 缺少 paper3 常见命令支持，例如 `\varepsilon`、`\rightarrow`、`\emptyset`、`\in`、`\ldots` 等。
4. OMML 写出曾把普通运算符写成不合适的成对结构，影响 Word 对公式的解释。
5. 算法块原来使用多列表格渲染，注释列宽固定，内容稍长就会挤压变形。
6. `write_run` 构造了 `xml:space="preserve"`，但实际没有写入该属性，导致空格/缩进无法可靠保留。
7. `itemize` 合并逻辑把列表项 run 拼成纯字符串，列表中的内联公式样式丢失，导致 `\varepsilon` 等 raw LaTeX 泄漏到普通文本节点。

## 3. 修复内容

### 3.1 DOCX 写出层

文件：`crates/docx-writer/src/serializer.rs`

1. 保留 `MathInline`、上下标等结构化 run，不再被正文归一化吞掉。
2. 内联公式改为经过 `doc-mathml::parse_latex_math` 解析后写入真实 `<m:oMath>`。
3. 修复 `write_run` 的 `xml:space="preserve"` 写出，保证缩进和连续空格保留。
4. 算法块改为 `JOSCode` 等宽文本段落，不再使用表格：
   - 行号使用文本前缀，如 ` 1 | ...`。
   - 缩进使用文本字符。
   - 注释使用 `// ...` 留在同一文本行中，让 Word 在整段宽度内自然换行。
5. 代码块按行写出 `JOSCode` 段落，避免单个长 run 造成排版不可控。
6. `itemize` 合并从纯字符串改为保留 `Run` 列表，避免列表内公式退化为 raw LaTeX。

### 3.2 数学解析与 OMML

文件：

- `crates/mathml/src/latex.rs`
- `crates/mathml/src/omml.rs`

优化内容：

1. 补齐 paper3 常见数学命令：
   - `\varepsilon`
   - `\rightarrow`
   - `\leftarrow`
   - `\emptyset`
   - `\in`
   - `\notin`
   - `\subset`
   - `\subseteq`
   - `\ldots`
   - `\dots`
   - `\%`
   - `\_`
2. 增加 `operatorname`、`textbf`、`textit` 等文本型命令的降级处理。
3. 普通数字和运算符改为普通 OMML run，避免生成不适合 `+`、`=`、`→` 等符号的结构。
4. OMML 文本节点增加 `xml:space="preserve"`。

### 3.3 桌面端转换验证

文件：`crates/desktop-slint/src/commands.rs`

在 paper3 桌面端本地转换回归测试中增加 DOCX XML 检查：

1. 断言转换后 profile 为 `jos-paper`。
2. 断言 `word/document.xml` 包含 `<m:oMath>`。
3. 断言关键 raw LaTeX 命令不再泄漏：
   - `\varepsilon`
   - `\rightarrow`
   - `\emptyset`
4. 断言算法/代码使用 `JOSCode` 段落。
5. 断言算法标题 `算法 1` 不在 `<w:tbl>` 内部。

## 4. 新增验证脚本

文件：`scripts/test_paper3_frontend_docx.ps1`

用途：

1. 运行数学解析、DOCX writer、桌面端 paper3 转换回归测试。
2. 可通过桌面端本地转换测试生成指定 DOCX。
3. 可直接检查已有 DOCX 的 XML 质量指标。

生成并验证 `D:\output3.docx`：

```powershell
.\scripts\test_paper3_frontend_docx.ps1 -GenerateDocx -DocxPath D:\output3.docx
```

如果已经通过 GUI 前端重新生成了 `D:\output3.docx`，只检查结果：

```powershell
.\scripts\test_paper3_frontend_docx.ps1 -SkipCargo -DocxPath D:\output3.docx
```

脚本检查项：

1. raw LaTeX 命令泄漏计数。
2. OMML 公式数量。
3. 表格数量与表格单元格数量。
4. 是否存在 `JOSCode` 段落。
5. 算法标题是否仍在表格内。

## 5. 已完成验证

已执行并通过：

```powershell
cargo test -p doc-mathml -- --nocapture
cargo test -p doc-docx-writer inline_math_uses_parsed_omml_not_raw_latex -- --nocapture
cargo test -p doc-docx-writer algorithm_serializes_as_text_block -- --nocapture
cargo test -p doc-docx-writer itemize_merge_preserves_inline_math_runs -- --nocapture
cargo test -p doc-desktop-slint paper3_conversion_overrides_stale_chinese_profile -- --nocapture
cargo check -p doc-desktop-slint
.\scripts\test_paper3_frontend_docx.ps1 -GenerateDocx -DocxPath E:\tmp\paper3-frontend-output.docx
```

脚本对生成 DOCX 的检查结果：

```text
OMML equations : 192
tables         : 55
table cells    : 978
JOSCode style  : True
algorithm tbl  : False
```

说明：

1. `OMML equations: 192` 表示内联公式已经大量进入 OMML，而不是 raw LaTeX 文本。
2. `algorithm tbl: False` 表示算法标题不再位于表格中，算法块已切换为文本段落渲染。
3. `tables: 55` 仍然存在是正常现象，这些主要来自论文正文真实表格，不是算法块生成的表格。

## 6. 结论

本次优化后，前端本地转换 paper3 的核心质量问题已经修复：

1. 内联公式不再被正文归一化吞掉。
2. 关键数学命令不再作为 raw LaTeX 泄漏到 DOCX 普通文本中。
3. 内联数学输出真实 OMML，Word 端公式渲染稳定性明显提升。
4. 算法块不再使用表格，长注释不会挤压固定列宽导致变形。
5. 代码和算法文本保留缩进与空格，阅读效果更接近纯文本伪代码。
6. 新增脚本可稳定复现前端同路径转换并检查 DOCX XML 质量，后续可作为 paper3 前端转换质量门禁。

## 7. 后续建议

1. 将 `scripts/test_paper3_frontend_docx.ps1 -GenerateDocx` 纳入 Windows 本地发布前检查。
2. 后续若继续扩展公式能力，应优先补 `doc-mathml` 的 AST 与 OMML 结构，而不是在 DOCX 写出层做字符串替换。
3. 如果需要进一步降低表格单元格数量，应单独分析正文真实表格的降级策略，不应再把算法块恢复为表格。
