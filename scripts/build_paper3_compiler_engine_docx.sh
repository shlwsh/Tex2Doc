#!/usr/bin/env bash
# 用 doc-compiler-engine 把 examples/paper3/latex/main-jos.tex 转成 DOCX。
#
# 输出目录：
#   examples/paper3/output/to-docx
#
# 用法：
#   bash scripts/build_paper3_compiler_engine_docx.sh [VERSION]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-v13}"
PAPER3_DIR="$ROOT/examples/paper3"
LATEX_DIR="$PAPER3_DIR/latex"
MAIN_TEX="$LATEX_DIR/main-jos.tex"
OUT_DIR="$PAPER3_DIR/output/to-docx"
STAMP="$(date +%Y%m%d-%H%M%S)"
OUT_DOCX="$OUT_DIR/${VERSION}-论文稿件-jos-${STAMP}-compiler-engine.docx"

command -v cargo >/dev/null 2>&1 || {
  echo "missing dependency: cargo" >&2
  exit 2
}

if [[ ! -d "$LATEX_DIR" ]]; then
  echo "missing paper3 latex directory: $LATEX_DIR" >&2
  exit 2
fi

if [[ ! -f "$MAIN_TEX" ]]; then
  echo "missing main tex: $MAIN_TEX" >&2
  exit 2
fi

mkdir -p "$OUT_DIR"

echo "=== building paper3 DOCX via doc-compiler-engine ==="
echo "main : $MAIN_TEX"
echo "out  : $OUT_DOCX"

(cd "$ROOT" && cargo run -p doc-compiler-engine --example paper3_to_docx -- \
  --project-root "$LATEX_DIR" \
  --main-tex "$MAIN_TEX" \
  --profile jos-paper \
  --out "$OUT_DOCX")

if [[ ! -f "$OUT_DOCX" ]]; then
  echo "DOCX not generated: $OUT_DOCX" >&2
  exit 1
fi

SIZE="$(stat -c%s "$OUT_DOCX" 2>/dev/null || stat -f%z "$OUT_DOCX")"
echo "=== done ==="
echo "$OUT_DOCX ($SIZE bytes)"
