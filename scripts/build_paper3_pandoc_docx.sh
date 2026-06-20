#!/usr/bin/env bash
# ============================================================================
# build_paper3_pandoc_docx.sh - 用 pandoc 把 paper3 LaTeX 转成 DOCX。
#
# 与 build_paper3_dual_docx.sh 一起使用时，会在同一目录下产出：
#   - v${VERSION}-论文稿件-jos-${TS}-sh.docx        (Python 路径)
#   - v${VERSION}-论文稿件-jos-${TS}-rust.docx      (rust doc-engine 路径)
#   - v${VERSION}-论文稿件-jos-${TS}-pandoc.docx    (pandoc 路径, 本脚本)
#
# 路径策略：
#   - 复用 paper3/upload.zip 同款 LaTeX 资源（main-jos.tex + sections/zh/*.tex）
#   - 把 \input{sections/...} 递归展开为单一 master.tex，pandoc latex reader
#     才能一次性看到全部内容（pandoc 不支持递归 \input）
#   - 把 \graphicspath{{../figures/}} 改写为 \includegraphics{绝对路径}，
#     让 pandoc 找得到图
#
# 用法：
#   bash scripts/build_paper3_pandoc_docx.sh [VERSION] [TS]
#     VERSION  - 论文版本号（默认 v12）
#     TS       - 时间戳（默认当前 YYYYMMDD-HHMMSS），dual 脚本会传同值保证命名一致
#
# 输出：examples/paper3/output/to-docx/${BASE}-pandoc.docx
# ============================================================================
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-v12}"
TS="${2:-$(date +%Y%m%d-%H%M%S)}"
PAPER3_DIR="$ROOT/examples/paper3"
LATEX_DIR="$PAPER3_DIR/latex"
MAIN_TEX="$LATEX_DIR/main-jos.tex"
FIG_DIR="$PAPER3_DIR/figures"
OUT_DIR="$PAPER3_DIR/output/to-docx"
BASE="${VERSION}-论文稿件-jos-${TS}"
PANDOC_DOCX="$OUT_DIR/${BASE}-pandoc.docx"
WORK_DIR="$(mktemp -d)"

check_deps() {
    local missing=()
    command -v pandoc >/dev/null 2>&1 || missing+=("pandoc (apt install pandoc)")
    command -v python3 >/dev/null 2>&1 || missing+=("python3")
    [[ -f "$MAIN_TEX" ]] || missing+=("$MAIN_TEX")
    [[ -d "$FIG_DIR" ]] || missing+=("$FIG_DIR")
    if [[ ${#missing[@]} -gt 0 ]]; then
        echo "❌ missing dependencies:" >&2
        for dep in "${missing[@]}"; do
            echo "  - $dep" >&2
        done
        exit 2
    fi
}

echo "=== building pandoc DOCX (v${VERSION} @ ${TS}) ==="
check_deps
mkdir -p "$OUT_DIR"

# ---------- 1. 构造 master.tex：展开 \input + 改写 \includegraphics ----------
MASTER_TEX="$WORK_DIR/master.tex"
python3 - "$LATEX_DIR" "$FIG_DIR" "$MAIN_TEX" "$MASTER_TEX" <<'PY'
"""从 paper3 LaTeX 项目构造 pandoc 友好的单一 master tex。

  - 递归展开 \\input{...}（pandoc latex reader 不递归）
  - 去掉 \\graphicspath{...}（pandoc 不解析）
  - 把 \\includegraphics{X} 改写为绝对路径，pandoc 才能嵌入图
  - 自动给无后缀的 \\input{path} 加 .tex
"""
import re
import sys
from pathlib import Path

latex_dir = Path(sys.argv[1]).resolve()
fig_dir = Path(sys.argv[2]).resolve()
main_tex = Path(sys.argv[3])
out_path = Path(sys.argv[4])

text = main_tex.read_text(encoding="utf-8")

# 1) \input{...} 递归展开
def expand_input(match):
    rel = match.group(1).strip()
    target = (latex_dir / rel).resolve()
    if not target.is_file() and not target.suffix:
        target = target.with_suffix(".tex")
    if target.is_file():
        return (
            f"\n% === expanded from {match.group(1)} ===\n"
            + target.read_text(encoding="utf-8")
            + f"\n% === end {match.group(1)} ===\n"
        )
    print(f"[pandoc] WARN: cannot expand \\input{{{rel}}} (not a file)", file=sys.stderr)
    return match.group(0)

text = re.sub(r"\\input\{([^}]+)\}", expand_input, text)

# 2) 去掉 \graphicspath{...}（pandoc 不解析，保留会触发警告）
text = re.sub(r"\\graphicspath\{[^}]*\}", "", text)

# 3) \includegraphics[opts]{X} → \includegraphics[opts]{绝对路径}
def rewrite_includegraphics(match):
    opts = match.group(1) or ""
    target = match.group(2)
    fig_path = (fig_dir / target).resolve()
    if fig_path.is_file():
        return f"\\includegraphics{opts}{{{fig_path}}}"
    print(f"[pandoc] WARN: image not found: {target}", file=sys.stderr)
    return match.group(0)

text = re.sub(
    r"\\includegraphics(\[[^\]]*\])?\{([^}]+)\}",
    rewrite_includegraphics, text,
)

out_path.write_text(text, encoding="utf-8")
print(f"[pandoc] master tex: {len(text)} chars → {out_path}")
PY

# ---------- 2. 调 pandoc：tex → docx ----------
echo "=== invoking pandoc ==="
pandoc \
    --from=latex \
    --to=docx \
    --resource-path="$LATEX_DIR" \
    --output="$PANDOC_DOCX" \
    "$MASTER_TEX" 2>&1 | sed 's/^/[pandoc] /' || {
        echo "❌ pandoc failed" >&2
        rm -rf "$WORK_DIR"
        exit 1
    }

if [[ ! -f "$PANDOC_DOCX" ]]; then
    echo "❌ pandoc DOCX not generated" >&2
    rm -rf "$WORK_DIR"
    exit 1
fi

FILE_SIZE=$(stat -c%s "$PANDOC_DOCX" 2>/dev/null || stat -f%z "$PANDOC_DOCX")
echo "✓ $(basename "$PANDOC_DOCX") (${FILE_SIZE} bytes)"

# ---------- 3. 清理 ----------
rm -rf "$WORK_DIR"

echo "$PANDOC_DOCX"
