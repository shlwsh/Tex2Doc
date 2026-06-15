# 08 · 一致性校验（verify_jos_docx.py）

> `verify_jos_docx.py` 是产线的最后一道卡口。它从生成的 docx 抽取 30+ 项结构指标，与 **PDF 源文** + **格式定义 JSON** 双重比对。
> 本章复刻所有校验项；Rust 重构时**强烈建议把这些校验内嵌到 `verify` 子命令**，并在 CI 中作为"投稿前最后关卡"。

## 8.1 校验脚本的输入

```bash
python3 scripts/verify_jos_docx.py \
    --docx docs/to-docx/v64-论文稿件-jos-20260613-174000.docx \
    --pdf  latex/main-jos.pdf \
    --format docs/format/jos_2025_docx_format_definitions.json \
    --report docs/to-docx/v64-论文稿件-jos-20260613-174000-docx校验报告.md \
    --json-report docs/to-docx/v64-论文稿件-jos-20260613-174000-docx校验报告.json \
    --allowed-footer 1260
```

| 参数 | 含义 |
|------|------|
| `--docx` | 必填：被校验的 docx |
| `--pdf` | 必填：原始 PDF（用于文本覆盖对比） |
| `--format` | 必填：JOS 格式定义 JSON |
| `--report` | 必填：Markdown 报告输出 |
| `--json-report` | 可选：JSON 报告输出（默认同 report 主名加 .json） |
| `--allowed-footer` | 默认 1260 twips：页脚距离容差 |

## 8.2 校验的 5 个数据源

1. **docx 文档本体** `word/document.xml`：表格数、图片数、段落样式、引用上标数等。
2. **docx 样式表** `word/styles.xml`：参考文献缩进。
3. **docx 页眉** `word/header1.xml` + `word/header2.xml`：页眉文本、页码字段数。
4. **docx 首页页眉** `word/header0.xml`：期刊三行制表位。
5. **PDF 源文**（pdftotext 抽取）：字符数、关键标记覆盖、页眉文本。

## 8.3 全部校验项（共 30+ 项）

下表与 `make_report` 中打印的检查项**一一对应**。表中"状态"字段对应 MD 报告里的"通过/失败"。

| # | 名称 | 期望 | 提取方式 | 失败影响 |
|---|------|------|---------|---------|
| 1 | 表格对象数 | `>= 5` | `count_tables` = `len(root.findall(".//w:tbl", NS))` | 大问题 |
| 2 | 图片数 | `= 8` | `count_images` = `len(root.findall(".//wp:inline", NS))` | 大问题 |
| 3 | 图题与图片一一对应 | `8/8` | `figure_records` 中每张图后续 3 段内有"图N"开头的 caption | 严重 |
| 4 | 图片段落非固定行距样式 | `8/8` | 所有图片段 `pStyle = "JOSImage"` | 严重 |
| 5 | 编号表题数 | `= 6` | `table_caption_records`：`pStyle="JOSCaption"` 且 `text` 以"表"开头 | 严重 |
| 6 | 表格中间竖线 | `n/n` | 每个 `<w:tbl>` 的 `<w:tblBorders><w:insideV w:val="single"/>` | 严重 |
| 7 | 表格左右开口 | `n/n` | `<w:left w:val="nil"/>` 且 `<w:right w:val="nil"/>` | 严重 |
| 8 | 表格单元格直接字体 | `n/n` | `table_font_stats` 检查每个 run 都带 `rFonts + sz=15` | 中 |
| 9 | 表头加粗正文常规 | 全部符合 | `header_bold == header_runs` 且 `body_not_bold == body_runs` | 中 |
| 10 | 页眉页码字段 | `= 2` | `header_page_field_count`：header1+header2 中 `b" PAGE "` 出现次数 | 严重 |
| 11 | 页眉内容 | 匹配 2 个 marker | `header1` 含"石洪雷 等: ..."；`header2` 含"Journal of Software 软件学报" | 严重 |
| 12 | 页眉与 main-jos.pdf 对照 | DOCX 2/2; PDF 2/2 | marker 在 docx headers 与 pdf text 中各出现 | 严重 |
| 13 | 页眉标题页码左右分开 | 制表位 2/2; 紧挤分隔符 0 | `tabbed_headers` = header1+header2 中同时含 `<w:tab>` 设置与 `<w:r><w:tab/></w:r>` | 中 |
| 14 | 首页期刊信息右对齐制表位 | `= 3` | `masthead_tab_count` = header0 前 3 段制表位 | 中 |
| 15 | 表格行禁止跨页拆分 | `n/n` | `table_rows_cant_split` = 每行有 `<w:cantSplit/>` | 中 |
| 16 | 表题与表格同页 | `n/n` | `table_caption_keep` = 每个表题段有 `<w:keepNext/>` | 中 |
| 17 | 图片与图题同页 | `n/n` | `image_keep` = 每个图片段有 `<w:keepNext/>` | 中 |
| 18 | 算法清单同页保持 | 全部符合 | caption `keep_next` + 代码 `keep_lines` + 代码 `keep_next` | 中 |
| 19 | 参考文献悬挂缩进 | `left=420, hanging=420` | 从 styles.xml 读 `JOSReference.pPr/w:ind` | 严重 |
| 20 | LaTeX 环境参数泄漏 | 无 | 文本中无 `leftmargin`、`labelwidth`、`itemindent`、`nosep`、`indent=-`、`sep=4pt`、`=0pt` | 大问题 |
| 21 | 正文数字引用上标 | `n/n` | `body_citation_superscript_stats`：所有 `[N]` run 都有 `<w:vertAlign w:val="superscript"/>` | 严重 |
| 22 | Word 上标 run 数 | `> 0` | 全 docx `<w:vertAlign w:val="superscript"/>` 计数 | 严重 |
| 23 | 上标源码残留 | 无 | `re.findall(r"\^[A-Za-z0-9*]+", dtext)` 为空 | 大问题 |
| 24 | 公式段落识别 | `>= 1` | 段落 `pStyle="JOSCode"` 且含 `rand(` 与 `(1)` | 中 |
| 25 | 公式上标 run | `> 0` | 公式段落中 `<w:vertAlign w:val="superscript"/>` 数 | 中 |
| 26 | 公式下标 run | `> 0` | 公式段落中 `<w:vertAlign w:val="subscript"/>` 数 | 中 |
| 27 | 公式 LaTeX 残留 | 无 | 公式段文本不含 `\bbigl|\bbigr` 或 `\^` `_` | 中 |
| 28 | 参考文献条目数 | `>= 51` | `reference_count` = `re.findall(r"^\[\d+\]", dtext, MULTILINE)` 数 | 中 |
| 29 | DOCX/PDF 字符比例 | `>= 0.75` | `len(docx_text) / len(pdf_text)` | 中 |
| 30 | 页面尺寸 | `{w:10433, h:14742}` | `pgSz.attrib` 与 format.json 比较 | 严重 |
| 31 | 页边距 | 全部匹配（除 footer） | `pgMar.attrib` 与 format.json 比较 | 严重 |
| 32 | 分栏 | `{space:720, num:1}` | `cols.attrib` 与 format.json 比较 | 严重 |
| 33 | 关键标记覆盖 | `n/n` | 22 个字符串 marker（标题/章节/图/表/算法等）必须都在 docx 文本中 | 严重 |

## 8.4 关键标记清单

校验脚本硬编码了 22 个 marker（`coverage(markers, docx_norm, pdf_norm)`）：

| 类型 | marker |
|------|--------|
| 标题 | `网关流量驱动的微服务定向日志采集框架` |
| 摘要标签 | `摘  要`, `关键词` |
| 英文摘要标签 | `Abstract`, `Key words` |
| 章节 | `1 引言`, `2 相关工作`, `3 系统总体设计`, `4 关键算法`, `5 系统实现`, `6 实验与分析`, `7 结束语` |
| 表 | `表 1`, `表 5` |
| 图 | `图 1`, `图 8` |
| 算法 | `算法 1` |
| 参考文献 | `References`, `附中文参考文献`, `作者简介` |
| 作者邮箱 | `shihonglei0042@link.tyut.edu.cn`, `zh_juanjuan@126.com` |

每个 marker 既要在 docx 文本中出现，也要在 pdf 文本中出现。`normalize` 把两边都归一（小写化、空格移除、标点统一）后做包含判断。

## 8.5 报告 MD 的内容结构

`make_report(result)` 输出的 markdown：

```text
# DOCX 与 PDF 一致性校验报告

- DOCX: `...`
- PDF:  `...`
- 结论: 通过 / 未通过

## 结构计数

| 项 | 值 | 要求 | 状态 |
|---|---:|---:|---|

## 关键内容覆盖

| 标记 | DOCX | PDF |
|---|---|---|

## 页面 XML

- 纸张: `{'w': '10433', 'h': '14742'}`
- 页边距: `{'top': '567', ...}`
- 分栏: `{'space': '720', 'num': '1'}`

## 文本量

- DOCX 提取文本字符数: 12345
- PDF 提取文本字符数: 15678
- DOCX/PDF 字符比例: 0.788

## 图像插入

| # | 段落 | 图片 | 尺寸 cm | 段落样式 | 图题 |
|---:|---:|---|---:|---|---|

## 表格与边框

编号表题：
- 表 1  ...
- ...

| 表格对象 | 上边 | 下边 | 左边 | 右边 | 横向内线 | 纵向内线 |
|---:|---|---|---|---|---|---|

## 公式输出

| 段落 | 文本 | 上标 run | 下标 run | LaTeX 残留 |
|---:|---|---:|---:|---|
```

## 8.6 报告 JSON 的内容结构

```jsonc
{
  "docx": "...",
  "pdf": "...",
  "passed": true,
  "checks": [
    { "name": "表格对象数", "actual": 6, "expected": ">=5", "ok": true, "status": "通过" },
    ...
  ],
  "marker_coverage": [
    { "marker": "网关流量...", "in_docx": true, "in_pdf": true },
    ...
  ],
  "page_setup": {
    "paper_twips": { "w": "10433", "h": "14742" },
    "margins_twips": { ... },
    "columns": { ... }
  },
  "figures": [
    { "index": 1, "paragraph": 42, "style": "JOSImage", "name": "fig1_system_overview", "cx_cm": 16.0, "cy_cm": 9.0, "caption": "图 1  系统总体架构" },
    ...
  ],
  "table_captions": ["表 1  ...", ...],
  "table_borders": [ { "index": 1, "top": "single", "bottom": "single", "left": "nil", "right": "nil", "insideH": "single", "insideV": "single" }, ... ],
  "formulas": [ { "paragraph": 65, "text": "|d_n=min(d_max,...)", "superscripts": 0, "subscripts": 2, "has_latex_residue": false }, ... ],
  "docx_chars": 12345,
  "pdf_chars": 15678,
  "char_ratio": 0.788,
  "paragraphs": 142
}
```

## 8.7 normalize 函数

```python
def normalize(text: str) -> str:
    text = text.replace("\u00a0", " ")        # nbsp
    text = re.sub(r"\s+", "", text)           # 全部空白
    text = text.replace("–", "-").replace("—", "-")  # en/em dash
    text = text.replace("“", '"').replace("”", '"')  # 中文左右引号
    return text
```

**注意**：normalize 把所有空白都删了——所以"图 1"和"图1"在比较时等价。这能容忍 docx 与 pdf 之间的细微换行差异。

## 8.8 PDF 文本提取

```python
def pdf_text(pdf: Path) -> str:
    result = subprocess.run(["pdftotext", str(pdf), "-"], check=True, text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return result.stdout
```

Rust 端用 `lopdf::Document::load(pdf)?.extract_text(&[])`。两者输出格式可能略不同（pdftotext 会按布局拆行、lopdf 也会），需要反复调整 normalize。

## 8.9 失败处理策略

| 失败项 | 是否阻断 | 备注 |
|--------|---------|------|
| LaTeX 残留 | 是 | 直接拒绝——意味着 IR 转换有 bug |
| 字符比例 < 0.75 | 是 | 内容缺失/丢失 |
| 关键 marker 缺失 | 是 | 内容缺失/丢失 |
| 页面尺寸不匹配 | 是 | 不再是 JOS 2025 模板 |
| 表格无 insideV | 是 | 表格结构异常 |
| 参考文献悬挂缩进不对 | 是 | 样式未应用 |
| 章节计数 < 8 个 | 是 | 内容缺失 |
| 上标残留 ^A-Za-z0-9 | 是 | 转换有 bug |
| 页眉缺页码字段 | 是 | 页眉装配失败 |
| 算法清单 keepLines 缺失 | 否 | 仅提示排版可能差 |

Rust 重构时**沿用相同的 0/1 判定**——任一 `ok == false` 即整体 fail，exit code 1。

## 8.10 在 CI 中的使用

```yaml
# .github/workflows/docx-check.yml （伪代码）
- name: Build DOCX
  run: bash scripts/build_docx.sh
- name: Verify DOCX
  run: |
    python3 scripts/verify_jos_docx.py \
      --docx docs/to-docx/v*-论文稿件-jos-*.docx \
      --pdf  latex/main-jos.pdf \
      --format docs/format/jos_2025_docx_format_definitions.json \
      --report docs/to-docx/ci-docx-校验.md \
      --json-report docs/to-docx/ci-docx-校验.json
- name: Upload report
  if: always()
  uses: actions/upload-artifact@v4
  with:
    name: docx-校验报告
    path: docs/to-docx/ci-docx-校验.*
```

## 8.11 Rust 端的最小校验实现

如果不想全量复刻，可以先实现以下"硬卡点"（其他留给人工 review）：

1. 页面尺寸/边距/分栏与 format JSON 完全匹配。
2. 8 张图片 + 8 段图题 + 6 个表题。
3. 22 个关键 marker 全部覆盖。
4. LaTeX 残留 = 0。
5. 字符比例 ≥ 0.75。
6. exit 1 当任一失败。

完整 33 项实现用 `quick-xml` + `lopdf` 大约 500 行 Rust 代码，参考 `verify_jos_docx.py` 的函数一一对应实现即可。
