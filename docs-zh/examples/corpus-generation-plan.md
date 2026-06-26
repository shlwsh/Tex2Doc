# Tex2Doc 转换引擎质量验证 - 30 个基线 Corpus 生成方案

> 日期：2026-06-26
> 输出目录：`docs-zh/examples`
> 目标：为 Tex2Doc 提供"可度量、可解释"的质量基线，覆盖主流期刊和极端排版场景。

---

## 变更历史

| 版本 | 日期 | 变更内容 |
|---|---|---|
| v2 | 2026-06-26 | 全面升级为《30 个 Corpus 详细设计》方案，配套 `corpus-design.md`；更新目录结构、quality_meta schema、Profile 矩阵、实施计划和验收标准 |

---

## 1. 设计目标与原则

根据商业化质量提升方案（`docs-zh/service/document-conversion-engine-quality-commercialization-plan-20260626.md`）P0 阶段要求，这 30 个 Corpus 将作为 Tex2Doc 引擎的**核心回归基线（Smoke & Golden Corpus）**。

每个 Corpus 必须包含：
- `main.tex` 及其依赖资源（图片、`.bib`、`.bbl` 等）。
- 对应的 `quality_meta.json`（描述适用 Profile、测试重点、预期警告和阻断阈值）。
- 标准参照物：Golden DOCX 和 Golden PDF（在 CI 中逐步生成）。

设计原则：
1. **多维度覆盖**：涵盖计算机、物理、数学、生化、人文等不同学科的经典模板。
2. **复杂元素聚焦**：强制纳入极端表格、超长公式、复杂浮动体、嵌套列表和自定义宏。
3. **中文/CJK 优先**：充分验证针对国内期刊（如《软件学报》）的特色版式支持。
4. **边缘降级验证**：包含一定会触发规则失败的场景，以验证降级审计（Fallback Audit）与报告解释功能。

---

## 2. 详细设计文档

> **完整详细设计请参阅 [`corpus-design.md`](./corpus-design.md)**，包含所有 30 个 Corpus 的：
> - 每篇文档的完整 LaTeX 源代码（核心段落）
> - 详细的 `quality_meta.json` Schema 定义
> - 完整的 Profile × Corpus 矩阵
> - 22 个 Quality Marker 映射表
> - 六维质量评分基线
> - 实施路线图（Phase 1-4）

---

## 3. 场景与样例清单（30 个）

### 3.1 计算机科学与工程 (CS & Engineering) — Corpus #1-6

| # | ID | 名称 | Profile | Tier | 核心元素 |
|---|---|---|---|---|---|
| 1 | `corpus-01-ieee-trans` | IEEE Transactions Standard | `ieee-trans` | Golden | `algorithmicx`, `align`, `booktabs`, `figure*`（跨栏） |
| 2 | `corpus-02-cvpr` | CVPR Conference Paper | `cvpr` | Golden | `subfig`, `\subfloat`, `\multirow` in `resizebox`, 15+ 引用 |
| 3 | `corpus-03-acm-sig` | ACM SIG Conference (acmart) | `acm-sig` | Golden | CCS 概念树, `\affiliation`, DOI 引用, `acmart` 元数据 |
| 4 | `corpus-04-jos-chinese` | 软件学报中文期刊 | `jos-paper` | Golden | `rjthesis`, 双语摘要, `\bicaption`, 定理环境, 算法2e, 附中文参考文献, 作者简介 |
| 5 | `corpus-05-cs-algorithms` | CS Algorithms & Code | `generic-article` | Golden | `listings`, `minted`, 多语言高亮, 算法跨页, `\State` |
| 6 | `corpus-06-cs-database` | CS Database & Systems | `generic-article` | Golden | `tikz` ER 图, 关系代数宏, `\semijoin` 自定义宏 |

### 3.2 数学与物理学 (Math & Physics) — Corpus #7-10

| # | ID | 名称 | Profile | Tier | 核心元素 |
|---|---|---|---|---|---|
| 7 | `corpus-07-arxiv-math` | ArXiv Math — amsart | `generic-article` | Golden | `amsthm`（10+ 定理类）, `DeclareMathOperator`, `\begin{Proof}`, 多种 proof 结尾符 |
| 8 | `corpus-08-prl-physics` | APS PRL — revtex4-2 | `aps-prl` | Golden | `gather`, `multline`, `bmatrix`, `physics` 宏, `\mhchem`, `\siunitx` |
| 9 | `corpus-09-math-edgecases` | Math Equation Edge Cases | `generic-article` | Golden | `tikz-cd` 交换图, `cases`, `numcases`, 10×10 超长矩阵, `split` 跨行 |
| 10 | `corpus-10-physics-optics` | Physics Optics Report | `generic-article` | Golden | `wrapfig`, `wraptable`, `floatrow`, `siunitx`, `chemfig` |

### 3.3 生命科学与化学 (Life Sciences & Chemistry) — Corpus #11-13

| # | ID | 名称 | Profile | Tier | 核心元素 |
|---|---|---|---|---|---|
| 11 | `corpus-11-nature-biology` | Nature / Science Biology | `nature` | Golden | `nature` 宏包, `\begin{abst}`, `biblatex`+`.bbl`, 上标引用 `\supercite`, 4 子图拼接 |
| 12 | `corpus-12-elsevier-chem` | Elsevier Chemistry | `elsevier-chem` | Golden | `elsarticle`, `chemfig`, `mhchem`, `bpchem`, `siunitx` S 列格式, `\frontmatter` |
| 13 | `corpus-13-bioinformatics` | Bioinformatics — longtable | `generic-article` | Golden | `longtable`+`sidewaystable`, 25+ 行跨页, `multirow`, `siunitx` 数值列 |

### 3.4 人文社科与经济学 (Humanities & Economics) — Corpus #14-16

| # | ID | 名称 | Profile | Tier | 核心元素 |
|---|---|---|---|---|---|
| 14 | `corpus-14-econ-econometrica` | Economics — Econometrica | `econ-econometrica` | Golden | `threeparttable`, `dcolumn` D 列, 显著性星号 `***`, `booktabs` 回归表 |
| 15 | `corpus-15-humanities-apa` | Humanities — APA 7th | `apa-7` | Golden | `apa7`, `biblatex` (apa style), `csquotes`, 长摘要, author-year 引用 |
| 16 | `corpus-16-linguistics-syntax` | Linguistics — Syntax Trees | `generic-article` | Golden | `tikz-qtree`/`forest` 句法树, `gb4e` 双行对照, `\gll...\glt` 格式 |

### 3.5 综合排版极限与组件压力测试 (Stress Tests) — Corpus #17-30

| # | ID | 名称 | Profile | Tier | 核心元素 |
|---|---|---|---|---|---|
| 17 | `corpus-17-table-stress` | Table Stress Test | `generic-article` | Smoke | `tabularx` X 列, `tabulary`, `multirow`, `\ multicolumn`, 单元格内嵌 `minipage`（含图片/列表） |
| 18 | `corpus-18-figure-stress` | Figure Stress Test | `generic-article` | Smoke | `[H]`/`[p]` 强制位置, `subcaptiongroup`, `minipage` 并排, 超长 caption |
| 19 | `corpus-19-ref-bibtex-complex` | BibTeX Complex Refs | `generic-article` | Smoke | 6+ 文献类型, 多语言人名, 特殊字符（`\'e`, `\"u`, `\~n`） |
| 20 | `corpus-20-ref-biblatex-complex` | BibLaTeX Complex Refs | `generic-article` | Smoke | `biblatex` 分组 `printbibliography`, `\parencite`/`\textcite`/`\supercite` 三种格式, `backref` |
| 21 | `corpus-21-macro-expansion` | Macro Expansion Stress | `generic-article` | Smoke | `\ExplSyntaxOn`, `xparse` `\NewDocumentCommand`, 递归宏, `etoolbox` |
| 22 | `corpus-22-list-nested` | List Nesting Stress | `generic-article` | Smoke | 6 层嵌套, `enumitem` 自定义, `description` 特殊列表 |
| 23 | `corpus-23-chinese-typography` | Chinese Typography Edge | `jos-paper` | Golden | `xeCJK`, `xpinyin`, 中文标点挤压, 繁体字, 生僻字 |
| 24 | `corpus-24-multicolumn` | Multicolumn Layout Test | `generic-article` | Smoke | `multicol` 双栏, `table*`/`figure*` 跨栏, 单栏中间切换 |
| 25 | `corpus-25-report-thesis` | Long Report / Thesis | `generic-article` | Golden | `\include` 多文件, `tocbibind`, `minitoc`, `fancyhdr`, 附录 |
| 26 | `corpus-26-color-hyperlink` | Color and Hyperlink | `generic-article` | Smoke | `xcolor`, `colortbl` 交替行, `\cellcolor`, `hyperref` 颜色链接, `mdframed` |
| 27 | `corpus-27-header-footer` | Header, Footer and Page Style | `generic-article` | Smoke | `fancyhdr` 奇偶页不同, `titletoc`, `\patchcmd` 章节级样式切换 |
| 28 | `corpus-28-footnote-marginnote` | Footnote and Marginpar | `generic-article` | Smoke | 长脚注跨页, `sidenotes`, `marginpar`, `marginfix` |
| 29 | `corpus-29-layout-absolute` | Absolute Positioning | `generic-article` | Smoke | `eso-pic` 水印, TikZ `remember picture, overlay`, `textpos` 绝对定位 |
| 30 | `corpus-30-legacy-deprecated` | Legacy / Deprecated Packages | `generic-article` | Smoke | `epsfig`, `eqnarray`, `times`, `latexsym`, `makeidx` 索引降级 |

---

## 4. Corpus 目录结构

```
docs-zh/examples/corpus/
├── corpus-01-ieee-trans/
│   ├── main.tex
│   ├── refs.bib
│   ├── figures/              # 合成图片（SVG→PDF/PNG）
│   ├── quality_meta.json
│   └── README.md
├── corpus-02-cvpr/
│   ├── main.tex
│   ├── refs.bib
│   ├── figures/
│   ├── quality_meta.json
│   └── README.md
├── corpus-03-acm-sig/
├── corpus-04-jos-chinese/     # 含 references-zh.tex
├── corpus-05-cs-algorithms/
├── corpus-06-cs-database/
├── corpus-07-arxiv-math/
├── corpus-08-prl-physics/
├── corpus-09-math-edgecases/
├── corpus-10-physics-optics/
├── corpus-11-nature-biology/  # 含 main.bbl
├── corpus-12-elsevier-chem/
├── corpus-13-bioinformatics/
├── corpus-14-econ-econometrica/
├── corpus-15-humanities-apa/
├── corpus-16-linguistics-syntax/
├── corpus-17-table-stress/
├── corpus-18-figure-stress/
├── corpus-19-ref-bibtex-complex/
├── corpus-20-ref-biblatex-complex/
├── corpus-21-macro-expansion/
├── corpus-22-list-nested/
├── corpus-23-chinese-typography/
├── corpus-24-multicolumn/
├── corpus-25-report-thesis/   # 含 chapters/ 和 appendices/
├── corpus-26-color-hyperlink/
├── corpus-27-header-footer/
├── corpus-28-footnote-marginnote/
├── corpus-29-layout-absolute/
├── corpus-30-legacy-deprecated/
├── _shared/                   # 共享辅助文件
│   ├── fig-placeholder.svg    # 通用占位图
│   ├── fig-placeholder-wide.svg
│   └── nature-bibliography-setup.tex
├── corpus-design.md           # 详细设计文档（核心内容）
└── corpus-generation-plan.md # 本文件
```

---

## 5. quality_meta.json Schema

每个 Corpus 根目录必须包含 `quality_meta.json`：

```json
{
  "corpus_id": "corpus-01-ieee-trans",
  "corpus_name": "IEEE Transactions Standard",
  "corpus_name_zh": "IEEE 期刊标准模板",
  "tier": "golden",
  "profile": "ieee-trans",
  "source": "synthetic",
  "page_count_approx": 8,
  "packages": [
    "IEEEtran", "amsmath", "amssymb", "graphicx",
    "booktabs", "algorithm", "algpseudocode", "cite", "hyperref"
  ],
  "focus_areas": [
    "float", "algorithm_environment", "cross_column_table",
    "numeric_citation", "equation_alignment"
  ],
  "hard_elements": [
    "跨栏浮动体", "算法伪代码（algorithmicx）",
    "对齐方程（align*）", "booktabs 三线表", "图形与子图"
  ],
  "thresholds": {
    "parse_score_min": 95,
    "semantic_score_min": 90,
    "docx_valid_required": true,
    "word_open_repair_allowed": false,
    "formula_omml_rate_min": 90,
    "citation_resolved_rate_min": 95
  },
  "expected_warnings": [
    "algorithmicx: \\State may not be natively represented in DOCX; lowered to numbered list"
  ],
  "expected_fallbacks": [],
  "markers_to_check": [
    "Abstract", "I. INTRODUCTION", "II. METHODOLOGY",
    "III. EXPERIMENTS", "CONCLUSION", "REFERENCES"
  ],
  "known_limitations": [],
  "references": { "type": "bibtex", "count": 8 },
  "figures": { "count": 3, "types": ["pdf", "svg", "png"] },
  "tables": { "count": 2, "types": ["tabular", "booktabs"] },
  "equations": { "count": 4, "types": ["equation", "align"] },
  "algorithms": { "count": 1 }
}
```

`tier` 取值含义：
- `smoke`：快速冒烟测试，复杂度低，每次 PR 必跑，允许部分降级。
- `golden`：核心基线，覆盖主流场景，有 Golden DOCX/PDF 参照物，Word 修复必须为 0。
- `visual`：视觉回归，需渲染 PNG diff。

---

## 6. Profile × Corpus 矩阵（概览）

| Profile | Corpus IDs | 文档类 | 数量 |
|---|---|---|---|
| `ieee-trans` | #1 | IEEEtran | 1 |
| `cvpr` | #2 | IEEEtran (conf) | 1 |
| `acm-sig` | #3 | acmart | 1 |
| `jos-paper` | #4, #23 | rjthesis / ctexart | 2 |
| `generic-article` | #5, #6, #7, #9, #10, #13, #16, #17, #18, #19, #20, #21, #22, #24, #25, #26, #27, #28, #29, #30 | article/report | 20 |
| `aps-prl` | #8 | revtex4-2 | 1 |
| `nature` | #11 | nature | 1 |
| `elsevier-chem` | #12 | elsarticle | 1 |
| `econ-econometrica` | #14 | aer | 1 |
| `apa-7` | #15 | apa7 | 1 |

---

## 7. 质量维度基线

| 维度 | 权重 | 测量方式 | Corpus 覆盖重点 |
|---|---|---|---|
| **解析完整性** | 20% | unknown macro 比例、fallback 事件数 | #21, #22, #30 |
| **语义保真** | 25% | 标题/段落/列表/表格/图/公式/引用 7 类保真率 | #1-#16 |
| **DOCX 结构** | 20% | OOXML 合法性、style/numbering/media 完整性 | #17, #18, #26 |
| **版面一致** | 20% | 页边距、字号、行距、表格宽度、PDF 视觉差异 | #8, #24, #25 |
| **可编辑性** | 10% | Word 可打开、段落样式、交叉引用可更新 | #1, #4, #7 |
| **性能与稳定** | 5% | 转换耗时、fallback 次数 | #30, #21 |

---

## 8. 实施计划

### Phase 1：建立骨架（Week 1）

- 在 `docs-zh/examples/corpus/` 下创建 30 个目录。
- 复用 `examples/journals/` 中已有的 realistic 文件，扩充符合对应 Corpus 类型的复杂元素。
- 生成 `_shared/` 中的共享图形资源（SVG placeholder）。

### Phase 2：内容生成（Week 1-2）

- **Corpus #1-#16**（Golden tier）：以现有 `paper2`/`paper3`/`journals/` 真实内容为底表，扩展复杂元素。
- **Corpus #17-#30**（Smoke/Stress tier）：人工构造专注于单一瓶颈的 LaTeX 文件。
- 为每个 Corpus 生成 `refs.bib`（#1-#16, #19, #20, #23）。
- 生成合成图片（SVG → PDF/PNG）放在各 Corpus 的 `figures/` 目录。

### Phase 3：质量闭环接入（Week 2-3）

- 将 30 个 Corpus 的路径写入 CI 配置文件。
- 利用 `crates/quality` 的 `QualityRun` 对所有 Corpus 运行质量评分。
- 生成每个 Corpus 的 `golden_docx/`、`golden_pdf/`、`report.json`。
- 对 Golden tier（#1-#16）配置 Word 打开回归（`word_open_repair_allowed: false`）。
- Smoke tier（#17-#30）允许部分降级（`expected_fallbacks` 非空）。

### Phase 4：持续运营（Ongoing）

- 新增 Corpus 场景 → 进入回归集。
- Profile 更新后 → 重新生成所有 Golden DOCX。
- 每季度更新 Corpus 以覆盖新期刊模板。

---

## 9. 验收标准

| 阶段 | 指标 | 目标 |
|---|---|---|
| CI 回归 | 30 个 Corpus 全部跑通（不 crash） | 100% |
| Smoke tier (#17-#30) | 解析完整性 | >= 70% |
| Golden tier (#1-#16) | 解析完整性 | >= 90% |
| Golden tier (#1-#16) | 语义保真 | >= 85% |
| 所有 Corpus | DOCX OOXML 合法性 | 100% |
| 所有 Corpus | Word 修复阻断 | 0 |
| 所有 Corpus | Fallback 审计记录 | 100%（必须写入报告） |

---

## 10. 参考文档

| 文档 | 路径 |
|---|---|
| 详细设计（完整 LaTeX 源代码） | [`corpus-design.md`](./corpus-design.md) |
| 商业化质量提升方案 | [`../service/document-conversion-engine-quality-commercialization-plan-20260626.md`](../service/document-conversion-engine-quality-commercialization-plan-20260626.md) |
| Quality 引擎 Schema | `crates/quality/src/layer.rs` |
| Quality Marker 定义 | `crates/quality/src/markers.rs` |
| 现有期刊模板 | `examples/journals/` |
| 现有论文语料 | `examples/paper2/`, `examples/paper3/` |
