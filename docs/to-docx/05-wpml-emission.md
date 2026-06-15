# 05 · WordprocessingML 写入

> 目标：把 §4 的 Block 列表，转换为一组合法的 OOXML 字符串。Rust 重构时**直接抄这些模板字符串**是最高效的做法——docx 的 XML schema 庞大，自己手写易错。

## 5.1 OOXML 概览

OOXML 文档 = ZIP 包 + 一堆 XML part。**对本项目重要的命名空间**：

| 前缀 | URI | 作用 |
|------|-----|------|
| `w` | `http://schemas.openxmlformats.org/wordprocessingml/2006/main` | 文字/段落/样式 |
| `r` | `http://schemas.openxmlformats.org/officeDocument/2006/relationships` | 关系 |
| `wp` | `http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing` | 绘图（图片） |
| `a` | `http://schemas.openxmlformats.org/drawingml/2006/main` | 图形通用 |
| `pic` | `http://schemas.openxmlformats.org/drawingml/2006/picture` | 图片 |

每个 part 都有自己的根元素 + 命名空间声明。本项目的所有 part 用一致的 `xmlns:w` / `xmlns:r` / `xmlns:wp` / `xmlns:a` / `xmlns:pic` 声明。

## 5.2 `inline_runs_xml` —— 文本 → 多 run

```python
def inline_runs_xml(text, enable_superscript=True, enable_subscript=False) -> str:
    if not enable_superscript: return run_xml(text)
    runs, buf, i = [], [], 0
    while i < len(text):
        citation = CITATION_RE.match(text, i)
        if citation:
            if buf: runs.append(run_xml("".join(buf))); buf = []
            runs.append(run_xml(citation.group(0), superscript=True))
            i = citation.end(); continue
        if text[i] == "^" and i + 1 < len(text):
            if buf: runs.append(run_xml("".join(buf))); buf = []
            if text[i + 1] == "{":
                end = text.find("}", i + 2)
                if end > i + 2:
                    runs.append(run_xml(text[i + 2 : end], superscript=True))
                    i = end + 1; continue
            m = re.match(r"\^([A-Za-z0-9*]+)", text[i:])
            if m:
                runs.append(run_xml(m.group(1), superscript=True))
                i += len(m.group(0)); continue
        if enable_subscript and text[i] == "_" and i + 1 < len(text):
            if buf: runs.append(run_xml("".join(buf))); buf = []
            if text[i + 1] == "{":
                end = text.find("}", i + 2)
                if end > i + 2:
                    runs.append(run_xml(text[i + 2 : end], subscript=True))
                    i = end + 1; continue
            m = re.match(r"_([A-Za-z0-9]+)", text[i:])
            if m:
                runs.append(run_xml(m.group(1), subscript=True))
                i += len(m.group(0)); continue
        buf.append(text[i]); i += 1
    if buf: runs.append(run_xml("".join(buf)))
    return "".join(runs)
```

### 5.2.1 规则表

| 输入模式 | 输出 | 例 |
|---------|------|----|
| `[N]` / `[N-M]` / `[N,M]` | 上标 | `[1]` → 上标 1 |
| `^X`（X 是单字符） | 上标 | `^2` → 上标 2 |
| `^{XYZ}` | 上标 | `^{10}` → 上标 10 |
| `_{XYZ}` | 下标 | `_{n}` → 下标 n |
| `_X`（X 是字符） | 下标 | `_1` → 下标 1 |
| 其他 | 普通 | |

注意**下标只在下述条件之一时启用**：

- 样式是 `JOSCode`（公式/算法）；
- 文本形如 `\b[A-Za-z]_[A-Za-z0-9]+`（如 `l_1`）。

这个启发式避免正文中的"路_a"之类被误切。

### 5.2.2 `run_xml` 与 `direct_run_xml`

```python
def run_xml(text, superscript=False, subscript=False):
    if text == "": return ""
    if superscript: rpr = '<w:rPr><w:vertAlign w:val="superscript"/></w:rPr>'
    elif subscript: rpr = '<w:rPr><w:vertAlign w:val="subscript"/></w:rPr>'
    else: rpr = ""
    return f'<w:r>{rpr}<w:t xml:space="preserve">{xml(text)}</w:t></w:r>'
```

```python
def direct_run_xml(text, *, size_half_points, east, ascii_font="Times New Roman",
                   bold=False, superscript=False, subscript=False):
    if text == "": return ""
    rpr = [
        f'<w:rFonts w:ascii="{ascii_font}" w:hAnsi="{ascii_font}" w:eastAsia="{east}" w:cs="{ascii_font}"/>',
        f'<w:sz w:val="{size_half_points}"/><w:szCs w:val="{size_half_points}"/>',
    ]
    if bold: rpr.append("<w:b/><w:bCs/>")
    if superscript: rpr.append('<w:vertAlign w:val="superscript"/>')
    elif subscript: rpr.append('<w:vertAlign w:val="subscript"/>')
    return f'<w:r><w:rPr>{"".join(rpr)}</w:rPr><w:t xml:space="preserve">{xml(text)}</w:t></w:r>'
```

**两者区别**：

- `run_xml`：不写 `rFonts` / `sz`，完全由段落样式决定字体字号。**正文段落用这个**。
- `direct_run_xml`：写死字体字号。**表格内文字用这个**（表内样式不接管字体）。

`table_inline_runs_xml` 是 `inline_runs_xml` 的表格特化版：把 `run_xml` 换成 `direct_run_xml(size=15 半磅 = 7.5pt, east="宋体")`、首行 `bold=True`。

## 5.3 `xml()` —— 文本转义

```python
def xml(text: str) -> str:
    return escape(text, {"\u00a0": " "})
```

`xml.sax.saxutils.escape` 把 `& < > " '` 转义为 XML 实体。**额外把 `&nbsp;` (\u00a0) 替换为普通空格**——避免 Word 把不间断空格粘到上一个词。

> 注意：本项目生成的 OOXML 中**不出现** `<` `>` `&`，因为 LaTeX 源已经被 `latex_to_text` 干净化了。`url{}` 的内容是 http URL，里面只含 `:` `/` `.` 等，无需转义。

Rust 推荐 `quick-xml` 的 `escape` 函数 + 自定义 `&nbsp;` 替换。

## 5.4 段落 `add_paragraph` / `add_kept_paragraph`

两者几乎一致，区别是 `add_kept_paragraph` 支持 `keep_next` / `keep_lines`：

```python
def add_kept_paragraph(self, text, style="JOSBody", align=None, *,
                       keep_next=False, keep_lines=False):
    if not text: return
    text = fix_display_text(text)
    ppr = [f'<w:pStyle w:val="{style}"/>']
    if keep_next: ppr.append("<w:keepNext/>")
    if keep_lines: ppr.append("<w:keepLines/>")
    if align: ppr.append(f'<w:jc w:val="{align}"/>')
    enable_superscript = not style.startswith("JOSReference")
    enable_subscript = style == "JOSCode" or bool(re.search(r"\b[A-Za-z]_[A-Za-z0-9]+", text))
    if style == "JOSCode":
        text = clean_formula_display_text(text)
    self.parts.append(
        "<w:p><w:pPr>" + "".join(ppr) + "</w:pPr>" +
        inline_runs_xml(text, enable_superscript, enable_subscript) +
        "</w:p>"
    )
```

## 5.5 `add_tabbed_paragraph` —— 制表位双列

用于首页期刊题头（左文 + 右文，中间用制表符拉成右对齐）：

```python
def add_tabbed_paragraph(self, left, right, style="JOSMasthead"):
    self.parts.append(
        "<w:p><w:pPr>"
        f'<w:pStyle w:val="{style}"/>'
        f'<w:tabs><w:tab w:val="right" w:pos="{self.text_width_twips}"/></w:tabs>'
        "</w:pPr>"
        f'<w:r><w:t xml:space="preserve">{xml(left)}</w:t></w:r>'
        "<w:r><w:tab/></w:r>"
        f'<w:r><w:t xml:space="preserve">{xml(right)}</w:t></w:r>'
        "</w:p>"
    )
```

`<w:tab w:val="right" w:pos="N"/>` 定义一个制表位，位置在 `text_width_twips`，对齐方式 `right`（右贴齐）。`<w:r><w:tab/></w:r>` 触发跳到下一个制表位。

## 5.6 `add_spacer` —— 纯空行

```python
def add_spacer(self, height_twips):
    self.parts.append(
        f'<w:p><w:pPr><w:spacing w:line="{height_twips}" w:lineRule="exact"/></w:pPr></w:p>'
    )
```

**严格行高**（`lineRule="exact"`）的空白段落，没有 run。Rust 端直接写字符串即可。

## 5.7 `document_xml` —— 整个 word/document.xml

```python
def document_xml(self) -> str:
    sect = (
        '<w:sectPr>'
        '<w:headerReference w:type="first" r:id="rId8"/>'
        '<w:headerReference w:type="default" r:id="rId3"/>'
        '<w:headerReference w:type="even" r:id="rId4"/>'
        '<w:footerReference w:type="first" r:id="rId5"/>'
        '<w:footerReference w:type="default" r:id="rId6"/>'
        '<w:footerReference w:type="even" r:id="rId7"/>'
        '<w:titlePg/>'
        f'<w:pgSz w:w="{self.paper["w"]}" w:h="{self.paper["h"]}"/>'
        f'<w:pgMar w:top="..." w:right="..." w:bottom="..." w:left="..." '
        f'w:header="..." w:footer="{self.profile.footer_distance_twips}" '
        f'w:gutter="{self.margins.get("gutter", "0")}"/>'
        f'<w:cols w:space="{self.columns.get("space", "720")}" '
        f'w:num="{self.columns.get("num", "1")}"/>'
        '</w:sectPr>'
    )
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:document xmlns:w="..." xmlns:r="..." xmlns:wp="..." '
        'xmlns:a="..." xmlns:pic="...">'
        '<w:body>' + "".join(self.parts) + sect + '</w:body></w:document>'
    )
```

**关键属性**：

- `<w:titlePg/>` 表示首页使用独立的页眉页脚（`type="first"`），其余页用 `default` 和 `even`。
- `w:pgSz` / `w:pgMar` / `w:cols` 直接来自 `format_data["page_setup"]`（JOS 2025 样例）。
- `w:footer` 距离由 `profile.footer_distance_twips` 覆盖（默认 1260 twips ≈ 2.22 cm，比 sample 的 567 大；原因是 sample 用 0.85 磅字号脚注页码时 567 偏小）。

### 5.7.1 段尾 sectPr 必须在 `<w:body>` 末尾

**注意**：`sectPr` 在 OOXML 中必须紧跟所有 paragraph 之后，作为最后一个子元素。Rust 端用 `format!` 拼装时不要把它包在 paragraph 里。

## 5.8 styles.xml

`style()` 函数构造单个 `<w:style>` 元素：

```python
def style(style_id, name, size, east, ascii_font="Times New Roman", bold=False,
          jc=None, first_line=None, left=None, hanging=None, before=None, after=None, line=None):
    ppr = [f'<w:jc w:val="{jc}"/>'] if jc else []
    spacing_attrs = []
    if before is not None: spacing_attrs.append(f'w:before="{before}"')
    if after is not None: spacing_attrs.append(f'w:after="{after}"')
    if line is not None: spacing_attrs.append(f'w:line="{line}" w:lineRule="exact"')
    if spacing_attrs: ppr.append("<w:spacing " + " ".join(spacing_attrs) + "/>")
    ind_attrs = []
    if first_line is not None: ind_attrs.append(f'w:firstLine="{first_line}"')
    if left is not None: ind_attrs.append(f'w:left="{left}"')
    if hanging is not None: ind_attrs.append(f'w:hanging="{hanging}"')
    if ind_attrs: ppr.append("<w:ind " + " ".join(ind_attrs) + "/>")
    rpr = (
        f'<w:rFonts w:ascii="{ascii_font}" w:hAnsi="{ascii_font}" w:eastAsia="{east}" w:cs="{ascii_font}"/>'
        f'<w:sz w:val="{int(size * 2)}"/><w:szCs w:val="{int(size * 2)}"/>'
        + ("<w:b/>" if bold else "")
    )
    return (
        f'<w:style w:type="paragraph" w:styleId="{style_id}">'
        f'<w:name w:val="{xml(name)}"/>'
        + ("<w:pPr>" + "".join(ppr) + "</w:pPr>" if ppr else "")
        + f"<w:rPr>{rpr}</w:rPr></w:style>"
    )
```

所有 21 个样式见 §7.5。

## 5.9 `header_xml` / `first_header_xml` / `footer_xml`

### 5.9.1 普通页眉 `header_xml`

```python
def header_xml(text, text_width_twips):
    runs = (
        f'<w:r><w:t xml:space="preserve">{xml(text)}</w:t></w:r>'
        "<w:r><w:tab/></w:r>"
        f"{page_field_xml()}"
    )
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        '<w:p><w:pPr><w:pStyle w:val="JOSMasthead"/>'
        f'<w:tabs><w:tab w:val="right" w:pos="{text_width_twips}"/></w:tabs>'
        "</w:pPr>"
        f"{runs}</w:p></w:hdr>"
    )
```

左侧：标题文字；右侧：页码字段。`JOSMasthead` 样式：宋体 7.5pt、行距 180。

### 5.9.2 首页页眉 `first_header_xml`

三行 + 制表位：

```text
软件学报 ISSN 1000-9825, CODEN RUXUEW              E-mail: jos@iscas.ac.cn
Journal of Software, [doi: 10.13328/j.cnki.jos.000000]   http://www.jos.org.cn
© 中国科学院软件研究所版权所有.                      Tel: +86-10-62562563
```

每行用 `add_tabbed_paragraph` 同样的机制：`<w:tabs><w:tab w:val="right" w:pos="..."/></w:tabs>` + `<w:r><w:tab/></w:r>`。

### 5.9.3 页脚 `footer_xml`

```python
def footer_xml(text="", indent_twips=0):
    if text:
        body = (
            '<w:p><w:pPr><w:pStyle w:val="JOSMasthead"/><w:jc w:val="left"/>'
            f'<w:ind w:left="{indent_twips}"/>'
            '<w:pBdr><w:top w:val="single" w:sz="4" w:space="1" w:color="auto"/></w:pBdr>'
            "</w:pPr>"
            f'<w:r><w:t xml:space="preserve">{xml(text)}</w:t></w:r></w:p>'
        )
    else:
        body = "<w:p/>"
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        f"{body}</w:ftr>"
    )
```

- 首页页脚：实线 + "收稿时间..." 文字（带 330 twips 缩进）。
- 其他页脚：`<w:p/>` 空段落（无内容，但 Word 仍要求 part 存在）。

## 5.10 `page_field_xml` —— 页码字段

```python
def page_field_xml():
    return (
        '<w:fldSimple w:instr=" PAGE ">'
        '<w:r><w:t>1</w:t></w:r>'
        "</w:fldSimple>"
    )
```

`<w:fldSimple>` 是简单字段；`w:instr=" PAGE "` 告诉 Word 渲染时替换为当前页号。`<w:t>1</w:t>` 是字段未计算时的回退显示值。Word 打开后会重算。

## 5.11 `settings.xml`

```python
def settings_xml():
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        '<w:evenAndOddHeaders/><w:characterSpacingControl w:val="doNotCompress"/>'
        '</w:settings>'
    )
```

- `<w:evenAndOddHeaders/>` 启用奇偶页不同页眉（与 `header2` 配合）。
- `<w:characterSpacingControl w:val="doNotCompress"/>` 禁用字符间距压缩（保持汉字宽度稳定）。

## 5.12 `content_types_xml` 与 `package_rels_xml`

见 §6。

## 5.13 写盘的 `write_docx`

```python
def write_docx(builder, output, manuscript, profile=JOS_PROFILE):
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("[Content_Types].xml", content_types_xml())
        zf.writestr("_rels/.rels", package_rels_xml())
        zf.writestr("word/document.xml", builder.document_xml())
        zf.writestr("word/_rels/document.xml.rels", builder.rels_xml())
        zf.writestr("word/styles.xml", styles_xml())
        zf.writestr("word/settings.xml", settings_xml())
        zf.writestr("word/header0.xml", first_header_xml(builder.text_width_twips, profile.first_header_rows))
        zf.writestr("word/header1.xml", header_xml(running_header, builder.text_width_twips))
        zf.writestr("word/header2.xml", header_xml(profile.even_header_text, builder.text_width_twips))
        zf.writestr("word/footer1.xml", footer_xml(first_footer, profile.first_footer_indent_twips))
        zf.writestr("word/footer2.xml", footer_xml())
        zf.writestr("word/footer3.xml", footer_xml())
        for src, arcname in builder.media:
            zf.write(src, arcname)
```

Rust 端用 `zip` crate 即可。要点：

- 压缩方式 `DEFLATED`（不要 STORED）。
- `[Content_Types].xml` **必须**是包内第一个 entry（虽然 OOXML 规范不强制，但 Word 习惯）。
- 媒体文件直接 `zip.write(src_path, arcname)`——二进制内容原样写入。

## 5.14 Rust 端实现细节

### 5.14.1 crate 选型

| 用途 | crate |
|------|-------|
| 读 tex/bbl/json | `std::fs`, `serde`, `serde_json`, `regex` |
| 写 XML | 不用 crate，**手写字符串模板**（最稳定） |
| 写 ZIP | `zip`（含 `deflate` 支持） |
| 读 PDF（校验用） | `lopdf` |
| 读图片尺寸 | `image` |
| 时间戳 | `chrono`（或纯 `SystemTime`） |
| 路径处理 | `std::path::PathBuf` |

### 5.14.2 XML 转义

手写一个 `xml_escape(s: &str) -> String`，把 `& < > " '` 替换为 `&amp;` 等。WordprocessingML **不** 接受 HTML 风格的 `&#NNN;` 数字实体——用 `&lt;` `&gt;` `&amp;` `&quot;` `&apos;`。

```rust
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\u{00A0}' => out.push(' '),  // nbsp
            _ => out.push(c),
        }
    }
    out
}
```

### 5.14.3 半磅 / twip 转换

```rust
fn pt_to_halfpt(pt: f32) -> u32 { (pt * 2.0).round() as u32 }
fn cm_to_twip(cm: f64) -> u32 { (cm * 567.0).round() as u32 }
fn cm_to_emu(cm: f64) -> u32 { (cm * 360_000.0).round() as u32 }
```

不要用浮点比较——Rust 推荐全程 `u32` 表示 twip/EMU。
