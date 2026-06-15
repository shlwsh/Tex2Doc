#!/usr/bin/env bash
# 将 .agent/skills/ 完整镜像到 .cursor/skills/（逐项符号链接），使 Cursor 自动发现项目技能。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/.agent/skills"
DST="$ROOT/.cursor/skills"

if [[ ! -d "$SRC" ]]; then
  echo "错误: 未找到技能源目录 $SRC" >&2
  exit 1
fi

mkdir -p "$DST"

# 清空 .cursor/skills/，仅保留目录本身
shopt -s dotglob nullglob
for entry in "$DST"/*; do
  rm -rf "$entry"
done

linked=0
for entry in "$SRC"/*; do
  name="$(basename "$entry")"
  target="../../.agent/skills/$name"
  link="$DST/$name"
  
  # Check if we're on MSYS/Cygwin/Git Bash where ln -s works normally, 
  # or if we should use fallback
  ln -s "$target" "$link" 2>/dev/null || cp -r "$SRC/$name" "$link"
  echo "→ $name"
  linked=$((linked + 1))
done

echo ""
echo "已链接 $linked 项 → .cursor/skills/（与 .agent/skills/ 一致）"
echo "重启 Cursor 或新开 Agent 会话后生效。"
