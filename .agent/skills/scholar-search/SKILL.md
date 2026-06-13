---
name: scholar-search
description: 科研文献搜索、下载与引用核实。当用户需要搜索文献、文献调研、下载 PDF、生成 BibTeX、核实 references.bib 中已引用条目、或归档至 data/papers/ 时使用。支持 SerpApi/Semantic Scholar/OpenAlex 搜索；与 scripts/verify_cited_papers.py 配合完成投稿前 C6 门禁。
---

# Goal

1. **调研扩展**：按主题搜索文献，下载开放获取 PDF，生成 BibTeX。
2. **引用门禁**：核实正文 `\cite{}` 条目 DOI/URL 有效，并归档至 `data/papers/`（`cited_papers_manifest.json`）。

> **p3 路径与踩坑经验**：[config.md](config.md)、[实战技巧.md](实战技巧.md)  
> **C6 细则**：[参考文献归档细则](../paper1-multi-agent-review/参考文献归档细则.md)

## Instructions

### 1. 环境准备

确认以下前置条件：

1. 项目根目录虚拟环境中已安装依赖：
   ```bash
   pip install -r .agent/skills/scholar-search/scripts/requirements-scholar.txt
   ```
2. （可选）如需使用 SerpApi（Google Scholar 后端），在项目根目录创建 `.env.scholar` 文件并填入 `SERPAPI_API_KEY`。参考模板：`resources/env.scholar.template`
3. **无 API Key 也可直接使用**——OpenAlex 后端完全免费、无需注册、无严格速率限制

### 2. 文献搜索

#### 2.1 基础搜索

```bash
python .agent/skills/scholar-search/scripts/scholar_search.py \
    --query "搜索关键词" \
    --num 10
```

#### 2.2 高级搜索参数

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `--query, -q` | 搜索关键词（必填） | — |
| `--num, -n` | 返回文献数量 | 10 |
| `--year-from` | 限制起始年份 | 无 |
| `--backend, -b` | 搜索后端：`auto`、`serpapi`、`semantic-scholar`、`openalex` | auto |
| `--lang` | 搜索语言（仅 SerpApi） | en |
| `--output, -o` | 输出 JSON 文件路径 | 无 |
| `--bibtex` | 输出 BibTeX 文件路径 | 无 |
| `--json-only` | 仅输出 JSON 到 stdout | 否 |
| `--arxiv-only` | 仅返回 arXiv 来源论文（保证可下载 PDF，仅 OpenAlex 支持） | 否 |
| `--relevance-keywords, -k` | 自定义相关性关键词列表，用于对结果重排序 | 无 |

#### 2.3 后端选择逻辑

- **auto**（默认）：SerpApi → Semantic Scholar → OpenAlex 逐级降级
- **serpapi**：仅使用 SerpApi，需要 `SERPAPI_API_KEY`
- **semantic-scholar**：仅使用 Semantic Scholar 免费 API，无需 Key（100次/5分钟限制，极易触发 429）
- **openalex**：仅使用 OpenAlex 免费 API，无需 Key，无严格速率限制（**推荐默认后端**）

#### 2.4 相关性过滤（重要）

OpenAlex 的宽泛搜索在专精学术领域容易返回"通用高引但不相关"的论文。使用 `--relevance-keywords` 可对结果进行关键词相关性二次排序：

```bash
python .agent/skills/scholar-search/scripts/scholar_search.py \
    --query "image quality assessment edge computing" \
    --backend openalex --num 20 \
    --relevance-keywords "IQA" "no-reference" "blind" "edge" "lightweight" "tongue" \
    --output results.json
```

打分规则：标题匹配关键词 +2 分、摘要匹配 +1 分、引用量对数加分（最多 +6 分）。

#### 2.5 仅搜索可下载论文（arXiv 过滤）

付费期刊论文通常无法下载。使用 `--arxiv-only` 仅返回有 arXiv 预印本的论文，**保证 100% 可下载**：

```bash
python .agent/skills/scholar-search/scripts/scholar_search.py \
    --query "blind image quality assessment transformer" \
    --backend openalex --arxiv-only --num 10 \
    --output results.json
```

### 3. 文献分析（可选 — 需要 Elicit API）

```bash
python .agent/skills/scholar-search/scripts/elicit_analysis.py \
    --query "Does multi-agent RL improve manufacturing efficiency?" \
    --output analysis.json
```

需要在 `.env.scholar` 中设置 `ELICIT_API_KEY`。

### 4. 论文下载

从搜索结果 JSON 自动下载可获取的开放获取 PDF：

```bash
python .agent/skills/scholar-search/scripts/download_papers.py \
    --input results.json \
    --output-dir data/papers/ \
    --max-downloads 10
```

#### 4.1 下载源优先级（实战优化后）

1. **直接 PDF 链接**（含 arXiv PDF URL）— 最快最可靠
2. **arXiv 库下载**（备选）— API 容易遭遇 429 限流
3. **Unpaywall**（通过 DOI 查询合法开放获取版本）

> **关键经验**：arXiv 的 export API 在短时间内多次调用会触发 429 限流。搜索脚本已优化为直接构造 `https://arxiv.org/pdf/{id}` URL，绕过 API 直接下载 PDF，成功率远高于 arxiv 库。

#### 4.2 归档策略

- 文件命名：`[年份]_[第一作者姓]_[简化标题].pdf`
- 去重：基于 DOI/标题哈希自动跳过已下载文献
- 索引：自动生成 `papers_index.json` 记录所有已下载文献元数据

### 5. 典型工作流

#### 5.1 通用搜索下载（推荐）

```bash
# 步骤 1：搜索文献，输出 JSON 和 BibTeX
python .agent/skills/scholar-search/scripts/scholar_search.py \
    --query "你的研究主题" \
    --year-from 2023 --num 15 \
    --output results.json --bibtex refs.bib

# 步骤 2：下载可获取的 PDF
python .agent/skills/scholar-search/scripts/download_papers.py \
    --input results.json --max-downloads 10

# 步骤 3：筛选后写入 latex/references.bib 并 \cite{key}

# 步骤 4（投稿前必跑）：核实已引用条目并归档
python3 scripts/verify_cited_papers.py --download
```

#### 5.2 专精领域精准搜索（高相关性需求）

对于特定学术领域（如 NR-IQA、舌象诊断），**分主题多轮搜索 + 关键词打分** 效果远优于单次宽泛搜索：

```bash
# 分 3 个子主题搜索
python scholar_search.py -q "no-reference image quality assessment" \
    --backend openalex --arxiv-only --num 10 \
    --relevance-keywords "IQA" "blind" "no-reference" "MUSIQ" "TOPIQ" \
    --output results_iqa.json

python scholar_search.py -q "tongue diagnosis deep learning" \
    --backend openalex --num 10 \
    --relevance-keywords "tongue" "TCM" "diagnosis" "segmentation" \
    --output results_tcm.json

python scholar_search.py -q "edge cloud inference robotic system" \
    --backend openalex --arxiv-only --num 10 \
    --relevance-keywords "edge" "cloud" "inference" "latency" "robot" \
    --output results_edge.json

# 合并去重后下载
python -c "
import json
from pathlib import Path
all_papers = []
for f in ['results_iqa.json', 'results_tcm.json', 'results_edge.json']:
    all_papers.extend(json.loads(Path(f).read_text()))
seen = set()
unique = [p for p in all_papers if p['title'].lower() not in seen and not seen.add(p['title'].lower())]
unique.sort(key=lambda x: x.get('_relevance_score', x.get('citations', 0)), reverse=True)
Path('results_merged.json').write_text(json.dumps(unique[:15], ensure_ascii=False, indent=2))
"
python download_papers.py --input results_merged.json --max-downloads 15
```

#### 5.3 已知论文精准下载（按 arXiv ID）

如果已知目标论文的 arXiv ID，可直接构造 JSON 输入进行下载（绕过搜索 API 的不稳定性）：

```bash
python -c "
import json
papers = [
    {'title': 'MUSIQ: Multi-scale Image Quality Transformer',
     'authors': ['Junjie Ke', 'Qifei Wang'],
     'year': 2021, 'arxiv_id': '2108.05997',
     'pdf_url': 'https://arxiv.org/pdf/2108.05997',
     'doi': '10.48550/arXiv.2108.05997'},
    # ... 添加更多论文
]
with open('curated.json', 'w') as f:
    json.dump(papers, f, ensure_ascii=False, indent=2)
"
python download_papers.py --input curated.json
```

### 6. 下载后文献管理

#### 6.1 生成文献说明清单

下载完成后，建议在 `papers/` 目录下生成 `README.md` 清单文件，包含：
- 各论文的出版信息（作者、年份、期刊/会议、DOI）
- 内容概要
- 与当前研究的关联说明
- 推荐阅读顺序

可参考：`data/papers/README.md`（若有）、`cited_papers_manifest.json`

#### 6.2 生成 BibTeX 引用

从已下载论文的索引生成 BibTeX 引用文件，可直接在 LaTeX 中使用：

```bash
python -c "
import json
from pathlib import Path
idx = json.loads(Path('data/papers/papers_index.json').read_text())
for h, p in idx['papers'].items():
    key = p['authors'][0].split()[-1].lower() + str(p['year'])
    print(f'@article{{{key},')
    print(f'  title  = {{{p[\"title\"]}}},')
    print(f'  author = {{{\", \".join(p[\"authors\"][:5])}}},')
    print(f'  year   = {{{p[\"year\"]}}},')
    if p.get('doi'): print(f'  doi    = {{{p[\"doi\"]}}},')
    print('}')
    print()
"
```

### 7. 正文引用核实与归档（C6 门禁）

**与调研下载分离**：仅 `download_papers.py` 不能保证投稿合规；正文每条 `\cite` 须跑核实脚本。

```bash
# 核实 DOI/URL（不下载）
python3 scripts/verify_cited_papers.py

# 核实 + 下载 PDF / 官网 HTML / CrossRef 快照
python3 scripts/verify_cited_papers.py --download
```

| 产出 | 用途 |
|------|------|
| `data/papers/cited_papers_manifest.json` | 审核预检 C6、投稿可追溯 |
| `data/papers/papers_index.json` | scholar-search 批量下载索引 |

**archive_status 含义**：

| 状态 | 说明 |
|------|------|
| `ok` | 开放 PDF 已落盘 |
| `snapshot` | 中文期刊 HTML、技术文档 HTML、闭源期刊 CrossRef JSON |
| `paywall` | **失败**——须修 DOI/换文献 |

**核对要点**（详见 [实战技巧.md](实战技巧.md)）：

- 加入 bib **前**用 CrossRef + CHNDOI 验证 DOI，勿手写记忆 DOI
- 中文期刊（JOS/CJC/CRAD）通常无 OA PDF → 官网 HTML 快照即合格
- ACM/Wiley/Springer 常 Cloudflare 拦截 → 以 CrossRef API 为准，JSON 快照归档
- arXiv：直接 `https://arxiv.org/pdf/{id}.pdf`

完整示例：[examples/03-p3-cited-verify-workflow.md](examples/03-p3-cited-verify-workflow.md)

### 8. p3 微服务日志方向搜索模板

```bash
mkdir -p data/scholar

python .agent/skills/scholar-search/scripts/scholar_search.py \
  --query "microservice observability log collection sampling" \
  --backend openalex --year-from 2020 --num 20 \
  --relevance-keywords "microservice" "log" "Loki" "Promtail" "tracing" "AIOps" "gateway" \
  --arxiv-only \
  --output data/scholar/results_obs.json --bibtex data/scholar/refs_obs.bib
```

中文文献须从 [软件学报](https://www.jos.org.cn/)、[计算机学报](https://cjc.ict.ac.cn/) 等官网核对 DOI 后手工写入 bib。

### 9. 脚本执行失败处理

若脚本运行失败，请按以下步骤排查：

1. 检查网络连接是否正常
2. 若遇到 429 速率限制：
   - Semantic Scholar：脚本自动重试 5 次（退避 5s→10s→15s→20s→25s），超过后 auto 模式自动降级到 OpenAlex
   - arXiv API：使用 `--arxiv-only` 标志时搜索不受影响（通过 OpenAlex 过滤），下载已优化为直接 PDF URL
   - OpenAlex：几乎不会限流，是最可靠的后端
3. 若大量论文下载失败，原因通常是付费墙——使用 `--arxiv-only` 仅搜索 arXiv 论文可保证 100% 可下载
4. 若仍无法解决，请阅读 `scripts/` 目录下的脚本源码定位问题

## Examples

### 输入 1：搜索特定领域文献并下载

用户说：「帮我搜索 NR-IQA 相关的最新文献并下载 PDF」

**Agent 执行策略：**
1. 使用 `--arxiv-only` 保证可下载性
2. 使用 `--relevance-keywords` 提升相关性
3. 分步搜索 + 下载

```bash
# 搜索（保证全部可下载）
python .agent/skills/scholar-search/scripts/scholar_search.py \
    --query "no-reference image quality assessment deep learning" \
    --backend openalex --arxiv-only --num 10 --year-from 2021 \
    --relevance-keywords "IQA" "no-reference" "blind" "MUSIQ" "TOPIQ" "CLIP" \
    --output results.json --bibtex refs.bib

# 下载
python .agent/skills/scholar-search/scripts/download_papers.py \
    --input results.json --max-downloads 10
```

### 输入 2：基于论文主题的文献调研

用户说：「基于当前论文主题搜索 10 篇相关高引文献并下载」

**Agent 执行策略：**
1. 先阅读论文摘要/标题，提取 3-5 个核心子主题
2. 每个子主题分别搜索 5-8 篇
3. 合并结果，用关键词打分排序
4. 取 Top 10 下载
5. 生成文献说明清单 README.md

### 输入 3：已知论文精准下载

用户说：「帮我下载 MUSIQ、CLIP-IQA、ReAct 这几篇论文」

**Agent 执行策略：**
1. 手动构造包含 arXiv ID 和 PDF URL 的 JSON
2. 直接调用 download_papers.py（绕过搜索 API）

### 输入 4：核实正文引用是否可投稿

用户说：「检查 references.bib 里引用的文献是否都能下载/核实」

**Agent 执行策略：**
1. 运行 `python3 scripts/verify_cited_papers.py --download`
2. 汇报 manifest 汇总：`ok` / `snapshot` / `paywall`
3. 对 `paywall` 或无效 DOI 条目提出替换方案（参考 [实战技巧.md](实战技巧.md) §3）
4. 与 `@paper1-multi-agent-review` 联动时粘贴 C6 一行汇总

## Constraints

- **API Key 保护**：禁止在日志、输出或代码中打印完整的 API Key
- **下载合规**：仅下载公开可访问的开放获取论文（arXiv 预印本、OA 论文），不进行非授权下载
- **速率限制**：
  - Semantic Scholar：免费限制 100 次/5 分钟，脚本内置 5 次退避重试
  - arXiv export API：短时间内多次调用易触发 429，已优化为直接 PDF URL 下载
  - OpenAlex：几乎无限制，推荐作为默认后端
- **存储安全**：`.env.scholar` 已被 `.gitignore` 排除，禁止将含有 Key 的配置文件提交到版本库
- **文件命名**：下载的 PDF 统一使用 `[年份]_[作者]_[标题].pdf` 格式，便于管理
- **去重保护**：通过 `papers_index.json` 索引和文件存在性检查，避免重复下载
- **网络依赖**：所有搜索和下载功能需要网络连接，脚本会在网络错误时给出明确提示

## 已知限制与解决方案

| 问题 | 原因 | 解决方案 |
|------|------|----------|
| OpenAlex 搜索返回不相关的高引论文 | 宽泛搜索按引用排序 | 使用 `--relevance-keywords` 二次打分 |
| 大量论文 PDF 下载失败 | 付费期刊无 OA 版本 | 使用 `--arxiv-only` 仅搜索 arXiv 论文 |
| Semantic Scholar 频繁 429 限流 | 免费层 100次/5分钟 | 使用 `--backend openalex` 直接跳过 |
| arXiv 库 download_pdf 报错 | arxiv 库版本兼容性 | 已改为直接 URL 下载，优先级高于库调用 |
| 搜索结果与论文领域不匹配 | 单次宽泛搜索 | 分子主题多轮搜索 + 合并去重 |
| bib 中 DOI 404 | 手写/错误 DOI | CrossRef + CHNDOI 预检；删或换条目 |
| 中文期刊 PDF 下载失败 | 无开放获取 | `verify_cited_papers.py` 抓官网 HTML |
| doi.org 返回 Cloudflare | 出版商反爬 | 以 CrossRef JSON 快照归档 |
| cited 与扩展阅读混淆 | 两套索引 | `cited_papers_manifest` vs `papers_index` |

## 延伸阅读

| 文件 | 内容 |
|------|------|
| [实战技巧.md](实战技巧.md) | 搜索/下载/核对踩坑与会话经验 |
| [config.md](config.md) | p3 路径与审核衔接 |
| [examples/03-p3-cited-verify-workflow.md](examples/03-p3-cited-verify-workflow.md) | C6 完整流程 |
