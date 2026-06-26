# Corpus 生成开发计划

> 日期：2026-06-26
> 状态：执行中
> 输出目录：`examples/demos/corpus/`

---

## 执行摘要

本计划基于以下设计文档：

- [`corpus-design.md`](./corpus-design.md) - 30 个 Corpus 详细设计
- [`corpus-generation-plan.md`](./corpus-generation-plan.md) - 整体生成方案

**目标**：为 Tex2Doc 引擎建立可量化、可回归、可解释的质量验证基线。

---

## 目录结构

```
examples/demos/
├── corpus/                          # 30 个质量验证 Corpus
│   ├── corpus-01-ieee-trans/
│   ├── corpus-02-cvpr/
│   ├── corpus-03-acm-sig/
│   ├── corpus-04-jos-chinese/
│   ├── corpus-05-cs-algorithms/
│   ├── corpus-06-cs-database/
│   ├── corpus-07-arxiv-math/
│   ├── corpus-08-prl-physics/
│   ├── corpus-09-math-edgecases/
│   ├── corpus-10-physics-optics/
│   ├── corpus-11-nature-biology/
│   ├── corpus-12-elsevier-chem/
│   ├── corpus-13-bioinformatics/
│   ├── corpus-14-econ-econometrica/
│   ├── corpus-15-humanities-apa/
│   ├── corpus-16-linguistics-syntax/
│   ├── corpus-17-table-stress/
│   ├── corpus-18-figure-stress/
│   ├── corpus-19-ref-bibtex-complex/
│   ├── corpus-20-ref-biblatex-complex/
│   ├── corpus-21-macro-expansion/
│   ├── corpus-22-list-nested/
│   ├── corpus-23-chinese-typography/
│   ├── corpus-24-multicolumn/
│   ├── corpus-25-report-thesis/
│   ├── corpus-26-color-hyperlink/
│   ├── corpus-27-header-footer/
│   ├── corpus-28-footnote-marginnote/
│   ├── corpus-29-layout-absolute/
│   ├── corpus-30-legacy-deprecated/
│   ├── _shared/                    # 共享资源
│   └── corpus-design.md           # 设计文档（复制）
├── corpus-development-plan.md      # 本文件
└── corpus-implementation-report.md  # 实施报告（完成后生成）
```

---

## Phase 1: 建立骨架（Day 1）

### 任务清单

- [ ] 创建 `examples/demos/corpus/` 根目录
- [ ] 创建 `_shared/` 共享资源目录
- [ ] 批量创建 30 个 Corpus 目录
- [ ] 为每个 Corpus 创建骨架文件：
  - `main.tex`（骨架）
  - `refs.bib`（骨架）
  - `figures/` 目录
  - `quality_meta.json`（骨架）
  - `README.md`（骨架）
- [ ] 生成 `_shared/` 共享资源：
  - `fig-placeholder.svg`
  - `fig-placeholder-wide.svg`
  - `chinese-bibliography.tex`

### 输出

```
examples/demos/corpus/
├── corpus-01-ieee-trans/
│   ├── main.tex          # 骨架
│   ├── refs.bib          # 骨架
│   ├── figures/
│   ├── quality_meta.json # 骨架
│   └── README.md         # 骨架
├── corpus-02-cvpr/
...（30 个目录）
├── _shared/
│   ├── fig-placeholder.svg
│   ├── fig-placeholder-wide.svg
│   └── chinese-bibliography.tex
└── corpus-design.md
```

---

## Phase 2: Golden Tier 内容生成（Day 2-7）

**目标**：生成 Corpus #1-#16 的完整 LaTeX 内容

### Corpus 批次

| 批次 | Corpus ID | 数量 | 学科领域 |
|---|---|---|---|
| CS & Engineering | #1-#6 | 6 | 计算机科学与工程 |
| Math & Physics | #7-#10 | 4 | 数学与物理学 |
| Life Sciences | #11-#13 | 3 | 生命科学与化学 |
| Humanities | #14-#16 | 3 | 人文社科与经济学 |

### 关键元素覆盖

- **表格**：booktabs, multirow, longtable, tabulary, resizebox
- **公式**：align, gather, amsthm, tikz-cd, cases, matrix
- **浮动体**：figure*, table*, subcaption, minipage
- **算法**：algorithmicx, algorithm2e, listings, minted
- **引用**：bibtex, biblatex, 上标引用, author-year

### 复用资源

- `examples/journals/` - 期刊模板（IEEE/ACM/Nature/Elsevier 等）
- `examples/paper2/`, `examples/paper3/` - 论文语料
- `examples/journals/cvpr/`, `examples/journals/jos-paper/` 等

### 详细列表

| # | ID | 名称 | Profile | 核心元素 |
|---|---|---|---|---|
| 1 | corpus-01-ieee-trans | IEEE Transactions Standard | ieee-trans | algorithmicx, align, booktabs, figure* |
| 2 | corpus-02-cvpr | CVPR Conference Paper | cvpr | subfig, multirow, 15+ 引用 |
| 3 | corpus-03-acm-sig | ACM SIG Conference | acm-sig | CCS 概念树, DOI 引用, acmart |
| 4 | corpus-04-jos-chinese | 软件学报中文期刊 | jos-paper | 双语摘要, 定理环境, 算法2e |
| 5 | corpus-05-cs-algorithms | CS Algorithms & Code | generic-article | listings, minted, 算法跨页 |
| 6 | corpus-06-cs-database | CS Database & Systems | generic-article | tikz ER 图, 自定义宏 |
| 7 | corpus-07-arxiv-math | ArXiv Math | generic-article | amsthm (10+ 定理), DeclareMathOperator |
| 8 | corpus-08-prl-physics | APS PRL | aps-prl | gather, bmatrix, physics, siunitx |
| 9 | corpus-09-math-edgecases | Math Equation Edge Cases | generic-article | tikz-cd, cases, 10x10 矩阵 |
| 10 | corpus-10-physics-optics | Physics Optics Report | generic-article | wrapfig, wraptable, siunitx, chemfig |
| 11 | corpus-11-nature-biology | Nature / Science Biology | nature | nature 宏包, biblatex, 4 子图 |
| 12 | corpus-12-elsevier-chem | Elsevier Chemistry | elsevier-chem | elsarticle, chemfig, mhchem, bpchem |
| 13 | corpus-13-bioinformatics | Bioinformatics | generic-article | longtable, sidewaystable, 25+ 行 |
| 14 | corpus-14-econ-econometrica | Economics | econ-econometrica | threeparttable, dcolumn, 回归表 |
| 15 | corpus-15-humanities-apa | Humanities APA 7th | apa-7 | apa7, biblatex, 长摘要 |
| 16 | corpus-16-linguistics-syntax | Linguistics Syntax Trees | generic-article | tikz-qtree, forest, gb4e |

---

## Phase 3: Smoke Tier 内容生成（Day 8-14）

**目标**：生成 Corpus #17-#30 的极限测试内容

| # | ID | 名称 | 焦点元素 |
|---|---|---|---|
| 17 | corpus-17-table-stress | Table Stress Test | tabularx, multirow, 内嵌 minipage |
| 18 | corpus-18-figure-stress | Figure Stress Test | 强制位置 [H], subcaptiongroup |
| 19 | corpus-19-ref-bibtex-complex | BibTeX Complex Refs | 6+ 文献类型, 特殊字符 |
| 20 | corpus-20-ref-biblatex-complex | BibLaTeX Complex Refs | 分组引用, backref |
| 21 | corpus-21-macro-expansion | Macro Expansion Stress | xparse, 递归宏, etoolbox |
| 22 | corpus-22-list-nested | List Nesting Stress | 6 层嵌套, enumitem |
| 23 | corpus-23-chinese-typography | Chinese Typography Edge | xeCJK, xpinyin, 繁体字, 生僻字 |
| 24 | corpus-24-multicolumn | Multicolumn Layout Test | multicol, table*/figure* 跨栏 |
| 25 | corpus-25-report-thesis | Long Report / Thesis | \include, minitoc, fancyhdr, 附录 |
| 26 | corpus-26-color-hyperlink | Color and Hyperlink | xcolor, colortbl, hyperref, mdframed |
| 27 | corpus-27-header-footer | Header, Footer and Page Style | fancyhdr 奇偶页, titletoc |
| 28 | corpus-28-footnote-marginnote | Footnote and Marginpar | 长脚注跨页, sidenotes, marginpar |
| 29 | corpus-29-layout-absolute | Absolute Positioning | eso-pic 水印, textpos, overlay |
| 30 | corpus-30-legacy-deprecated | Legacy / Deprecated Packages | epsfig, eqnarray, times, latexsym |

---

## Phase 4: Quality Meta 生成（Day 15-17）

**目标**：为每个 Corpus 生成完整的 `quality_meta.json`

### Schema 字段

```json
{
  "corpus_id": "corpus-01-ieee-trans",
  "corpus_name": "IEEE Transactions Standard",
  "corpus_name_zh": "IEEE 期刊标准模板",
  "tier": "golden",
  "profile": "ieee-trans",
  "source": "synthetic",
  "page_count_approx": 8,
  "packages": ["IEEEtran", "amsmath", "amssymb", ...],
  "focus_areas": ["float", "algorithm_environment", ...],
  "hard_elements": ["跨栏浮动体", ...],
  "thresholds": {
    "parse_score_min": 95,
    "semantic_score_min": 90,
    "docx_valid_required": true,
    "word_open_repair_allowed": false,
    "formula_omml_rate_min": 90,
    "citation_resolved_rate_min": 95
  },
  "expected_warnings": [...],
  "expected_fallbacks": [...],
  "markers_to_check": [...],
  "known_limitations": [],
  "references": { "type": "bibtex", "count": 8 },
  "figures": { "count": 3, "types": ["pdf", "svg", "png"] },
  "tables": { "count": 2, "types": ["tabular", "booktabs"] },
  "equations": { "count": 4, "types": ["equation", "align"] },
  "algorithms": { "count": 1 }
}
```

### Tier 含义

| Tier | 用途 | 复杂度 | 回归要求 |
|---|---|---|---|
| `smoke` | 快速冒烟测试 | 低 | 允许部分降级 |
| `golden` | 核心基线 | 中 | Word 修复必须为 0 |
| `visual` | 视觉回归 | 高 | 渲染 PNG diff |

---

## Phase 5: CI 集成与验证（Day 18-21）

**目标**：将 Corpus 集成到 CI 流程

### CI 配置

1. 在 CI 配置文件中注册 30 个 Corpus 路径
2. 配置 Golden tier 严格模式：
   - `word_open_repair_allowed: false`
   - `docx_valid_required: true`
3. 配置 Smoke tier 宽松模式：
   - 允许 `expected_fallbacks` 非空
   - 允许 `parse_score_min >= 70%`

### 质量运行

```bash
# 对所有 Corpus 运行质量评分
tex2doc quality --corpus examples/demos/corpus/ --output reports/

# 生成 Golden DOCX/PDF
for corpus in examples/demos/corpus/corpus-*/; do
  tex2doc convert "$corpus/main.tex" --output "$corpus/golden_docx/"
done
```

### 验收检查

| 指标 | 目标 |
|---|---|
| CI 回归 | 30 个 Corpus 全部跑通（不 crash） |
| Smoke tier | 解析完整性 >= 70% |
| Golden tier | 解析完整性 >= 90% |
| Golden tier | 语义保真 >= 85% |
| 所有 Corpus | DOCX OOXML 合法性 100% |
| 所有 Corpus | Word 修复阻断 0 |
| 所有 Corpus | Fallback 审计记录 100% |

---

## Phase 6: 实施报告生成（Day 22）

**目标**：输出实施报告到 `examples/demos/corpus-implementation-report.md`

### 报告内容

- **执行摘要**：完成状态、关键指标
- **目录结构**：最终 Corpus 树形结构
- **按 Corpus 详情**：每个 Corpus 的生成状态和文件列表
- **验收结果**：按维度（解析完整性、语义保真、版面一致等）的质量评分
- **问题与限制**：已知问题和未来改进

---

## 时间线总览

```
Day 1:   Phase 1 - 骨架创建
Day 2-7: Phase 2 - Golden Tier (#1-#16)
Day 8-14: Phase 3 - Smoke Tier (#17-#30)
Day 15-17: Phase 4 - Quality Meta
Day 18-21: Phase 5 - CI 集成
Day 22:   Phase 6 - 报告生成
```

---

## 关键依赖

| 依赖 | 来源 |
|---|---|
| 期刊模板 | `examples/journals/` |
| 论文语料 | `examples/paper2/`, `examples/paper3/` |
| Quality 引擎 | `crates/quality/` |
| CI 配置 | `.github/workflows/` |

---

## Profile 映射表

| Profile | Document Class | Corpus IDs |
|---|---|---|
| `ieee-trans` | IEEEtran | #1 |
| `cvpr` | IEEEtran (conf) | #2 |
| `acm-sig` | acmart | #3 |
| `jos-paper` | rjthesis / ctexart | #4, #23 |
| `generic-article` | article / report | #5-#7, #9-#10, #13, #16-#22, #24-#30 |
| `aps-prl` | revtex4-2 | #8 |
| `nature` | nature | #11 |
| `elsevier-chem` | elsarticle | #12 |
| `econ-econometrica` | aer | #14 |
| `apa-7` | apa7 | #15 |

---

## 参考文档

| 文档 | 路径 |
|---|---|
| 详细设计 | [`docs-zh/examples/corpus-design.md`](docs-zh/examples/corpus-design.md) |
| 生成方案 | [`docs-zh/examples/corpus-generation-plan.md`](docs-zh/examples/corpus-generation-plan.md) |
| 商业化方案 | [`docs-zh/service/document-conversion-engine-quality-commercialization-plan-20260626.md`](../service/document-conversion-engine-quality-commercialization-plan-20260626.md) |
