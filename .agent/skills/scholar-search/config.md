# scholar-search — 项目配置（p3-microservice）

> 移植到其他项目时修改下表路径。

| 变量 | p3 默认路径 | 说明 |
|------|-------------|------|
| **PAPER_ROOT** | `.` | 仓库根 |
| **BIB** | `latex/references.bib` | LaTeX 参考文献 |
| **SECTIONS** | `latex/sections/` | 提取 `\cite{}` |
| **PAPERS_DIR** | `data/papers/` | PDF/快照归档（根 `papers/` 为符号链接） |
| **PAPERS_INDEX** | `data/papers/papers_index.json` | download_papers 索引 |
| **CITED_MANIFEST** | `data/papers/cited_papers_manifest.json` | **正文引用门禁清单** |
| **SEARCH_OUT** | `data/scholar/` | 搜索结果 JSON / BibTeX |
| **VERIFY_SCRIPT** | `scripts/verify_cited_papers.py` | 核实 + 归档 cited 条目 |

## 与 paper1-multi-agent-review 衔接

| 阶段 | 工具 |
|------|------|
| 扩展调研、找新文献 | `scholar-search`（本技能） |
| 写入 bib + 正文 cite 后 | `verify_cited_papers.py`（**C6 门禁**） |
| 投稿前审核 | `@paper1-multi-agent-review` 必读 manifest |

## 触发语

- 搜索微服务/日志/可观测性相关文献
- 下载论文 PDF 到 papers
- 核实 references.bib 引用是否有效
- @scholar-search
