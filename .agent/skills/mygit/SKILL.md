---
name: mygit
description: AI 智能 Git 提交工具。当用户需要提交代码、推送代码、同步代码、git commit、git push、推送变更、代码提交、代码推送、保存并提交、提交到仓库、或执行 `./scripts/mygit.sh` / `bun run mygit` 时使用此技能。自动检测代码变更，调用 AI 生成中文 Conventional Commits 格式的提交信息，并一键完成 add → commit → push 流程。
---

# Goal

自动化 Git 提交流程：检测代码变更 → 调用 AI 生成中文提交信息 → `git add` → `git commit` → `git push`，一条命令完成全部操作。

适配 **p3-microservice**（Go Agent/Center、实验脚本、LaTeX 论文、Docker 部署）。

## Instructions

### 1. 环境准备

确保以下前置条件满足：

1. 项目根目录下存在 `.env.mygit` 配置文件（格式参见 `resources/env.mygit.template`）
2. 配置文件中的三个必填字段不为空：`DASHSCOPE_API_KEY`、`DASHSCOPE_BASE_URL`、`DASHSCOPE_MODEL`
3. 已安装 Python 3 与 `requests`：`pip3 install requests`
4. 当前目录为有效的 Git 仓库

首次配置：

```bash
cp .agent/skills/mygit/resources/env.mygit.template .env.mygit
# 编辑 .env.mygit 填入 DashScope API 密钥
bash .agent/skills/mygit/scripts/setup-check.sh
```

### 2. 执行命令

```bash
./scripts/mygit.sh          # 推荐（WSL / Linux）
bun run mygit               # 等价入口（需 bun）
bun run mygit:check         # 环境检查
```

### 3. 执行流程

脚本按以下步骤顺序执行：

1. **加载配置**：从 `.env.mygit` 读取 AI API 密钥；从 `.env.local` 读取 `GITHUB_TOKEN`（可选）
2. **验证 Git 仓库**：执行 `git rev-parse --git-dir`
3. **检测变更**：执行 `git status --porcelain`，解析新增/修改/删除的文件列表
4. **排除路径**：自动跳过 `.gitnexus/`、`.env`、`.env.local` 等
5. **版本文件检测**：若变更包含 `agent/go.mod`、`center/go.mod`、`proto/go.mod`、`deploy/docker/docker-compose*.yml`，提示用户确认后继续
6. **AI 生成提交信息**：
   - 调用 OpenAI 兼容 API（`POST {baseUrl}/chat/completions`）
   - System Prompt 包含 p3-microservice 项目上下文（Agent/Center、gRPC、实验、论文）
   - 参数：`max_tokens=500`，`temperature=0.7`
7. **托底逻辑**：
   - 未配置 API Key、含 PDF 等二进制、或 AI 失败时，按规则生成提交信息
   - 支持 scope：`agent` / `center` / `deploy` / `experiments` / `docs` / `latex` 等
8. **暂存**：`git add -A`，并排除构建/索引目录
9. **提交**：`git commit --no-verify`
10. **推送**：
    - 自动检测当前分支和远程仓库
    - WSL 优先使用 Windows Git 复用凭据
    - 支持 `GITHUB_TOKEN`（写入 `.env.local`，不提交）
    - 首次推送自动 `--set-upstream`

### 4. AI 配置说明

| 配置项 | 环境变量名 | 说明 |
|-------|-----------|------|
| API 密钥 | `DASHSCOPE_API_KEY` | 阿里云 DashScope 平台密钥 |
| API 地址 | `DASHSCOPE_BASE_URL` | OpenAI 兼容接口地址 |
| 模型名称 | `DASHSCOPE_MODEL` | 推荐 `deepseek-v3` |
| HTTP 代理 | `MYGIT_HTTP_PROXY` | WSL 下推荐 `http://127.0.0.1:7897` |
| 跳过 AI | `MYGIT_NO_AI=1` | 始终用规则生成（最快） |
| 强制 AI | `MYGIT_FORCE_AI=1` | 含 PDF 时仍调用 AI |

### 5. 提交信息生成规则

AI 被要求按以下规则生成提交信息：

1. 使用中文
2. 第一行为简短标题（不超过 50 字符）
3. 使用 Conventional Commits 前缀：`feat` / `fix` / `docs` / `style` / `refactor` / `test` / `chore`
4. scope 示例：`feat(agent): 优化定向策略匹配`
5. 描述清晰、准确

### 6. 脚本执行失败处理

若脚本运行失败，请按以下步骤排查：

1. 运行 `bun run mygit:check` 或 `bash .agent/skills/mygit/scripts/setup-check.sh`
2. 检查 `.env.mygit` 文件是否存在且配置正确
3. 检查网络连接和 API 密钥有效性
4. 若 AI 调用失败，脚本会自动降级使用托底提交信息
5. 若推送失败，本地提交仍然保留；在 `.env.local` 配置 `GITHUB_TOKEN` 后重试
6. 若仍无法解决，请阅读 `scripts/mygit.py` 源码定位问题

## Examples

### 输入 1：正常提交

用户执行 `./scripts/mygit.sh`，有 3 个文件变更。

**输出：**

```text
🚀 AI Git 提交工具启动 (p3-microservice)

📝 正在检查代码变更...

发现 3 个文件变更：
  修改: agent/pkg/matcher/rule_matcher.go
  修改: center/pkg/strategy/triple_transform.go
  新增: experiments/scripts/phase3_benchmark.py

🤖 正在使用 AI 生成提交信息...

提交信息 (AI 生成)：
──────────────────────────────────────────────────
feat(agent): 优化定向策略匹配与三次转换链路

- 更新 rule_matcher 支持高价值日志过滤
- 完善 center 侧 triple_transform 策略转换
- 新增三期实验基准脚本
──────────────────────────────────────────────────

📦 正在添加变更到暂存区...
💾 正在创建提交...
🚀 正在推送到远程仓库...
📡 远程仓库: origin, 分支: main

✨ 提交并推送成功！
```

### 输入 2：AI 调用失败，自动降级

API 返回 429 错误。

**输出：**

```text
⚠️  AI 生成提交信息失败 (API 请求失败: 429)，正在使用托底逻辑...

提交信息 (托底生成)：
──────────────────────────────────────────────────
chore: 自动同步代码变更 (2026-06-05)

变更摘要：
修改 2 个文件，新增 1 个文件

由于 AI 生成失败，此信息由系统自动生成。
──────────────────────────────────────────────────
```

### 输入 3：无代码变更

当前工作区干净，无待提交内容。

**输出：**

```text
🚀 AI Git 提交工具启动 (p3-microservice)

📝 正在检查代码变更...
✅ 没有检测到代码变更
```

## Constraints

- `.env.mygit` 包含 API 密钥，建议纳入 Git 跟踪（团队仓库请确保为私有库）
- `GITHUB_TOKEN` 仅写入 `.env.local`，禁止提交
- 禁止在无变更时执行提交
- 提交和推送均使用 `--no-verify` 跳过 Git hooks
- `go.mod` / `docker-compose` 变更时会提示确认，避免误发版本
- 禁止修改 `.env.mygit` 中用户已配置的 API 密钥
- 禁止在日志或输出中打印完整的 API 密钥
