# 示例 3：p3 正文引用核实 + 归档（C6 门禁）

## 场景

论文 `latex/sections/zh/*.tex` 已写入 `\cite{}`，投稿《软件学报》前须：

1. 每条引用 DOI/URL **真实有效**
2. 每条引用在 `data/papers/` 有 **PDF 或快照**
3. 产出 `cited_papers_manifest.json` 供 `@paper1-multi-agent-review` 预检

## 步骤 1：仅核实（不下载）

```bash
python3 scripts/verify_cited_papers.py
```

若有输出 `❌ 无法核实: ['some_key']` → 修正 bib 中 DOI/URL 或更换文献。

## 步骤 2：核实并归档

```bash
python3 scripts/verify_cited_papers.py --download
```

## 预期输出（本会话实测：18 篇 cited）

```text
✅ limingshu2019logmgmt: 大规模分布式系统日志管理技术综述
...
=== 汇总 ===
{
  "ok": 4,
  "snapshot": 14,
  "paywall": 0,
  "doi_valid": 13
}
✅ 全部 cited 文献已核实且已归档（PDF 或官网 HTML 快照）
```

## 归档目录结构

```text
data/papers/          # 根目录 papers/ → 符号链接
├── cited_papers_manifest.json   ★ 门禁清单
├── papers_index.json            # scholar-search 下载索引
├── 2024_Tingting_wang2024rca.pdf
├── 2023_Shenglin_zhang2023multimodal.pdf
├── web_snapshots/
│   ├── limingshu2019logmgmt.html      # 软件学报官网
│   ├── soldani2024logs_crossref.json  # 闭源期刊 CrossRef 元数据
│   ├── otel_tail.html
│   └── ...
└── pending/                 # 应为空
```

## 典型修复

### 无效 DOI

**现象**：`zhangjianxun2020microservice` → doi.org 404  

**处理**：替换为 CHNDOI 可解析条目（如 `meiyudong2020logcnn`，计算机学报 2020）。

### 书籍 URL 404

**现象**：`turnbull2014monitoring` Leanpub 404  

**处理**：bib 中 `url` 改为 `https://openlibrary.org/works/OL27309078W`。

### 闭源 ACM/Wiley

**现象**：无法下载 PDF  

**处理**：无需人工干预；脚本自动保存 CrossRef JSON 快照，`archive_status: snapshot`。

## 与 scholar-search 搜索流程的关系

```text
scholar_search.py  →  发现文献、生成 refs.bib
        ↓ 人工筛选写入 references.bib + \cite
verify_cited_papers.py  →  C6 门禁（投稿前必跑）
paper1-multi-agent-review  →  预检读 manifest
```

## 扩展阅读（可选）

```bash
python .agent/skills/scholar-search/scripts/scholar_search.py \
  --query "microservice log observability" \
  --backend openalex --arxiv-only --num 15 \
  --output data/scholar/results_arxiv.json

python .agent/skills/scholar-search/scripts/download_papers.py \
  --input data/scholar/results_arxiv.json \
  --output-dir data/papers
```

扩展 PDF **不自动**进入 cited manifest——仅当写入 bib 并 `\cite` 后，再跑 `verify_cited_papers.py`。
