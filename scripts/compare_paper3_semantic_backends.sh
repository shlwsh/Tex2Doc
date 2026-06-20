#!/usr/bin/env bash
# Compare semantic backend selection paths for examples/paper3.
#
# This script only exercises the new doc-compiler-engine path. It does not
# call or alter the existing doc-core conversion path.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PAPER3_DIR="$ROOT/examples/paper3"
LATEX_DIR="$PAPER3_DIR/latex"
MAIN_TEX="$LATEX_DIR/main-jos.tex"
OUT_DIR="$PAPER3_DIR/output/to-docx"
STAMP="$(date +%Y%m%d-%H%M%S)"
REPORT="$OUT_DIR/semantic-backends-${STAMP}-report.md"
STRICT="${STRICT:-0}"

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

media_count() {
  local docx="$1"
  if command -v zipinfo >/dev/null 2>&1; then
    zipinfo -1 "$docx" | grep -c '^word/media/' || true
  elif command -v unzip >/dev/null 2>&1; then
    unzip -Z1 "$docx" | grep -c '^word/media/' || true
  else
    echo "n/a"
  fi
}

run_backend() {
  local backend="$1"
  local slug="${backend//-/_}"
  local out_docx="$OUT_DIR/paper3-${STAMP}-${slug}.docx"
  local log="$OUT_DIR/paper3-${STAMP}-${slug}.log"
  local fallback_flag=()

  if [[ "$STRICT" == "1" ]]; then
    fallback_flag=(--no-backend-fallback)
  fi

  echo "=== backend: $backend ==="
  echo "out: $out_docx"

  if (cd "$ROOT" && cargo run -p doc-compiler-engine --example paper3_to_docx -- \
    --project-root "$LATEX_DIR" \
    --main-tex "$MAIN_TEX" \
    --profile jos-paper \
    --semantic-backend "$backend" \
    --out "$out_docx" \
    "${fallback_flag[@]}") >"$log" 2>&1; then
    local status="ok"
    local bytes
    bytes="$(stat -c%s "$out_docx" 2>/dev/null || stat -f%z "$out_docx")"
    local media
    media="$(media_count "$out_docx")"
    local selected
    selected="$(grep -m1 '^backend-selected:' "$log" | cut -d' ' -f2- || true)"
    local fallback_from
    fallback_from="$(grep -m1 '^backend-fallback-from:' "$log" | cut -d' ' -f2- || true)"
    printf '| `%s` | `%s` | `%s` | `%s` | %s | %s | [%s](./%s) | [%s](./%s) |\n' \
      "$backend" "$selected" "${fallback_from:-}" "$status" "$bytes" "$media" \
      "$(basename "$out_docx")" "$(basename "$out_docx")" "$(basename "$log")" "$(basename "$log")" >>"$REPORT"
  else
    local status="failed"
    printf '| `%s` |  |  | `%s` |  |  |  | [%s](./%s) |\n' \
      "$backend" "$status" "$(basename "$log")" "$(basename "$log")" >>"$REPORT"
    if [[ "$STRICT" == "1" ]]; then
      return 1
    fi
  fi
}

{
  echo "# paper3 Semantic Backend 对比报告"
  echo
  echo "- 时间戳：$STAMP"
  echo "- 主文件：$MAIN_TEX"
  echo "- 输出目录：$OUT_DIR"
  echo "- strict runtime：$STRICT"
  echo
  echo "| requested | selected | fallback_from | status | bytes | media | docx | log |"
  echo "|---|---|---|---|---:|---:|---|---|"
} >"$REPORT"

run_backend "auto"
run_backend "rule-based"
run_backend "xelatex-hook"
run_backend "luatex-node"

echo
echo "report: $REPORT"
