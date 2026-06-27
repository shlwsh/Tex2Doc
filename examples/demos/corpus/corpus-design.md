# Tex2Doc 30 个质量验证 Corpus 详细设计

> 日期：2026-06-26
> 输出目录：`examples/demos/corpus/`
> 配套文件：`docs-zh/examples/corpus-generation-plan.md`
> 适用范围：Tex2Doc 引擎质量基线（Smoke / Golden / Visual 三层回归）

---

## 1. 设计目标与目录结构

### 1.1 总体目标

根据商业化整改方案 P0 阶段要求，建立 **30 个可量化、可回归、可解释** 的质量验证 Corpus。覆盖主流期刊模板（IEEE/ACM/APS/Elsevier/Springer/Nature/ACL/软件学报等）、极限排版场景（跨页表格、超长公式、嵌套列表、自定义宏）以及降级边界场景（废弃宏包、不兼容包、老旧语法）。

### 1.2 Corpus 根目录结构

```
examples/demos/corpus/
├── corpus-01-ieee-trans/              # Corpus #1
│   ├── main.tex
│   ├── refs.bib
│   ├── figures/                       # 合成 SVG/PNG
│   │   ├── fig-architecture.pdf
│   │   ├── fig-experiment-results.pdf
│   │   └── fig-dataset-overview.pdf
│   ├── quality_meta.json
│   └── README.md
├── corpus-02-cvpr/                   # Corpus #2
├── corpus-03-acm-sig/                # Corpus #3
├── corpus-04-jos-chinese/             # Corpus #4
├── corpus-05-cs-algorithms/           # Corpus #5
├── corpus-06-cs-database/             # Corpus #6
├── corpus-07-arxiv-math/              # Corpus #7
├── corpus-08-prl-physics/             # Corpus #8
├── corpus-09-math-edgecases/          # Corpus #9
├── corpus-10-physics-optics/          # Corpus #10
├── corpus-11-nature-biology/          # Corpus #11
├── corpus-12-elsevier-chem/           # Corpus #12
├── corpus-13-bioinformatics/         # Corpus #13
├── corpus-14-econ-econometrica/       # Corpus #14
├── corpus-15-humanities-apa/           # Corpus #15
├── corpus-16-linguistics-syntax/      # Corpus #16
├── corpus-17-table-stress/            # Corpus #17
├── corpus-18-figure-stress/           # Corpus #18
├── corpus-19-ref-bibtex-complex/      # Corpus #19
├── corpus-20-ref-biblatex-complex/     # Corpus #20
├── corpus-21-macro-expansion/         # Corpus #21
├── corpus-22-list-nested/             # Corpus #22
├── corpus-23-chinese-typography/      # Corpus #23
├── corpus-24-multicolumn/             # Corpus #24
├── corpus-25-report-thesis/           # Corpus #25
├── corpus-26-color-hyperlink/         # Corpus #26
├── corpus-27-header-footer/           # Corpus #27
├── corpus-28-footnote-marginnote/     # Corpus #28
├── corpus-29-layout-absolute/         # Corpus #29
├── corpus-30-legacy-deprecated/       # Corpus #30
├── _shared/                           # 共享辅助文件
│   ├── fig-placeholder.svg
│   ├── fig-placeholder-wide.svg
│   └── chinese-bibliography.tex
└── corpus-design.md                   # 本文件
```

---

## 2. Quality Meta 元数据规范

每个 Corpus 根目录必须包含 `quality_meta.json`，Schema 如下：

```json
{
  "corpus_id": "corpus-01-ieee-trans",
  "corpus_name": "IEEE Transactions Standard",
  "corpus_name_zh": "IEEE 期刊标准模板",
  "tier": "golden",
  "profile": "ieee-trans",
  "source": "synthetic",
  "source_note": "基于 IEEEtran 官方模板合成，覆盖标准双栏论文全部元素",
  "page_count_approx": 8,
  "packages": [
    "IEEEtran",
    "amsmath",
    "amssymb",
    "graphicx",
    "booktabs",
    "algorithm",
    "algpseudocode",
    "cite",
    "hyperref",
    "url"
  ],
  "focus_areas": [
    "float",
    "algorithm_environment",
    "cross_column_table",
    "numeric_citation",
    "equation_alignment"
  ],
  "hard_elements": [
    "跨栏浮动体",
    "算法伪代码（algorithmicx）",
    "对齐方程（align*）",
    "booktabs 三线表",
    "图形与子图"
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
    "Abstract",
    "I. INTRODUCTION",
    "II. METHODOLOGY",
    "III. EXPERIMENTS",
    "CONCLUSION",
    "REFERENCES"
  ],
  "known_limitations": [],
  "references": {
    "type": "bibtex",
    "count": 8
  },
  "figures": {
    "count": 3,
    "types": ["pdf", "svg", "png"],
    "note": "使用 _shared/fig-placeholder 生成"
  },
  "tables": {
    "count": 2,
    "types": ["tabular", "booktabs"]
  },
  "equations": {
    "count": 4,
    "types": ["equation", "align"]
  },
  "algorithms": {
    "count": 1
  },
  "author_note": "合成样例，无真实作者"
}
```

`tier` 取值含义：
- `smoke`：快速冒烟测试，复杂度低，每次 PR 必跑。
- `golden`：核心基线，覆盖主流场景，有 Golden DOCX/PDF 参照物。
- `visual`：视觉回归，需渲染 PNG diff。

---

## 3. 三十个 Corpus 详细规格

---

### Corpus #1 — `corpus-01-ieee-trans`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-01-ieee-trans` |
| **名称** | IEEE Transactions Standard |
| **Profile** | `ieee-trans` |
| **Tier** | `golden` |
| **类型** | 合成（基于 IEEEtran 官方模板结构） |

**目的**：验证 IEEE Transactions 双栏模板的标准论文转换能力，是所有 IEEE 系列 Profile 的基线。

**文件**：`main.tex` + `refs.bib`

**核心 LaTeX 内容**：

```latex
\documentclass[journal,12pt]{IEEEtran}
\usepackage[compatibility=false]{amsmath}
\usepackage{amssymb}
\usepackage{graphicx}
\usepackage{booktabs}
\usepackage{algorithm}
\usepackage{algpseudocode}
\usepackage{cite}
\usepackage{hyperref}
\usepackage{url}

\hypersetup{
  colorlinks=true,
  citecolor=blue,
  linkcolor=red,
  urlcolor=blue
}

\graphicspath{{./figures/}}

% ---- 跨栏大表（IEEEtran 特色场景）----
\begin{figure*}[t]
  \centering
  \includegraphics[width=0.85\textwidth]{fig-architecture.pdf}
  \caption{Overall system architecture. The pipeline consists of
    three stages: preprocessing, feature extraction, and classification.}
  \label{fig:arch}
\end{figure*}

% ---- 对齐公式 ----
\begin{align}
  \mathcal{L}_{\text{total}}
    &= \alpha \cdot \mathcal{L}_{\text{ce}}
       + \beta  \cdot \mathcal{L}_{\text{dice}}
       + \gamma \cdot \mathcal{L}_{\text{edge}} \label{eq:loss} \\
  \mathcal{L}_{\text{ce}}
    &= -\frac{1}{N}\sum_{i=1}^{N}
       \sum_{c=1}^{C} y_{ic}\log(\hat{y}_{ic}) \label{eq:ce} \\
  \text{DSC}(S,\hat{S})
    &= \frac{2|S\cap\hat{S}|}{|S|+|\hat{S}|}
       \in [0,1] \label{eq:dsc}
\end{align}

% ---- 算法环境 ----
\begin{algorithm}[t]
  \caption{Training Procedure}
  \begin{algorithmic}[1]
    \State $\theta \gets \theta_0$
    \For{$epoch = 1$ \textbf{to} $E$}
      \For{each batch $(x,y) \in \mathcal{D}$}
        \State $\hat{y} \gets f_\theta(x)$
        \State $J \gets \mathcal{L}(y,\hat{y})$
        \State $\theta \gets \theta - \eta \nabla_\theta J$
      \EndFor
    \EndFor
    \State \Return $\theta$
  \end{algorithmic}
\end{algorithm}

% ---- booktabs 三线表 ----
\begin{table}[t]
\centering
\caption{Comparison with state-of-the-art methods on the validation set.}
\label{tab:compare}
\begin{tabular}{lccc}
  \toprule
  Method & Dice $\uparrow$ & HD$\downarrow$ & Params \\
  \midrule
  U-Net~\cite{ronneberger2015unet} & 0.863 & 4.21 & 7.8M \\
  Attention U-Net~\cite{oktay2018attention} & 0.871 & 3.95 & 8.2M \\
  \textbf{Ours} & \textbf{0.894} & \textbf{3.12} & 5.1M \\
  \bottomrule
\end{tabular}
\end{table}
```

**指标要求**：

| 维度 | 最低值 |
|---|---|
| 解析完整性 | 95% |
| 语义保真 | 90% |
| 公式 OMML 转换率 | >= 90% |
| 引用解析率 | >= 95% |
| DOCX 合法性 | 必须通过 |
| Word 修复 | 不允许 |

**预期降级**：`algorithmicx` 的 `\State` 降级为编号列表；`\text{}` 中的下标样式降级。

---

### Corpus #2 — `corpus-02-cvpr`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-02-cvpr` |
| **名称** | CVPR Conference Paper |
| **Profile** | `cvpr` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：验证 CVPR 会议论文的子图（subfigure）并排、AP/FPS 指标表、超长引用列表。

**文件**：`main.tex` + `refs.bib` + 3 个子图 PNG

**核心 LaTeX 内容**：

```latex
\documentclass[conference]{IEEEtran}
\usepackage{graphicx}
\usepackage{amsmath}
\usepackage{booktabs}
\usepackage[caption=false,font=footnotesize]{subfig}

\graphicspath{{./figures/}}

% ---- 子图并排（CVPR 特色）----
\begin{figure}[t]
  \centering
  \subfloat[Input image]{\includegraphics[width=0.3\textwidth]{fig-input.png}\label{fig:input}}
  \hfill
  \subfloat[Ground truth]{\includegraphics[width=0.3\textwidth]{fig-gt.png}\label{fig:gt}}
  \hfill
  \subfloat[Ours result]{\includegraphics[width=0.3\textwidth]{fig-ours.png}\label{fig:ours}}
  \caption{Qualitative comparison on the test set.}
  \label{fig:qual}
\end{figure}

% ---- 跨页超长 equation ----
\begin{equation}
\mathcal{J}_{\text{CVPR}} =
\underbrace{\frac{1}{|\mathcal{V}|}\sum_{v\in\mathcal{V}}
  \mathcal{L}_{\text{mse}}\bigl(\hat{I}_v, I_v^\star\bigr)}_{\text{reconstruction}}
+ \lambda_1\underbrace{\sum_{(u,v)\in\mathcal{E}}
  \bigl\|\nabla\phi(\hat{I}_u)-\nabla\phi(\hat{I}_v)\bigr\|^2}_{\text{smoothness}}
+ \lambda_2\underbrace{\sum_{v\in\mathcal{V}}
  \mathcal{R}\bigl(\hat{I}_v\bigr)}_{\text{regularization}} \label{eq:cvpr-energy}
\end{equation}

% ---- 横向大表 ----
\begin{table}[t]
\centering
\caption{Quantitative results on COCO-Stuff validation set.}
\label{tab:quant}
\resizebox{\linewidth}{!}{
\begin{tabular}{l|rrrr|rrrr}
  \toprule
  \multirow{2}{*}{Method} & \multicolumn{4}{c|}{mIoU$\uparrow$} &
                             \multicolumn{4}{c}{FPS$\uparrow$} \\
  \cmidrule{2-5} \cmidrule{6-9}
  & S & M & L & Avg & S & M & L & Avg \\
  \midrule
  DeepLabV3+ & 31.2 & 41.6 & 54.8 & 42.5 & 8.1 & 8.1 & 8.1 & 8.1 \\
  \textbf{Ours} & \textbf{33.7} & \textbf{44.2} & \textbf{57.1} & \textbf{45.0} &
                 \textbf{28.4} & \textbf{28.4} & \textbf{28.4} & \textbf{28.4} \\
  \bottomrule
\end{tabular}
}
\end{table}
```

**质量重点**：
- `\subfloat` 转换为 Word 独立图片 + 统一 caption 的能力。
- `\multirow` 在 `\resizebox{\linewidth}{!}{...}` 中的宽度自适应。
- 大数字引用集合（15+ 条引用）的批量解析。

---

### Corpus #3 — `corpus-03-acm-sig`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-03-acm-sig` |
| **名称** | ACM SIG Conference (acmart) |
| **Profile** | `acm-sig` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：验证 ACM `acmart` 文档类特有的 CCS 概念树、复杂机构脚注、DOI 条目引用。

**文件**：`main.tex` + `refs.bib`

**核心 LaTeX 内容**：

```latex
\documentclass[sigconf,anonymous,review]{acmart}
\settopmatter{printacmref=true}
\usepackage{booktabs}
\usepackage{amsmath}
\usepackage{graphicx}
\usepackage[utf8]{inputenc}

\title{CCS Concepts for LaTeX-to-DOCX Conversion Benchmark}

\author{First Author}
\affiliation{\institution{Example University}
  \city{City} \country{Country}}
\author{Second Author}
\affiliation{\institution{Another Institute}
  \city{City} \country{Country}}

% ---- ACM CCS 概念树 ----
\beginCCS
\begin{textblock}{0.5}(-1.0,-0.5)
\ccsdesc[500]{Software and its engineering~Software libraries}
\ccsdesc[300]{Human-centered computing~Visualization techniques}
\end{textblock}
\endCCS

% ---- ACM metadata note ----
\iffalse
ACM keywords: software engineering, visualization, tool support
\fi

\begin{document}
\maketitle

\section{Introduction}
ACM SIG papers require careful handling of CCS concepts, author metadata,
and institution formatting~\citeauthor{lamport1994latex} have documented this~\cite{lamport1994latex}.

% ---- 复杂多行表格 ----
\begin{table}[t]
\caption{API usage comparison across three visualization libraries.}
\label{tab:api}
\begin{tabular}{l|p{4.5cm}|p{3.5cm}}
  \toprule
  Library & Typical Call Pattern & Extensibility \\
  \midrule
  D3.js & \texttt{d3.select().data().enter().append()} & High (JS) \\
  Vega-Lite & \texttt{vl.markBar().encode(x,y).run()} & Medium (JSON) \\
  Observable Plot & \texttt{Plot.plot(\{marks: [...] \})} & Medium (JS) \\
  \bottomrule
\end{tabular}
\end{table}

\bibliographystyle{ACM-Reference-Format}
\bibliography{refs}
\end{document}
```

---

### Corpus #4 — `corpus-04-jos-chinese`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-04-jos-chinese` |
| **名称** | 软件学报中文期刊 (rjthesis) |
| **Profile** | `jos-paper` |
| **Tier** | `golden` |
| **类型** | 合成（参考 paper3 风格） |

**目的**：验证中文 CJK 排版、双语摘要、图表双语标题、作者简介、附中文参考文献等JOS特色元素。

**文件**：`main.tex` + `refs.bib` + `references-zh.tex` + `figures/`（4 图）

**核心 LaTeX 内容**：

```latex
\documentclass{rjthesis}
\geometry{paperwidth=18.40cm,paperheight=26.00cm,
  left=1.45cm,right=1.45cm,top=1.00cm,bottom=2.20cm}
\usepackage{graphicx}
\graphicspath{{./figures/}}
\usepackage{amsmath}
\usepackage{amsthm}
\newtheorem{definition}{定义}
\newtheorem{property}{性质}
\usepackage{booktabs}
\usepackage[super,square,sort&compress]{natbib}
\usepackage{hyperref}
\hypersetup{colorlinks=true,linkcolor=blue,citecolor=blue}
\usepackage[ruled,vlined,linesnumbered]{algorithm2e}

% ---- 元数据 ----
\rjtitle{基于深度学习的文档智能转换方法研究}
\rjauthor{张三, 李四}
\rjinfor{（某某大学计算机科学与技术系，某某 123456）\\
通讯作者: 张三, E-mail: zhangsan@example.edu.cn}

\begin{document}
\rjmakenobibintoc
\rjmaketitle

% ---- 双语摘要 ----
\begin{rjabstract}
\AbstractContentZh
\end{rjabstract}
\rjkeywords{文档转换; 深度学习; 格式分析; 双向对齐}
\AbstractENContent
{This paper proposes a deep learning based method for intelligent document
conversion. We introduce a bidirectional alignment mechanism to preserve
layout semantics during LaTeX-to-DOCX transformation.}
\rjkeywordsEn{Document Conversion; Deep Learning; Format Analysis; Bidirectional Alignment}

% ---- 图表双语 caption ----
\begin{figure}[ht]
  \centering
  \includegraphics[width=.7\textwidth]{fig-system-overview.pdf}
  \bicaption{系统总体架构}{System Overall Architecture}
  \label{fig:system}
\end{figure}

\begin{table}[ht]
\centering
\bicaption{数据集统计}{Dataset Statistics}
\label{tab:stats}
\begin{tabular}{lccc}
  \toprule
  数据集 & 训练集 & 验证集 & 测试集 \\
  \midrule
  JOS-1000 & 800 & 100 & 100 \\
  IEEE-500  & 400 & 50  & 50  \\
  \bottomrule
\end{tabular}
\end{table}

% ---- 定理环境 ----
\begin{definition}
\label{def:alignment}
双向对齐（Bidirectional Alignment）指在 LaTeX→DOCX 转换过程中，
源端与目标端语义块之间的双向映射关系，满足对称性：
$\mathcal{A} = \mathcal{A}^{-1}$。
\end{definition}

% ---- 算法环境 ----
\begin{algorithm}[H]
  \caption{双向对齐算法}
  \LinesNumbered
  \KwIn{源节点集合 $S$, 目标节点集合 $T$}
  \KwOut{对齐集合 $\mathcal{A}$}
  \BlankLine
  $\mathcal{A} \gets \emptyset$ \\
  \ForEach{$s \in S$}{
    $t^* \gets \arg\max_{t\in T} \text{Sim}(s,t)$ \\
    \If{$\text{Sim}(s,t^*) > \tau$}{
      $\mathcal{A} \gets \mathcal{A} \cup \{(s,t^*)\}$
    }
  }
  \Return $\mathcal{A}$
\end{algorithm}

% ---- 参考文献 ----
\bibliographystyle{unsrt}
\bibliography{refs}

% ---- 附中文参考文献 ----
\noindent{\hei 附中文参考文献:}
\input{references-zh.tex}

% ---- 作者简介 ----
\noindent{\textbf{作者简介:}
\begin{description}[font=\normalfont,labelwidth=2em,leftmargin=2.5em]
\item[{[张三]}] 博士，CCF 专业会员，主要研究方向为文档智能处理与机器学习。
\item[{[李四]}] 硕士，主要研究方向为自然语言处理。
\end{description}}
\end{document}
```

**质量重点**：
- `rjthesis` 特有命令：`\rjtitle`、`\rjmaketile`、`\rjkeywords`、`\bicaption`。
- 中文标点挤压与 CJK 字体 fallback。
- `algorithm2e`（与 algorithmicx 不同的算法格式）。
- 定理/定义环境的编号与引用。
- 附中文参考文献（纯文本格式，手工编写的中文 bib 列表）。

---

### Corpus #5 — `corpus-05-cs-algorithms`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-05-cs-algorithms` |
| **名称** | CS Algorithms — Code and Algorithm |
| **Profile** | `generic-article` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：验证 `listings`/`minted` 代码块（多语言高亮）、算法跨页、行号引用。

**文件**：`main.tex` + `refs.bib`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage[utf8]{inputenc}
\usepackage{amsmath}
\usepackage{graphicx}
\usepackage{algorithm}
\usepackage[noend]{algpseudocode}
\usepackage{listings}
\usepackage{minted}
\usepackage{xcolor}
\usepackage{booktabs}

% ---- listings 配置 ----
\lstset{
  language=Python,
  frame=tb,
  numbers=left,
  numberstyle=\tiny\color{gray},
  backgroundcolor=\color{white},
  keywordstyle=\color{blue},
  stringstyle=\color{orange},
  commentstyle=\color{green},
  breaklines=true
}

% ---- minted（需 Python pygments）----
\usemintedstyle{monokai}

% ---- 算法跨页 ----
\begin{algorithm}[p]
  \caption{Union-Find with Path Compression}
  \begin{algorithmic}[1]
    \Function{Find}{$x$}
      \If{$\text{parent}[x] \neq x$}
        \State $\text{parent}[x] \gets \Call{Find}{\text{parent}[x]}$
      \EndIf
      \State \Return $\text{parent}[x]$
    \EndFunction
    \Statex
    \Function{Union}{$x, y$}
      \State $p \gets \Call{Find}{x}$
      \State $q \gets \Call{Find}{y}$
      \If{$p \neq q$}
        \State $\text{parent}[q] \gets p$
      \EndIf
    \EndFunction
    \Statex
    % ... 填充到足够长以触发跨页 ...
    \For{$i \gets 1$ \textbf{to} $100$}
      \State $j \gets \Call{Random}{\,}$
      \State $k \gets \Call{Random}{\,}$
      \State $\Call{Union}{i, j}$ \Comment{simulate many operations}
    \EndFor
  \end{algorithmic}
\end{algorithm}

% ---- Python 代码块 ----
\begin{lstlisting}[language=Python, caption={K-means clustering implementation}]
import numpy as np

def kmeans(X: np.ndarray, k: int, max_iter: int = 100) -> tuple:
    centroids = X[:k]          # init: first k points
    labels = np.zeros(len(X), dtype=int)

    for _ in range(max_iter):
        # E-step: assign points to nearest centroid
        distances = np.linalg.norm(X[:, None] - centroids, axis=2)
        labels = np.argmin(distances, axis=1)

        # M-step: update centroids
        new_centroids = np.array([
            X[labels == i].mean(axis=0) if np.any(labels == i) else c
            for i, c in enumerate(centroids)
        ])

        if np.allclose(centroids, new_centroids):
            break
        centroids = new_centroids

    return centroids, labels
\end{lstlisting}

% ---- Rust 代码块（minted）----
\begin{minted}[caption={Graph adjacency list}, frame=lines]{rust}
fn dijkstra(graph: &[Vec<(usize, i32)>], start: usize) -> Vec<i32> {
    let n = graph.len();
    let mut dist = vec![i32::MAX; n];
    dist[start] = 0;
    let mut pq = std::collections::BinaryHeap::new();
    pq.push((0, start));

    while let Some((d, u)) = pq.pop() {
        if -d > dist[u] { continue; }
        for &(v, w) in &graph[u] {
            if dist[u] + w < dist[v] {
                dist[v] = dist[u] + w;
                pq.push((-dist[v], v));
            }
        }
    }
    dist
}
\end{minted}
```

**质量重点**：
- `listings` vs `minted` 两种代码高亮方式的降级策略（转为等宽文本 + 保留行号）。
- 算法跨页时的 caption 和 label 引用。
- 跨页浮动体（`[!p]` 强制分页浮动）中 figure/table/algorithm 的混排。

---

### Corpus #6 — `corpus-06-cs-database`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-06-cs-database` |
| **名称** | CS Database and Systems |
| **Profile** | `generic-article` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：验证 TikZ 绘制实体关系图（ER 图）、关系代数符号、自定义宏嵌套。

**文件**：`main.tex` + `refs.bib` + `figures/`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage{amsmath}
\usepackage{graphicx}
\usepackage{tikz}
\usetikzlibrary{shapes.geometric,arrows,positioning,calc}
\usepackage{booktabs}
\usepackage{hyperref}

% ---- 自定义宏（深度嵌套）----
\newcommand{\Rel}[1]{\text{Rel}(#1)}
\newcommand{\Proj}[2]{\pi_{#1}\left(#2\right)}
\newcommand{\Select}[3]{\sigma_{#1}\left(#2,#3\right)}
\newcommand{\Join}[2]{{#1}\bowtie{#2}}
\newcommand{\semijoin}[2]{{#1}\ltimes{#2}}

% ---- TikZ ER 图 ----
\begin{figure}[ht]
  \centering
  \begin{tikzpicture}[
    node distance=2.5cm,
    entity/.style={rectangle,draw,fill=blue!10,minimum width=2.5cm},
    attr/.style={ellipse,draw,fill=gray!10,minimum width=1.8cm},
    rel/.style={diamond,draw,fill=green!10,minimum width=2cm,aspect=2}
  ]
    \node[entity] (student) {Student};
    \node[attr,above of=student] (sid) {\underline{ID}};
    \node[attr,left of=sid] (sname) {Name};
    \node[attr,right of=sid] (smajor) {Major};
    \draw (sid) -- (student);
    \draw (sname) -- (sid);
    \draw (smajor) -- (sid);

    \node[entity,right=3cm of student] (course) {Course};
    \node[attr,above of=course] (cid) {\underline{Code}};
    \node[attr,right of=cid] (ctitle) {Title};
    \draw (cid) -- (course);
    \draw (ctitle) -- (cid);

    \node[rel,right=1.5cm of student] (enroll) {Enroll};
    \node[attr,below of=enroll] (grade) {Grade};

    \draw (student) -- node[above] {N} (enroll);
    \draw (course)  -- node[above] {M} (enroll);
    \draw (enroll)  -- (grade);
  \end{tikzpicture}
  \caption{Entity-Relationship diagram for a university database.}
  \label{fig:er}
\end{figure}

% ---- 关系代数公式（大量自定义宏）----
\begin{align}
  &\Proj{\text{Name},\text{Grade}}(
    \Select{\text{Grade} > 3.5}{\sigma}{\Join{\sigma}{\text{Student}}}
  ) \label{eq:ra} \\
  &\semijoin{\Rel{\text{Enroll}}}{\Rel{\text{Course}}}
    \:=\:
    \Proj{\text{SID}}{\Rel{\text{Enroll}}}
    \;\cap\;
    \Proj{\text{SID}}{\Join{\Rel{\text{Enroll}}}{\Rel{\text{Course}}}} \label{eq:semijoin}
\end{align}
```

---

### Corpus #7 — `corpus-07-arxiv-math`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-07-arxiv-math` |
| **名称** | ArXiv Math — amsart with Theorems |
| **Profile** | `generic-article` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：验证 `amsthm` 海量定理/引理/证明环境、`\DeclareMathOperator`、多行证明。

**文件**：`main.tex` + `refs.bib`

**核心 LaTeX 内容**：

```latex
\documentclass[reqno]{amsart}
\usepackage{amsmath}
\usepackage{amsthm}
\usepackage{amssymb}
\usepackage{graphicx}
\usepackage{hyperref}

% ---- 自定义数学操作符 ----
\DeclareMathOperator{\dist}{dist}
\DeclareMathOperator{\vol}{vol}
\DeclareMathOperator{\supp}{supp}
\DeclareMathOperator*{\argmin}{arg\,min}
\DeclareMathOperator*{\argmax}{arg\,max}

% ---- 定理结构 ----
\newtheoremstyle{break}%
  {9pt}{9pt}{\itshape}{}%
  {\bfseries}{.}{\newline}{}

\theoremstyle{plain}
\newtheorem{axiom}{Axiom}[section]
\newtheorem{conjecture}[axiom]{Conjecture}
\newtheorem{criterion}[axiom]{Criterion}
\newtheorem{ theorem}{Theorem}[section]
\newtheorem{case}{Case}
\newtheorem{claim}{Claim}
\newtheorem{conclusion}{Conclusion}
\newtheorem{condition}{Condition}
\newtheorem{convention}{Convention}
\newtheorem{criterion}{Criterion}

\theoremstyle{definition}
\newtheorem{definition}[theorem]{Definition}
\newtheorem{exercise}{Exercise}
\newtheorem{notation}[theorem]{Notation}
\newtheorem{problem}{Problem}
\newtheorem{question}{Question}

\theoremstyle{example}
\newtheorem{example}[theorem]{Example}

\theoremstyle{note}
\newtheorem{note}[theorem]{Note}
\newtheorem{summary}[theorem]{Summary}

\theoremstyle{rem}
\newtheorem*{remark}{Remark}
\newtheorem*{remarks}{Remarks}

\theoremstyle{Proof}
\newtheorem*{proof}{Proof}

% ---- 定理内容 ----
\begin{definition}[Gromov--Hausdorff distance]
\label{def:gh}
For two compact metric spaces $X$ and $Y$, the Gromov--Hausdorff distance
is defined as
\[
  d_{GH}(X,Y) \;=\; \inf_{Z,f,g} d_H^Z\bigl(f(X),g(Y)\bigr),
\]
where the infimum runs over all metric spaces $Z$ and isometric embeddings
$f:X\hookrightarrow Z$, $g:Y\hookrightarrow Z$, and $d_H^Z$ denotes the
Hausdorff distance in $Z$.
\end{definition}

\begin{case}
Assume $p > 2$. Then by H\"older's inequality we have
\[
  \Bigl\|\sum_{i=1}^n a_i\Bigr\|_p^p
  \;\le\; n^{p-1}\sum_{i=1}^n \|a_i\|_p^p .
\]
\end{case}

\begin{claim}
The sequence $\{x_k\}$ converges to $x^\star$ in the $\|\cdot\|_\infty$ norm.
\end{claim}
\begin{claim*}
We claim further that the convergence rate is geometric:
$\|x_k - x^\star\|_\infty \le C\lambda^k$ for some $\lambda\in(0,1)$.
\end{claim*}

\begin{problem}
Let $f:\mathbb{R}^n\to\mathbb{R}$ be convex and $L$-smooth.
Show that gradient descent with step size $1/L$ satisfies
$f(x_{k+1}) \le f(x_k) - \frac{1}{2L}\|\nabla f(x_k)\|^2$.
\end{problem}

% ---- 证明中的多行对齐 ----
\begin{Proof}
We expand the left-hand side:
\[
  f(x_k - \tfrac{1}{L}\nabla f(x_k))
  \;\le\; f(x_k) - \tfrac{1}{L}\nabla f(x_k)^\top\nabla f(x_k)
              + \tfrac{L}{2}\Bigl\|\tfrac{1}{L}\nabla f(x_k)\Bigr\|^2
  \;=\; f(x_k) - \tfrac{1}{2L}\|\nabla f(x_k)\|^2 .
\]
\Relineqed
\end{Proof}
```

**质量重点**：
- 10+ 种 theorem 类环境的识别与渲染（定理、引理、命题、推论、定义等）。
- `proof` 环境中 `QED` 符号（`\QED`, `\qedsymbol`）的正确处理。
- `amsthm` 的 `\$...\$` 格式和 equation 环境的混用。
- `\DeclareMathOperator` 的下标格式（`\!` 和 `\,` 修饰符）。
- `\mathbb`, `\mathcal`, `\mathfrak` 等字体的渲染。

---

### Corpus #8 — `corpus-08-prl-physics`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-08-prl-physics` |
| **名称** | APS PRL — revtex4-2 Physics |
| **Profile** | `aps-prl` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：验证 APS `revtex4-2` 双栏物理排版、`align`/`gather`/`multline` 多行公式、物理单位、矩阵。

**文件**：`main.tex` + `refs.bib` + `figures/`

**核心 LaTeX 内容**：

```latex
%\pdfoutput=1
\documentclass[aps,prl,twocolumn,showkeys,grouped]{revtex4-2}
\usepackage{graphicx}
\usepackage{amsmath}
\usepackage{amssymb}
\usepackage{physics}
\usepackage{mhchem}

% ---- APS 特有元数据 ----
\draft
\preprint{PRL-2026-XXXXX}
\selectlanguage{english}

\title{Quantum Entanglement in Topological Materials}

\begin{abstract}
We report the first experimental observation of long-range entanglement...
\end{abstract}

% ---- gather 多行 ----
\begin{equation}
\begin{gather}
  \rho(t) = \rho_0 - i\hbar^{-1}[H,\rho_0]t
            -\frac{\hbar^{-2}}{2!}[H,[H,\rho_0]]t^2
            +\cdots \label{eq:master} \\
  \frac{d\rho}{dt} = -\frac{i}{\hbar}[H,\rho] + \sum_{k}\Bigl(
    L_k\rho L_k^\dagger - \tfrac{1}{2}\{L_k^\dagger L_k,\rho\}\Bigr) \label{eq:lindblad}
\end{gather}
\end{equation}

% ---- multline ----
\begin{equation}
\begin{multline}
  S(\rho_{AB}) = -\tr(\rho_{AB}\log_2\rho_{AB})
               = -\tr_A\tr_B\bigl(\rho_{AB}(\log_2\rho_{AB})\bigr) \\
               \le -\tr_A\tr_B\bigl(\rho_{AB}(\log_2\rho_A\otimes\rho_B)\bigr)
               = S(\rho_A) + S(\rho_B) .
\end{multline}
\end{equation}

% ---- bmatrix 矩阵 ----
\begin{equation}
\mathbf{M} =
\begin{pmatrix}
  \alpha & \beta  & 0      & \cdots & 0      \\
  \beta^* & \gamma & \delta & \cdots & 0      \\
  0      & \delta^* & \epsilon & \ddots & \vdots \\
  \vdots & \vdots & \ddots & \ddots & \eta   \\
  0      & 0      & \cdots & \eta^* & \zeta
\end{pmatrix}
\in \mathbb{C}^{5\times 5} .
\end{equation}

% ---- 化学式 ----
The reaction dynamics is governed by
\ce{2H2 + O2 -> 2H2O} with energy release $E = 285.8\,\text{kJ/mol}$.
The rate constant follows the Arrhenius form:
\begin{equation}
  k(T) = A\exp\!\Bigl(-\frac{E_a}{RT}\Bigr) .
\end{equation}

% ---- 物理单位 ----
The measured transition frequency is
$\nu = 1.420\,405\,\text{GHz}$ with uncertainty
$\delta\nu = 0.001\,\text{MHz}$.
The correlation length is $\xi = 2.4 \pm 0.3\,\mu\text{m}$.
\end{document}
```

---

### Corpus #9 — `corpus-09-math-edgecases`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-09-math-edgecases` |
| **名称** | Math Equation Edge Cases |
| **Profile** | `generic-article` |
| **Tier** | `golden` |
| **类型** | 合成（专注极限公式） |

**目的**：极限公式场景测试：超长矩阵、`cases` 分段函数、`tikz-cd` 交换图、`split` 跨行公式。

**文件**：`main.tex`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage{amsmath}
\usepackage{amssymb}
\usepackage{mathtools}
\usepackage{tikz-cd}
\usepackage{cases}

% ---- 超长 bmatrix ----
\[
\mathbf{A}_{10\times 10} =
\begin{bmatrix}
  a_{11} & a_{12} & a_{13} & a_{14} & a_{15} & a_{16} & a_{17} & a_{18} & a_{19} & a_{110} \\
  a_{21} & a_{22} & a_{23} & a_{24} & a_{25} & a_{26} & a_{27} & a_{28} & a_{29} & a_{210} \\
  \vdots & \vdots & \vdots & \vdots & \vdots & \vdots & \vdots & \vdots & \vdots & \vdots \\
  a_{101} & a_{102} & a_{103} & a_{104} & a_{105} & a_{106} & a_{107} & a_{108} & a_{109} & a_{1010}
\end{bmatrix}
\]

% ---- cases 分段函数 ----
\[
f(x,y) =
\begin{cases}
  \displaystyle\frac{\sin(x^2+y^2)}{x^2+y^2} & \text{if } x^2+y^2 \neq 0, \\[8pt]
  1                                            & \text{if } x^2+y^2 = 0 .
\end{cases}
\]

% ---- tikz-cd 交换图 ----
\[
\begin{tikzcd}
  A \arrow[r, "f"] \arrow[d, "g"'] &
  B \arrow[d, "h"] \\
  C \arrow[r, "k"'] &
  D \arrow[lu, "\exists! m", dashed]
\end{tikzcd}
\]

% ---- numcases ----
\begin{numcases}{|x|=}
  x, & for $x \ge 0$ \\
  -x, & for $x < 0$
\end{numcases}

% ---- split 跨行公式 ----
\begin{equation}
\begin{split}
  \mathcal{L}(\theta)
   &= \sum_{i=1}^n \Bigl[
        y_i\log\sigma(\theta^\top x_i)
        + (1-y_i)\log\bigl(1-\sigma(\theta^\top x_i)\bigr)
      \Bigr] \\
   &= \sum_{i=1}^n \ell_i(\theta) \\
   &= -\mathcal{J}(\theta) .
\end{split}
\end{equation}
```

**质量重点**：
- `tikz-cd` 渲染为图片（当前引擎 fallback）或解析为 ASCII 图表。
- `numcases` 来自 `cases` 宏包，左侧花括号需要特殊处理。
- 10×10 大矩阵的排版（宽度超出单栏时触发 columnbreak）。
- `\dots`, `\vdots`, `\ddots` 的渲染。

---

### Corpus #10 — `corpus-10-physics-optics`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-10-physics-optics` |
| **名称** | Physics — Optics Report with wrapfig |
| **Profile** | `generic-article` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：验证 `wrapfig` 图文环绕、物理实验报告常用表格、tif/tiff 高分辨率图。

**文件**：`main.tex` + `refs.bib` + `figures/`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage{graphicx}
\usepackage{wrapfig}
\usepackage{booktabs}
\usepackage{amsmath}
\usepackage{floatrow}
\usepackage{siunitx}
\usepackage{chemfig}

\DeclareSIUnit\angstrom{\AA}
\sisetup{per-mode=symbol}

% ---- wrapfig 图文环绕 ----
\begin{wrapfigure}{R}{0.45\textwidth}
  \centering
  \includegraphics[width=0.43\textwidth]{fig-interference.pdf}
  \caption{Interference pattern observed at the detector plane.}
  \label{fig:interference}
\end{wrapfigure}

% ---- 大量文字触发图文环绕 ----
The intensity distribution on the screen follows from the Fresnel diffraction
integral. For a circular aperture of radius $a$, illuminated by a plane wave
of wavelength $\lambda$, the on-axis intensity is given by
\[
  I(0,z) = I_0 \left[1 - J_0\!\left(\frac{\pi a^2}{\lambda z}\right)\right]^2
         + \left[N_0\!\left(\frac{\pi a^2}{\lambda z}\right)\right]^2 ,
\]
where $J_0$ and $N_0$ are Bessel functions of the first and second kind.
The first minimum occurs when $\pi a^2/(\lambda z) \approx 2.405$, yielding
the well-known Rayleigh criterion for circular apertures.

\begin{wraptable}{L}{0.45\textwidth}
  \caption{Experimental parameters for the double-slit setup.}
  \label{tab:params}
  \begin{tabular}{lc}
    \toprule
    Parameter & Value \\
    \midrule
    Wavelength $\lambda$ & \SI{632.8}{\nano\meter} \\
    Slit separation $d$ & \SI{0.50}{\milli\meter} \\
    Slit width $a$ & \SI{0.10}{\milli\meter} \\
    Screen distance $L$ & \SI{2.00}{\meter} \\
    \bottomrule
  \end{tabular}
\end{wraptable}
```

---

### Corpus #11 — `corpus-11-nature-biology`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-11-nature-biology` |
| **名称** | Nature / Science — Biology Report |
| **Profile** | `nature` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：验证 `nature` 宏包、`biblatex`+`biber` 大型文献库、上标引用 `\textsuperscript`、组图拼接。

**文件**：`main.tex` + `main.bbl` + `figures/`

**核心 LaTeX 内容**：

```latex
\documentclass{nature}
\input{../_shared/nature-bibliography-setup.tex}

\title{Genomic Analysis of Drug Resistance in \textit{Mycobacterium tuberculosis}}

\begin{singlespace}
\begin{abst}
We performed whole-genome sequencing of 10,000 clinical isolates of
\textit{Mycobacterium tuberculosis} collected across 47 countries to
characterize the genetic basis of drug resistance. Our analysis
reveals 23 novel resistance mutations not previously described in
the WHO catalog, with significant implications for diagnostic
target selection~\supercite{mingala2024,peter2025}.
\end{abst}
\end{singlespace}

% ---- 组图（4 张子图拼接）----
\begin{figure}[ht]
  \centering
  \begin{minipage}[b]{0.45\textwidth}
    \centering
    \includegraphics[width=\textwidth]{fig-phylo-tree.pdf}
    (a) Phylogenetic tree
  \end{minipage}\hfill
  \begin{minipage}[b]{0.45\textwidth}
    \centering
    \includegraphics[width=\textwidth]{fig-resist-map.pdf}
    (b) Geographic distribution
  \end{minipage}
  \\
  \begin{minipage}[b]{0.45\textwidth}
    \centering
    \includegraphics[width=\textwidth]{fig-mutations.pdf}
    (c) Mutation frequency
  \end{minipage}\hfill
  \begin{minipage}[b]{0.45\textwidth}
    \centering
    \includegraphics[width=\textwidth]{fig-survival.pdf}
    (d) Patient survival curves
  \end{minipage}
  \caption{Comprehensive genomic analysis of drug-resistant \textit{M. tuberculosis}.
    (a) Maximum-likelihood phylogenetic tree of all isolates ($n=10{,}000$).
    (b) Global distribution of resistance genotypes. (c) Frequency of
    mutations across 13 drug targets. (d) Kaplan-Meier survival analysis
    stratified by genotype.}
  \label{fig:overview}
\end{figure}

% ---- biblatex 引用样式 ----
\section*{Results}
This work extends previous studies on bacterial genomics\textsuperscript{1,2}
and builds on the foundational bioinformatics tools developed by
Li et al.\textsuperscript{3} and the WHO working group on TB diagnostics\textsuperscript{4}.

\section*{Data availability}
Raw sequencing data are available at the European Nucleotide Archive under
accession numbers ERPXXXXXX. Processed variant calls and metadata are
available at \url{https://doi.org/10.xxxx/zenodo.xxxxxx}.

% ---- bbl 文件（biblatex 输出）----
\input{main.bbl}
\end{document}
```

**质量重点**：
- `nature` 的 `\begin{abst}...\end{abst}`（无 `\begin{abstract}`）特殊格式。
- `\textsuperscript{}` 批量引用（非 `\cite{}` 而是 `\supercite{}`）。
- `biblatex` 输出的 `.bbl` 文件（已编译的参考文献条目列表）。
- 4 张子图拼接成组图（通过 `\begin{minipage}` 手动并排）。
- `singlespace` 特殊间距环境。

---

### Corpus #12 — `corpus-12-elsevier-chem`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-12-elsevier-chem` |
| **名称** | Elsevier Chemistry — elsarticle + chemfig |
| **Profile** | `elsevier-chem` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：验证 `elsarticle` 模板、`chemfig` 化学结构式、`mhchem` 化学方程式、`siunitx` 单位。

**文件**：`main.tex` + `refs.bib` + `figures/`

**核心 LaTeX 内容**：

```latex
\documentclass[preprint,3p,times,twocolumn]{elsarticle}
\usepackage{graphicx}
\usepackage{amsmath}
\usepackage{booktabs}
\usepackage{siunitx}
\usepackage[version=4]{mhchem}
\usepackage{chemfig}
\usepackage{bpchem}
\usepackage[version=4]{mhchem}

\sisetup{
  detect-all,
  per-mode=symbol,
  inter-unit-product=\cdot
}

\begin{document}
\begin{frontmatter}
\title{Electrocatalytic Reduction of \ce{CO2} to Ethanol on Copper-Based Catalysts}
\author[inst1]{Wei Liu}
\author[inst1,inst2]{Sarah Chen}
\affiliation[inst1]{Department of Chemistry, Example University}
\affiliation[inst2]{National Laboratory for Chemical Sciences}

% ---- 化学结构式 ----
\begin{figure}[ht]
  \centering
  \schemestart
  \chemfig{[2,3]CH_2=CH-CH_2-OH}
  \+
  \chemfig{CO_2}
  \arrow{->}
  \chemname{\chemfig{CH_3-CH_2-CH_2-OH}}{\textit{n}-propanol}
  \+
  \chemname{\chemfig{CH_3-CH_2-OH}}{\ethanol}
  \schemestop
  \caption{Electrocatalytic reduction pathway of \ce{CO2} to \ce{C2} products
    on Cu(100) surfaces. The mechanism proceeds via a nine-electron transfer
    cascade with intermediate *\ce{CO} binding energies determining selectivity.}
  \label{fig:co2-pathway}
\end{figure}

% ---- mhchem 化学方程式 ----
The overall reaction on the copper cathode is:
\begin{equation}
  \ce{2 CO2 + 12 H+ + 12 e- -> CH3CH2OH + 3 H2O}
  \qquad E^\circ = -0.08\ \text{V vs. RHE} \label{eq:co2}
\end{equation}
For comparison, the competing hydrogen evolution reaction:
\begin{equation}
  \ce{2 H2O + 2 e- -> H2 + 2 OH-} \qquad E^\circ = -0.83\ \text{V vs. RHE} \label{eq:her}
\end{equation}

% ---- siunitx 数值表格 ----
\begin{table}[ht]
  \caption{Electrocatalytic performance metrics at \SI{0.8}{\volt} vs. RHE.}
  \label{tab:catalyst}
  \begin{tabular}{
    l
    S[table-format=2.1]
    S[table-format=2.2]
    S[table-format=2.3]
    S[table-format=1.1]
  }
    \toprule
    Catalyst & {FE$_{\text{EtOH}}$ / \%} & {j / \si{\mA\cm^{-2}}} & {TOF / \si{\s^{-1}} $10^{-3}$} & {Stability / h} \\
    \midrule
    Cu foil       & 11.2 & 0.34 & 2.1 & 8.2 \\
    Cu NPs        & 23.5 & 0.71 & 4.8 & 12.1 \\
    Cu\@Ag 10\%   & 31.8 & 0.96 & 6.2 & 18.4 \\
    Cu\@Ag 25\%   & 28.4 & 0.86 & 5.5 & 15.3 \\
    \textbf{Cu\@Ag 15\%} & \textbf{38.2} & \textbf{1.16} & \textbf{7.3} & \textbf{22.1} \\
    \bottomrule
  \end{tabular}
\end{table}
```

**质量重点**：
- `chemfig` 结构式渲染为图片（fallback）。
- `mhchem` `\ce{...}` 语法的降级文本表示。
- `bpchem`（生物过程化学宏包）的兼容。
- `S` 列格式（`siunitx`）在表格中的正确处理。
- `elsarticle` 的 `\begin{frontmatter}...\end{frontmatter}` 结构。

---

### Corpus #13 — `corpus-13-bioinformatics`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-13-bioinformatics` |
| **名称** | Bioinformatics — longtable / landscape |
| **Profile** | `generic-article` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：验证 `longtable` 跨页大表、`lscape`/`rotating` 横向表格、`array` 宏包复杂列格式。

**文件**：`main.tex` + `data.csv`（嵌入 LaTeX）

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage[margin=1in]{geometry}
\usepackage{graphicx}
\usepackage{longtable}
\usepackage{booktabs}
\usepackage{array}
\usepackage{rotating}
\usepackage{amsmath}
\usepackage{multicol}
\usepackage{multirow}

% ---- 横向大表（landscape + longtable）----
\begin{sidewaystable}
\begin{center}
\caption{Complete gene expression profiling across 48 tissue samples
  (RNA-seq, TPM values, $n=3$ biological replicates).}
\label{tab:gene-expression}
\begin{longtable}{
  l
  *{12}{S[table-format=4.1]}
}
  \toprule
  \multirow{2}{*}{Gene ID} &
    \multicolumn{3}{c}{Brain} &
    \multicolumn{3}{c}{Liver} &
    \multicolumn{3}{c}{Heart} &
    \multicolumn{3}{c}{Lung} &
    \multirow{2}{*}{Mean} \\
  \cmidrule{2-4}\cmidrule{5-7}\cmidrule{8-10}\cmidrule{11-13}
  &
  \multicolumn{1}{c}{R1} &
  \multicolumn{1}{c}{R2} &
  \multicolumn{1}{c}{R3} &
  \multicolumn{1}{c}{R1} &
  \multicolumn{1}{c}{R2} &
  \multicolumn{1}{c}{R3} &
  \multicolumn{1}{c}{R1} &
  \multicolumn{1}{c}{R2} &
  \multicolumn{1}{c}{R3} &
  \multicolumn{1}{c}{R1} &
  \multicolumn{1}{c}{R2} &
  \multicolumn{1}{c}{R3} &
  \\
  \midrule
  \endfirsthead
  \midrule
  \multirow{2}{*}{Gene ID} &
    \multicolumn{3}{c}{Brain} &
    \multicolumn{3}{c}{Liver} &
    \multicolumn{3}{c}{Heart} &
    \multicolumn{3}{c}{Lung} &
    \multirow{2}{*}{Mean} \\
  \cmidrule{2-4}\cmidrule{5-7}\cmidrule{8-10}\cmidrule{11-13}
  &
  \multicolumn{1}{c}{R1} &
  \multicolumn{1}{c}{R2} &
  \multicolumn{1}{c}{R3} &
  \multicolumn{1}{c}{R1} &
  \multicolumn{1}{c}{R2} &
  \multicolumn{1}{c}{R3} &
  \multicolumn{1}{c}{R1} &
  \multicolumn{1}{c}{R2} &
  \multicolumn{1}{c}{R3} &
  \multicolumn{1}{c}{R1} &
  \multicolumn{1}{c}{R2} &
  \multicolumn{1}{c}{R3} &
  \\
  \midrule
  \endhead
  \midrule
  \multicolumn{14}{r}{{Continued on next page}} \\
  \endfoot
  \bottomrule
  \endlastfoot
  % 数据行（填充到 20+ 行触发跨页）
  Gene\_001 & 1023.4 & 1051.2 &  998.7 &  12.3 &  14.1 &  11.8 &  45.2 &  43.1 &  46.7 &  78.9 &  75.4 &  81.2 & 389.0 \\
  Gene\_002 &  234.5 &  241.8 &  229.1 & 892.3 & 905.6 & 878.4 &  23.1 &  21.9 &  24.8 &  56.2 &  54.8 &  57.1 & 348.1 \\
  % ... (重复填充到 25+ 行)
  Gene\_025 &  567.8 &  573.2 &  561.4 & 456.7 & 462.1 & 449.8 & 321.5 & 315.8 & 327.9 & 234.6 & 229.1 & 240.3 & 411.1 \\
\end{longtable}
\end{center}
\end{sidewaystable}
```

---

### Corpus #14 — `corpus-14-econ-econometrica`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-14-econ-econometrica` |
| **名称** | Economics — Econometrica Style |
| **Profile** | `econ-econometrica` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：验证经济学期刊特有的 `threeparttable` 脚注表、显著性星号（`***`）、`booktabs` 回归分析表、`natbib` author-year 引用。

**文件**：`main.tex` + `refs.bib`

**核心 LaTeX 内容**：

```latex
\documentclass{aer}
\usepackage{amsmath}
\usepackage{booktabs}
\usepackage{threeparttable}
\usepackage{dcolumn}
\usepackage{graphicx}
\usepackage[authoryear,longnamesfirst]{natbib}

% ---- threeparttable（Econometrics 标准格式）----
\begin{table}[t]
\begin{center}
\begin{threeparttable}
\caption{OLS Regression: Effect of FDI on Regional Productivity}
\label{tab:ols}
\begin{tabular}{l*{3}{D{,}{}{-1}}}
  \toprule
  & \multicolumn{1}{c}{(1)} & \multicolumn{1}{c}{(2)} & \multicolumn{1}{c}{(3)} \\
  \cmidrule{2-4}
  & \multicolumn{1}{c}{Baseline} &
    \multicolumn{1}{c}{+Controls} &
    \multicolumn{1}{c}{+FE} \\
  \midrule
  $\ln$\textit{FDI stock} &
    0.284^{***} &
    0.231^{**} &
    0.187^{*} \\
  &
  (0.042) &
  (0.039) &
  (0.036) \\
  \addlinespace
  $\ln$\textit{Human capital} &
    &
    0.412^{***} &
    0.338^{***} \\
  &
  &
  (0.056) &
  (0.048) \\
  \addlinespace
  Constant &
    -2.134^{***} &
    -1.876^{***} &
    -1.542^{***} \\
  &
  (0.312) &
  (0.298) &
  (0.271) \\
  \midrule
  Observations &
    \multicolumn{1}{c}{1{,}248} &
    \multicolumn{1}{c}{1{,}248} &
    \multicolumn{1}{c}{1{,}248} \\
  $R^2$ &
    \multicolumn{1}{c}{0.34} &
    \multicolumn{1}{c}{0.48} &
    \multicolumn{1}{c}{0.61} \\
  \midrule
\end{tabular}
\begin{tablenotes}
  \small
  \item Standard errors in parentheses.
    $^{***}p<0.01$, $^{**}p<0.05$, $^{*}p<0.1$.
  \item Column (3) includes province and year fixed effects.
  \item Dependent variable: $\ln$(labor productivity).
\end{tablenotes}
\end{threeparttable}
\end{center}
\end{table}
```

**质量重点**：
- `threeparttable` 三段结构（表格+注释+标签）。
- `D{sep}{fmt}{pre}` 列格式（`dcolumn`）的数字对齐。
- 显著性星号（`***`, `**`, `*`）与括号内标准误的行对齐。
- `\addlinespace`（`booktabs`）的垂直间距。
- `aer`（American Economic Review）文档类特有元数据。

---

### Corpus #15 — `corpus-15-humanities-apa`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-15-humanities-apa` |
| **名称** | Humanities — APA 7th Edition |
| **Profile** | `apa-7` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：验证 APA 7 格式、`natbib` author-year 引用、挂行缩进文献列表、多级无编号标题。

**文件**：`main.tex` + `refs.bib`

**核心 LaTeX 内容**：

```latex
\documentclass[man,12pt,a4paper]{apa7}
\usepackage[utf8]{inputenc}
\usepackage{amsmath}
\usepackage{graphicx}
\usepackage{booktabs}
\usepackage[backend=biber,style=apa,sorting=nyt]{biblatex}
\usepackage{csquotes}

% ---- APA 特有的长摘要 ----
\begin{abstract}
This study examines the relationship between digital literacy and
academic achievement among university students in East Asia.
Drawing on data from 3,241 students across 12 universities in
China, Japan, and South Korea, we employ hierarchical multiple
regression and structural equation modeling to test our hypotheses.
Results indicate that digital information literacy fully mediates
the relationship between socioeconomic status and academic
achievement, explaining 34\% of the variance after controlling
for prior academic performance and demographic variables.
\end{abstract}

% ---- 多级无编号标题（APA 特色）----
\section{Research Questions}
This investigation addresses three research questions:
\begin{enumerate}
  \item What is the relationship between digital literacy and
    academic achievement among East Asian university students?
  \item Does socioeconomic status predict academic achievement,
    and if so, is this relationship mediated by digital literacy?
  \item Are there cross-cultural differences in the strength of
    this mediation effect?
\end{enumerate}

% ---- 统计结果（APA 格式）----
\subsection{Mediation Analysis}
The indirect effect of SES on academic achievement through digital
literacy was significant, $b = 0.34$, $SE = 0.08$, $95\%$ CI $[0.19, 0.51]$,
accounting for 34\% of the total effect. This pattern of results
is consistent with the theoretical predictions of \citeauthor{bandura1997}
\citep{bandura1997} regarding self-efficacy as a mediator.

\printbibliography
\end{document}
```

---

### Corpus #16 — `corpus-16-linguistics-syntax`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-16-linguistics-syntax` |
| **名称** | Linguistics — Syntax Trees |
| **Profile** | `generic-article` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：验证 `tikz-qtree` / `forest` 句法树、`gb4e` 语料标注环境、多语言引例。

**文件**：`main.tex` + `refs.bib` + `figures/`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage{amsmath}
\usepackage{graphicx}
\usepackage{tikz-qtree}
\usepackage{forest}
\usepackage{gb4e}
\noautomath

% ---- 句法树（tikz-qtree）----
\begin{figure}[ht]
  \centering
  \begin{tikzpicture}[scale=0.85]
    \Tree
    [.IP
      [.NP [.N 游客 ] ]
      [.I$'$ [.I \node{会};] [.VP [.V 参观 ] [.NP [.D 那座 ] [.N 大楼 ] ] ] ]
    ]
  \end{tikzpicture}
  \qquad
  \Tree
  [.S
    [.NP The tourists ]
    [.VP will visit [.NP that building ] ]
  ]
  \caption{Comparative syntactic trees: (a) Chinese SVO order;
    (b) English SVO order.}
  \label{fig:syntree}
\end{figure}

% ---- forest 复杂树 ----
\begin{exe}
\ex\label{ex:binding}
\begin{exe}
  \ex\label{ex:binding-a}
  \ag
  \node{John}$_i$ thinks that Mary likes himself$_{i/k}$.
  \zg
  (a) In coreference reading, \emph{himself} can refer to \emph{John}
      but not to \emph{Mary}.
\end{exe}
\end{exe}

% ---- gb4e 编号引例 ----
\begin{exe}
\ex\label{ex:morphology}
\xlist
\ex\label{ex:morph-a}
\gll John-apu=lla=ka    q'entu-Ø        mikhun-Ø\\
      John-\Stem{day}=\Attr{ABL}=\Attr{NOM}  mountain-\Attr{NOM} see-\Attr{NFIN}\\
\glt `John sees the mountain from a day's journey away.'
\ex\label{ex:morph-b}
\gll Maria=paq=ta    ayllu=man       ri-Ø-nqa\\
     Maria-\Stem{go.away}={\Dest}=3\Attr{SG.FUT}\\
\glt `Maria will go to the community.'
\xlist
\end{exe}

% ---- 语料库格式 ----
\begin{commentary}
Table~\ref{tab:corpus-stats} summarizes the corpus statistics:
\end{commentary}
```

**质量重点**：
- `forest`/`tikz-qtree` 句法树渲染为图片。
- `gb4e` 的 `\gll...\glt` 双行对照格式（语音学/语言学标准格式）。
- `\xlist` / `\zl` / `\zg` 引例编号系统。
- `\noautomath` 防止自动转义对语言学符号的干扰。

---

### Corpus #17 — `corpus-17-table-stress`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-17-table-stress` |
| **名称** | Table Stress Test |
| **Profile** | `generic-article` |
| **Tier** | `smoke` |
| **类型** | 合成（压力测试） |

**目的**：极限表格测试，涵盖所有 `tabular` 变体。

**文件**：`main.tex`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage[margin=1in]{geometry}
\usepackage{graphicx}
\usepackage{array}
\usepackage{booktabs}
\usepackage{multirow}
\usepackage{tabularx}
\usepackage{tabulary}
\usepackage{makecell}
\usepackage{longtable}
\usepackage{collcell}
\usepackage{rotating}

% ---- tabularx（自动宽度列）----
\begin{table}[ht]
\caption{tabularx: paragraph-width column with automatic line breaking}
\label{tab:tabularx}
\begin{tabularx}{\linewidth}{lXc}
  \toprule
  ID & \textbf{Description} & Value \\
  \midrule
  Exp-001 & This is a very long description that needs to wrap
    across multiple lines within the X-type column in a tabularx
    environment, demonstrating automatic text wrapping. &
    42.5 \\
  Exp-002 & Short & 13.7 \\
  Exp-003 & Another lengthy description covering the full width
    of the available column space, with more detailed explanation. &
    87.3 \\
  \bottomrule
\end{tabularx}
\end{table}

% ---- tabulary（自动高度列）----
\begin{table}[ht]
\caption{tabulary: automatic height adjustment}
\label{tab:tabulary}
\begin{tabulary}{\linewidth}{LCC}
  \toprule
  Method & {Complexity} & Accuracy \\
  \midrule
  Algorithm A & $O(n^2)$ very long description here &
    \multirow{2}{*}{98.5\%} \\
  Algorithm B & $O(n\log n)$ &  \\
  \bottomrule
\end{tabulary}
\end{table}

% ---- 单元格内嵌图片 ----
\begin{table}[ht]
\caption{Cell containing minipage with image}
\label{tab:cell-image}
\begin{tabular}{cp{3cm}c}
  \toprule
  Item & Figure & Score \\
  \midrule
  Sample A &
  \begin{minipage}[c]{3cm}
    \centering
    \includegraphics[width=2.8cm]{fig-sample-a.pdf}
  \end{minipage} &
  0.94 \\
  \bottomrule
\end{tabular}
\end{table}

% ---- 单元格内嵌列表 ----
\begin{table}[ht]
\caption{Cell containing itemize}
\label{tab:cell-list}
\begin{tabular}{lp{5cm}}
  \toprule
  Model & \textbf{Features} \\
  \midrule
  BERT &
  \begin{minipage}[t]{5cm}
    \begin{itemize}
      \item Masked language model
      \item Deep bidirectional
      \item 12 layers, 768 hidden
    \end{itemize}
  \end{minipage} \\
  GPT-3 &
  \begin{minipage}[t]{5cm}
    \begin{itemize}
      \item Autoregressive
      \item In-context learning
      \item 96 layers, 12288 hidden
    \end{itemize}
  \end{minipage} \\
  \bottomrule
\end{tabular}
\end{table}

% ---- \multicolumn + \cline ----
\begin{table}[ht]
\caption{Multicolumn with custom borders}
\label{tab:multicol}
\begin{tabular}{l|c|r|}
  \cline{2-3}
  \multicolumn{1}{c|}{} &
  \multicolumn{2}{c|}{\textbf{Performance}} \\
  \cline{2-3}
  \multicolumn{1}{c|}{\textbf{Group}} &
  \textbf{Precision} &
  \textbf{Recall} \\
  \hline
  Control & 0.72 & 0.68 \\
  Treatment A & 0.81 & 0.79 \\
  Treatment B & 0.77 & 0.74 \\
  \hline
\end{tabular}
\end{table}
```

**质量重点**：
- `tabularx` 的 `X` 列自动宽度计算。
- `tabulary` 的 `L/C/J` 列自动高度。
- `\multirow` 单元格跨行（需外部图片支持 `multirow`）。
- 单元格内嵌 `minipage`（含图片/列表）。
- `\cline`, `\multicolum` 的不规则边框组合。

---

### Corpus #18 — `corpus-18-figure-stress`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-18-figure-stress` |
| **名称** | Figure Stress Test |
| **Profile** | `generic-article` |
| **Tier** | `smoke` |
| **类型** | 合成 |

**目的**：浮动体极限测试。

**文件**：`main.tex` + `figures/`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage{graphicx}
\usepackage{subcaption}
\usepackage{float}
\usepackage{minipage}
\usepackage{picinpar}

% ---- 极端浮动体位置指令 ----
\begin{figure}[H]
  \centering
  \includegraphics[width=0.9\textwidth,height=0.3\textheight,keepaspectratio]
    {fig-wide.pdf}
  \caption{Force placement with [H] option. This figure is placed
    exactly where it appears in the source.}
  \label{fig:forced}
\end{figure}

% ---- subcaption 子图 ----
\begin{figure}[ht]
  \centering
  \begin{subcaptiongroup}
  \subcaptionpanel[0.3\textwidth]{Sub-panel A}\label{fig:sp-a}
  \subcaptionpanel[0.3\textwidth]{Sub-panel B}\label{fig:sp-b}
  \subcaptionpanel[0.3\textwidth]{Sub-panel C}\label{fig:sp-c}
  \end{subcaptiongroup}
  \caption{Three panels placed with subcaptiongroup.}
  \label{fig:three-panels}
\end{figure}

% ---- minipage 并排图 ----
\begin{figure}[ht]
  \centering
  \begin{minipage}[t]{0.48\textwidth}
    \centering
    \includegraphics[width=\textwidth]{fig-left.pdf}
    \caption{Left panel with longer caption text that
      needs to wrap across multiple lines in the figure environment.}
    \label{fig:left}
  \end{minipage}
  \hfill
  \begin{minipage}[t]{0.48\textwidth}
    \centering
    \includegraphics[width=\textwidth]{fig-right.pdf}
    \caption{Right panel.}
    \label{fig:right}
  \end{minipage}
  \caption{Comparison of left and right experimental conditions.}
  \label{fig:comparison}
\end{figure}

% ---- 超长 caption ----
\begin{figure}[p]
  \centering
  \includegraphics[height=0.9\textheight,width=.8\textwidth,keepaspectratio]
    {fig-fullpage.pdf}
  \caption{This is an extremely long figure caption that goes on for
    many sentences to test how the DOCX renderer handles figure captions
    that exceed typical length expectations. It should be preserved as-is
    without truncation. The figure itself is placed on a dedicated page
    using the [p] float specifier.}
  \label{fig:long-caption}
\end{figure}
```

---

### Corpus #19 — `corpus-19-ref-bibtex-complex`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-19-ref-bibtex-complex` |
| **名称** | BibTeX Complex References |
| **Profile** | `generic-article` |
| **Tier** | `smoke` |
| **类型** | 合成 |

**目的**：BibTeX 极限测试——多种文献类型、多语言人名、特殊字符。

**文件**：`main.tex` + `refs.bib`

**核心 LaTeX 内容**（refs.bib 中覆盖所有 BibTeX 条目类型）：

```bibtex
@article{art1,
  author  = { van der Waals, J. D. and Plessix, B. Y. and O'Connor, T. J. and Müller, K.},
  title   = {A Survey on Graph Neural Networks for Code Analysis},
  journal = {IEEE Transactions on Software Engineering},
  volume  = {50},
  number  = {2},
  pages   = {342--391},
  year    = {2024},
  doi     = {10.1109/TSE.2024.3352101},
  eprint  = {2401.12345},
  archiveprefix = {arXiv},
}

@book{book1,
  author    = {Kr\"uger, I. and Grützner, S. and Bj\"orn, N.},
  title     = {Foundations of Distributed Systems: From Theory to Practice},
  publisher = {Springer Nature},
  address   = {Berlin},
  year      = {2023},
  edition   = {3rd},
  isbn      = {978-3-031-23456-7},
}

@inproceedings{conf1,
  author    = {Zhang, W. and Liu, H. and Müller, K. and García-López, J. R.},
  title     = {Cross-lingual Transfer Learning for Low-Resource Languages},
  booktitle = {Proceedings of the 62nd Annual Meeting of the ACL},
  pages     = {11234--11249},
  year      = {2024},
  address   = {Bangkok, Thailand},
  publisher = {Association for Computational Linguistics},
  doi       = {10.18653/v1/2024.acl-long.892},
}

@techreport{tech1,
  author  = {Nguyen, T. H. and Sánchez-Hernández, A. and Ye, L. and Zhou, X.},
  title   = {Security Analysis of LLM-Based Code Generation},
  institution = {MIT CSAIL},
  year    = {2025},
  number  = {MIT-CSAIL-TR-2025-001},
  url     = {https://hdl.handle.net/1721.1/154321},
}

@misc{online1,
  author    = {Python Core Team},
  title     = {The Python Language Reference --- Version 3.12},
  year      = {2024},
  howpublished = {\url{https://docs.python.org/3.12/}},
  note      = {Accessed: 2026-01-15},
}

@phdthesis{phd1,
  author = {Gonçalvez, M. F. and Śmiatacz, M.},
  title  = {Adversarial Robustness of Vision-Language Models},
  school = {ETH Zürich},
  year   = {2025},
  doi    = {10.3929/ethz-b-000654321},
}
```

**引用用法**：

```latex
% 各种引用方式
The survey by \citeauthor{art1} \citep{art1} covers GNNs extensively.
Figure 2 of \citet{conf1} shows the architecture.
Technical details are in \citep[Chapter 5]{book1}.
See also the online documentation \citep{online1}.
```

---

### Corpus #20 — `corpus-20-ref-biblatex-complex`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-20-ref-biblatex-complex` |
| **名称** | BibLaTeX Complex References |
| **Profile** | `generic-article` |
| **Tier** | `smoke` |
| **类型** | 合成 |

**目的**：`biblatex` 反向引用（`backref`）、分类文献列表（`printbibliography[type=article]`）、 `\citeauthor`/`\textcite`/`\parencite` 三种引用格式。

**文件**：`main.tex` + `refs.bib`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage[backend=biber,style=authoryear,
  isbn=true,url=true,doi=true,
  dashed=false,maxbibnames=99,
  date=year,urldate=long]{biblatex}
\usepackage{csquotes}
\usepackage{hyperref}
\usepackage{nameref}

\addbibresource{refs.bib}

% ---- 按类型分组的文献列表 ----
\printbibheading[title={References by Type}]
\printbibliography[type=article,heading=subbibliography,
  title={Journal Articles}]
\printbibliography[type=inproceedings,heading=subbibliography,
  title={Conference Papers}]
\printbibliography[type=book,heading=subbibliography,
  title={Books}]
\printbibliography[type=online,heading=subbibliography,
  title={Online Resources}]

% ---- 正文中的各种引用格式 ----
\parencite{smith2024foundation}      % (Smith, 2024)
\textcite{smith2024foundation}       % Smith (2024)
\citeauthor{smith2024foundation}     % Smith
\supercite{smith2024foundation}      % 上标引用

% ---- 后向引用（backref）----
This is discussed in \cite[pp. 12--15]{smith2024foundation}.
```

**质量重点**：
- `biblatex` 的 `\printbibliography[heading=subbibliography]` 分组输出。
- `backref`（反向引用）功能的降级处理。
- `\parencite` vs `\textcite` vs `\supercite` 的格式区分。
- `biber` 后端（需要 `.bcf` 文件）。

---

### Corpus #21 — `corpus-21-macro-expansion`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-21-macro-expansion` |
| **名称** | Macro Expansion Stress Test |
| **Profile** | `generic-article` |
| **Tier** | `smoke` |
| **类型** | 合成 |

**目的**：深度宏展开、`\ifthenelse`、`\ExplSyntaxOn`（LaTeX3 语法）、自定义命令嵌套。

**文件**：`main.tex`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage{etoolbox}
\usepackage{xparse}
\usepackage{ifthen}
\usepackage{amsmath}
\usepackage{graphicx}

% ---- 深度嵌套宏 ----
\def\outermacro#1{%
  \def\inner@macro##1{%
    \def\innermost####1{#1(####1)}%
    \innermost{##1}%
  }%
  \inner@macro{#1}%
}

% ---- ifthenelse 逻辑 ----
\newcounter{testcounter}
\providecommand{\ifthenequals}[3]{\ifthenelse{\equal{#1}{#2}}{#3}{}}

% ---- LaTeX3 语法 ----
\ExplSyntaxOn
\NewDocumentCommand \tl_const:Nn { m m }
  { \tl_const:cN { g__#1__tl } #2 }
\tl_const:Nn \g_my_const_tl { 42 }

\NewDocumentEnvironment { myenv } { +b }
  { \centering \textbf{Environment~content:}~#1 }
  {}
\ExplSyntaxOff

% ---- \自行车宏（递归）----
\def\makelist#1{%
  \ifx\relax#1\relax\else
    \item #1%
    \expandafter\makelist
  \fi}

\begin{document}

% 宏展开结果渲染
\outermacro{Result}
\[
  f(x) = \ifthenequals{x}{0}{1}{\outermacro{x}}
\]
```

**质量重点**：
- `\ExplSyntaxOn`/`Off` 切换对空白处理的影响。
- `xparse` `\NewDocumentCommand` 的参数解析。
- 递归宏的展开边界（防止无限递归）。
- `etoolbox` 的 `\ifdefvoid` 等条件测试。

---

### Corpus #22 — `corpus-22-list-nested`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-22-list-nested` |
| **名称** | List Nesting Stress Test |
| **Profile** | `generic-article` |
| **Tier** | `smoke` |
| **类型** | 合成 |

**目的**：六层以上嵌套列表，`enumitem` 自定义列表格式，`description` 特殊列表。

**文件**：`main.tex`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage{enumitem}
\usepackage{amsmath}

% ---- 自定义列表 ----
\setlist[itemize,1]{label=$\bullet$}
\setlist[itemize,2]{label=$-$}
\setlist[itemize,3]{label=$\ast$}
\setlist[itemize,4]{label=$\dagger$}
\setlist[itemize,5]{label=$\ddagger$}
\setlist[enumerate,1]{label=(\roman*)}

% ---- 六层嵌套 ----
\begin{enumerate}
  \item Top-level item
  \begin{itemize}
    \item Second level
    \begin{enumerate}
      \item Third level
      \begin{description}
        \item[Alpha] First description key
          \begin{itemize}
            \item Fourth level
            \begin{enumerate}
              \item Fifth level
              \begin{enumerate}
                \item Sixth level — deeply nested item
                with longer text that wraps across lines
                to test how line breaking is handled in
                deeply nested list environments.
              \end{enumerate}
            \end{enumerate}
          \end{itemize}
        \item[Beta] Second description key
      \end{description}
    \end{enumerate}
  \end{itemize}
  \item Another top-level item
\end{enumerate}

% ---- description 宏包 ----
\usepackage{description}
\begin{、甘description}
  \item[Convergence] The algorithm converges in $O(1/\epsilon^2)$ iterations.
  \item[Complexity] Time complexity is $O(n^2)$ and space complexity is $O(n)$.
  \item[Robustness] The method is robust to outliers up to $\alpha = 0.2$.
\end{、甘description}
```

---

### Corpus #23 — `corpus-23-chinese-typography`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-23-chinese-typography` |
| **名称** | Chinese Typography Edge Cases |
| **Profile** | `jos-paper` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：CJK 极限场景——中英文无缝混排、`xeCJK` 微调、中文标点挤压、拼音标注、繁体字。

**文件**：`main.tex` + `refs.bib`（中文）

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage{xeCJK}
\usepackage{xpinyin}
\usepackage{ctex}
\usepackage{amsmath}
\usepackage{graphicx}
\usepackage{booktabs}
\usepackage{zhnumber}

\xeCJKsetup{
  CJKspace=true,
  CJKmath=true,
  CheckFullwidth=true,
}

% ---- xeCJK 字体配置 ----
\setCJKmainfont{Noto Serif CJK SC}[AutoFakeBold=true]
\setCJKsansfont{Noto Sans CJK SC}
\newCJKfontfamily\rjrare{Droid Sans Fallback}

% ---- 拼音标注 ----
% 演示 xpinyin 宏包（用于给汉字注音）
\pinyin{xi\\ da\\ li\\ hua}  % 西班牙化

% ---- 中文标点挤压 ----
\ExplSyntaxOn
\ExplSyntaxOff

\begin{document}

% ---- 中英混排段落 ----
\section{研究背景与现状}
随着深度学习技术（Deep Learning, DL）的快速发展，
计算机视觉（Computer Vision, CV）和自然语言处理（Natural Language Processing, NLP）
领域取得了突破性进展~\citep{zhang2020tongue,qi2016tongue}。
然而，现有的跨模态方法（如 CLIP~\citep{radford2021clip}）
在中文场景下的性能仍有较大提升空间。

% ---- 繁体与生僻字 ----
\begin{table}[ht]
  \caption{測試表格（含繁體字與生僻字）}
  \begin{tabular}{lc}
    \toprule
    \textbf{詞彙} & \textbf{釋義} \\
    \midrule
    機器學習 & Machine Learning \\
    深度學習 & Deep Learning \\
    神經網絡 & Neural Network \\
    人工智能 & Artificial Intelligence \\
    區塊鏈 & Blockchain \\
    \bottomrule
  \end{tabular}
\end{table}

% ---- 中文标点处理 ----
这是一个完整的中文段落，包含多种标点符号：逗号，顿号、分号；冒号：
引号"内嵌引用"，以及省略号……破折号——和括号（嵌套的括号）。
中文与英文和数字12345混合排版，测试断行与对齐。
\end{document}
```

**质量重点**：
- `xeCJK` 的 CJK space、CJK math 切换。
- `xpinyin` 拼音标注的降级处理。
- 中文标点（`，。；：""``''`）的全角/半角处理。
- 繁体字与生僻字（如 `測試`、`區塊鏈`）的渲染。

---

### Corpus #24 — `corpus-24-multicolumn`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-24-multicolumn` |
| **名称** | Multicolumn Layout Test |
| **Profile** | `generic-article` |
| **Tier** | `smoke` |
| **类型** | 合成 |

**目的**：`multicol` 动态栏切换、跨栏标题（`\section*`）、跨栏浮动体（`\begin{figure*}...`）。

**文件**：`main.tex` + `figures/`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage{multicol}
\usepackage{graphicx}
\usepackage{amsmath}
\usepackage{booktabs}
\usepackage{caption}

\begin{document}

% ---- 双栏正文 ----
\begin{multicols}{2}
\section{Introduction}
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do
eiusmod tempor incididunt ut labore et dolore magna aliqua.

% ---- 单栏切换（跨栏标题）----
\begin{center}
\section*{Appendix A: Proof Details}\label{sec:appendix}
\addcontentsline{toc}{section}{Appendix A: Proof Details}
\end{center}

% ---- 跨栏大表 ----
\begin{table*}[t]
\centering
\caption{Comprehensive benchmark results across 10 datasets.}
\begin{tabular}{lccccccccc}
  \toprule
  Method & D1 & D2 & D3 & D4 & D5 & D6 & D7 & D8 & Avg \\
  \midrule
  A & 91.2 & 88.4 & 93.1 & 86.7 & 90.3 & 92.1 & 87.9 & 89.5 & 89.9 \\
  B & 93.5 & 90.2 & 94.8 & 89.1 & 92.7 & 93.4 & 90.3 & 91.2 & 91.9 \\
  \bottomrule
\end{tabular}
\end{table*}

% ---- 跨栏图片 ----
\begin{figure*}[t]
  \centering
  \includegraphics[width=0.9\textwidth]{fig-wide.pdf}
  \caption{Wide figure spanning two columns.}
  \label{fig:wide}
\end{figure*}

% ---- 继续双栏 ----
\section{Experiments}
继续的双栏文本。

\end{multicols}
```

---

### Corpus #25 — `corpus-25-report-thesis`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-25-report-thesis` |
| **名称** | Long Report / Thesis Template |
| **Profile** | `generic-article` |
| **Tier** | `golden` |
| **类型** | 合成 |

**目的**：超长文档——多文件 `\include`、复杂 TOC、附录、参考文献。

**文件**：

```
main.tex
chapters/
  01-intro.tex
  02-background.tex
  03-method.tex
  04-experiments.tex
  05-conclusion.tex
appendices/
  A-proofs.tex
  B-data.tex
  C-code.tex
refs.bib
figures/
  fig-ch1-1.pdf
  fig-ch3-1.pdf
  fig-ch4-1.pdf
  fig-ch4-2.pdf
```

**`main.tex` 结构**：

```latex
\documentclass[12pt]{report}
\usepackage[utf8]{inputenc}
\usepackage{graphicx}
\usepackage{amsmath}
\usepackage{booktabs}
\usepackage{hyperref}
\usepackage{tocbibind}
\usepackage{natbib}
\usepackage{minitoc}
\usepackage{fancyhdr}

\graphicspath{{./figures/}}

% ---- minitoc 章节目录 ----
\dominitoc
\dominilof
\dominilot

% ---- fancyhdr 复杂页眉页脚 ----
\pagestyle{fancy}
\fancyhf{}
\fancyhead[LE,RO]{\thepage}
\fancyhead[RE]{\leftmark}
\fancyhead[LO]{\rightmark}
\fancyfoot[C]{\textit{Draft --- Thesis Proposal}}

\begin{document}

\pagenumbering{roman}
\tableofcontents
\listoffigures
\listoftables

\cleardoublepage
\pagenumbering{arabic}
\include{chapters/01-intro}
\include{chapters/02-background}
\include{chapters/03-method}
\include{chapters/04-experiments}
\include{chapters/05-conclusion}

\appendix
\include{appendices/A-proofs}
\include{appendices/B-data}
\include{appendices/C-code}

\bibliographystyle{plainnat}
\bibliography{refs}

\end{document}
```

---

### Corpus #26 — `corpus-26-color-hyperlink`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-26-color-hyperlink` |
| **名称** | Color and Hyperlink Test |
| **Profile** | `generic-article` |
| **Tier** | `smoke` |
| **类型** | 合成 |

**目的**：`xcolor`、`colortbl`、`hyperref`、`url` 的颜色与链接处理。

**文件**：`main.tex` + `refs.bib`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage[table,svgnames]{xcolor}
\usepackage{colortbl}
\usepackage{graphicx}
\usepackage{hyperref}
\usepackage{url}
\usepackage{booktabs}
\usepackage{mdframed}
\usepackage[colorlinks=true,
  linkcolor=red,
  urlcolor=blue,
  citecolor=green,
  anchorcolor=yellow]{hyperref}

% ---- 彩色表格 ----
\begin{table}[ht]
\caption{Colored table with alternating rows and custom column colors}
\label{tab:color}
\begin{tabular}{
  >{\columncolor{LightBlue}}l
  >{\columncolor{LightGreen}}c
  >{\columncolor{Orange}}c
  >{\columncolor{MistyRose}}c
}
  \toprule
  \rowcolor{SteelBlue}\textcolor{white}{\textbf{Config}} &
    \textbf{Precision} &
    \textbf{Recall} &
    \textbf{F1} \\
  \midrule
  \cellcolor{white} A & 0.91 & 0.88 & 0.89 \\
  \cellcolor{white} B & 0.93 & 0.90 & 0.91 \\
  \rowcolor{WhiteSmoke}
  \cellcolor{white} C & 0.89 & 0.92 & 0.90 \\
  \bottomrule
\end{tabular}
\end{table}

% ---- 文本高亮 ----
\textcolor{red}{This text is highlighted in red.}
\colorbox{yellow}{This text has a yellow background.}
\hl{This is highlighted with the soul-like package.}
\passemathtrue${\color{blue} x^2 + \color{red} y^2 = \color{green} z^2}$

% ---- URL 与链接 ----
\url{https://example.com/very-long-url/with/many/segments/to/test/line-breaking}
\nolinkurl{https://no-protocol.example.com/path/to/resource}
```

---

### Corpus #27 — `corpus-27-header-footer`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-27-header-footer` |
| **名称** | Header, Footer and Page Style Test |
| **Profile** | `generic-article` |
| **Tier** | `smoke` |
| **类型** | 合成 |

**目的**：`fancyhdr` 复杂页眉页脚、章节级不同样式、`titletoc` 目录样式。

**文件**：`main.tex`

**核心 LaTeX 内容**：

```latex
\documentclass{book}
\usepackage{fancyhdr}
\usepackage{graphicx}
\usepackage{amsmath}
\usepackage{booktabs}
\usepackage{titletoc}
\usepackage{etoolbox}

% ---- fancyhdr 配置 ----
\pagestyle{fancy}
\fancyhf{}
\fancyhead[LE,RO]{\thepage}
\fancyhead[CE]{\textit{My Research Journal}}
\fancyhead[CO]{\textbf{\nouppercase{\leftmark}}}
\fancyfoot[LE,RO]{\textit{Draft -- \today}}
\fancyfoot[CO,CE]{\thepage}

% ---- 奇偶页不同 ----
\fancypagestyle{plain}{%
  \fancyhf{}
  \fancyfoot[C]{\thepage}
  \fancyfoot[R]{\textit{Page \thepage\ of \pageref{LastPage}}}
}

% ---- 章节级页眉切换 ----
\patchcmd{\chapter}{plain}{fancy}{}{}
\patchcmd{\part}{plain}{fancy}{}{}

% ---- 装饰线 ----
\let\headrule OLDheadrule
\def\OLDheadrule{\global\let\headrule\relax}
\patchcmd{\chapter}{\thispagestyle{plain}}{\thispagestyle{fancy}}{}{}
```

---

### Corpus #28 — `corpus-28-footnote-marginnote`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-28-footnote-marginnote` |
| **名称** | Footnote and Marginpar Test |
| **Profile** | `generic-article` |
| **Tier** | `smoke` |
| **类型** | 合成 |

**目的**：长脚注跨页、多重脚注系统、`marginpar`/`sidenotes` 旁注。

**文件**：`main.tex`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage{sidenotes}
\usepackage{marginfix}
\usepackage{graphicx}
\usepackage{amsmath}
\usepackage{booktabs}

% ---- 标准脚注 ----
This is the main text with a standard footnote.\footnote{
  This is a standard footnote that explains the first reference
  in detail, including bibliographic information:
  Smith et al. proposed the baseline method in 2020~\citep{smith2020},
  which achieved 82.3\% accuracy on the benchmark dataset.
  Subsequent work by Jones extended this to 87.1\% by incorporating
  attention mechanisms. The key insight is the residual connection
  that enables training deeper networks without degradation.
}

% ---- 脚注跨页 ----
This paragraph has a very long footnote attached.\footnote{
  Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do
  eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim
  ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut
  aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit
  in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur
  sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt
  mollit anim id est laborum. Sed ut perspiciatis unde omnis iste natus
  error sit voluptatem accusantium doloremque laudantium, totam rem aperiam,
  eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae
  vitae dicta sunt explicabo. Nemo enim ipsam voluptatem quia voluptas sit
  aspernatur aut odit aut fugit.
}

% ---- 旁注（marginpar）----
\marginpar{
  \raggedright
  \footnotesize
  \textbf{Note:} This is a margin note that appears in the margin
  of the page, useful for author annotations and editorial comments.
}

% ---- sidenotes ----
\begin{sidenotefigure}
  \centering
  \includegraphics[width=\marginparwidth]{fig-margin.pdf}
  \caption{This is a side caption in the margin.}
  \label{fig:margin}
\end{sidenotefigure}
```

---

### Corpus #29 — `corpus-29-layout-absolute`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-29-layout-absolute` |
| **名称** | Absolute Positioning and Overlays |
| **Profile** | `generic-article` |
| **Tier** | `smoke` |
| **类型** | 合成 |

**目的**：`eso-pic` 背景水印、`tikz` 页面坐标绘图、`textpos` 绝对定位。

**文件**：`main.tex` + `figures/`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage{eso-pic}
\usepackage{graphicx}
\usepackage{tikz}
\usetikzlibrary{positioning,calc}
\usepackage[absolute,overlay]{textpos}
\usepackage{xcolor}

% ---- eso-pic 背景水印 ----
\AddToShipoutPictureBG{%
  \AtTextCenter{%
    \put(0,0){\scalebox{5}{\rotatebox{45}{\color[gray]{0.9}DRAFT}}}%
  }
}

% ---- TikZ 页面坐标绝对绘图 ----
\AddToShipoutPictureFG*{%
  \AtPageLowerLeft{%
    \begin{tikzpicture}[remember picture, overlay]
      \coordinate (A) at (current page.north east);
      \coordinate (B) at (current page.south east);
      \draw[red,thick] ($(A) + (-1cm,0)$) -- ($(B) + (-1cm,0)$);
    \end{tikzpicture}
  }
}

% ---- textpos 绝对定位 ----
\begin{textblock}{3}(11,1)
  \framebox{
    \parbox{2.5cm}{
      \centering
      \textbf{Author's} \\
      \textbf{Notes} \\
      Confidential
    }
  }
\end{textblock}

\begin{document}
% 内容中嵌入坐标绘图
\begin{tikzpicture}[remember picture, overlay]
  \node[draw=red,circle,inner sep=2pt]
    at ($(current page.center) + (0,3cm)$) {Important};
\end{tikzpicture}
```

**质量重点**：
- `eso-pic` 背景水印应被忽略或渲染为最底层内容。
- `textpos` 绝对定位元素的坐标转换。
- TikZ `remember picture, overlay` 的跨页面坐标引用。

---

### Corpus #30 — `corpus-30-legacy-deprecated`

| 字段 | 值 |
|---|---|
| **ID** | `corpus-30-legacy-deprecated` |
| **名称** | Legacy / Deprecated Packages Test |
| **Profile** | `generic-article` |
| **Tier** | `smoke` |
| **类型** | 合成（刻意错误） |

**目的**：验证陈旧/废弃宏包（`epsfig`, `eqnarray`, `times` 等）的降级审计与错误报告。

**文件**：`main.tex`

**核心 LaTeX 内容**：

```latex
\documentclass{article}
\usepackage{epsfig}        % deprecated, use graphicx instead
\usepackage{eqnarray}       % deprecated, use align instead
\usepackage{times}         % deprecated, use mathptmx or txfonts
\usepackage{latexsym}     % deprecated, use amssymb
\usepackage{makeidx}
\usepackage{showidx}       % deprecated, use imakeidx
\usepackage{floatflt}      % deprecated floating figure
\usepackage{picinpar}      % deprecated inline figure
\usepackage{mathrsfs}      % still okay but rare

\makeindex

\begin{document}

% ---- eqnarray（已废弃）----
\begin_eqnarray}
  a & = & b + c \label{eq:ea} \\
  d & = & e + f \label{eq:eb}
\end_eqnarray

% ---- epsfig（已废弃）----
\begin{figure}[ht]
  \epsfig(file=fig-old-format.eps, width=0.5\textwidth)
  \caption{Legacy figure using epsfig.}
\end{figure}

% ---- times 字体（已废弃）----
\fontfamily{times}\selectfont
This paragraph uses the deprecated \texttt{times} font family.

% ---- showidx（边注索引）----
\index{quantum entanglement|see{entanglement, quantum}}
\index{machine learning!supervised}
\index{neural network!deep}

\printindex
```

**质量重点**：
- 转换器必须记录所有废弃宏包并写入 `CompileReport.rule_engine` 的 `semantic_loss_events`。
- 错误信息必须给出替换建议（`epsfig` → `graphicx`, `eqnarray` → `align` 等）。
- 降级不应 panic，应优雅降级并附加 `Severity::Warning`。
- `makeidx`/`printindex` 的索引内容降级为普通文本。

---

## 4. Corpus 与 Quality Marker 映射表

以下映射将 30 个 Corpus 映射到 `crates/quality/src/markers.rs` 中的 22 个 marker，确保每个 marker 在至少 1 个 Corpus 中被覆盖：

| Marker | Corpus(s) | 验证方式 |
|---|---|---|
| 标题（长中文标题） | #4, #23 | `rjtitle`, 直接文本 |
| `\textbf{摘  要}` | #4 | `main.tex` 中保留 |
| `\textbf{关键词}` | #4 | `main.tex` 中保留 |
| `Abstract` | #1-#13, #15 | 标准英文摘要 |
| `Key words` | #7, #8 | `keywords` 环境 |
| `1 引言` 等章节 | #4, #23 | 中文章节编号 |
| `表 1`, `表 5` | #4, #17 | 有 `\caption` 的表格 |
| `图 1`, `图 8` | #4, #6, #18 | 有 `\caption` 的图片 |
| `算法 1` | #4, #5 | `algorithm2e`, `algorithmicx` caption |
| `References` | #1-#22, #25 | 参考文献节标题 |
| `\textbf{附中文参考文献}` | #4 | 中文参考文献节标题 |
| `\textbf{作者简介}` | #4 | 作者简介节标题 |
| 邮箱（`@`） | #4, #6 | 通讯作者邮箱 |

---

## 5. 质量维度基线

每个 Corpus 的 `quality_meta.json` 中的 `thresholds` 必须映射到以下六维评分体系：

| 维度 | 权重 | 测量方式 | Corpus 覆盖重点 |
|---|---|---|---|
| **解析完整性** | 20% | unknown macro 比例、fallback 事件数 | #21, #22, #30 |
| **语义保真** | 25% | 标题/段落/列表/表格/图/公式/引用 7 类保真率 | #1-#16（全部） |
| **DOCX 结构** | 20% | OOXML 合法性、style/numbering/media 完整性 | #17, #18, #26 |
| **版面一致** | 20% | 页边距、字号、行距、表格宽度、PDF 视觉差异 | #8, #24, #25 |
| **可编辑性** | 10% | Word 可打开、段落样式、交叉引用可更新 | #1, #4, #7 |
| **性能与稳定** | 5% | 转换耗时、fallback 次数 | #30, #21 |

---

## 6. 实施计划

### Phase 1：建立骨架（Week 1）

1. 在 `examples/demos/corpus/` 下创建 30 个目录。
2. 为每个 Corpus 生成空的 `quality_meta.json`。
3. 复用 `examples/journals/` 中已有的 realistic 文件，将符合对应 Corpus 类型的复制并扩充。
4. 使用 `_shared/` 中的共享图形资源（SVG placeholder）生成 Corpus 所需的图片。

### Phase 2：内容生成（Week 1-2）

1. **Corpus #1-#16**（Golden tier）：以现有 `examples/paper2`/`paper3`/`journals/` 真实内容为底表，扩展复杂元素。
2. **Corpus #17-#30**（Smoke/Stress tier）：人工构造专注于单一瓶颈的 LaTeX 文件。
3. 为每个 Corpus 生成 `refs.bib`（#1-#16, #19, #20, #23）。
4. 生成合成图片（SVG → PDF/PNG）放在各 Corpus 的 `figures/` 目录。

### Phase 3：质量闭环接入（Week 2-3）

1. 将 30 个 Corpus 的路径写入 CI 配置文件（`.github/workflows/corpus-qa.yml`）。
2. 利用 `crates/quality` 的 `QualityRun` 对所有 Corpus 运行质量评分。
3. 生成每个 Corpus 的 `golden_docx/`、`golden_pdf/`、`report.json`。
4. 对 Golden tier（#1-#16）配置 Word 打开回归（`word_open_repair_allowed: false`）。
5. Smoke tier（#17-#30）允许部分降级（`expected_fallbacks` 不为空）。

### Phase 4：持续运营（Ongoing）

1. 新增 Corpus 场景 → 进入回归集。
2. Profile 更新后 → 重新生成所有 Golden DOCX。
3. 每季度更新 corpus 以覆盖新期刊模板。

---

## 7. Corpus 与 Profile 矩阵

| Corpus ID | Profile | 文档类 | 核心 Package | 难度 |
|---|---|---|---|---|
| #1 | `ieee-trans` | IEEEtran | amsmath, booktabs, algorithmicx | ★★★ |
| #2 | `cvpr` | IEEEtran (conf) | subfig, amsmath | ★★★ |
| #3 | `acm-sig` | acmart | booktabs, graphicx | ★★★ |
| #4 | `jos-paper` | rjthesis | amsthm, algorithm2e, bicaption | ★★★★ |
| #5 | `generic-article` | article | listings, minted, algorithm | ★★★ |
| #6 | `generic-article` | article | tikz, array | ★★★ |
| #7 | `generic-article` | amsart | amsthm, amssymb | ★★★ |
| #8 | `aps-prl` | revtex4-2 | physics, mhchem, siunitx | ★★★★ |
| #9 | `generic-article` | article | tikz-cd, mathtools, cases | ★★★ |
| #10 | `generic-article` | article | wrapfig, siunitx, floatrow | ★★★ |
| #11 | `nature` | nature | biblatex, subfig | ★★★★ |
| #12 | `elsevier-chem` | elsarticle | chemfig, mhchem, bpchem, siunitx | ★★★★ |
| #13 | `generic-article` | article | longtable, rotating, multirow | ★★★★ |
| #14 | `econ-econometrica` | aer | threeparttable, dcolumn, natbib | ★★★ |
| #15 | `apa-7` | apa7 | biblatex (apa), csquotes | ★★★ |
| #16 | `generic-article` | article | tikz-qtree, forest, gb4e | ★★★ |
| #17 | `generic-article` | article | multirow, tabularx, tabulary | ★★★ |
| #18 | `generic-article` | article | subcaption, minipage, picinpar | ★★ |
| #19 | `generic-article` | article | （纯 BibTeX 测试） | ★★ |
| #20 | `generic-article` | article | biblatex, csquotes | ★★★ |
| #21 | `generic-article` | article | etoolbox, xparse, ifthen | ★★ |
| #22 | `generic-article` | article | enumitem | ★★ |
| #23 | `jos-paper` | ctexart | xeCJK, xpinyin, ctex | ★★★★ |
| #24 | `generic-article` | article | multicol | ★★ |
| #25 | `generic-article` | report | tocbibind, minitoc, fancyhdr | ★★★ |
| #26 | `generic-article` | article | xcolor, colortbl, hyperref | ★★ |
| #27 | `generic-article` | book | fancyhdr, titletoc, etoolbox | ★★ |
| #28 | `generic-article` | article | sidenotes, marginfix | ★★ |
| #29 | `generic-article` | article | eso-pic, textpos | ★★ |
| #30 | `generic-article` | article | epsfig, eqnarray, times, latexsym | ★★ |

---

## 8. 验收标准

| 阶段 | 指标 | 目标 |
|---|---|---|
| CI 回归 | 30 个 Corpus 全部跑通（不 crash） | 100% |
| Smoke tier (#17-#30) | 解析完整性 | >= 70% |
| Golden tier (#1-#16) | 解析完整性 | >= 90% |
| Golden tier (#1-#16) | 语义保真 | >= 85% |
| 所有 Corpus | DOCX OOXML 合法性 | 100% |
| 所有 Corpus | Word 修复阻断 | 0 |
| 所有 Corpus | Fallback 审计记录 | 100%（必须写入报告） |
