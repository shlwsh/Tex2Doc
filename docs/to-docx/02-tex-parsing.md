# 02 · LaTeX 解析算法

> 目标：在不调用 `pdflatex` / `xelatex` 的前提下，从源文件直接构建出**结构化的中间表示** `Manuscript`。
> 本章复刻 `build_jos_docx.py` 中所有字符串/正则操作；Rust 端用 `regex` + 手写 `Reader` 即可等价实现。

## 2.1 入口：`build_manuscript(root: &Path) -> Manuscript`

调用顺序（参见 `build_jos_docx.py` 的 `build_manuscript`）：

```text
1. main_tex = read_text(root/"latex/main-jos.tex")
2. expanded = expand_inputs(main_tex, root/"latex")
3. macros   = parse_newcommands(read_text(root/"latex/sections/zh/00_abstract.tex"))
4. cite_map, references = parse_bbl(root/"latex/main-jos.bbl")
5. section_texts = [strip_comments(read_text(root/SECTION_FILES[i])) for i in 1..7]
6. label_map     = collect_labels(section_texts)
7. title    = command_arg(main_tex, "rjtitle")
8. authors  = command_arg(main_tex, "rjauthor")
9. info     = command_arg(main_tex, "rjinfor")
10. (title_en, authors_en, institute_en) = extract_english_front_matter(main_tex, ...)
11. running_header   = extract_command_text(main_tex, "rjhead", ...)
12. first_footer_text = extract_footnote_text(main_tex, ...)
13. blocks = parse_sections(root, cite_map, label_map)
14. cn_references = extract_cn_references(expanded, ...)
15. author_bio    = extract_author_bio(expanded, ...)
16. return Manuscript(...)
```

Rust 端把每个步骤拆成独立函数，签名照搬 `command_arg` 等返回 `(Option<(inner, start, end)>)` 模式即可。

## 2.2 章节文件列表 `SECTION_FILES`

`build_jos_docx.py` 顶部硬编码了 7 个章节文件（**顺序敏感**）：

```text
latex/sections/zh/01_intro.tex
latex/sections/zh/02_related.tex
latex/sections/zh/03_system.tex
latex/sections/zh/04_algorithms.tex
latex/sections/zh/05_implementation.tex
latex/sections/zh/06_experiments.tex
latex/sections/zh/07_conclusion.tex
```

`parse_sections` 严格按这个顺序遍历，章节计数器 `section_no` 也按此顺序自增。Rust 端**必须保持**：

- 不读 `00_abstract.tex`（它只贡献摘要/关键词宏，不进章节流）
- 顺序与 `main-jos.tex` 中 `\input{}` 的顺序一致

## 2.3 `find_matching_brace` —— 大括号匹配

这是所有解析的原语。Python 实现：

```python
def find_matching_brace(text: str, open_index: int) -> int:
    depth = 0
    i = open_index
    while i < len(text):
        ch = text[i]
        prev = text[i - 1] if i else ""
        if ch == "{" and prev != "\\":
            depth += 1
        elif ch == "}" and prev != "\\" and depth:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    raise ValueError(f"Unmatched brace at offset {open_index}")
```

**关键不变量**：

1. `open_index` 指向 `{` 本身。
2. `prev != "\\"` 的判定是为了**跳过** `\{` 和 `\}`（LaTeX 中表示字面花括号）。
3. 不支持字符串字面量、注释预处理；调用方应**先 strip 注释**再传。
4. **不平衡时抛异常**——上层捕获后停止解析。

Rust 伪代码：

```rust
fn find_matching_brace(text: &str, open_index: usize) -> Result<usize, ParseError> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut i = open_index;
    while i < bytes.len() {
        let c = bytes[i] as char;
        let prev = if i == 0 { '\0' } else { bytes[i - 1] as char };
        if c == '{' && prev != '\\' {
            depth += 1;
        } else if c == '}' && prev != '\\' {
            if depth == 0 {
                return Err(ParseError::UnmatchedBrace(i));
            }
            depth -= 1;
            if depth == 0 {
                return Ok(i);
            }
        }
        i += 1;
    }
    Err(ParseError::UnclosedBrace(open_index))
}
```

## 2.4 `command_arg` —— 单参数命令取值

`\foo{bar}` 抓取 `bar`。如果存在多对花括号嵌套，`find_matching_brace` 会正确处理。

```python
def command_arg(text: str, command: str, start: int = 0) -> tuple[str, int, int] | None:
    token = f"\\{command}"
    pos = text.find(token, start)
    if pos < 0: return None
    brace = text.find("{", pos + len(token))
    if brace < 0: return None
    end = find_matching_brace(text, brace)
    return text[brace + 1 : end], pos, end + 1
```

返回 `(inner_text, command_start, end_of_arg)` 三元组，方便调用方用 `[0:pos] + [end+1:]` 替换。

## 2.5 `strip_comments` —— 注释剥离

```python
def strip_comments(text: str) -> str:
    lines = []
    for line in text.splitlines():
        if line.lstrip().startswith("%"): continue
        line = re.sub(r"(?<!\\)%.*$", "", line)
        lines.append(line.rstrip())
    return "\n".join(lines)
```

**两条规则**：

1. 整行注释（`%` 在行首空白后）整行丢掉。
2. 行内注释用 `(?<!\\)%.*$` 找**未被反斜杠转义**的 `%`。
3. 不解析 `%` 后到行尾为 verbatim 模式（如 `verbatim` / `comment` 环境）——`p3-microservice` 不使用这些环境，足够。

## 2.6 `expand_inputs` —— `\input` 递归展开

```python
def expand_inputs(tex: str, base_dir: Path) -> str:
    def repl(match):
        name = match.group(1)
        rel = Path(name if name.endswith(".tex") else f"{name}.tex")
        path = base_dir / rel
        if not path.exists(): return ""
        return expand_inputs(read_text(path), path.parent)
    return re.sub(r"\\input\{([^}]+)\}", repl, tex)
```

注意：

- `\input` 嵌套时**递归展开**，但**不重复 strip 注释**——注释由调用方在外层处理。
- 文件不存在时**静默删除**该 `\input`（不报错）。Rust 端建议改为**记 warning 但继续**。
- base_dir 跟随嵌套路径更新（每深入一层就改成该文件所在目录）。

## 2.7 `parse_newcommands` —— 宏定义提取

从 `00_abstract.tex` 中提取所有 `\newcommand{\Name}{...}`。两种形式都支持：

```latex
% 多行（行尾 % 续行 + 起始 % 续行）
\newcommand{\AbstractContentZh}{%
  正文...
}

% 单行
\newcommand{\KeywordsZh}{微服务;日志采集;...}
```

正则：

```python
block_pattern = r"\\newcommand\{\\(\w+)\}\{%\n(.*?)%\n\}"
line_pattern = r"\\newcommand\{\\(\w+)\}\{([^{}\n]+)\}"
```

返回 `dict[str, str]`（宏名 → 宏体）。**单行优先**——多行匹配使用 `setdefault` 避免覆盖。

## 2.8 `parse_bbl` —— 参考文献解析

`main-jos.bbl` 是 BibTeX 跑出的格式化结果，结构：

```latex
\begin{thebibliography}{10}
\bibitem{burns2016patterns}
Brendan Burns, David Oppenheimer, ...
\newblock Design patterns for container-based distributed systems.
\newblock In {\em Proceedings of the 7th Workshop on Hot Topics in Cloud
  Computing (HotCloud)}. ACM, 2016.
...
\end{thebibliography}
```

`parse_bbl` 步骤：

1. 用 `re.split(r"\\bibitem(?:\[[^\]]*\])?\{([^}]+)\}", raw)` 切片。
2. 切片规则：`[..., "burns2016patterns", "Brendan Burns...\n\\newblock ...", "burns2016borg", ..., "", ...]`
3. 按 `(key, body)` 两两一组取出。
4. `key_to_num[key] = index + 1`（**1-based**编号），与正文中 `\cite{...}` 替换对应。
5. `clean_bibitem(body)` 内部：剥 `\begin{thebibliography}` / `\end{thebibliography}` 标记；`\newblock → " "`；`{\em X} → X`；最后走 `latex_to_text`。

Rust 实现关键点：

- `re.split` 的输出模式（奇数索引是 key、偶数是 body）要严格保留。
- `\bibitem[opt]{key}` 的 `[opt]` 部分在 JOS 模板中**不出现**，但要兼容。
- 引用编号就是 `key_to_num` 的值，**不重排不补号**。

## 2.9 `collect_labels` —— 标签编号表

正文里 `\ref{alg:attention}` 之类的引用需要替换成 `1`、`(1)` 等数字。`collect_labels` 把"已出现的环境 + 顺序"映射到标签号：

```python
for env, prefix, fmt in [
    ("table", "tab", "{}"),
    ("figure", "fig", "{}"),
    ("algorithm", "alg", "{}"),
    ("equation", "eq", "({})"),
]:
    count = 0
    for body in find_envs(joined, env):
        count += 1
        label_match = re.search(r"\\label\{([^}]+)\}", body)
        if label_match:
            label = label_match.group(1)
            labels[label] = fmt.format(count)
            if ":" not in label:
                labels[f"{prefix}:{label}"] = fmt.format(count)
```

关键点：

1. **顺序按出现先后**扫描所有 7 个章节文件。
2. 每个环境都从 1 开始累加（不是全局唯一编号）。
3. **同时存 `alg:attention` 和 `algattention` 两种 key**（后者防御源代码漏写冒号）。
4. table/figure/algorithm 用 `{}` 格式（输出 "1"），equation 用 `({})`（输出 "(1)"）——这与 `equation` 环境的最终输出 `block.caption = f"({equation_no})"` 互为冗余备份。

## 2.10 `find_envs` —— 抓取所有同名 LaTeX 环境

```python
def find_envs(text: str, env: str) -> Iterable[str]:
    pattern = re.compile(rf"\\begin\{{{env}\}}(?:\[[^\]]*\])?(.*?)\\end\{{{env}\}}", re.DOTALL)
    for match in pattern.finditer(text):
        yield match.group(1)
```

注意点：

- `(?:\[[^\]]*\])?` 处理 `\begin{tabular}{cccc}` 这种带方括号的——JOS 模板中**只在 `algorithm` 等少数环境出现**。
- `re.DOTALL` 让 `.` 匹配换行符。
- 嵌套同名环境**不处理**——`\begin{enumerate} ... \begin{enumerate} ...` 在本项目不出现；如有需要，改用栈式扫描。

## 2.11 `parse_sections` —— 章节流的 tokenize 循环

这是解析的核心循环。算法：

```text
tokens = 交替扫描每行，遇到以下之一：
  - \section{TITLE}        → heading level=1
  - \subsection{TITLE}     → heading level=2
  - \subsubsection{TITLE}  → heading level=3
  - \begin{table} ... \end{table}
  - \begin{figure} ... \end{figure}
  - \begin{algorithm} ... \end{algorithm}
  - \begin{equation} ... \end{equation}
  - \begin{enumerate} ... \end{enumerate}
  - \begin{center} ... \begin{tabular} ... \end{tabular} ... \end{center}
对每个 token：
  前一个 token 末尾到当前 token 起始之间的文本 → add_text_blocks（拆段、清洗、入 blocks）
  当前 token 自身 → 转成 Block（heading/table/figure/...）入 blocks
章节计数器（section_no/subsection_no/...）在 heading 触发时维护
```

用到的正则：

```python
token_re = re.compile(
    r"\\(section|subsection|subsubsection)\{"
    r"|\\begin\{(table|figure|algorithm|equation|enumerate|center)\}(?:\[[^\]]*\])?",
    re.DOTALL,
)
```

注意 `r"\\begin\{...\}(?:\[[^\]]*\])?"` 不会匹配 `\end{...}`——`\end` 通过 `text.find(end_token, match.end())` 手动定位。

### 2.11.1 编号写入

| 环境 | 编号字段 | 写入规则 |
|------|---------|---------|
| section | `section_no` | 从 1 自增；触发时把 `subsection_no=subsubsection_no=0` |
| subsection | `subsection_no` | 输出 `section.subsection 标题` |
| subsubsection | `subsubsection_no` | 输出 `section.subsection.subsubsection 标题` |
| table | `table_no` | caption 前缀：`表 N  <原caption>` |
| figure | `figure_no` | caption 前缀：`图 N  <原caption>` |
| algorithm | `algorithm_no` | caption 前缀：`算法 N  <原caption>` |
| equation | `equation_no` | caption 强制为 `(N)` |

### 2.11.2 章节正文 → 段落块 `add_text_blocks`

非环境文本的处理：

```python
chunk = re.sub(r"\\vspace\{[^}]+\}", "\n\n", chunk)
chunk = re.sub(r"\\noindent", "", chunk)
for paragraph in re.split(r"\n\s*\n", chunk):
    paragraph = paragraph.strip()
    if not paragraph: continue
    cleaned = latex_to_text(paragraph, cite_map, label_map)
    if cleaned:
        blocks.append(Block(kind="paragraph", text=cleaned))
```

- `\vspace{...}` 视为段落分隔（强制换段）。
- `\noindent` 显式删除（首行缩进由样式决定）。
- 空行（`\n\s*\n`）是段落分隔符。
- 每段都走 `latex_to_text` 清洗。

## 2.12 `extract_*` 系列 —— front matter 抽取

| 函数 | 输入 | 输出 |
|------|------|------|
| `extract_command_text` | `tex`, `command` | 命令参数清洗后的字符串 |
| `extract_footnote_text` | `tex` | `\footnotetext{...}` 内容（剥 `\xiaowuhao\song` 等字体宏） |
| `extract_english_front_matter` | `tex` | `(title_en, authors_en, institute_en)`，用 `command_arg(..., "textbf")` + 正则 |
| `extract_cn_references` | 展开后全文 | `\begin{description}...\end{description}` 中每 `\item[{[N]}]` 转为纯字符串 |
| `extract_author_bio` | 展开后全文 | `\begin{list}...\end{list}` 中每 `\item` 转为纯字符串 |
| `extract_line_with` | `tex`, 关键字 | 含该关键字的行（清洗后） |
| `derived_running_header` | `Manuscript` | `f"{首作者} 等: {中文标题}"` 当 `\rjhead` 缺省时 |

### 2.12.1 `extract_english_front_matter` 详解

```text
起点：第一个出现 "% ---------- 英文标题/作者/机构" 注释的下一行
终点：下一个 "% ---------- 英文摘要" 注释之前

在 [起点, 终点) 区间内：
  title_en  = \textbf{...} 参数内容
  authors_en = \vspace{...}{... \xiaowuhao ...} 内层内容
  institute_en = 第一个匹配 \( ... China ... \) 的小括号整体
```

Rust 端用 `find` + `find_matching_brace` 即可。

### 2.12.2 `extract_cn_references` 详解

```latex
\begin{description}[font=\normalfont,labelwidth=2em,leftmargin=2.5em,nosep]
\item[{[5]}] 冯志勇, ... 2020, 57(5): 1103--1122.
\item[{[6]}] ...
\end{description}
```

`extract_cn_references` 抓取 `description` 环境，按 `\item[{[N]}]` 切分，最后用 `re.sub(r"\[(\d+)\s+\]", r"[\1]", text)` 把 `[5 ]` 紧凑为 `[5]`。注意**只取 `\item` 的正文**（去除标签 `[5]` 后的内容），caption 部分由调用方加上 `附中文参考文献:` 标题。

## 2.13 算法块 `parse_algorithm`

这是最复杂的一段（`parse_algorithm_rows` ~100 行）。要复刻请严格按以下算法：

```text
入口：algorithm2e 环境体（含 \KwIn, \KwOut, \ForEach, \If, \Return, \;, 普通语句）

1. 提取 caption、label、algorithm_io = [(Input, ...), (Output, ...)]
2. 移除所有 \caption{...} \label{...} \KwIn{...} \KwOut{...} 元数据命令
3. 主体进入 parse_algorithm_rows
4. 每行解析为 {line_no, indent, guides, end_guides, code, comment, keyword?}
```

### 2.13.1 `parse_algorithm_rows` 状态机

```text
cur ← 0
indent ← 初始
active_guides ← 父层所有 indent 列表
counter ← [0]

循环直到遇到 '}'：
  if 当前位置是 \ForEach 或 \If：
      提交 [cur, 当前位置) 之间累积的语句为一条 statement
      解析两个参数 {cond}{body}
      输出一行 keyword 行："foreach <cond> do" 或 "if <cond> then"
      递归调用 parse_algorithm_rows(body, indent+1, active_guides+(indent,))
      在递归返回的 rows 最后一条的 end_guides 追加 indent（用于悬挂的 end 线）
      cur ← body 结束位置
  if 当前位置是 \Return：
      提交当前累积
      解析 \Return{value}
      如果本行含 \;，则追加 ";" 到 value
      输出一行 "return <value>" / "return <value>;"
      cur ← 行尾（包含 \;）
  if 当前位置是 \;：
      提交 [cur, \;+2) 为一条 statement
      cur ← \;+2
  else：
      cur += 1

最后提交 [cur, 末尾] 为一条 statement
```

### 2.13.2 `\tcp*` 行尾注释

`\tcp*{...}` 是 algorithm2e 的行尾注释。`algorithm_statement_text` 在清洗单条语句时：

```text
if 存在 \tcp*：
  comment = latex_to_text(注释内容)
  从 raw 中删除 \tcp*{...} 子串
将 raw 中的 \; → " "
调用 normalize_inline_text 把多行展平
最后 latex_to_text
if 原始 raw 含 \; 且结果不以 ";" 结尾 → 补 ";"
```

### 2.13.3 `guides` / `end_guides` 含义

- `guides` = 此行**上方**的悬挂缩进竖线位置（用于在编辑器中画 │）
- `end_guides` = 此行**下方**的竖线位置
- 最终渲染时，公式：

```text
缩进列 = 2 × indent 半角空格
竖线   = 在 guides 中每个位置画一个 │
end   = 在 end_guides 中每个位置画 ┘
```

注意：当前 docx 输出**不渲染竖线**（只用 `JOSCode` 样式 + 缩进），但数据结构里仍保留以备扩展。Rust 端可以**先实现简化版**（仅用 `indent × 4` 空格）并保留字段。

## 2.14 equation 块 `parse_equation`

`\begin{equation} ... \end{equation}` 内部：

1. 抓 `\label{...}` 取出 label
2. 删除 label，剩下 `content`
3. `eq_no = label_map.get(label, "")` 拿到编号
4. `Block(kind="equation", text=clean_math(content), label=label, caption=eq_no)`

最终输出（在 `populate` 中）：`"${text}    ${caption}"` 居中。

## 2.15 figure 块 `parse_figure`

```text
1. caption = \caption{...} 参数
2. label   = \label{...} 抓出
3. \includegraphics[options]{filename}：
   - 后缀是 .pdf  → 找 figures/<filename>.stem.png
   - 后缀非空     → 找 figures/<basename>
   - 无后缀       → 找 figures/<filename>.png
4. 如果仍找不到且原文件是 .pdf：
     调用 maybe_convert_pdf(pdf, png) 触发 pdftoppm 转 220 dpi PNG
5. 从 options 中匹配 width=N\textwidth → width_factor = N
```

## 2.16 table 块 `parse_table`

```text
1. caption / label 同 figure
2. extract_tabular(env_text)：
     找 \begin{tabular} 或 \begin{tabular*}{width}
     跳过 \begin 后到 { 之间的空白
     如果是 tabular*，跳过 {width}（含花括号匹配）
     读 column spec 直到 }
     在 spec 结束位置到 \end{tabular} 之间的内容 = 表体字符串
3. 移除 \toprule \midrule \bottomrule \hline
4. 按 \\ 切行（支持 \\ [...]）
5. 每行用 split_cells 切 cell（按 &，但 { ... } 嵌套内 & 不切）
6. 每个 cell 走 latex_to_text
```

## 2.17 输出 IR：`Manuscript`

```text
struct Manuscript {
    title_zh: String,
    authors_zh: String,
    institute_lines: Vec<String>,
    abstract_zh: String,
    keywords_zh: String,
    category: String,
    citation_zh: String,
    citation_en: String,
    title_en: String,
    authors_en: String,
    institute_en: String,
    abstract_en: String,
    keywords_en: String,
    running_header: String,
    first_footer_text: String,
    blocks: Vec<Block>,
    references: Vec<String>,        // 英文文献清洗后条目
    cn_references: Vec<String>,     // 中文参考
    author_bio: Vec<String>,        // 作者简介
}

enum Block {
    Heading { level: u8, text: String },
    Paragraph { text: String },
    ListItem { text: String },     // "1. xxx"
    Table { caption: String, label: String, rows: Vec<Vec<String>> },
    Figure { caption: String, label: String, image_path: Option<Path>, width_factor: f32 },
    Algorithm { caption: String, label: String, lines: Vec<String>, io: Vec<(String,String)>, rows: Vec<AlgRow> },
    Equation { text: String, label: String, caption: String },
}

struct AlgRow {
    line_no: u32,
    indent: u8,
    guides: Vec<u8>,
    end_guides: Vec<u8>,
    code: String,
    comment: String,
    keyword: Option<String>,  // "ForEach" | "If" | "Return"
}
```

Rust 推荐用 `enum Block { ... }` 模式，配合 `match` 在 `populate` 中分支。

## 2.18 解析顺序与依赖图

```text
                      main_tex
                          │
              ┌───────────┼────────────┐
              ▼           ▼            ▼
       command_arg()  expand_inputs()  extract_english_front_matter()
              │           │
              ▼           ▼
        title/authors  expanded   ◄─────┐
        institute                  │
                                    │
                       extract_cn_references(expanded)
                       extract_author_bio(expanded)

        00_abstract.tex
              │
              ▼
       parse_newcommands → macros

        main-jos.bbl
              │
              ▼
       parse_bbl → cite_map, references

        sections/zh/0[1-7]_*.tex
              │
              ▼
       strip_comments
              │
              ▼
       collect_labels
              │
              ▼
       parse_sections → blocks
              │
              ▼
       Manuscript
```

Rust 实现时**先做完没有依赖的步骤**（`parse_bbl`、`parse_newcommands`），再处理 `main_tex` 自身的命令抽取，最后用 `parse_sections` 收尾。

## 2.19 失败模式与回退

| 异常 | 触发条件 | 现有行为 | Rust 推荐 |
|------|---------|---------|----------|
| 花括号不闭合 | 解析宏时 | `ValueError` 上抛，shell 整体 fail | 返回 `Err(ParseError::UnclosedBrace)` |
| `\input` 文件缺失 | 解析宏时 | 静默替换为空串 | 记 warning（`eprintln!`）+ 继续 |
| `\bibitem` 缺失（cite 指向未在 bbl 中的 key） | 替换 `\cite` 时 | 输出 `[?]` | 同上 |
| `\ref` 指向未定义 label | 替换 `\ref` 时 | 输出 `??` | 同上 |
| 图片文件缺失 | figure 处理时 | 走 `[缺图] <caption>` 占位 | 同上 |
