# `docs/to-docx` — p3-microservice TeX→DOCX 转换技术文档

> 目标受众：希望**用 Rust 重写** `scripts/build_docx.sh` 这条管线的开发者。
> 阅读完这一系列文档后，你应该能不参考现有 Python 源码、仅依赖本文档与《软件学报》2025 样例的格式定义 JSON，独立实现一个能生成版心、字体、版式与样本完全一致的 `.docx` 文件的程序。

## 1. 这是什么

`scripts/build_docx.sh` 是 `p3-microservice` 项目的投稿前自动化步骤：把作者用 LaTeX 写好的稿件（`latex/main-jos.tex` + 7 个章节文件 + 摘要/关键词共享宏 + `references.bib` 编译出的 `main-jos.bbl`）一键转成符合《软件学报》2025 年版式规范的 Word 文档。整套管线**不依赖 Pandoc / LibreOffice / Microsoft Word**，而是直接用 Python（`zipfile` + 手写 `WordprocessingML`）组装 OOXML 包。

输出物每次运行都会按版本号增量落到 `docs/to-docx/`，命名规则：

```text
v{N}-论文稿件-jos-{YYYYMMDD-HHMMSS}.docx
v{N}-论文稿件-jos-{YYYYMMDD-HHMMSS}-docx校验报告.md
v{N}-论文稿件-jos-{YYYYMMDD-HHMMSS}-docx校验报告.json
```

## 2. 文档清单

| # | 文档 | 内容 |
|---|------|------|
| 01 | [`01-pipeline-overview.md`](01-pipeline-overview.md) | shell 入口、依赖检查、版本号策略、子脚本调用顺序、产物命名 |
| 02 | [`02-tex-parsing.md`](02-tex-parsing.md) | LaTeX 解析：花括号匹配、`\input` 递归展开、`\newcommand` 提取、`.bbl` 解析、`\section`/`\subsection` 遍历 |
| 03 | [`03-syntax-normalization.md`](03-syntax-normalization.md) | `latex_to_text` / `clean_math`：注释剥离、宏展开、行内数学→Unicode、上下标识别、中文标点归一化 |
| 04 | [`04-block-construction.md`](04-block-construction.md) | 中间表示 `Block` 枚举、front matter 抽取、`table` / `figure` / `algorithm` / `equation` 各环境的块构造算法 |
| 05 | [`05-wpml-emission.md`](05-wpml-emission.md) | WordprocessingML 写入：`styles.xml`、段落 run、`<w:tbl>`、`<w:drawing>`、`<w:sectPr>`、`<w:header>`/`<w:footer>` |
| 06 | [`06-zip-relationships.md`](06-zip-relationships.md) | docx 物理包结构、`[Content_Types].xml` / `_rels/.rels` / `word/_rels/document.xml.rels`、媒体文件、图片尺寸计算 |
| 07 | [`07-format-profiles.md`](07-format-profiles.md) | 《软件学报》2025 格式数据：页面尺寸、版心、样式表、首选字、字号、缩进、版式策略 |
| 08 | [`08-verification.md`](08-verification.md) | `verify_jos_docx.py` 30+ 项一致性校验：与 PDF 文本、与格式定义 JSON、LaTeX 残留、悬挂缩进、上下标等 |
| 09 | [`09-rust-port.md`](09-rust-port.md) | **Rust 重构指南**：crate 选型、模块划分、数据结构、关键算法伪代码、与 Python 实现的一一对应 |

## 3. 关键术语

| 术语 | 含义 |
|------|------|
| **WPML** | WordprocessingML，Word 文档的核心 XML 词汇表（命名空间 `http://schemas.openxmlformats.org/wordprocessingml/2006/main`） |
| **OOXML** | Office Open XML，`.docx` 物理上是 ZIP 容器，成员是多个命名空间的 XML |
| **twip** | 1/1440 英寸；Word 内部距离单位（1 cm = 567 twips） |
| **half-point** | 1/144 英寸；`<w:sz w:val="X"/>` 中 X 是半磅数（9 pt → `w:val="18"`） |
| **EMU** | English Metric Unit，1/360 000 cm；图片 `<wp:extent cx cy>` 使用 EMU |
| **bibitem** | `\bibitem{key}` LaTeX 文献条目；`main-jos.bbl` 是 `bibtex` 的格式化结果 |
| **Twp** | `{"w": ..., "h": ...}` 纸张尺寸 dict，键名对应 `<w:pgSz>` 属性 |
| **rId** | relationship Id；docx 包内资源的相对引用 |
| **first/even/default** | Word 中首页/偶数页/默认三种页眉页脚类型，由 `<w:titlePg/>` 切换 |

## 4. 一次运行的完整数据流

```text
                          ┌────────────────────────────────┐
                          │ latex/main-jos.tex (源文)        │
                          │ latex/sections/zh/*.tex (7 章)  │
                          │ latex/main-jos.bbl  (BibTeX 产物)│
                          │ figures/*.png  (8 张图)          │
                          └────────────┬───────────────────┘
                                       │
                          ┌────────────▼───────────────────┐
                          │ scripts/build_docx.sh            │
                          │   1. check_deps()                │
                          │   2. 版本号 vN+1, 时间戳 TS       │
                          │   3. python3 build_jos_docx.py   │
                          │   4. python3 verify_jos_docx.py  │
                          └────────────┬───────────────────┘
                                       │
              ┌────────────────────────┼─────────────────────────┐
              ▼                        ▼                         ▼
   build_jos_docx.py 解析       写出 OOXML ZIP            verify_jos_docx.py 30+ 项校验
   ┌──────────────────────┐  ┌──────────────────────┐  ┌──────────────────────┐
   │ latex → Manuscript    │  │ [Content_Types].xml  │  │ DOCX ↔ PDF 文本覆盖    │
   │   - 标题/作者/机构    │  │ _rels/.rels          │  │ 上下标 run 计数        │
   │   - 摘要/关键词      │  │ word/document.xml    │  │ 悬挂缩进/表格边框       │
   │   - 7 章正文块       │  │ word/styles.xml      │  │ 公式残留/图片尺寸       │
   │   - 表格/图/算法     │  │ word/settings.xml    │  │ LaTeX 参数泄漏        │
   │   - 公式/参考文献    │  │ word/header[012].xml │  │ 页眉页码/PAGE 字段     │
   │   - 中文参考/作者简介 │  │ word/footer[123].xml │  │ → MD + JSON 报告     │
   └──────────┬───────────┘  │ word/media/*.png     │  └──────────────────────┘
              │              └──────────────────────┘
              ▼
       docs/to-docx/v{N}-论文稿件-jos-{TS}.docx
       docs/to-docx/v{N}-论文稿件-jos-{TS}-docx校验报告.{md,json}
```

## 5. 复刻工作流（不读 Python 源码的前提下）

按下面顺序读 9 篇文档，并用 `09-rust-port.md` 提供的脚手架直接开工：

1. **01** 了解 shell 入口做了什么（依赖、版本、产物路径）。
2. **02 / 03 / 04** 理解 LaTeX→中间表示（IR）的全过程——这是整个工具的核心。
3. **05 / 06** 理解 IR→OOXML ZIP 包的写入规则。
4. **07** 知道要生成什么样的样式——所有数值都来自 `jos_2025_docx_format_definitions.json`。
5. **08** 知道生成完要做哪些校验，**强烈建议在 Rust 实现里直接复刻这些校验**作为 CI 卡点。
6. **09** 落地为 Rust crate：模块边界、数据结构、关键算法伪代码。

## 6. 与旧 Pandoc 流水线的关系

`scripts/build_jos_docx.py` 顶部的 docstring 明确说："旧流水线让 Pandoc 去推断高度自定义的 rjthesis 文档，结果丢掉了 front matter 和若干自定义环境。本生成器保持转换的确定性。" 也就是说，**新管线完全替换了 Pandoc**。`scripts/build_jos_reference_docx.py` 仍然存在，但它只用来**生成** Pandoc reference template（属于历史工件），并不参与 `build_docx.sh` 的执行。

所以 Rust 重构时**不需要考虑 Pandoc**，直接走"解析 LaTeX → 写 OOXML"这条路即可。
