# 项目 Agent 技能

本目录是 **Tex2Doc** 仓库的 Agent 技能**源目录**。Cursor 通过 `.cursor/skills/` 符号链接镜像本目录，实现自动发现与按需触发。

---

## 目录布局

| 路径 | 作用 |
|------|------|
| **`.agent/skills/<name>/`** | 技能**源目录**（`SKILL.md`、脚本、参考文档） |
| **`.cursor/skills/<name>/`** | Cursor **发现入口**（符号链接 → `.agent/skills/`） |

### 镜像同步

```bash
bash scripts/link_cursor_skills.sh   # 清空 .cursor/skills/ 并逐项链接
```

新增或删除技能后执行上述命令，然后**重启 Cursor** 或新开 Agent 会话。

---

## 技能分类总览

| 技能 | 触发场景 |
|------|----------|
| `scholar-search` | 文献搜索、下载 PDF、BibTeX、`references.bib` 核实、文献调研 |
| `mygit` | git commit/push、智能提交、`mygit.sh` |
| `makeskill` | 新建/规范项目技能 |

---

## 新建技能

1. 用 `makeskill` 或手动在 `.agent/skills/<name>/` 创建 `SKILL.md`
2. `description` 写清 **做什么 + 何时用**（第三人称，含中英文触发词）
3. 勿加 `disable-model-invocation: true`（除非仅 `@` 手动触发）
4. 运行 `bash scripts/link_cursor_skills.sh`
5. 若需 Agent 自动匹配，同步更新 `.cursor/rules/project-skills.mdc` 索引表
