# 03 · 语法归一化（latex_to_text / clean_math）

> 目标：把任意 LaTeX 文本段**降级**为**保留富文本语义**的"近 plain"字符串——所有 `\cite`/`\ref` 都已替换为编号，所有 `$..$` 已转为含 Unicode 符号的线性文本，所有上下标已用 `^`/`_` 标记（后续 inline_runs_xml 会再切回 `<w:vertAlign>`）。
>
> 本章是整个 IR 链条里**最绕**的一步，建议按顺序读"识别 → 替换 → 后处理"三段。

## 3.1 `latex_to_text(text, cite_map, label_map) -> String`

签名：(text: str, cite_map: dict[str, int], label_map: dict[str, str]) -> str
- `cite_map`：来自 `parse_bbl`，key 是 `\cite{...}` 中的引用 key，value 是该 key 在文末的编号
- `label_map`：来自 `collect_labels`，key 是 `\ref{...}` 中的 label，value 是 `1` / `(1)` 等占位字符串

Rust 端对应：

```rust
fn latex_to_text(
    text: &str,
    cite_map: &HashMap<String, usize>,
    label_map: &HashMap<String, String>,
) -> String
```

## 3.2 整体流水线（与 Python 完全对齐）

```text
1. strip_comments(text)                          // 剥注释
2. text.replace("\r", "\n")
3. re.sub(r"\\\\\s*", " ", text)                 // \\  → 空格
4. text.replace(r"\,", " ")                      // 不可断窄空格 → 普通空格
5. 把 \{ 和 \} 替换为私用区占位符 U+FFF0 / U+FFF1
   (原因：稍后剥外层花括号时要保留 \{ \} 字面量)
6. \cite{...}      → [N,M,...]（编号压缩）
7. \ref{...}       → label_map[label]   （默认 "??"）
8. \label{...}     → ""
9. $...$  与  \(...\)  → clean_math(content)
10. \footnote{...}  → "（注：递归清洗后内容）"
11. \textbf \textit \emph \url \nolinkurl \texttt \mathrm \rjrare
    全部走 replace_command_arg(..., lambda s: 递归清洗)
12. \item[LABEL]  → "LABEL "     （列表项标签去括号）
13. \item         → ""           （列表项标记删除）
14. 引号替换：
    `` → "  （左双引号 U+201C）
    '' → "  （右双引号 U+201D）
15. 破折号：
    --- → — (em dash U+2014)
    --  → – (en dash U+2013)
16. 反斜杠转义：
    \% → %
    \& → &
    \_ → _
    \# → #
    \$ → $
17. ~ → " "（不可断空格 → 普通空格）
18. 字体/字号宏删除：
    \xiaowuhao \wuhao \small \centering \noindent
    \song \kai \hei \fs \par \allowbreak
    → ""
19. \fontsize{...}{...}\selectfont → ""
20. \hspace{...} \vspace{...} → " "
21. 通用兜底：\\[A-Za-z]+\*?(?:\[[^\]]*\])? → ""
    （前面没匹配到的命令一律删）
22. 把 { 和 } 删掉（外层）
23. 把私用区占位符还原为 { 和 }
24. 多个连续空格 → 1 个空格
25. 跨行空白 → 1 个空格
26. strip()
```

## 3.3 编号压缩 `compress_numbers`

`\cite{foo,bar,baz}` 当 `foo=1, bar=2, baz=3` 时输出 `[1-3]` 而不是 `[1,2,3]`。算法：

```python
def compress_numbers(numbers: list[int]) -> str:
    if not numbers: return ""
    numbers = sorted(dict.fromkeys(numbers))  # 去重
    ranges = []
    start = prev = numbers[0]
    for num in numbers[1:]:
        if num == prev + 1: prev = num; continue
        ranges.append(f"{start}" if start == prev else f"{start}-{prev}")
        start = prev = num
    ranges.append(f"{start}" if start == prev else f"{start}-{prev}")
    return ",".join(ranges)
```

测试用例：

| 输入 | 输出 |
|------|------|
| `[1]` | `"1"` |
| `[1,2,3]` | `"1-3"` |
| `[1,2,4,5,7]` | `"1-2,4-5,7"` |
| `[3,1,2]` | `"1-3"`（先排序）|
| `[]` | `""`（在调用方已处理） |

## 3.4 `replace_command_arg` —— 对所有匹配的命令统一改写

```python
def replace_command_arg(text: str, command: str, repl) -> str:
    start = 0
    token = f"\\{command}"
    while True:
        pos = text.find(token, start)
        if pos < 0: break
        brace = text.find("{", pos + len(token))
        if brace < 0:
            start = pos + len(token); continue
        try:
            end = find_matching_brace(text, brace)
        except ValueError:
            break
        inner = text[brace + 1 : end]
        text = text[:pos] + repl(inner) + text[end + 1 :]
        start = pos
    return text
```

用途：把 `\foo{inner}` 的 inner 拎出来走 `repl`，然后回写。比如 `repl = lambda s: f"《{s}》"` 可以把 `\emph{xxx}` 变成 `《xxx》`。

Rust 实现要注意：循环中**`start = pos`** 而不是 `pos + len(repl_result)`——因为 `repl` 可能改变长度，下一次查找要重头在 `pos` 开始，否则会漏掉嵌套。

## 3.5 数学替换 `clean_math`

`clean_math` 处理行内数学 `$...$` 提取出的**纯字符内容**。注意它**不递归调** `latex_to_text`，因为数学语法的子集更窄。

```text
1. 字符级宏替换（注意顺序！先保护 \{ \}, \, ~）：
   把 \{ → 占位符 U+FFF0
   把 \} → 占位符 U+FFF1
   把 \, → " "
   把 ~  → " "
2. \mathrm{...} \textbf{...} \textit{...} 内的内容原样保留（去掉外层花括号）
3. 标准符号替换表：
   \pm         → ±
   \%          → %
   \rightarrow → →     (U+2192)
   \leftarrow  → ←
   \infty      → ∞
   \leq        → ≤
   \geq        → ≥
   \ll         → ≪
   \times      → ×
   \cdot       → ·
   \emptyset   → ∅
   \alpha      → α
   \beta       → β
   \rho        → ρ
   \xi         → ξ
   \ldots      → …
   \log        → " log " （前后带空格，避免与相邻字母粘连）
   \min        → "min"
   \max        → "max"
   \in         → ∈
4. 反复（6 次）做 re.sub(r"\{([^{}]*)\}", r"\1") 剥去所有外层花括号
   （多于 6 层嵌套时手工再调一次）
5. 把 \[A-Za-z]+ 命令名剥为字母：
   \foo → foo
6. 还原占位符：占位符 → { / }
7. 把 _ 还原（之前是 LaTeX 下标，会在 inline_runs_xml 中再处理）：
   但这一步的 _ 不直接转成 <sub>，而是**保留 _** —— 因为 `latex_to_text` 已经先走
   `inline_runs_xml` 时统一处理 ^ 与 _。在 clean_math 中只做最低限度归一化。
8. 多个空白 → 1 个空白
9. strip
```

### 3.5.1 关于下标 `_`

注意：行内数学中的下标（如 `$L=\{l_1,\ldots,l_N\}$`）经过 clean_math 之后输出 `L={l_1,...,l_N}`。`inline_runs_xml` 在切 run 时会识别 `_1` 这种子串并切为下标 run。详见 §5.3。

## 3.6 `clean_formula_display_text` —— 公式居中显示前的额外清理

`populate` 中给公式段落用：`block.caption` 之前先 `clean_formula_display_text(text)`。

```python
def clean_formula_display_text(text: str) -> str:
    if not ("bigl" in text or "bigr" in text or re.search(r"\bd_(?:0|n|max)\b", text)):
        return text
    text = text.replace("minbigl", "min")
    text = text.replace("bigl", "").replace("bigr", "")
    text = re.sub(r"\s*([=+])\s*", r" \1 ", text)
    text = re.sub(r"\s*,\s*", ", ", text)
    text = re.sub(r"\(\s+", "(", text)
    text = re.sub(r"\s+\)", ")", text)
    text = re.sub(r"\s{2,}", " ", text)
    text = text.replace(" .", ".")
    return text.strip()
```

专门处理 `d_{\max}`、`d_0` 这种被 `clean_math` 剥为 `d_(0)` 的情况——`bigl`/`bigr` 是 algorithm2e 包偶尔吐出的伪指令。

## 3.7 `fix_display_text` —— 后置硬编码修正

```python
def fix_display_text(text: str) -> str:
    text = re.sub(r"\btheta\b", "θ", text)
    text = text.replace("数字→id，UUID→uuid", "数字→{id}，UUID→{uuid}")
    text = text.replace("映射为 id，UUID 或长哈希段映射为 uuid",
                        "映射为 {id}，UUID 或长哈希段映射为 {uuid}")
    text = text.replace("L=l_1,…,l_N", "L={l_1,…,l_N}")
    text = text.replace("A=(p_i,w_i)", "A={(p_i,w_i)}")
    return text
```

**这是临时补丁**——历史 commit 中某些行内数学被 lint 误伤，需要后置恢复。Rust 端**应当照搬**以保持与现有产物字节级一致。

## 3.8 `inline_runs_xml` 与上下标识别

详见 §5.3。简单先看一下识别规则（影响归一化的精度）：

| 输入 | 输出 run 序列 |
|------|--------------|
| `Top-$K$` | `[Top-] [K 上标]` |
| `L=\{l_1,\ldots,l_N\}` | `[L={l] [1 下标] [,…,l] [N 下标] [}]` |
| `\cite{foo,bar}` | `[ [1-2 上标] ]`（被 `latex_to_text` 提前转 `[1-2]` 后再走 inline_runs） |
| `^2` | `[2 上标]` |
| `^{10}` | `[10 上标]` |
| `_n` | `[n 下标]`（仅在 style="JOSCode" 或文本内含 `x_n` 模式时启用） |
| `[5,6]` | `[ [5,6 上标] ]` |

正则：

```python
CITATION_RE = re.compile(r"\[[0-9][0-9,\-\s]*\]")
```

捕获 `[N,M-N,...]` 形式。

## 3.9 字符级映射的字符来源

`clean_math` 的所有 Unicode 字符必须在源码中是合法 UTF-8。Python 源文件头 `from __future__ import annotations` 与默认 UTF-8 编码保证这一点。Rust 端**只需在源文件顶部加 `#![allow(non_snake_case)]` 等 attribute**——但字符串字面量里直接写 `"\u00B1"`（±）即可。

`clean_math` 涉及的全部 Unicode：

| LaTeX | Unicode | 名称 |
|------|---------|------|
| `\pm` | `U+00B1` | ± |
| `%` (裸) | `U+0025` | % |
| `\rightarrow` | `U+2192` | → |
| `\leftarrow` | `U+2190` | ← |
| `\infty` | `U+221E` | ∞ |
| `\leq` | `U+2264` | ≤ |
| `\geq` | `U+2265` | ≥ |
| `\ll` | `U+226A` | ≪ |
| `\times` | `U+00D7` | × |
| `\cdot` | `U+00B7` | · |
| `\emptyset` | `U+2205` | ∅ |
| `\alpha` | `U+03B1` | α |
| `\beta` | `U+03B2` | β |
| `\rho` | `U+03C1` | ρ |
| `\xi` | `U+03BE` | ξ |
| `\ldots` | `U+2026` | … |
| `\in` | `U+2208` | ∈ |
| `\bigl` / `\bigr` | (删除) | — |
| 占位符 U+FFF0 / U+FFF1 | (私用区) | — |

`latex_to_text` 引号/破折号：

| LaTeX | Unicode | 名称 |
|------|---------|------|
| `` `` `` | `U+201C` | " |
| `''` | `U+201D` | " |
| `---` | `U+2014` | — |
| `--` | `U+2013` | – |

## 3.10 文本宽度单位 `token_width_units` 与 `wrap_text_units`

`populate` 在写入**中文引用格式**和**英文引用格式**前，要按 52 units 的"版心宽度"自动换行。

```python
def token_width_units(token: str) -> float:
    total = 0.0
    for ch in token:
        code = ord(ch)
        if ch.isspace(): total += 0.35
        elif 0x4E00 <= code <= 0x9FFF: total += 1.0
        elif ch.isupper(): total += 0.62
        elif ch.islower() or ch.isdigit(): total += 0.52
        elif ch in "-/.": total += 0.28
        else: total += 0.35
    return total
```

每个字符的"宽度"单位是经验值：**汉字 1.0、大写英文 0.62、小写英文/数字 0.52、标点 -/./ 0.28、空白 0.35、其他 0.35**。

```python
def wrap_text_units(text: str, max_units: float) -> list[str]:
    tokens = re.findall(r"https?://\S+|\s+|[A-Za-z0-9]+(?:[-/][A-Za-z0-9]+)*|[\u4e00-\u9fff]|.", text)
    lines, current, width = [], [], 0.0
    for token in tokens:
        if token.isspace(): token = " "
        token_width = token_width_units(token)
        if current and width + token_width > max_units:
            lines.append("".join(current).strip())
            current, width = [], 0.0
            token = token.lstrip()
            token_width = token_width_units(token)
        if token or current:
            current.append(token); width += token_width
    if current: lines.append("".join(current).strip())
    return [line for line in lines if line]
```

要点：

- token 化正则必须把**URL**先抠出来（不然会被 `.[A-Za-z]` 切散）。
- `width + token_width > max_units` 时换行（**超容许宽度就换**，不一定要等满）。
- 新行起始要 `lstrip()` 避免前导空格。
- 永远不输出空行（最后过滤）。

Rust 实现建议直接用 `unicode-segmentation` crate 做更细的图簇切分（中文混排时按 char 切即可，不必图簇）。

## 3.11 `spaced_keywords`

```python
def spaced_keywords(text: str) -> str:
    return re.sub(r";\s*", "; ", text).strip()
```

把 `;` 后的多个空白归一为 `; `。用于"关键词"字段。
