# 关键技术 2：Logos 词法 + Rowan 语法树

> 本节深入解析 `doc-latex-reader::lexer` 与 `doc-latex-reader::parser`（Pass-2）。解决的核心问题：把字符流变成有结构的语法树，同时保证**绝不 panic**。

---

## 1. Logos 词法（`lexer.rs`）

### 1.1 Logos 是什么

* `logos = 0.14`：基于正则 DFA 的零拷贝 Rust 词法库。
* 通过 `#[derive(Logos)]` 自动生成 DFA + `Lexer<T>` 迭代器。
* 每个 token 提供字节范围切片；不分配内存。

### 1.2 词法元素

```rust
#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokKind {
    #[regex(r"\\[A-Za-z@]+")]
    Command,                                  // \command

    #[token("{")]
    LBrace, #[token("}")] RBrace,
    #[token("[")] LBracket, #[token("]")] RBracket,

    #[token("$")] Dollar,                     // $
    #[token("$$")] DollarDollar,              // $$

    #[regex(r"%[^\n]*")]
    Comment,                                  // % ... 直至行尾

    #[regex(r"[ \t]+")]
    Whitespace,                               // 空格 / Tab
    #[regex(r"\r?\n")]
    Newline,                                  // LF / CRLF
    #[regex(r"\\\\")]
    LineBreak,                                // \\
    #[token(r"\par")]
    Par,                                      // \par 关键字

    Error,                                    // 兜底
}
```

### 1.3 映射到 `SyntaxKind`

```rust
impl TokKind {
    pub fn into_syntax(self) -> SyntaxKind {
        match self {
            TokKind::Command => SyntaxKind::Command,
            TokKind::LBrace => SyntaxKind::LBrace,
            // ...
            TokKind::Dollar | TokKind::DollarDollar => SyntaxKind::MathInline,
            TokKind::Error => SyntaxKind::Error,
        }
    }
}
```

* `Dollar` / `DollarDollar` 都映射到 `MathInline`（在 Pass-3 中由 `split_inline_math` 二次区分）。
* `Newline` / `LineBreak` / `Par` 都映射到 `TokNewline`（在 Pass-3 中触发段落 flush）。

### 1.4 Logos 用法示例

```rust
for (tok, span) in TokKind::lexer(text).spanned() {
    let tok = tok.unwrap_or(TokKind::Error);
    let slice = &text[span.start..span.end];
    // ...
}
```

* `lexer(text)` 返回 `Lexer<TokKind>`。
* `spanned()` 返回 `(Result<TokKind, Error>, Range<usize>)`。
* `Error` token：被 regex 不匹配的字符。

### 1.5 测试

* `lex_section`：`\section{Hi}` 解析为 Command + LBrace + RBrace。
* `lex_brace_pair`：`{}` 解析为 LBrace + RBrace。
* `lex_comment`：`a%bb\nc` 包含 Comment token。

---

## 2. Rowan 语法树（`green.rs` + `parser.rs`）

### 2.1 Rowan 是什么

* `rowan = 0.15`：rust-analyzer 团队开发的增量式红绿树（CST）。
* 提供 `GreenNodeBuilder` 构造不可变树 + `SyntaxNode` 遍历。
* **零拷贝**：节点仅持切片范围，不复制文本。

### 2.2 节点类型（`green.rs`）

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SyntaxKind {
    // 容器
    Root,
    Group,    // { ... }
    Env,      // \begin{name} ... \end{name}

    // 叶子
    Command,
    Text,
    Whitespace,
    Comment,
    MathInline,
    MathDisplay,
    Begin,
    End,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Error,
    TokNewline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Lang {}
impl Language for Lang {
    type Kind = SyntaxKind;
    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind { /* 枚举反查 */ }
    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind { kind.to_raw() }
}

pub type SyntaxNode = rowan::SyntaxNode<Lang>;
pub type SyntaxToken = rowan::SyntaxToken<Lang>;
pub type SyntaxElement = rowan::SyntaxElement<Lang>;
pub type GreenNode = rowan::GreenNode;
```

* **rowan 要求**：`Language` trait 实现必须能 `u16 ↔ SyntaxKind` 转换。
* 我们的 `Lang` 是单元类型，仅为满足 trait 约束。

### 2.3 Pass-2 解析算法（`parser.rs`）

**核心思想：朴素无文法硬编码**——不实现 LR / LALR / GLR，仅做最小组配对。

```rust
pub fn parse(text: &str) -> Parse {
    let mut b = GreenNodeBuilder::new();
    b.start_node(S::Root.to_raw());
    parse_into(text, &mut b);
    b.finish_node();
    let green = b.finish();
    let root = SyntaxNode::new_root(green.clone());
    Parse { green, root, source: text.to_string() }
}

fn parse_into(text: &str, b: &mut GreenNodeBuilder<'static>) {
    use TokKind as T;
    let mut brace_depth: i32 = 0;
    let mut env_stack: Vec<u16> = Vec::new();

    for (tok, span) in T::lexer(text).spanned() {
        let tok = tok.unwrap_or(T::Error);
        let slice = &text[span.start..span.end];
        match tok {
            T::LBrace => {
                b.start_node(S::Group.to_raw());
                brace_depth += 1;
            }
            T::RBrace => {
                if brace_depth > 0 {
                    b.finish_node();
                    brace_depth -= 1;
                } else {
                    b.start_node(S::Error.to_raw());
                    b.token(S::RBrace.to_raw(), slice);
                    b.finish_node();
                }
            }
            T::Command => {
                if slice.starts_with("\\begin") {
                    b.start_node(S::Env.to_raw());
                    env_stack.push(S::Env as u16);
                    b.token(S::Begin.to_raw(), slice);
                } else if slice.starts_with("\\end") {
                    if env_stack.pop().is_some() {
                        b.finish_node();
                    } else {
                        b.start_node(S::Error.to_raw());
                        b.token(S::End.to_raw(), slice);
                        b.finish_node();
                    }
                } else {
                    b.token(S::Command.to_raw(), slice);
                }
            }
            T::Whitespace => b.token(S::Whitespace.to_raw(), slice),
            T::Newline | T::LineBreak | T::Par => b.token(S::TokNewline.to_raw(), slice),
            T::Comment => b.token(S::Comment.to_raw(), slice),
            T::LBracket => b.token(S::LBracket.to_raw(), slice),
            T::RBracket => b.token(S::RBracket.to_raw(), slice),
            T::Dollar | T::DollarDollar => b.token(S::MathInline.to_raw(), slice),
            T::Error => b.token(S::Error.to_raw(), slice),
        }
    }
    // 收尾：未闭合自动闭合（V1 容错：绝不 panic）
    while brace_depth > 0 {
        b.finish_node();
        brace_depth -= 1;
    }
    while env_stack.pop().is_some() {
        b.finish_node();
    }
}
```

### 2.4 关键设计

* **朴素无文法**：不实现 LR parser；只在 token 流上做配对。
* **Group 容器**：`{ ... }` 配对 → `Group` 节点；大括号本身不计入叶子。
* **Env 容器**：`\begin{name} ... \end{name}` 配对 → `Env` 节点。
* **错误恢复**：
  * 多余 `}` → `Error` 节点包住。
  * 多余 `\end` → `Error` 节点包住。
  * 未闭合 → 自动 `finish_node`，但不报错。
* **不识别 `\begin{name}` 后的可选参数**（V1 简化）。

### 2.5 输出

```rust
pub struct Parse {
    pub green: GreenNode,
    pub root: SyntaxNode,
    pub source: String,  // 保留原文本，供 Pass-3 使用
}
```

* `green`：红绿树根节点。
* `root`：包成 `SyntaxNode<Lang>`，可遍历。
* `source`：原文本（Pass-3 不通过 SyntaxNode 拿 text，而是按位置切片 source）。

### 2.6 树结构示例

源文本：
```
\section{Intro}

Hello world.
```

生成的语法树：
```
Root
├── Env (Group?)
│   ├── Command "\section"
│   ├── LBrace "{"
│   ├── Text "Intro"
│   └── RBrace "}"
├── Whitespace "\n\n"
└── Text "Hello world."
```

注：实际树形取决于 `\section{Intro}` 前的 whitespace 状态；V1 把 `{...}` 配为 `Group` 节点。

### 2.7 测试

* `parse_braces`：`{a}` 的 `root.text() == "a"`（大括号不计入 text）。
* `parse_unbalanced_recovers`：`{a` 自动补 RBrace，`text() == "a"`，不 panic。
* `parse_extra_rbrace_recovers`：`}a}` 不 panic。

---

## 3. Pass-2 在降级中的作用

### 3.1 实际：Pass-3 不直接用 SyntaxNode

```rust
// crates/latex-reader/src/lower.rs
pub fn lower_to_document(parse: &Parse, joined: Option<&JoinedStream>) -> Document {
    // ...
    let text = joined.map(|j| j.text.clone()).unwrap_or_else(|| parse.source.clone());
    let text = expand_macros_in(&text, macros);
    let text = strip_preamble(&text);
    // 字符级扫描 + 环境优先 + 段命令 + 段落 buffer
    // ...
}
```

* **关键洞察**：Pass-3 主要用 `parse.source`（原文本），**不**遍历 `SyntaxNode`。
* 这样简化了实现（不需要复杂的 visitor），但失去 CST 精度。
* Rowan 的存在更多是为了**「万一 V2 需要完整 visitor」**保留基础设施。

### 3.2 未来扩展

* V2 路线图：用 Rowan visitor 替换部分字符级扫描（如 `\begin{...}` 配对）。
* 当前实现里 `parser.rs` 与 `lower.rs` 的 `scan_environment` 实际上是**重复实现**——一个用 Rowan 节点，一个用字符级。

---

## 4. Logos 词法 vs Rowan 解析的耦合

| 维度 | Logos | Rowan |
|------|-------|-------|
| 输入 | 字符流 | token 流 |
| 输出 | `TokKind` 序列 | `GreenNode` 树 |
| 性能 | O(n) 单遍 | O(n) 单遍 |
| 内存 | 零拷贝切片 | 树节点 + 切片 |
| 错误恢复 | `Error` token | 节点 + 收尾补 |
| 扩展性 | 加 token 即生效 | 复杂（需 visitor） |

---

## 5. 关键测试

### 5.1 词法（`lexer.rs::tests`）

```rust
#[test]
fn lex_section() {
    let kinds: Vec<_> = TokKind::lexer("\\section{Hi}")
        .spanned().map(|(t, _)| t.unwrap_or(TokKind::Error)).collect();
    assert_eq!(kinds[0], TokKind::Command);
    assert!(kinds.contains(&TokKind::LBrace));
    assert!(kinds.contains(&TokKind::RBrace));
}
```

### 5.2 语法（`parser.rs::tests`）

```rust
#[test]
fn parse_unbalanced_recovers() {
    let p = parse("{a");
    let txt = p.root.text().to_string();
    assert_eq!(txt, "a");
}

#[test]
fn parse_extra_rbrace_recovers() {
    let p = parse("}a}");
    assert!(!p.root.text().to_string().is_empty());
}
```

---

## 6. 已知限制

| 当前限制 | 影响 |
|----------|------|
| Logos token `MathInline` 把 `$$` 当作单 token | Pass-3 需用 `split_inline_math` 二次区分 |
| 朴素解析器不识别 `\begin{tabular}{ccc}` 后的列规范 | Pass-3 字符级处理（`lower_table`） |
| `\verb|...|` 字面量被误分词 | `\verb` 内的 `|` 被识别为 `RBrace` 等 |
| 不区分行内 vs 块级数学 | Pass-3 字符级处理 |

---

## 7. 进一步阅读

* [01-include-topology.md](./01-include-topology.md) — Pass-1
* [03-semantic-lowering.md](./03-semantic-lowering.md) — Pass-3
* [Logos 文档](https://docs.rs/logos/) — 词法库
* [Rowan 文档](https://docs.rs/rowan/) — 语法树库
