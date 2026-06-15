# 关键技术 5：LaTeX 公式 → OMML 数学管道

> 本节深入解析 `doc-mathml` crate。解决的核心问题：把 LaTeX 数学源码嵌入到 docx，**保持 Word 可编辑**（而非退化为图片）。

---

## 1. 模块组成

| 文件 | 职责 |
|------|------|
| `lib.rs` | 公共 re-export（`parse_latex_math` / `to_mathml` / `to_omml` / `MathExpr`） |
| `expr.rs` | `MathExpr` 简化 AST |
| `latex.rs` | LaTeX 数学子集 → `MathExpr` 解析器 |
| `mathml.rs` | `MathExpr` → Presentation MathML |
| `omml.rs` | `MathExpr` → Office MathML（`<m:oMath>`） |

---

## 2. 数据流

```
LaTeX 源码        MathExpr AST       OMML 字节流
   │                  │                  │
   │  parse_latex_math  │  to_omml        │
   ▼──────────────────▼──────────────────▼
"E=mc^2"    MathExpr::Seq([...])    <m:oMath>...
```

---

## 3. `MathExpr` AST

```rust
// crates/mathml/src/expr.rs
pub enum MathExpr {
    /// 字面量
    Number(String),
    /// 标识符（按 italic 渲染）
    Ident(String),
    /// 文本（\text{...}）
    Text(String),
    /// 运算符：+ - * / = < >
    Op(char),
    /// 空格
    Space,
    /// 上下标
    Sub { base: Box<MathExpr>, sub: Box<MathExpr> },
    Sup { base: Box<MathExpr>, sup: Box<MathExpr> },
    SubSup { base, sub, sup },
    /// 分式
    Frac { num: Box<MathExpr>, den: Box<MathExpr> },
    /// 根式
    Sqrt { body: Box<MathExpr>, index: Option<Box<MathExpr>> },
    /// 括号包裹
    Fenced { open: String, body: Box<MathExpr>, close: String },
    /// 函数应用 \sin \cos \tan ...
    Function { name: String, arg: Box<MathExpr> },
    /// 行内矩阵
    Matrix { rows: Vec<Vec<MathExpr>> },
    /// 错误降级：原文
    Raw(String),
    /// 序列
    Seq(Vec<MathExpr>),
}

impl MathExpr {
    /// 折叠空 Seq / 单元素 Seq
    pub fn flatten(seq: Vec<MathExpr>) -> MathExpr { /* ... */ }
    /// 测试辅助：剥开单元素 Seq 包装
    pub fn unwrap_seq(self) -> MathExpr { /* ... */ }
}
```

---

## 4. LaTeX 解析（`latex.rs`）

### 4.1 入口

```rust
pub fn parse_latex_math(input: &str) -> MathExpr {
    let mut p = Parser { s: input, i: 0, depth: 0 };
    let seq = p.parse_seq(false);
    p.skip_ws();
    if p.i < p.s.len() {
        let rest = p.s[p.i..].to_string();
        let mut out = seq;
        out.push(MathExpr::Raw(rest));
        MathExpr::flatten(out)
    } else {
        MathExpr::flatten(seq)
    }
}
```

### 4.2 保护

```rust
const MAX_EXPR_DEPTH: usize = 100;
```

* 嵌套深度超过 100 截断为 `Raw`（防 OOM）。

### 4.3 Parser 结构

```rust
struct Parser<'a> {
    s: &'a str,
    i: usize,
    depth: usize,
}
```

* `depth` 跟踪当前嵌套深度。

### 4.4 主循环（`parse_seq`）

```rust
fn parse_seq(&mut self, stop_brace: bool) -> Vec<MathExpr> {
    let mut out = Vec::new();
    loop {
        self.skip_ws();
        if self.i >= self.s.len() { break; }
        let c = self.peek().unwrap();
        if c == b'}' && stop_brace { break; }
        if c == b']' { break; }  // 留给 caller 处理
        if c == b'&' || c == b'\\' && self.s[self.i..].starts_with("\\\\") {
            break;  // 矩阵行分隔
        }
        if c == b'\\' {
            if self.depth >= MAX_EXPR_DEPTH {
                let rest = self.s[self.i..].to_string();
                out.push(MathExpr::Raw(rest));
                self.i = self.s.len();
                break;
            }
            if let Some(e) = self.parse_command() {
                out.push(e);
                continue;
            } else {
                self.i += 1;
                continue;
            }
        }
        if c == b'^' || c == b'_' {
            // 上下标修饰（基元是上一个 token）
            // ...
        }
        // 单字符（数字 / 字母 / 符号）
        // ...
    }
    out
}
```

### 4.5 支持的命令

| LaTeX | 行为 |
|-------|------|
| `\frac{a}{b}` | `Frac { num: parse(a), den: parse(b) }` |
| `\sqrt{x}` | `Sqrt { body: parse(x), index: None }` |
| `\sqrt[n]{x}` | `Sqrt { body: parse(x), index: Some(parse(n)) }` |
| `\left( ... \right)` | `Fenced { open: "(", body: parse(...), close: ")" }` |
| `\sin / \cos / \tan / \log / \ln / \max / \min` | `Function { name, arg }` |
| `\alpha` ~ `\omega` | `Ident`（希腊字母） |
| `\text{x}` | `Text(x)` |
| `x^{a}_{b}` | `SubSup` |
| `x^{a}` / `x_{a}` | `Sup` / `Sub` |
| `\,` / `\:` / `\;` / `\ ` / `\quad` / `\qquad` | `Space` |
| `\begin{matrix} ... \end{matrix}` | `Matrix { rows }` |
| 未知命令 | 吞一字符 |

### 4.6 错误降级

* 未知命令：吞 1 字节。
* 嵌套超深：剩余部分 → `Raw`。
* parse_seq 末尾有未消费内容：`Raw(rest)`。

### 4.7 测试

* 覆盖：基本数字 / 标识符 / 二元运算 / 上下标 / 分式 / 根式 / 括号 / 三角函数 / 矩阵 / CJK 文本 / 嵌套。

---

## 5. MathML 序列化（`mathml.rs`）

### 5.1 输出

```xml
<?xml version="1.0" encoding="UTF-8"?>
<math xmlns="http://www.w3.org/1998/Math/MathML">
  <mi>x</mi>
  <mo>=</mo>
  <mi>m</mi>
  <mi>c</mi>
  <msup><mi>c</mi><mn>2</mn></msup>
</math>
```

### 5.2 关键元素

| `MathExpr::*` | MathML |
|---------------|--------|
| `Number` | `<mn>` |
| `Ident` | `<mi>` |
| `Text` | `<mtext>` |
| `Op` | `<mo>` |
| `Space` | `<mspace/>` |
| `Sub` | `<msub>` |
| `Sup` | `<msup>` |
| `SubSup` | `<msubsup>` |
| `Frac` | `<mfrac>` |
| `Sqrt { index: None }` | `<msqrt>` |
| `Sqrt { index: Some(_) }` | `<mroot>` |
| `Fenced` | `<mrow><mo>{open}</mo>{body}<mo>{close}</mo></mrow>` |
| `Function` | `<mi>{name}</mi><mo>(</mo><mrow>{arg}</mrow><mo>)</mo>` |
| `Matrix` | `<mtable>` + 多 `<mtr>` + 多 `<mtd>` |
| `Raw` | `<mtext>` |

---

## 6. OMML 序列化（`omml.rs`）

### 6.1 输出

```xml
<?xml version="1.0" encoding="UTF-8"?>
<m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <m:sSup><m:e><mi>x</mi></m:e><m:sup><mn>2</mn></m:sup></m:sSup>
</m:oMath>
```

### 6.2 关键元素

| `MathExpr::*` | OMML |
|---------------|------|
| `Number` | `<m:num>{text}</m:num>` |
| `Ident` / `Text` | `<m:r><w:rPr>...</w:rPr><m:t>...</m:t></m:r>`（run） |
| `Op` | `<m:oSupp><m:begChr>{c}</m:begChr><m:endChr>{c}</m:endChr></m:oSupp>` |
| `Space` | （无） |
| `Sub` | `<m:sSub><m:e>{base}</m:e><m:sub>{sub}</m:sub></m:sSub>` |
| `Sup` | `<m:sSup><m:e>{base}</m:e><m:sup>{sup}</m:sup></m:sSup>` |
| `SubSup` | `<m:sSubSup><m:e>{base}</m:e><m:sub>{sub}</m:sub><m:sup>{sup}</m:sup></m:sSubSup>` |
| `Frac` | `<m:f><m:num>{num}</m:num><m:den>{den}</m:den></m:f>` |
| `Sqrt { index: None }` | `<m:rad><m:deg><m:begChr>...</m:begChr>...</m:deg><m:e>{body}</m:e></m:rad>` |
| `Fenced` | `<m:d>{open}<m:e>{body}</m:e>{close}</m:d>` |
| `Function` | `<m:func><m:fName>{name}</m:fName><m:e>{arg}</m:e></m:func>` |
| `Matrix` | `<m:m><m:mr><m:e>...</m:e></m:mr>...</m:m>` |

### 6.3 关键 run 包装（`write_run_text`）

```rust
fn write_run_text(w: &mut Writer<Vec<u8>>, s: &str) {
    w.write_event(Event::Start(BytesStart::new("m:r"))).unwrap();
    w.write_event(Event::Start(BytesStart::new("m:rPr"))).unwrap();
    // 默认样式（Roman，无加粗）
    w.write_event(Event::Start(BytesStart::new("m:sty"))).unwrap();
    w.write_event(Event::Start(BytesStart::new("m:rStyle"))).unwrap();
    w.write_event(Event::End(BytesEnd::new("m:rStyle"))).unwrap();
    w.write_event(Event::End(BytesEnd::new("m:sty"))).unwrap();
    w.write_event(Event::End(BytesEnd::new("m:rPr"))).unwrap();
    // 文本
    w.write_event(Event::Start(BytesStart::new("m:t"))).unwrap();
    w.write_event(Event::Text(quick_xml::events::BytesText::new(s))).unwrap();
    w.write_event(Event::End(BytesEnd::new("m:t"))).unwrap();
    w.write_event(Event::End(BytesEnd::new("m:r"))).unwrap();
}
```

* 所有 OMML 文本必须包在 `<m:r>` 中（含 `<m:rPr>`）。

### 6.4 嵌入 docx

`doc-docx-writer::serializer::write_equation`：
1. 写 `<w:p>` 起始。
2. 调 `parse_latex_math` + `to_omml`。
3. 剥出 `<m:oMath>` 段，直接 `write_all` 到当前 writer（混入 document.xml）。
4. 写 `<w:p>` 结束。

---

## 7. 完整流程示例

**LaTeX**：`E = mc^2`

**`parse_latex_math`** 输出：
```rust
MathExpr::Seq(vec![
    MathExpr::Ident("E"),
    MathExpr::Space,
    MathExpr::Op('='),
    MathExpr::Space,
    MathExpr::Ident("m"),
    MathExpr::Ident("c"),
    MathExpr::Sup {
        base: Box::new(MathExpr::Ident("c")),
        sup: Box::new(MathExpr::Number("2")),
    },
])
```

**`to_omml`** 输出（摘选）：
```xml
<m:oMath xmlns:m="...">
  <m:r>...<m:t xml:space="preserve">E</m:t></m:r>
  <m:sSup><m:e>...<m:t>c</m:t>...</m:e><m:sup>...<m:t>2</m:t>...</m:sup></m:sSup>
</m:oMath>
```

**docx 嵌入**（`word/document.xml`）：
```xml
<w:p>
  <m:oMath xmlns:m="...">
    <m:r><m:t>E</m:t></m:r>
    ...
  </m:oMath>
</w:p>
```

**Word 打开**：公式显示为 "E = mc²"（可编辑数学对象）。

---

## 8. 测试

| 文件 | 覆盖 |
|------|------|
| `src/latex.rs::tests` | 基础解析、上下标、分式、根式、矩阵 |
| `src/mathml.rs::tests` | Presentation MathML 输出 |
| `src/omml.rs::tests` | OMML 输出 |

---

## 9. 已知限制

| 当前限制 | 影响 | V2 方向 |
|----------|------|---------|
| 不识别 `\begin{bmatrix}` 等变体 | LaTeX 丰富矩阵 | V2 加 |
| `\hat{a}` / `\bar{a}` 等重音命令 | 整段 Raw | V2 加 |
| `\sum` / `\int` 等大型运算符 | 直译为 Ident | V2 加 `<m:nary>` |
| `\begin{cases}` 多 case | 整段 Raw | V2 加 |
| `\mathrm{...}` / `\mathbf{...}` 等字体切换 | 整段 Raw | V2 加 |
| `\overset{x}{y}` / `\underset{x}{y}` | 整段 Raw | V2 评估 |
| 长表达式性能 | 慢 | V2 优化 |

---

## 10. 进一步阅读

* [03-semantic-lowering.md](./03-semantic-lowering.md) — 数学在 Pass-3 怎么进 `Block::Equation`
* [04-docx-serialization.md](./04-docx-serialization.md) — `<m:oMath>` 嵌入 docx
* [MathML 规范](https://www.w3.org/Math/) — Presentation MathML
* [OMML 规范（ECMA-376 Part 1, 17.18）](http://www.ecma-international.org/publications/standards/Ecma-376.htm) — Office MathML
