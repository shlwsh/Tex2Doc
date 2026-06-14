# 关键技术 3：CST → 语义 AST 降级

> 本节深入解析 `doc-latex-reader::lower`（Pass-3）。这是整个 Tex2Doc **最复杂、最容易改错**的部分。读完应能：理解所有 `Block::*` 变体的来源、知道如何加新命令、知道如何加新环境。

---

## 1. 入口与调用链

```rust
// crates/latex-reader/src/lib.rs
pub use lower::lower_to_document;

// crates/core/src/convert.rs
let parse = parse_tex(&joined.text);
let doc = lower_to_document(&parse, Some(&joined));
```

```rust
// crates/latex-reader/src/lower.rs
pub fn lower_to_document(parse: &Parse, joined: Option<&JoinedStream>) -> Document {
    let mut owned = MacroMap::new();
    lower_with_macros(parse, joined, &mut owned)
}

pub fn lower_with_macros(parse, joined, macros) -> Document {
    let mut numbering = NumberingState::default();
    lower_with_macros_and_numbering(parse, joined, macros, &mut numbering)
}
```

* `lower_to_document`：自包含宏表 + 全新编号状态。
* `lower_with_macros`：共享宏表（跨段收集）。
* `lower_with_macros_and_numbering`：内部使用，便于测试时复用编号。

---

## 2. 流水线（高阶）

```rust
pub fn lower_with_macros_and_numbering(
    parse: &Parse,
    joined: Option<&JoinedStream>,
    macros: &mut MacroMap,
    numbering: &mut NumberingState,
) -> Document {
    let text = joined.map(|j| j.text.clone()).unwrap_or_else(|| parse.source.clone());

    // 1) 宏展开
    let text = expand_macros_in(&text, macros);

    // 2) 跳过 preamble
    let text = strip_preamble(&text);

    let mut doc = Document::new();
    let mut buffer = String::new();
    let mut buffer_start = 0u32;
    let default_span = Span::default();
    let mut pos: usize = 0;
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut cite_numbers: HashMap<String, usize> = HashMap::new();

    while pos < len {
        // 防御：pos 必须落在 char 边界上（CJK 字符可能让某些路径产生非边界 offset）
        if !text.is_char_boundary(pos) {
            let mut next = pos + 1;
            while next < len && !text.is_char_boundary(next) { next += 1; }
            pos = next;
            continue;
        }

        // 跳过 ASCII 空白 / 注释
        if let Some(next) = skip_whitespace_and_comment(text, pos) {
            if next != pos { pos = next; continue; }
        }

        // 环境优先
        if let Some((name, body, end)) = scan_environment(text, pos) {
            flush_paragraph(...);
            let blk = lower_environment(name, body, default_span, macros, numbering);
            doc.push(blk);
            pos = end;
            continue;
        }

        // 段命令
        if let Some((consumed, block)) = try_top_level_command(&text[pos..], default_span, numbering) {
            flush_paragraph(...);
            doc.push(block);
            pos += consumed;
            continue;
        }

        // 元数据命令
        if let Some((consumed, _)) = try_top_level_metadata_command(&text[pos..]) {
            flush_paragraph(...);
            pos += consumed;
            continue;
        }

        // 取一行
        let nl = text[pos..].find('\n').map(|n| pos + n + 1).unwrap_or(len);
        let line = &text[pos..nl];
        let stripped = strip_inline(line, &mut cite_numbers);
        let trimmed = stripped.trim();

        if trimmed.is_empty() {
            flush_paragraph(...);
        } else {
            if buffer.is_empty() { buffer_start = pos as u32; }
            buffer.push_str(&stripped);
            buffer.push('\n');
        }
        pos = nl;
    }

    flush_paragraph(...);
    doc
}
```

---

## 3. 关键模块

### 3.1 宏展开（`expand.rs`）

* **算法**：单 pass，扫描 `\newcommand` / `\providecommand` / `\renewcommand`，加入宏表。
* **宏调用替换**：按「单词边界」判定，`\X` 紧接非字母才替换。
* **不识别 `#1` 实参**：V1 简化，仅做字面替换。
* **跨段共享**：`outer` 收集的宏表可被 `inner` 段（rjabstract / rjtitle 等）复用。

```rust
pub fn expand_macros_in(text: &str, macros: &mut MacroMap) -> String {
    // ...
    for byte 扫描：
        if 当前是 \ 开头：
            读命令名
            if 命令名是 newcommand/...：
                parse_definition_end 把 name → body 加入 macros
                // 整段吞到行末（除非同行还有内容）
                continue
            if 命令名是已知名：
                if 单词边界成立：
                    替换为 body
                    continue
            // 其它：原样写入
        // fallthrough：逐字符输出
    out
}
```

### 3.2 Preamble 剥离（`strip_preamble`）

```rust
fn strip_preamble(text: &str) -> &str {
    let needle = "\\begin{document}";
    match text.find(needle) {
        Some(idx) => {
            let after = idx + needle.len();
            let bytes = text.as_bytes();
            let mut p = after;
            while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t' || bytes[p] == b'\n' || bytes[p] == b'\r') {
                p += 1;
            }
            &text[p..]
        }
        None => text,
    }
}
```

* 找到 `\begin{document}` 后，跳过尾部空白，返回其后的内容。
* 找不到 → 返回原文（视为退化）。
* 跳过 `\documentclass{ctexart}` / `\usepackage{...}` 等 preamble 命令。

### 3.3 顶层段命令（`try_top_level_command`）

```rust
fn try_top_level_command(s: &str, span: Span, numbering: &mut NumberingState) -> Option<(usize, Block)> {
    let prefixes: &[(&str, PrefixHandler)] = &[
        ("\\section", |b, sp, n| Block::Heading { level: 1, text: b.to_string(), number: Some(n.next_heading(1)), span: sp }),
        ("\\subsection", |b, sp, n| Block::Heading { level: 2, ..., number: Some(n.next_heading(2)), ... }),
        ("\\subsubsection", |b, sp, n| Block::Heading { level: 3, ..., number: Some(n.next_heading(3)), ... }),
        ("\\paragraph", |b, sp, n| Block::Heading { level: 4, ..., number: Some(n.next_heading(4)), ... }),
        ("\\caption", |b, sp, _n| Block::Paragraph { runs: vec![TextRun { text: b.to_string(), style: TextStyle::default(), span: sp }], span: sp }),
    ];
    for (prefix, handler) in prefixes {
        if let Some(rest) = s.strip_prefix(prefix) {
            let trimmed = rest.trim();
            if trimmed.strip_prefix('{').is_some() {
                if let Some(end) = find_matching_brace(trimmed, 0) {
                    let slice_end = end + 1;
                    if slice_end > trimmed.len() || !trimmed.is_char_boundary(slice_end) { return None; }
                    let inner = &trimmed[1..slice_end];
                    let consumed = prefix.len() + (rest.len() - trimmed.len()) + end + 2;
                    return Some((consumed, handler(inner, span, numbering)));
                }
            }
            return Some((prefix.len(), handler("", span, numbering)));
        }
    }
    None
}
```

* 5 个段命令：section / subsection / subsubsection / paragraph / caption。
* 自动编号：`NumberingState` 维护 `1.1.1` / `图 1` / `表 1`。

### 3.4 顶层元数据命令（`try_top_level_metadata_command`）

50+ 命令列表：

```rust
const META_CMDS: &[&str] = &[
    "rjtitle", "rjauthor", "rjinfor", "rjhead", "rjkeywords", "rjcategory", "rjmaketitle",
    "fancyhead", "fancyfoot", "fancyhf",
    "bibliographystyle", "bibliography",
    "hypersetup", "graphicspath",
    "newCJKfontfamily",
    "providecommand", "newcommand", "renewcommand",
    "setlength", "geometry", "PassOptionsToClass",
    "documentclass", "usepackage",
    "newif", "newcounter", "newlength", "newenvironment", "newtheorem", "newlabel",
    "pagestyle", "thispagestyle",
    "linespread", "fontsize", "selectfont",
    "CJKfamily", "songti", "kaishu", "fangsong", "heiti", "lishu",
    "kai", "hei", "song",
    "wuhao", "xiaowuhao", "xiaosihao", "sihao",
];
```

* 行为：识别命令后，跳过所有 `[..]` / `{..}` 配对，再吃掉行尾剩余字符。
* 不产生块；只推进 `pos`。
* **关键**：这些命令是「装饰 / 模板元数据」，对 Word 文档正文无视觉贡献，保留只会污染段落流。

### 3.5 环境扫描（`scan_environment`）

```rust
fn scan_environment(text: &str, pos: usize) -> Option<(&str, &str, usize)> {
    if pos >= bytes.len() || bytes[pos] != b'\\' { return None; }
    if !text[pos..].starts_with("\\begin{") { return None; }
    let after = pos + "\\begin{".len();
    let name_end = text[after..].find('}')? + after;
    let name = &text[after..name_end];
    let body_start = name_end + 1;

    // Skip optional argument braces like {ccc} in \begin{tabular}{ccc}
    let mut actual_body_start = body_start;
    while actual_body_start < bytes.len() && bytes[actual_body_start] == b'{' {
        if let Some(offset) = find_matching_brace(text, actual_body_start) {
            actual_body_start = actual_body_start + 1 + offset + 1;
        } else { break; }
    }

    let end_pat = format!("\\end{{{name}}}");
    let end_pos = text[actual_body_start..].find(&end_pat)
        .map(|p| actual_body_start + p).unwrap_or(text.len());
    let after_end = (end_pos + end_pat.len()).min(text.len());
    let body = &text[actual_body_start..end_pos];
    Some((name, body, after_end))
}
```

* 找到 `\begin{name}…\end{name}`，返回 `(name, body, end_pos)`。
* 跳过可选参数（如 `\begin{tabular}{ccc}` 的 `{ccc}`）。
* 找不到 `\end` → body 取到 EOF（不致命）。

### 3.6 环境降级（`lower_environment`）

```rust
fn lower_environment(name, body, span, macros, numbering) -> Block {
    match name {
        "itemize" => lower_list(body, false, span, macros, numbering),
        "enumerate" => lower_list(body, true, span, macros, numbering),
        "description" => lower_list(body, false, span, macros, numbering),
        "tabular" | "tabular*" | "array" => lower_table(body, span),
        "figure" | "figure*" | "table" | "table*" => lower_captioned_env(name, body, span, macros, numbering),
        "equation" | "equation*" | "align" | "align*" | "gather" | "gather*" => Block::Equation { latex: body.trim().to_string(), is_block: true, span },
        "document" => { /* 递归降级 body，折叠首个非空块 */ },
        "flushleft" | "flushright" | "center" | "quote" | "quotation" | "verbatim"
        | "rjkeywords" | "rjcategory" | "rjhead" | "rjtitle" | "rjauthor"
        | "rjinfor" | "rjmaketitle" => lower_paragraph_container(body, span, macros, numbering),
        "rjabstract" => lower_abstract_paragraph(body, span, macros, numbering),
        _ => Block::RawFallback { text: format!("\\begin{{{name}}}…\\end{{{name}}}"), span },
    }
}
```

* **itemize / enumerate / description** → `Block::List`。
* **tabular / tabular* / array** → `Block::Table`。
* **figure / table** → `Block::Figure`（带 `\includegraphics` + `\caption`）。
* **equation / align / gather** → `Block::Equation { is_block: true }`。
* **document** → 递归降级 body，折叠首个非空块（M3 简化）。
* **rjabstract** → 特殊：找第一个 `Paragraph`（避免 inline math 占位符覆盖中文摘要）。
* 未知环境 → `RawFallback`。

### 3.7 列表降级（`lower_list`）

```rust
fn lower_list(body, is_ordered, span, macros, numbering) -> Block {
    let mut items: Vec<Vec<Block>> = Vec::new();
    let mut current: Option<&str> = None;
    for line in body.split_inclusive('\n') {
        let s = line.trim_end_matches(&['\r', '\n'][..]);
        if s.trim_start().starts_with("\\item") {
            if let Some(buf) = current {
                let blocks = lower_item_body(buf, span, macros, numbering);
                items.push(blocks);
            }
            let after = s.trim_start().trim_start_matches("\\item").trim_start();
            if let Some(rest) = after.strip_prefix('[') {
                if let Some(close) = rest.find(']') {
                    let label = &rest[..close];
                    let rest2 = rest[close + 1..].trim();
                    let mut owned = format!("{label} — ");
                    owned.push_str(rest2);
                    current = Some(Box::leak(owned.into_boxed_str()));
                    continue;
                }
            }
            current = Some(after);
        } else if current.is_some() {
            let buf = current.unwrap();
            let mut owned = String::from(buf);
            owned.push('\n');
            owned.push_str(s);
            current = Some(Box::leak(owned.into_boxed_str()));
        }
    }
    if let Some(buf) = current {
        items.push(lower_item_body(buf, span, macros, numbering));
    }
    Block::List { is_ordered, items, span }
}
```

* 按 `\item` 切分；每项降级为 `Vec<Block>`（子项可以嵌套）。
* 支持 `\item[label] text`：label 与 text 用 " — " 拼合。

### 3.8 表格降级（`lower_table`）

```rust
fn lower_table(body, span) -> Block {
    let rows_text: Vec<&str> = body.split("\\\\").collect();
    let mut rows: Vec<TableRow> = Vec::new();
    for row in rows_text {
        // 处理 \rowcolor[model]{color} 或 \rowcolor{color}
        // ...
        let cells_text: Vec<&str> = row_text.split('&').collect();
        if cells_text.iter().all(|c| c.trim().is_empty()) { continue; }
        let mut cells: Vec<TableCell> = Vec::new();
        for c in cells_text {
            // 处理 \multicolumn{n}{spec}{text}
            // 处理嵌套 tabular（[TAB: ...] 占位符）
            // 调 strip_inline 清洗
            cells.push(TableCell { runs, colspan, rowspan, bg_color });
        }
        rows.push(TableRow { cells });
    }
    Block::Table { rows, caption: None, number: None, span }
}
```

* 主体按 `\\` 切行，按 `&` 切列。
* 支持 `\rowcolor`（提取颜色作为行/单元格背景）。
* 支持 `\multicolumn{n}{spec}{text}`（解析 n 作为 colspan）。
* 嵌套 tabular 检测：strip_inline 保留 `[TAB: ...]` 占位符，lower_table 用 `extract_nested_tabulary` 提取并扁平化。

### 3.9 段落 buffer 与 flush

```rust
fn flush_paragraph(doc, buffer, start, span, _macros) {
    if buffer.trim().is_empty() { buffer.clear(); return; }
    let body = buffer.trim().to_string();
    let s = *start;

    let parts = split_inline_math(&body);
    let has_inline_math = parts.iter().any(|p| matches!(p, RunPart::InlineMath(_)));

    if !has_inline_math {
        doc.push(Block::Paragraph { runs: vec![TextRun { text: body, style: TextStyle::Plain, span }], span });
    } else {
        let mut runs = Vec::new();
        for part in parts {
            match part {
                RunPart::Text(text) if !text.is_empty() => {
                    runs.push(TextRun { text: text.to_string(), style: TextStyle::Plain, span });
                }
                RunPart::InlineMath(math) => {
                    runs.push(TextRun { text: format!("[公式：{}]", math), style: TextStyle::Italic, span });
                    doc.push(Block::Equation { latex: math.to_string(), is_block: false, span });
                }
                _ => {}
            }
        }
        if !runs.is_empty() { doc.push(Block::Paragraph { runs, span }); }
    }
    buffer.clear();
}
```

* 段落 buffer 满 / 段命令 / 新环境 / EOF 触发 flush。
* 检测 inline math（`$...$`）：抽出为独立 `Block::Equation` + 在段落中插入占位符 `[公式：...]`（italic）。
* 块级 math（`$$...$$`）：仍留在 paragraph text 中（不做 block equation）。

### 3.10 inline math 切分（`split_inline_math`）

```rust
fn split_inline_math(text: &str) -> Vec<RunPart<'_>> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut parts = Vec::new();
    while i < len {
        if bytes[i] == b'$' {
            // $$...$$ = 块级，留在原文本
            if i + 1 < len && bytes[i + 1] == b'$' {
                let mut j = i + 1;
                while j < len && bytes[j] == b'$' { j += 1; }
                parts.push(RunPart::Text(&text[i..j]));
                i = j;
                continue;
            }
            // 单 $...$ 找闭合
            let mut j = i + 1;
            while j < len && bytes[j] != b'$' { j += 1; }
            let math = &text[i + 1..j];
            if !math.is_empty() {
                parts.push(RunPart::InlineMath(math));
            }
            i = j + 1;
        } else {
            let mut j = i + 1;
            while j < len && bytes[j] != b'$' { j += 1; }
            if j > i { parts.push(RunPart::Text(&text[i..j])); }
            i = j;
        }
    }
    parts
}
```

* `$$` 块级 → 留在原文本。
* 单 `$` 行内 → 抽出为 `RunPart::InlineMath`。
* 不平衡的 `$`（找不到闭合）→ 当作普通文本。

### 3.11 段落内联清洗（`strip_inline`）

`strip_inline` 是**最复杂**的子函数（300+ 行）。处理：

| 命令 | 行为 |
|------|------|
| `\textbf{x}` | 保留 `x` + 加粗（不传字体给 Writer） |
| `\textit{x}` | 保留 `x` |
| `\texttt{x}` | 保留 `x` |
| `\emph{x}` | 保留 `x` |
| `\cite{key}` | 全局 `[n]` 编号（首次出现分配） |
| `\ref{x}` / `\label{x}` / `\footnote{x}` / `\href{u}{t}` / `\url{u}` / `\nolinkurl{u}` | 整段吞掉 |
| `\rowcolor{color}` | 保留命令（lower_table 提取） |
| `\multicolumn{n}{spec}{text}` | 保留完整命令（lower_table 解析） |
| `\begin{tabular}{...}` | 标记 `[TAB: ...]`（lower_table 检测嵌套） |
| `\end{tabular}` | 标记 `]` |
| `\par` | 输出 `\n` |
| `\\` | 输出 `\n` |
| 字体切换 `\hei{x}` / `\song{x}` 等 | 保留 `x`（不带字体信息） |
| 字号切换 `\wuhao{x}` 等 | 保留 `x` |
| 装饰 `\hspace{x}` / `\vspace{x}` / `\bigskip` / `\smallskip` / `\noindent` / `\indent` / `\quad` / `\qquad` / `\mbox{x}` 等 | 整段吞掉 |
| `$...$` | 保持原样（不在 strip_inline 处理；由 split_inline_math 处理） |

### 3.12 关键 bug 防御：UTF-8 二次编码

```rust
// 关键代码（在 strip_inline）：
if let Some(ch) = line[i..].chars().next() {
    out.push(ch);
    i += ch.len_utf8();
} else {
    i += 1;
}
```

* `bytes[i] as char` 会把 UTF-8 多字节字符当 Latin-1 解码再编码，导致「微」(`E5 BE AE`) 变成 `C3 A5 C2 BE C2 AE`（mojibake）。
* **必须**走 `chars().next()` 拿完整 `char`，再 `len_utf8()` 推进。
* 详见 `strip_inline` 中的详细注释。

### 3.13 错误降级

* 未匹配内容进入 `Block::RawFallback`。
* 未闭合 group / env 自动补。
* **绝不 panic**——所有 `unwrap` 都被替换为 `if let Some(...)` + 错误降级。

---

## 4. Block 类型与来源对照

| `Block::*` 变体 | 触发源 | 关键代码 |
|-----------------|--------|----------|
| `Heading { level, text, number, span }` | `\section` / `\subsection` / `\subsubsection` / `\paragraph` | `try_top_level_command` |
| `Paragraph { runs, span }` | 段落 buffer flush / 顶层 `\caption` | `flush_paragraph` |
| `List { is_ordered, items, span }` | `\begin{itemize}` / `\begin{enumerate}` / `\begin{description}` | `lower_list` |
| `Table { rows, caption, number, span }` | `\begin{tabular}` / `\begin{array}` | `lower_table` |
| `Figure { path, caption, scale, number, span }` | `\begin{figure}` (含 `\includegraphics` + `\caption`) | `lower_captioned_env` |
| `Equation { latex, is_block, span }` | `equation` 环境 / 行内 `$...$` | `lower_environment` / `flush_paragraph` |
| `Bibliography { entries }` | （V1 暂未直接触发；保留供 BibTeX Writer 集成） | — |
| `RawFallback { text, span }` | 未匹配 / 未知环境 | 多处 |

---

## 5. 关键测试覆盖

`crates/latex-reader/src/lower.rs::tests` 50+ 测试：

* `lower_heading_and_paragraph` — Heading + Paragraph
* `lower_textbf_kept` — 段内粗体保留
* `lower_itemize` / `lower_enumerate` — 列表
* `lower_tabular_basic` — 简单表格
* `lower_figure_with_caption` — 图形 + caption
* `lower_equation_block` — 块级方程
* `lower_href_in_paragraph` — 链接吞掉
* `lower_unbalanced_recovers` — 不闭合不 panic
* `lower_inline_math` / `lower_inline_math_multiple` / `lower_inline_math_block_math_not_affected`
* `lower_cite_single` / `lower_cite_multiple` / `lower_cite_no_punct` / `lower_cite_with_optional`
* `lower_*_chinese` 等 CJK 验证

---

## 6. 修改降级层的标准流程

加新 LaTeX 命令的标准流程：

1. **在 `lower.rs::strip_inline` 或 `try_top_level_command` 加分支**。
2. **在 `try_top_level_metadata_command` 的 `META_CMDS` 列表加装饰命令**。
3. **在 `lower_environment` 加新环境名**。
4. **写单元测试**（`#[cfg(test)] mod tests` 内联）。
5. **跑 `cargo test -p doc-latex-reader`**。
6. **跑端到端 `cargo test -p doc-core --test paper3_e2e`**。
7. **跑 `node scripts/verify_paper3.mjs` 内容断言**。
8. **跑 `gitnexus impact({target: "doc_latex_reader::lower"})`**。

---

## 7. 已知限制与 V2 方向

| 当前限制 | 影响 | V2 方向 |
|----------|------|---------|
| 字符级扫描不识别注释中的命令 | `%\input{x}` 会被误吞 | 加 `// % 阻断` |
| 不支持 `\verb|...|` 字面量 | V1 静默错误 | V2 加 |
| 不展开 `\let` / `\def` | 部分宏包失效 | V2 路线图 |
| 不识别行内 math 中嵌套 `$` | LaTeX 罕见语法 | V2 评估 |
| cite 编号全局，但 `paper3` 跨段 cite 没问题 | — | — |

---

## 8. 进一步阅读

* [01-include-topology.md](./01-include-topology.md) — Pass-1
* [02-lexer-and-cst.md](./02-lexer-and-cst.md) — Pass-2
* [04-docx-serialization.md](./04-docx-serialization.md) — Pass-4
* [05-math-pipeline.md](./05-math-pipeline.md) — 公式独立管道
