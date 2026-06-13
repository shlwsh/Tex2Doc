#!/usr/bin/env bash
# p3-microservice AI Git 自动提交入口
set -euo pipefail

WORKSPACE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$WORKSPACE_DIR"

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo "❌ 错误: 当前目录不是一个有效的 Git 仓库"
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "❌ 错误: 未找到 python3"
  exit 1
fi

if ! python3 -c "import requests" 2>/dev/null; then
  echo "❌ 错误: 未安装 Python requests，请执行: pip3 install requests"
  exit 1
fi

if [ ! -f ".env.mygit" ]; then
  echo "❌ 错误: 找不到配置文件 .env.mygit"
  echo "请执行:"
  echo "  cp .agent/skills/mygit/resources/env.mygit.template .env.mygit"
  echo "  然后编辑 .env.mygit 填入 DashScope API 密钥"
  exit 1
fi

exec python3 "$WORKSPACE_DIR/scripts/mygit.py"
