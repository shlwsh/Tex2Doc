#!/usr/bin/env bash
# p3-microservice AI Git 自动提交入口
set -euo pipefail

# 优先使用传入的目录，否则默认当前目录
TARGET_DIR="${1:-$PWD}"
cd "$TARGET_DIR"

# 定位脚本自身所在目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GLOBAL_ENV="$(cd "$SCRIPT_DIR/.." && pwd)/.env.mygit"

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
  if [ ! -f "$GLOBAL_ENV" ]; then
    echo "❌ 错误: 找不到配置文件 .env.mygit (当前项目及全局安装目录均未找到)"
    echo "请执行:"
    echo "  cp $SCRIPT_DIR/../.agent/skills/mygit/resources/env.mygit.template .env.mygit"
    echo "  然后编辑 .env.mygit 填入 DashScope API 密钥"
    exit 1
  fi
fi

exec python3 "$SCRIPT_DIR/mygit.py" "$TARGET_DIR" "$SCRIPT_DIR"
