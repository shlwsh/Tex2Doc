# Corpus 生成实施报告

> 日期：2026-06-26
> 状态：Phase 1-3 完成

---

## 执行摘要

本报告记录了 Tex2Doc 30 个质量验证 Corpus 生成项目的实施状态。

### 关键指标

| 指标 | 目标 | 当前状态 |
|---|---|---|
| Corpus 数量 | 30 | 30 ✅ |
| main.tex 完整内容 | 30 | 30 ✅ |
| refs.bib 完整内容 | 24 | 24 ✅ |
| quality_meta.json | 30 | 30 ✅ |
| README.md | 30 | 30 ✅ |
| 合成图片 | 30 | 30 ✅ |

### Tier 分布

| Tier | 数量 | Corpus IDs |
|---|---|---|
| Golden | 16 | #1-#16 |
| Smoke | 14 | #17-#30 |

---

## 目录结构

```
examples/demos/corpus/
├── _shared/                           # 共享资源
│   ├── fig-placeholder.svg             # ✅
│   ├── fig-placeholder-wide.svg      # ✅
│   ├── fig-architecture.svg          # ✅
│   ├── fig-experiment-results.svg    # ✅
│   ├── fig-dataset-overview.svg      # ✅
│   └── chinese-bibliography.tex      # ✅
├── corpus-01-ieee-trans/            # ✅ Golden - 完整
├── corpus-02-cvpr/                  # ✅ Golden - 完整
├── corpus-03-acm-sig/               # ✅ Golden - 完整
├── corpus-04-jos-chinese/           # ✅ Golden - 完整
├── corpus-05-cs-algorithms/         # ✅ Golden - 完整
├── corpus-06-cs-database/            # ✅ Golden - 完整
├── corpus-07-arxiv-math/            # ✅ Golden - 完整
├── corpus-08-prl-physics/           # ✅ Golden - 完整
├── corpus-09-math-edgecases/         # ✅ Golden - 完整
├── corpus-10-physics-optics/         # ✅ Golden - 完整
├── corpus-11-nature-biology/         # ✅ Golden - 完整
├── corpus-12-elsevier-chem/         # ✅ Golden - 完整
├── corpus-13-bioinformatics/         # ✅ Golden - 完整
├── corpus-14-econ-econometrica/     # ✅ Golden - 完整
├── corpus-15-humanities-apa/         # ✅ Golden - 完整
├── corpus-16-linguistics-syntax/     # ✅ Golden - 完整
├── corpus-17-table-stress/           # ✅ Smoke - 完整
├── corpus-18-figure-stress/          # ✅ Smoke - 完整
├── corpus-19-ref-bibtex-complex/     # ✅ Smoke - 完整
├── corpus-20-ref-biblatex-complex/   # ✅ Smoke - 完整
├── corpus-21-macro-expansion/        # ✅ Smoke - 完整
├── corpus-22-list-nested/            # ✅ Smoke - 完整
├── corpus-23-chinese-typography/      # ✅ Golden - 完整
├── corpus-24-multicolumn/            # ✅ Smoke - 完整
├── corpus-25-report-thesis/           # ✅ Golden - 完整
├── corpus-26-color-hyperlink/       # ✅ Smoke - 完整
├── corpus-27-header-footer/          # ✅ Smoke - 完整
├── corpus-28-footnote-marginnote/    # ✅ Smoke - 完整
├── corpus-29-layout-absolute/         # ✅ Smoke - 完整
├── corpus-30-legacy-deprecated/      # ✅ Smoke - 完整
├── corpus-design.md                  # ✅ 设计文档
└── corpus-development-plan.md         # ✅ 开发计划
```

---

## Corpus 详细状态

### Golden Tier (#1-#16)

| # | ID | 名称 | Profile | main.tex | refs.bib | quality_meta | README | figures |
|---|---|---|---|---|---|---|---|---|
| 1 | corpus-01-ieee-trans | IEEE Transactions Standard | ieee-trans | ✅ | ✅ | ✅ | ✅ | ✅ |
| 2 | corpus-02-cvpr | CVPR Conference Paper | cvpr | ✅ | ✅ | ✅ | ✅ | ✅ |
| 3 | corpus-03-acm-sig | ACM SIG Conference | acm-sig | ✅ | ✅ | ✅ | ✅ | ✅ |
| 4 | corpus-04-jos-chinese | 软件学报中文期刊 | jos-paper | ✅ | ✅ | ✅ | ✅ | ✅ |
| 5 | corpus-05-cs-algorithms | CS Algorithms & Code | generic-article | ✅ | ✅ | ✅ | ✅ | ✅ |
| 6 | corpus-06-cs-database | CS Database & Systems | generic-article | ✅ | ✅ | ✅ | ✅ | ✅ |
| 7 | corpus-07-arxiv-math | ArXiv Math | generic-article | ✅ | ✅ | ✅ | ✅ | ✅ |
| 8 | corpus-08-prl-physics | APS PRL Physics | aps-prl | ✅ | ✅ | ✅ | ✅ | ✅ |
| 9 | corpus-09-math-edgecases | Math Equation Edge Cases | generic-article | ✅ | ✅ | ✅ | ✅ | ✅ |
| 10 | corpus-10-physics-optics | Physics Optics Report | generic-article | ✅ | ✅ | ✅ | ✅ | ✅ |
| 11 | corpus-11-nature-biology | Nature / Science Biology | nature | ✅ | ✅ | ✅ | ✅ | ✅ |
| 12 | corpus-12-elsevier-chem | Elsevier Chemistry | elsevier-chem | ✅ | ✅ | ✅ | ✅ | ✅ |
| 13 | corpus-13-bioinformatics | Bioinformatics | generic-article | ✅ | ✅ | ✅ | ✅ | ✅ |
| 14 | corpus-14-econ-econometrica | Economics | econ-econometrica | ✅ | ✅ | ✅ | ✅ | ✅ |
| 15 | corpus-15-humanities-apa | Humanities APA 7th | apa-7 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 16 | corpus-16-linguistics-syntax | Linguistics Syntax Trees | generic-article | ✅ | ✅ | ✅ | ✅ | ✅ |

### Smoke Tier (#17-#30)

| # | ID | 名称 | main.tex | quality_meta | README |
|---|---|---|---|---|---|
| 17 | corpus-17-table-stress | Table Stress Test | ✅ | ✅ | ✅ |
| 18 | corpus-18-figure-stress | Figure Stress Test | ✅ | ✅ | ✅ |
| 19 | corpus-19-ref-bibtex-complex | BibTeX Complex Refs | ✅ | ✅ | ✅ |
| 20 | corpus-20-ref-biblatex-complex | BibLaTeX Complex Refs | ✅ | ✅ | ✅ |
| 21 | corpus-21-macro-expansion | Macro Expansion Stress | ✅ | ✅ | ✅ |
| 22 | corpus-22-list-nested | List Nesting Stress | ✅ | ✅ | ✅ |
| 23 | corpus-23-chinese-typography | Chinese Typography Edge | ✅ | ✅ | ✅ |
| 24 | corpus-24-multicolumn | Multicolumn Layout Test | ✅ | ✅ | ✅ |
| 25 | corpus-25-report-thesis | Long Report / Thesis | ✅ | ✅ | ✅ |
| 26 | corpus-26-color-hyperlink | Color and Hyperlink | ✅ | ✅ | ✅ |
| 27 | corpus-27-header-footer | Header, Footer and Page Style | ✅ | ✅ | ✅ |
| 28 | corpus-28-footnote-marginnote | Footnote and Marginpar | ✅ | ✅ | ✅ |
| 29 | corpus-29-layout-absolute | Absolute Positioning | ✅ | ✅ | ✅ |
| 30 | corpus-30-legacy-deprecated | Legacy / Deprecated Packages | ✅ | ✅ | ✅ |

---

## 实施阶段

### ✅ Phase 1: 建立骨架（已完成）

- [x] 创建 30 个 Corpus 目录
- [x] 创建 `_shared/` 共享资源目录
- [x] 为每个 Corpus 创建骨架文件
- [x] 生成 `_shared/` 共享资源文件

### ✅ Phase 2: Golden Tier 内容生成（已完成）

- [x] corpus-01-ieee-trans：IEEEtran 模板，algorithmicx，align，booktabs
- [x] corpus-02-cvpr：subfig，子图，跨页公式
- [x] corpus-03-acm-sig：acmart，CCS 概念树
- [x] corpus-04-jos-chinese：rjthesis，双语摘要，algorithm2e
- [x] corpus-05-cs-algorithms：listings/minted，Python/Rust 代码
- [x] corpus-06-cs-database：TikZ ER 图，关系代数宏
- [x] corpus-07-arxiv-math：10+ 定理环境，DeclareMathOperator
- [x] corpus-08-prl-physics：revtex4-2，gather，bmatrix，siunitx
- [x] corpus-09-math-edgecases：tikz-cd，cases，10x10 矩阵
- [x] corpus-10-physics-optics：wrapfig，chemfig，siunitx
- [x] corpus-11-nature-biology：nature，biblatex，supercite
- [x] corpus-12-elsevier-chem：elsarticle，chemfig，mhchem
- [x] corpus-13-bioinformatics：longtable，25+ 行
- [x] corpus-14-econ-econometrica：threeparttable，dcolumn
- [x] corpus-15-humanities-apa：apa7，biblatex，APA
- [x] corpus-16-linguistics-syntax：tikz-qtree，forest，gb4e

### ✅ Phase 3: Smoke Tier 内容生成（已完成）

- [x] corpus-17-table-stress：tabularx，tabulary，multirow，minipage
- [x] corpus-18-figure-stress：[H] 位置，subcaptiongroup
- [x] corpus-19-ref-bibtex-complex：10 种引用类型，特殊字符
- [x] corpus-20-ref-biblatex-complex：分组引用，backref
- [x] corpus-21-macro-expansion：xparse，ExplSyntaxOn
- [x] corpus-22-list-nested：6 层嵌套，enumitem
- [x] corpus-23-chinese-typography：xeCJK，繁体字，生僻字
- [x] corpus-24-multicolumn：multicol，column break
- [x] corpus-25-report-thesis：fancyhdr，tocbibind，appendix
- [x] corpus-26-color-hyperlink：xcolor，colortbl，mdframed
- [x] corpus-27-header-footer：fancyhdr，odd/even headers
- [x] corpus-28-footnote-marginnote：sidenotes，marginpar
- [x] corpus-29-layout-absolute：eso-pic，textpos
- [x] corpus-30-legacy-deprecated：epsfig，eqnarray，times

### ⏳ Phase 4: Quality Meta 完善（待执行）

**计划任务**：
- 根据实际内容更新 `quality_meta.json`
- 完善 `expected_warnings` 和 `expected_fallbacks`
- 调整 `thresholds` 阈值

### ⏳ Phase 5: CI 集成与验证（待执行）

**计划任务**：
- 在 CI 配置中注册 Corpus 路径
- 运行质量评分
- 生成 Golden DOCX/PDF

### ⏳ Phase 6: 报告更新（待执行）

**计划任务**：
- 补充验收结果
- 生成质量评分汇总

---

## 已生成内容亮点

### Golden Tier 特色内容

| Corpus | 特色元素 | 复杂度 |
|---|---|---|
| #1 IEEE | algorithmicx, align, figure*, booktabs | 高 |
| #4 JOS | 双语摘要, bicaption, algorithm2e | 高 |
| #7 Math | 10+ 定理类型, proof 环境 | 高 |
| #9 Math Edge | tikz-cd, 10x10 矩阵 | 极高 |
| #11 Nature | nature class, supercite | 高 |
| #16 Linguistics | gb4e, tikz-qtree, forest | 高 |

### Smoke Tier 特色内容

| Corpus | 特色元素 | 难度 |
|---|---|---|
| #17 Table | tabularx, multirow, minipage 嵌套 | 高 |
| #19 BibTeX | 10 种引用类型, 特殊字符 | 中 |
| #21 Macro | xparse, ExplSyntaxOn | 高 |
| #22 List | 6 层嵌套列表 | 中 |
| #23 Chinese | xeCJK, xpinyin, 生僻字 | 高 |
| #30 Legacy | epsfig, eqnarray, makeidx | 中 |

---

## 问题与限制

### 当前已知限制

1. **SVG 图片未转换为 PDF/PNG**：部分 Corpus 需要 PDF/PNG 格式图片
2. **corpus-11-nature-biology 需要 main.bbl**：biblatex 预处理需要 .bbl 文件
3. **corpus-25-report-thesis 章节文件未创建**：可选择使用 \input 包含

### 后续改进

1. 将 SVG 转换为 PDF/PNG 以提高兼容性
2. 预生成 .bbl 文件用于 biblatex Corpus
3. 集成 CI 流程，建立自动化回归测试

---

## 参考文档

| 文档 | 路径 |
|---|---|
| 开发计划 | [`corpus-development-plan.md`](./corpus-development-plan.md) |
| 详细设计 | [`corpus/corpus-design.md`](./corpus/corpus-design.md) |
| 生成方案 | [`docs-zh/examples/corpus-generation-plan.md`](../docs-zh/examples/corpus-generation-plan.md) |

---

## 附录：Profile 映射表

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
