# 04 · 块构造（front matter + body blocks）

> 上一章我们把 LaTeX 文本压平成"富文本字符串"，本章把所有内容**重新组合**为结构化 `Block` 列表，再交给 `populate` 写出 OOXML。
>
> 块构造是"格式/语义对齐到 JOS 模板"的关键步骤。

## 4.1 `populate(builder, ms, profile)` —— 整篇文档的"装配顺序"

```text
顺序写死的（不能改）：

 1. 中文标题             style=JOSTitleZh  align=left
 2. 中文作者             style=JOSAuthorZh align=left
 3. 中文机构（多行）       style=JOSInstituteZh align=left
 4. spacer               height = profile.after_institute_twips (300)
 5. 中文摘要             style=JOSAbstractZh
       内容：profile.zh_abstract_label + " " + ms.abstract_zh
 6. 中文关键词           style=JOSKeywords
       内容：profile.zh_keywords_label + " " + spaced_keywords(ms.keywords_zh)
 7. 中图法分类号         style=JOSBodyNoIndent
       内容：profile.category_label + " " + ms.category
 8. spacer               height = profile.before_citation_twips (300)
 9. 中文引用格式（多行）   style=JOSCitation
       文本先 wrap_text_units(52) 拆段
10. 英文引用格式（多行）   style=JOSCitation
11. spacer               height = profile.before_english_title_twips (220)
12. 英文标题             style=JOSEnglishTitle
13. 英文作者             style=JOSCitation
14. 英文机构             style=JOSCitation
15. spacer               height = profile.before_english_abstract_twips (340)
16. 英文摘要             style=JOSAbstractEn
       内容：profile.en_abstract_label + "   " + ms.abstract_en
17. 英文关键词           style=JOSKeywords
       内容：profile.en_keywords_label + " " + spaced_keywords(ms.keywords_en)

18..N. 遍历 ms.blocks（章节流）：
       heading    → JOSHeading1/2/3
       paragraph  → JOSBody
       list_item  → JOSBody
       table      → 先 caption (keep_next + keep_lines + center) 再 <w:tbl>
       figure     → 先图片段落 (center) 再 caption
       algorithm  → 先 caption (keep_next + keep_lines) 再逐行 JOSCode
       equation   → "text    (N)" center 走 JOSCode

N+1. 诚信声明           style=JOSBody
       "本文撰写与实验脚本生成过程中使用了大语言模型辅助，作者对全部内容与数据负责。"
N+2. References         style=JOSReferenceHeading
N+3.. 英文文献条目       style=JOSReference  "[" + N + "] " + ref
随后： 附中文参考文献     style=JOSReferenceHeading
随后： 中文参考条目       style=JOSReference
随后： 作者简介           style=JOSReferenceHeading
随后： 作者简介条目       style=JOSReference
```

Rust 端把 populate 写成 1 个长 `for` 循环或 `match` 即可。

## 4.2 标题层级映射

| `Block.level` | 样式 | 编号格式 |
|--------------|------|---------|
| 1 | `JOSHeading1` | `1 标题` |
| 2 | `JOSHeading2` | `1.1 标题` |
| 3 | `JOSHeading3` | `1.1.1 标题` |
| (其他) | `JOSHeading3` | （兜底） |

注意 `parse_sections` 已经在 level 1 的触发处把 `subsection_no` / `subsubsection_no` 重置为 0，所以编号正确。

## 4.3 段落（paragraph / list_item）

```python
def add_paragraph(text, style="JOSBody", align=None):
    ppr = [f'<w:pStyle w:val="{style}"/>']
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

要点：

- 参考文献样式（`JOSReference*`）**禁用上标**——否则 `[1]` 这种会被识别成上标数字。
- **下标**只在 `JOSCode` 或文本形如 `x_y` 时启用（避免正文误伤）。
- `JOSCode` 走 `clean_formula_display_text` 额外清理（见 §3.6）。

## 4.4 表格

```python
def add_table(self, rows):
    if not rows: return
    max_cols = max(len(r) for r in rows)
    cell_width = max(1, int(self.text_width_twips / max_cols))
    table_rows = []
    for row_idx, row in enumerate(rows):
        cells = []
        for cell in row + [""] * (max_cols - len(row)):
            cell = fix_display_text(cell)
            ppr = '<w:pStyle w:val="JOSTableText"/><w:keepLines/>'
            if row_idx < row_count - 1:
                ppr += "<w:keepNext/>"
            cells.append(
                f'<w:tc><w:tcPr><w:tcW w:w="{cell_width}" w:type="dxa"/></w:tcPr>'
                f"<w:p><w:pPr>{ppr}</w:pPr>"
                f"{table_inline_runs_xml(cell, bold=row_idx == 0)}</w:p></w:tc>"
            )
        table_rows.append("<w:tr><w:trPr><w:cantSplit/></w:trPr>" + "".join(cells) + "</w:tr>")
    self.parts.append(
        '<w:tbl><w:tblPr><w:tblW w:w="5000" w:type="pct"/>'
        '<w:tblBorders>'
        '<w:top w:val="single" w:sz="6" w:space="0" w:color="000000"/>'
        '<w:left w:val="nil"/>'
        '<w:bottom w:val="single" w:sz="6" w:space="0" w:color="000000"/>'
        '<w:right w:val="nil"/>'
        '<w:insideH w:val="single" w:sz="3" w:space="0" w:color="666666"/>'
        '<w:insideV w:val="single" w:sz="3" w:space="0" w:color="666666"/>'
        '</w:tblBorders></w:tblPr>'
        + "".join(table_rows) + "</w:tbl>"
    )
```

规则：

1. **首行加粗**（`bold = row_idx == 0`）—— 这是 JOS 模板的"表头"。
2. **首列不加粗**（项目里列名不一定在首列，但 JOS 模板中"表头"特指首行）。
3. **所有 cell 等宽**：`text_width_twips / max_cols`（不再按内容调整列宽）。
4. **每个 cell 内段落**走 `JOSTableText` 样式 + 上下间距为零 + 居中。
5. **首列以外所有 cell 的段落都加 `<w:keepNext/>` + `<w:keepLines/>`**——保证长 cell 内容不被切到下一页。
6. 整张表宽 100% 页面（`w:tblW w:w="5000" w:type="pct"`）。
7. **边框样式**：
   - 上边 6pt 黑实线
   - 下边 6pt 黑实线
   - 左右 0pt（开口）
   - 内部横线 3pt 灰（666666）
   - 内部竖线 3pt 灰
8. `<w:cantSplit/>` 让整行不跨页。

> Rust 实现：把每个 `<w:tbl>` 写成一个完整的 XML 字符串；不要尝试用 DOM 拼装，原因是 docx 的表结构嵌套深、属性多，纯字符串拼装与原 Python 版字节级一致最省事。

## 4.5 图片

```python
def add_image(self, path, width_factor, caption):
    if path is None or not path.exists():
        self.add_paragraph(f"[缺图] {caption}", "JOSCaption", "center")
        return
    if Image is None: raise RuntimeError("PIL is required to read image dimensions")
    suffix = path.suffix.lower().lstrip(".") or "png"
    media_name = f"image{len(self.media) + 1}.{suffix}"
    rid = f"rId{self.next_rid}"; self.next_rid += 1
    self.media.append((path, f"word/media/{media_name}"))
    self.image_rels.append((rid, f"media/{media_name}"))
    with Image.open(path) as img:
        px_w, px_h = img.size
    width_cm = min(self.text_width_cm * width_factor, self.text_width_cm)
    height_cm = width_cm * px_h / max(px_w, 1)
    cx = int(width_cm * EMU_PER_CM)
    cy = int(height_cm * EMU_PER_CM)
    docpr = self.next_docpr; self.next_docpr += 1
    drawing = f"""
<w:p><w:pPr><w:pStyle w:val="JOSImage"/><w:keepNext/><w:keepLines/><w:jc w:val="center"/></w:pPr><w:r><w:drawing>
<wp:inline distT="0" distB="0" distL="0" distR="0">
<wp:extent cx="{cx}" cy="{cy}"/><wp:effectExtent l="0" t="0" r="0" b="0"/>
<wp:docPr id="{docpr}" name="{xml(path.stem)}"/><wp:cNvGraphicFramePr/>
<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
<pic:pic><pic:nvPicPr><pic:cNvPr id="{docpr}" name="{xml(path.name)}"/><pic:cNvPicPr/></pic:nvPicPr>
<pic:blipFill><a:blip r:embed="{rid}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>
<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm>
<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic>
</a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>
"""
    self.parts.append(drawing)
```

要点（Rust 复刻时一字不差）：

1. 图片后缀取自文件实际后缀（`.png`/`.jpg`/`.jpeg`），没有就默认 `.png`。
2. `rId` 从 20 起递增（rId1~rId8 已用于 styles/settings/header/footer，见 §6）。
3. 媒体文件目标路径 `word/media/imageN.{suffix}`。
4. 用 PIL 读像素尺寸后**按 width_factor 缩放**——width_factor 来自 LaTeX `width=0.85\textwidth`。
5. 实际宽度 = `text_width_cm × width_factor`（**不能超过** text_width_cm）。
6. EMU 单位 = `cm × 360000`。
7. `<wp:inline>` 不带 anchor（不允许浮动）。
8. `<a:blip r:embed="rId...">` 引用关系——后续 `document.xml.rels` 必须给这个 rId 配 `<Relationship Type=".../image" Target="media/imageN.png"/>`。
9. `<wp:docPr id name>` 提供 doc-level 元数据（用于悬浮提示）。
10. 段落属性：`JOSImage` 样式 + `keepNext` + `keepLines` + `center`，**让图片与图题同页**。

> 缺图处理：调用 `add_paragraph("[缺图] " + caption, "JOSCaption", "center")`——这一行至关重要，**不能**让整篇文档因为缺一张图就 fail。

### 4.5.1 `maybe_convert_pdf` 兜底

如果 `figures/xxx.pdf` 在但 `figures/xxx.png` 不在，调用：

```bash
pdftoppm -png -singlefile -r 220 <pdf> <prefix>
```

得到 `<prefix>.png`。若 pdftoppm 不可用，再次失败时图片路径就是 `None`，走缺图分支。Rust 端可以用 `pdfium-render` 自己做。

## 4.6 算法块（caption + 多行代码）

```python
elif block.kind == "algorithm":
    builder.add_kept_paragraph(
        block.caption, "JOSCaption", "center",
        keep_next=bool(block.lines), keep_lines=True,
    )
    for idx, line in enumerate(block.lines):
        builder.add_kept_paragraph(
            line, "JOSCode",
            keep_next=idx < len(block.lines) - 1,
            keep_lines=True,
        )
```

- caption 走 `keep_next + keep_lines`——caption 与第一行代码同页。
- 每个代码行 `keep_lines`——单行不跨页。
- 除最后一行外**全部** `keep_next`——保证整个算法块不被分页。
- `JOSCode` 样式（详见 §7.5.13）：宋体 8pt（half-point 16）、Courier New 拉丁、行距 220 twip。

`block.lines` 是 `legacy` 字段，由 `parse_algorithm` 拼成：

```python
legacy_lines = [
    *(f"{label}: {text}" for label, text in algorithm_io),  # "Input: ..." / "Output: ..."
    *(str(row["code"]) for row in rows),                     # 逐行算法体
]
```

## 4.7 公式

```python
elif block.kind == "equation":
    suffix = f"    {block.caption}" if block.caption else ""
    builder.add_paragraph(f"{block.text}{suffix}", "JOSCode", "center")
```

- 公式内容 `block.text` 已走过 `clean_math`，所以是纯文本。
- `block.caption` 形如 `(1)`——比正文缩进 4 个半角空格。
- 居中显示（`align=center`）。

> ⚠️ **没有用 Office Math / `<m:oMath>`**——而是用普通段落 + 半角空格 + 居中对齐。这意味着公式**不可二次编辑为真正的数学公式**。这是本项目的折中：保证版式与样本一致即可，编辑需求不强。

## 4.8 页眉/页脚内容生成

页眉/页脚**不**经过 `populate`——它们是单独的 XML part：

```python
running_header = manuscript.running_header or derived_running_header(manuscript)
first_footer = manuscript.first_footer_text or profile.first_footer_text
```

- `running_header` = `\rjhead{...}` 的内容，例如"石洪雷 等: 网关流量驱动的微服务定向日志采集框架"
- `first_footer` = `\footnotetext{...}` 的内容，例如"收稿时间: XXXX-XX-XX; 修改时间: XXXX-XX-XX; 采用时间: XXXX-XX-XX"
- 若 `running_header` 为空，则用 `derived_running_header` 兜底（首作者 + "等:" + 中文标题）

### 4.8.1 `derived_running_header`

```python
def first_author_name(authors: str) -> str:
    first = re.split(r"[,，;；]", authors, maxsplit=1)[0]
    return re.sub(r"\s+", "", first).strip()

def derived_running_header(ms: Manuscript) -> str:
    author = first_author_name(ms.authors_zh)
    return f"{author} 等: {ms.title_zh}" if author and ms.title_zh else ms.title_zh
```

`first_author_name` 会把"石 洪 雷, 赵 涓 涓"（注意每个汉字间有空格）切成"石洪雷"——`\s+` 删掉所有空白。

## 4.9 列表项 `parse_enumerate`

```python
def parse_enumerate(env_text, cite_map, label_map) -> list[Block]:
    result = []
    parts = re.split(r"\\item\s*", env_text)
    for idx, item in enumerate(parts[1:], 1):
        text = latex_to_text(item, cite_map, label_map)
        if text: result.append(Block(kind="list_item", text=f"{idx}. {text}"))
    return result
```

特点：

- 编号是**手工加上**的"1. "、"2. "——而不是用 Word 真正的 `<w:numPr>` 列表。
- 因此不会出现在 `Word Numbering` 侧栏，但**视觉等价**。

## 4.10 后置诚信声明

```python
builder.add_paragraph(
    "本文撰写与实验脚本生成过程中使用了大语言模型辅助，作者对全部内容与数据负责。",
    "JOSBody",
)
```

这是 JOS 投稿模板**必须**的声明，**不能**与正文段落混合——单独一行加在所有正文块之后、参考文献标题之前。

## 4.11 参考文献节

```python
builder.add_paragraph("References", "JOSReferenceHeading")
for idx, ref in enumerate(ms.references, 1):
    builder.add_paragraph(f"[{idx}] {ref}", "JOSReference")
builder.add_paragraph("附中文参考文献:", "JOSReferenceHeading")
for ref in ms.cn_references:
    builder.add_paragraph(ref, "JOSReference")
builder.add_paragraph("作者简介", "JOSReferenceHeading")
for bio in ms.author_bio:
    builder.add_paragraph(bio, "JOSReference")
```

- 英文文献前自动补 `[1]` / `[2]` / ...（**与正文 \cite 对应的编号一致**）。
- 中文参考**保留原文**的方括号编号（如 `[5]`、`[6]`），不重排。
- 作者简介保留 `\item` 后的纯文本。

## 4.12 块构造时序图

```text
build_manuscript(root)
   │
   ├─ read main-jos.tex
   ├─ read 00_abstract.tex, parse_newcommands
   ├─ parse_bbl → cite_map, references
   ├─ read 7 sections, strip_comments
   ├─ collect_labels → label_map
   ├─ extract command args (rjtitle, rjauthor, rjinfor, rjcategory, rjhead, footnotetext)
   ├─ extract_english_front_matter → title_en, authors_en, institute_en
   ├─ expand_inputs → expanded main
   ├─ extract_cn_references, extract_author_bio (from expanded)
   └─ parse_sections → blocks
                                                │
   populate(builder, ms) ──────────────────────┘
   │
   ├─ front matter (title, authors, institute, abstract, keywords, category, citation, english)
   ├─ for block in blocks: emit per kind
   ├─ 诚信声明
   └─ 参考文献 (References / 附中文 / 作者简介)
                                                │
   write_docx(builder, output, ms) ─────────────┘
   │
   ├─ header0 = first_header_xml(profile.first_header_rows)  ← 期刊三行题头
   ├─ header1 = header_xml(running_header)                    ← 奇数页页眉
   ├─ header2 = header_xml(profile.even_header_text)          ← 偶数页页眉
   ├─ footer1 = footer_xml(first_footer)                       ← 首页页脚（带横线）
   ├─ footer2 = footer_xml()                                   ← 奇数页页脚（空）
   └─ footer3 = footer_xml()                                   ← 偶数页页脚（空）
```

## 4.13 一个完整的 Block→OOXML 映射表

| `Block.kind` | 输出段落数 | 段落样式 | 段落对齐 | 段落属性 | 是否含 run 子结构 |
|--------------|----------|---------|---------|---------|------------------|
| heading (L=1) | 1 | JOSHeading1 | 缺省 | before=160, after=160 | 单 run + 普通文本 |
| heading (L=2) | 1 | JOSHeading2 | 缺省 | before=25, after=25 | 同上 |
| heading (L=3) | 1 | JOSHeading3 | 缺省 | before=20, after=20 | 同上 |
| paragraph | 1 | JOSBody | jc=both | firstLine=420, line=260 | 文本 + 上标 [n] / 下标 x_y |
| list_item | 1 | JOSBody | jc=both | firstLine=420, line=260 | 文本（同上） |
| table | 1 标题 + 1 表 | JOSCaption + tbl | center | keep_next + keep_lines | 标题 + `<w:tbl>` |
| figure | 1 图片 + 1 标题 | JOSImage + JOSCaption | center | keep_next + keep_lines | 标题 + `<w:drawing>` |
| algorithm | 1 标题 + N 行 | JOSCaption + JOSCode | center / left | 标题 keep_next，代码 keep_next+keep_lines | 文本（可能含 [n] 上标） |
| equation | 1 | JOSCode | center | (无) | 文本 + 空格 + (N) |

## 4.14 缺数据/异常处理

| 情况 | 行为 | Rust 端建议 |
|------|------|------------|
| 标题为空 | 跳过 add_paragraph | `if text.is_empty() { return; }` |
| 机构多行为空 | 跳过 | 同上 |
| 关键词为空 | 仍然写出"关键词:"标签 | 不变 |
| 章节流为空 | front matter + 诚信声明 + 参考文献 | 不变 |
| figure 缺图 | 写"[缺图] <caption>" 段 | 同上 |
| 没有 cn_references | "附中文参考文献:"标题仍写，但下面 0 条 | 不变 |
| 参考文献 0 条 | "References" 标题仍写 | 不变 |
