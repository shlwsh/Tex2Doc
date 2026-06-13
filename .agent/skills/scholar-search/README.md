# scholar-search 技能包

科研文献 **搜索 → 下载 → 引用核实** 三阶段工具链，适配 p3-microservice 论文工程。

## 文件索引

| 文件 | 用途 |
|------|------|
| [SKILL.md](SKILL.md) | Agent 主入口：命令、参数、工作流 |
| [实战技巧.md](实战技巧.md) | **会话沉淀**：踩坑、DOI 核对、中英文策略 |
| [`.cursor/rules/scholar-cited-checklist.mdc`](../../../.cursor/rules/scholar-cited-checklist.mdc) | Cursor Rule：引用检查清单（编辑 bib/tex 时加载） |
| [config.md](config.md) | p3 路径与审核衔接 |
| [examples/01-basic-search.md](examples/01-basic-search.md) | 基础搜索 |
| [examples/02-download-and-archive.md](examples/02-download-and-archive.md) | 搜索 + 下载 |
| [examples/03-p3-cited-verify-workflow.md](examples/03-p3-cited-verify-workflow.md) | **C6 引用门禁** |
| `scripts/` | scholar_search、download_papers、elicit_analysis |

## 快速命令

```bash
# 调研：搜索 + 下载 arXiv
python .agent/skills/scholar-search/scripts/scholar_search.py \
  -q "microservice log observability" -b openalex --arxiv-only -n 10 \
  -o data/scholar/results.json
python .agent/skills/scholar-search/scripts/download_papers.py \
  -i data/scholar/results.json

# 门禁：正文 cited 核实（投稿前必跑）
python3 scripts/verify_cited_papers.py --download
```

## 双清单

| 清单 | 生成者 | 含义 |
|------|--------|------|
| `papers_index.json` | download_papers | 扩展阅读 PDF |
| `cited_papers_manifest.json` | verify_cited_papers | **正文引用** 合规证明 |
