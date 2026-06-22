# 示例 1：基础文献搜索

## 场景

用户需要搜索某个研究方向的最新高引用文献，获取元数据和引用排名。

## 输入

```bash
python .agent/skills/scholar-search/scripts/scholar_search.py \
    --query "robot manipulation reinforcement learning" \
    --backend semantic-scholar \
    --year-from 2023 \
    --num 5 \
    --output results.json \
    --bibtex refs.bib
```

## 预期输出

1. 终端显示彩色表格，包含 5 篇文献的标题、作者、年份、引用数和 PDF 可用性
2. 生成 `results.json` 文件，包含完整元数据
3. 生成 `refs.bib` 文件，包含 BibTeX 格式引用

```text
🔍 正在搜索: "robot manipulation reinforcement learning" (后端: semantic-scholar, 数量: 5)
📅 限定年份: 2023 年至今

📚 文献搜索结果
┏━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━┳━━━━━━┳━━━━━━━━━━━━━━━━━━┳━━━━┓
┃ # ┃ 标题                                       ┃ 作者                      ┃ 年份 ┃ 引用 ┃ 来源             ┃ PDF┃
┡━━━╇━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━━━━━━╇━━━━━━╇━━━━━━╇━━━━━━━━━━━━━━━━━━╇━━━━┩
│ 1 │ Language-Conditioned Robot Manipulation...  │ Brohan, Chebotar et al.   │ 2023 │  450 │ semantic-scholar │ ✅ │
│ 2 │ Foundation Models for Decision Making...    │ Yang, Nachum et al.       │ 2023 │  380 │ semantic-scholar │ ✅ │
│ 3 │ RoboCat: A Self-Improving Generalist...    │ Bousmalis et al.          │ 2023 │  210 │ semantic-scholar │ ✅ │
│ 4 │ Dexterous Manipulation via RL...            │ Chen, Wang et al.         │ 2024 │   95 │ semantic-scholar │ ❌ │
│ 5 │ Multi-Task Learning for Robotic Grasping... │ Liu, Li et al.           │ 2024 │   42 │ semantic-scholar │ ✅ │
└───┴────────────────────────────────────────────┴───────────────────────────┴──────┴──────┴──────────────────┴────┘

🔍 共找到 5 篇文献

💾 结果已保存到: results.json
📖 BibTeX 已保存到: refs.bib
```

## 生成的 BibTeX 示例

```bibtex
@article{brohan2023language-conditioned,
  title  = {Language-Conditioned Robot Manipulation with Large Language Models},
  author = {Anthony Brohan and Yevgen Chebotar and ...},
  year   = {2023},
  doi    = {10.48550/arXiv.2302.12766},
  url    = {https://www.semanticscholar.org/paper/...},
}
```
