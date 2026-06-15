# 07 · 《软件学报》2025 格式定义（jos_2025）

> 本章是 Rust 重构时**直接抄录**样式数值的单一来源。所有样式 ID、字体、字号、缩进、行距均来自 `docs/format/jos_2025_docx_format_definitions.json`，与 `build_jos_docx.py` 中的 `style()` 调用**逐字段对应**。
>
> 如果将来 JOS 改版，只需更新这个 JSON + 这章的样式表——其他章节的代码逻辑不需要改。

## 7.1 页面设置（page_setup）

```jsonc
"page_setup": {
  "paper_twips": { "w": "10433", "h": "14742" },  // 18.4cm × 26.0cm
  "paper_cm":    { "width": 18.4, "height": 26.0 },
  "margins_twips": {
    "top":    "567",    // 1.00 cm
    "right":  "822",    // 1.45 cm
    "bottom": "1247",   // 2.199 cm
    "left":   "822",    // 1.45 cm
    "header": "737",    // 1.30 cm
    "footer": "567",    // 1.00 cm
    "gutter": "0"
  },
  "margins_cm": { "top": 1.0, "right": 1.45, "bottom": 2.199, "left": 1.45,
                  "header": 1.3, "footer": 1.0, "gutter": 0.0 },
  "columns":   { "space": "720", "num": "1" },
  "title_page_enabled": true
}
```

**版心宽度** = `paper.w - margin.left - margin.right = 10433 - 822 - 822 = 8789 twips ≈ 15.5 cm`。

> `build_jos_docx.py` 的 `DocxBuilder` 把"footer 距离"硬覆盖为 1260 twips（≈ 2.22 cm），比样例的 567 大。原因：JOS 2025 实际投稿时审核要求 2.0 cm 以上的下边距。`verify_jos_docx.py` 通过 `--allowed-footer 1260` 容忍这个差异。

## 7.2 字体约定

| 字段 | 拉丁（ascii） | 东亚（eastAsia） | cs | 备注 |
|------|--------------|------------------|------|------|
| 标题/正文 | Times New Roman | 宋体 | Times New Roman | 全文统一 |
| 中文标题 | Times New Roman | 黑体 | Times New Roman | 加粗 |
| 作者 | Times New Roman | 仿宋_GB2312 | Times New Roman | |
| 单位 | Times New Roman | 宋体 | Times New Roman | |
| 摘要（中） | Times New Roman | 楷体_GB2312 | Times New Roman | |
| 英文标题 | Times New Roman | 黑体 | Times New Roman | 加粗 |
| 章节标题 | Times New Roman | 黑体 | Times New Roman | 加粗 |
| 图表题 | Times New Roman | 宋体 | Times New Roman | |
| 表格内文 | Times New Roman | 宋体 | Times New Roman | direct_run_xml |
| 算法/代码 | Courier New | 宋体 | Times New Roman | 改用等宽 |
| 参考文献 | Times New Roman | 宋体 | Times New Roman | 悬挂缩进 |

> ⚠️ Fandol 字体集（pdfLaTeX 用的）不包含"楷体_GB2312"——LaTeX 编译时 `\providecommand{\fangsong}{\kaishu}` 走的是替代。本项目的 DOCX 输出**真实写出"楷体_GB2312"等 Windows 字库名**——只有用户的 Word 安装了这些字库（或 LibreOffice 的 Noto CJK 替代映射）才显示正确。Word 默认能识别"楷体/宋体/黑体/仿宋_GB2312"。

## 7.3 文本尺寸约定（half-points → pt）

| 用途 | 字号 pt | 半数 w:sz |
|------|---------|-----------|
| 一级标题 | 10.5 | 21 |
| 二级标题 | 9 | 18 |
| 三级标题 | 9 | 18 |
| 英文大标题 | 12 | 24 |
| 中文大标题 | 14 | 28 |
| 中文作者 | 12 | 24 |
| 摘要（中/英） | 9/10 | 18/20 |
| 关键词 | 9 | 18 |
| 引用格式 | 9 | 18 |
| 正文 | 9 | 18 |
| 图表题 | 9 | 18 |
| 表格内文 | 7.5 | 15 |
| 算法/代码 | 8 | 16 |
| 参考文献 | 7.5 | 15 |
| 页眉/页脚 | 7.5 | 15 |

## 7.4 行距约定（twips）

JOS 模板的"行距 260"是 13 pt（`260 / 20 = 13`），用于正文。算法/代码用 220（11 pt），表格用 220，标题样式没有强制行距。

| 用途 | w:line | w:lineRule | 实际 |
|------|--------|-----------|------|
| 正文 | 260 | exact | 13pt |
| 摘要/关键词 | 240 | exact | 12pt |
| 引用格式 | 220 | exact | 11pt |
| 标题 | (无) | (无) | 默认 |
| 图表题 | (无) | (无) | 默认 |
| 表格内文 | 220 | exact | 11pt |
| 算法/代码 | 220 | exact | 11pt |
| 参考文献 | 260 | exact | 13pt |
| 页眉 | 180 | exact | 9pt |
| 页脚 | 180 | exact | 9pt |

## 7.5 样式表

下表与 `styles_xml()` 中的 21 个 `style(...)` 调用**一一对应**。所有 `w:before` / `w:after` 都是 twips（1/20 pt）。

| 序号 | styleId | 名称 | pt | 东亚字 | ascii | bold | jc | firstLine | left/hanging | before | after | line |
|------|---------|------|-----|--------|-------|------|-----|-----------|--------------|--------|-------|------|
| 1 | Normal | Normal | 9 | 宋体 | Times New Roman | - | both | - | - | - | - | 260 |
| 2 | JOSMasthead | JOS masthead from sample body style 4 | 7.5 | 宋体 | Times New Roman | - | - | - | - | - | - | 180 |
| 3 | JOSTitleZh | JOS Chinese title from sample style 64 | 14 | 黑体 | Times New Roman | ✓ | center | - | - | 0 | 120 | - |
| 4 | JOSAuthorZh | JOS Chinese author from sample style 65 | 12 | 仿宋_GB2312 | Times New Roman | - | center | - | - | 120 | 120 | - |
| 5 | JOSInstituteZh | JOS institute from sample style 66 | 8 | 宋体 | Times New Roman | - | center | - | - | - | - | 220 |
| 6 | JOSAbstractZh | JOS abstract from sample style 117 | 9 | 楷体_GB2312 | Times New Roman | - | both | - | - | - | - | 240 |
| 7 | JOSAbstractEn | JOS English abstract from sample first page | 10 | 宋体 | Times New Roman | - | left | - | - | - | - | 240 |
| 8 | JOSKeywords | JOS keywords from sample style 118 | 9 | 宋体 | Times New Roman | - | - | - | left=430 / hanging=430 | - | - | 240 |
| 9 | JOSCitation | JOS citation from sample style 121 | 9 | 宋体 | Times New Roman | - | both | - | - | - | - | 220 |
| 10 | JOSEnglishTitle | JOS English title from sample style 120 | 12 | 黑体 | Times New Roman | ✓ | - | - | - | 120 | 100 | - |
| 11 | JOSBody | JOS body from sample style 145 | 9 | 宋体 | Times New Roman | - | both | firstLine=420 | - | - | - | 260 |
| 12 | JOSBodyNoIndent | JOS body without first-line indent | 9 | 宋体 | Times New Roman | - | both | - | - | - | - | 260 |
| 13 | JOSHeading1 | JOS heading 1 from sample style 213 | 10.5 | 黑体 | Times New Roman | ✓ | - | - | - | 160 | 160 | - |
| 14 | JOSHeading2 | JOS heading 2 from sample style 215 | 9 | 黑体 | Times New Roman | ✓ | - | - | - | 25 | 25 | - |
| 15 | JOSHeading3 | JOS heading 3 from sample style 217 | 9 | 黑体 | Times New Roman | ✓ | - | - | - | 20 | 20 | - |
| 16 | JOSCaption | JOS caption from sample figure/table captions | 9 | 宋体 | Times New Roman | - | center | - | - | - | 120 | - |
| 17 | JOSImage | JOS image paragraph with automatic line height | 9 | 宋体 | Times New Roman | - | center | - | - | 80 | 80 | - |
| 18 | JOSTableText | JOS table text | 7.5 | 宋体 | Times New Roman | - | center | - | - | - | - | 220 |
| 19 | JOSCode | JOS algorithm/code text | 8 | 宋体 | Courier New | - | - | - | - | - | - | 220 |
| 20 | JOSReferenceHeading | JOS reference heading from sample style 126 | 9 | 黑体 | Times New Roman | ✓ | - | - | - | 280 | - | - |
| 21 | JOSReference | JOS reference text from sample style 129 | 7.5 | 宋体 | Times New Roman | - | both | - | left=420 / hanging=420 | - | - | 260 |

> JOSBody 的 `firstLine=420` ≈ 21 pt ≈ 2 个汉字宽（9pt 字号下 1 个汉字约 9pt 宽）。
> 同样的 firstLine 在 JOSInstituteZh 中用 left=70 + hanging=70 表示"首行额外缩进 70 twips 再悬挂 70"——但 Python 端**只**给 JOSKeywords / JOSReference 用 left+hanging，其他都依赖段首两空格手动加。
>
> 由于本项目输出时直接给中文段落加"  "（两个半角空格）首行缩进，**建议 Rust 端也照搬这个 trick**——保持视觉一致。

## 7.6 段前/段后间距

| 样式 | before | after | 备注 |
|------|--------|-------|------|
| JOSTitleZh | 0 | 120 | 标题紧贴作者 |
| JOSAuthorZh | 120 | 120 | 作者与单位间留 6pt |
| JOSEnglishTitle | 120 | 100 | 英文标题前留 6pt |
| JOSHeading1 | 160 | 160 | 一级标题前后各 8pt |
| JOSHeading2 | 25 | 25 | 二级标题前后各 1.25pt（接近 0） |
| JOSHeading3 | 20 | 20 | 三级标题前后各 1pt |
| JOSCaption | (无) | 120 | 题注下方 6pt |
| JOSImage | 80 | 80 | 图片上下 4pt |
| JOSReferenceHeading | 280 | (无) | "References" 标题前 14pt |
| JOSKeywords | (无) | (无) | 紧贴 abstract 段 |

## 7.7 缩进约定

| 样式 | w:firstLine | w:left | w:hanging | 实际效果 |
|------|-------------|--------|-----------|---------|
| JOSBody | 420 | - | - | 首行缩进 2 字符（21pt） |
| JOSKeywords | - | 430 | 430 | 悬挂缩进 21.5pt，缩进到 21.5pt 处 |
| JOSReference | - | 420 | 420 | 同上 |
| JOSInstituteZh | - | - | - | 由段落直接前导两空格模拟 |

> Rust 端如果要更"地道"，可以给 JOSInstituteZh 也加 left=70 + hanging=70——但**保持和 Python 一致**是最稳的，因为 verify 脚本不会检查这段。

## 7.8 页眉页脚内容

`JOS_PROFILE`（在 `build_jos_docx.py` 中硬编码）：

```python
JOS_PROFILE = DocxProfile(
    first_header_rows=(
        ("软件学报 ISSN 1000-9825, CODEN RUXUEW", "E-mail: jos@iscas.ac.cn"),
        ("Journal of Software, [doi: 10.13328/j.cnki.jos.000000]", "http://www.jos.org.cn"),
        ("© 中国科学院软件研究所版权所有.", "Tel: +86-10-62562563"),
    ),
    even_header_text="Journal of Software 软件学报",
    first_footer_text="收稿时间: XXXX-XX-XX; 修改时间: XXXX-XX-XX; 采用时间: XXXX-XX-XX",
    first_footer_indent_twips=330,
    footer_distance_twips=1260,
    after_institute_twips=300,
    before_citation_twips=300,
    before_english_title_twips=220,
    before_english_abstract_twips=340,
    citation_wrap_units=52.0,
    zh_abstract_label="摘   要:",
    zh_keywords_label="关键词:",
    category_label="中图法分类号:",
    en_abstract_label="Abstract:",
    en_keywords_label="Key words:",
)
```

Rust 端**应当**把它做成一个独立的 `JOS_PROFILE: DocxProfile = ...` 常量；不放在 JSON 里是因为这些值经常微调、不像"页面尺寸"那样可机器化。

## 7.9 引用格式自动换行宽度 52 units 的换算

```python
def token_width_units(token: str) -> float:
    total = 0.0
    for ch in token:
        if ch.isspace():          total += 0.35
        elif 0x4E00 <= ord(ch) <= 0x9FFF: total += 1.00  # CJK
        elif ch.isupper():        total += 0.62
        elif ch.islower() or ch.isdigit(): total += 0.52
        elif ch in "-/.":         total += 0.28
        else:                     total += 0.35
```

**经验值**，不是数学换算。版心宽 15.5 cm，9pt 字号下约 78 个汉字，**中文宽度** = 78；**英文** 26pt/字 × N ≈ 130 字符，**英文宽度** ≈ 80。52 是按"32 个汉字 + 半行英文混排"经验值。

Rust 端实现就是按字符 switch 累加，没有更复杂的逻辑。

## 7.10 JOS 2025 格式定义 JSON 的完整 schema

```text
{
  "summary": {
    "source_docx": "docs/latex-models/software-journal/软件学报排版样例2025年版.docx",
    "page_setup": { ... },                        // 见 §7.1
    "header_references": [ { "id": "rId4", "type": "default" }, ... ],
    "footer_references": [ { "id": "rId8", "type": "first" }, ... ],
    "style_count": 233,
    "paragraph_count": 196,
    "nonempty_paragraph_count": 183,
    "paragraph_style_counts": [ ["145", 83], ... ],
    "paragraph_class_counts": [ ["body_or_heading", 162], ... ],
    "headers": [ { "filename": "word/header1.xml", "text": "...", "style_id": "4" }, ... ],
    "footers": [ ... ],
    "first_page_paragraphs": [ ... ],  // 40 条首页段落详情
    "styles": [ { "style_id": "1", "name": "Normal", "type": "paragraph", "jc": "both", "size_pt": 9, "fonts": {...}, "spacing": {...}, "ind": {...} }, ... ],
    "caption_paragraphs": [ { "index": 27, "type": "figure_caption", "style_id": "4", "align": "center", "spacing": {...}, "text": "图1  跨项目..." }, ... ],
    "reference_paragraphs": [ ... ],
    "media_count": 79,
    "ole_object_count": 71
  }
}
```

> JSON 的 233 个样式是**样本文档**自带的；我们生成时只用 21 个。`extract_jos_docx_format.py` 用于一次性把样本 docx 抽成这个 JSON，**不在 build 流水线中**。

## 7.11 与 LaTeX rjthesis.cls 的对应

JOS 2025 LaTeX 模板的 `rjthesis.cls`（`latex/rjthesis.cls`，6232 字节）定义了一系列 `\xxxYH` / `\xxxKW` 等命令。本项目用 docx 重写时**完全不用** rjthesis，而是把 LaTeX 命令作为"提取锚点"在 `build_jos_docx.py` 中硬编码：

| LaTeX 命令 | docx 提取方式 |
|----------|--------------|
| `\rjtitle{...}` | `command_arg(main_tex, "rjtitle")` |
| `\rjauthor{...}` | `command_arg(main_tex, "rjauthor")` |
| `\rjinfor{...}` | `command_arg(main_tex, "rjinfor")`，按 `\\` 拆行 |
| `\rjhead{...}` | `command_arg(main_tex, "rjhead")` |
| `\rjcategory{...}` | `command_arg(main_tex, "rjcategory")` |
| `\footnotetext{...}` | `command_arg(main_tex, "footnotetext")` |
| `\AbstractContentZh` | `parse_newcommands` 提取 |
| `\KeywordsZh` | 同上 |
| `\AbstractContentEn` | 同上 |
| `\KeywordsEn` | 同上 |
| `\section{...}` | `parse_sections` 切块 |
| `\subsection{...}` | 同上 |
| `\subsubsection{...}` | 同上 |
| `\begin{table}...\end{table}` | `parse_table` |
| `\begin{figure}...\end{figure}` | `parse_figure` |
| `\begin{algorithm}...\end{algorithm}` | `parse_algorithm` |
| `\begin{equation}...\end{equation}` | `parse_equation` |
| `\begin{enumerate}...\end{enumerate}` | `parse_enumerate` |
| `\begin{description}...\end{description}` | `extract_cn_references` |
| `\begin{list}...\end{list}` | `extract_author_bio` |
| `\cite{...}` | `parse_bbl` 给出编号后替换 |
| `\ref{...}` | `collect_labels` 替换 |

> 如果以后 JOS 改了 rjthesis.cls 的命令名，**只需要更新这一张表**——docx 端不需要重新设计。

## 7.12 Rust 端的 `DocxProfile` 结构

```rust
pub struct DocxProfile {
    pub first_header_rows: Vec<(String, String)>,
    pub even_header_text: String,
    pub first_footer_text: String,
    pub first_footer_indent_twips: u32,
    pub footer_distance_twips: u32,
    pub after_institute_twips: u32,
    pub before_citation_twips: u32,
    pub before_english_title_twips: u32,
    pub before_english_abstract_twips: u32,
    pub citation_wrap_units: f32,
    pub zh_abstract_label: String,
    pub zh_keywords_label: String,
    pub category_label: String,
    pub en_abstract_label: String,
    pub en_keywords_label: String,
}

pub const JOS_PROFILE: DocxProfile = DocxProfile {
    first_header_rows: vec![
        ("软件学报 ISSN 1000-9825, CODEN RUXUEW".into(),
         "E-mail: jos@iscas.ac.cn".into()),
        ("Journal of Software, [doi: 10.13328/j.cnki.jos.000000]".into(),
         "http://www.jos.org.cn".into()),
        ("© 中国科学院软件研究所版权所有.".into(),
         "Tel: +86-10-62562563".into()),
    ],
    even_header_text: "Journal of Software 软件学报".into(),
    first_footer_text: "收稿时间: XXXX-XX-XX; 修改时间: XXXX-XX-XX; 采用时间: XXXX-XX-XX".into(),
    first_footer_indent_twips: 330,
    footer_distance_twips: 1260,
    after_institute_twips: 300,
    before_citation_twips: 300,
    before_english_title_twips: 220,
    before_english_abstract_twips: 340,
    citation_wrap_units: 52.0,
    zh_abstract_label: "摘   要:".into(),
    zh_keywords_label: "关键词:".into(),
    category_label: "中图法分类号:".into(),
    en_abstract_label: "Abstract:".into(),
    en_keywords_label: "Key words:".into(),
};
```
