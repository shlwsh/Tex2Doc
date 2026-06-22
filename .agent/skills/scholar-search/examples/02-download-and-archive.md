# 示例 2：搜索 + 下载 + 归档完整流程

## 场景

用户需要对某个研究方向进行完整的文献调研：搜索文献、下载可获取的 PDF、生成引用，并将成果归档到项目目录。

## 输入

### 步骤 1：搜索

```bash
python .agent/skills/scholar-search/scripts/scholar_search.py \
    --query "edge image quality assessment deep learning" \
    --year-from 2022 --num 10 \
    --output data/scholar/search_results.json \
    --bibtex data/scholar/refs_iqa.bib
```

### 步骤 2：下载

```bash
python .agent/skills/scholar-search/scripts/download_papers.py \
    --input data/scholar/search_results.json \
    --output-dir data/papers/ \
    --max-downloads 5
```

### 步骤 3：合并引用（可选）

```bash
cat data/scholar/refs_iqa.bib >> latex/references.bib
# 写入正文 \cite 后，投稿前运行：
# python3 scripts/verify_cited_papers.py --download
```

## 预期输出

### 搜索阶段

- 终端输出 10 篇文献的彩色表格
- 生成 `data/scholar/search_results.json`（完整元数据）
- 生成 `data/scholar/refs_iqa.bib`（BibTeX 引用）

### 下载阶段

```text
📦 准备下载 10 篇论文
📂 存储目录: data/papers
🔢 最大下载数: 5

[1/10] 📄 MUSIQ: Multi-scale Image Quality Transformer
  📥 尝试 arXiv 下载 (2108.05997)...
  ✅ data/papers/2021_Ke_MUSIQ_Multi-scale_Image_Quality_Transformer.pdf

[2/10] 📄 TOPIQ: A Top-down Approach for Full-Reference…
  📥 尝试直接 PDF 下载...
  ✅ data/papers/2023_Chen_TOPIQ_A_Top-down_Approach.pdf

[3/10] 📄 Re-IQA: Unsupervised Learning for Image Quality…
  📥 尝试 Unpaywall 开放获取查询 (DOI: 10.1109/CVPR52729.2023)...
  ✅ data/papers/2023_Saha_Re-IQA_Unsupervised_Learning.pdf

[4/10] 📄 Edge-Aware No-Reference Quality Assessment…
  ❌ 无可用的开放获取 PDF 下载源

[5/10] 📄 Perceptual Image Quality Assessment via…
  📥 尝试 arXiv 下载 (2211.12345)...
  ✅ data/papers/2022_Wang_Perceptual_Image_Quality.pdf

============================================================
📊 下载统计
============================================================
  总计: 10 篇
  ✅ 成功下载: 4 篇
  ⏭️  跳过(已存在): 0 篇
  ❌ 下载失败: 1 篇
  📂 存储目录: data/papers
============================================================
```

### 归档结果目录

```text
data/papers/
├── 2021_Ke_MUSIQ_Multi-scale_Image_Quality_Transformer.pdf
├── 2022_Wang_Perceptual_Image_Quality.pdf
├── 2023_Chen_TOPIQ_A_Top-down_Approach.pdf
├── 2023_Saha_Re-IQA_Unsupervised_Learning.pdf
└── papers_index.json   ← 自动生成的元数据索引
```

## 异常处理

- 若搜索无结果，检查关键词是否过于宽泛或过于狭窄
- 若所有 PDF 下载失败，可能是论文均在付费墙后，工具仅下载合法开放获取版本
- 重复执行下载命令会自动跳过已下载的论文（基于 `papers_index.json` 去重）
