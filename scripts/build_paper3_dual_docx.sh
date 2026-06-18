#!/usr/bin/env bash
# 生成 paper3 的双版本 DOCX：sh 版 + rust 版。
#
# 输出到：examples/paper3/output/to-docx
# 用法：
#   bash scripts/build_paper3_dual_docx.sh [VERSION] [ZIP]
#
# 说明：
# - sh 版本使用 V1 默认页面设置（Letter）
# - rust 版本使用当前 doc-engine 的 page-setup / header-footer 逻辑
# - 若 ZIP 不存在，会从 examples/paper3/latex/main-jos.tex 所在目录自动生成 upload.zip
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-v12}"
PAPER3_DIR="$ROOT/examples/paper3"
LATEX_DIR="$PAPER3_DIR/latex"
MAIN_TEX="$LATEX_DIR/main-jos.tex"
ZIP="${2:-$PAPER3_DIR/upload.zip}"
OUT_DIR="$PAPER3_DIR/output/to-docx"
STAMP="$(date +%Y%m%d-%H%M%S)"
BASE="${VERSION}-论文稿件-jos-${STAMP}"
SH_DOCX="$OUT_DIR/${BASE}-sh.docx"
RUST_DOCX="$OUT_DIR/${BASE}-rust.docx"
DOCX_TOOL="$ROOT/target/release/doc-engine"

mkdir -p "$OUT_DIR"

need() {
  command -v "$1" >/dev/null 2>&1 || { echo "❌ missing: $1" >&2; exit 3; }
}

need cargo
need python3

if [[ ! -f "$ZIP" ]]; then
  echo "=== zip not found; building upload.zip from $MAIN_TEX ==="
  if [[ ! -f "$MAIN_TEX" ]]; then
    echo "❌ missing main tex: $MAIN_TEX" >&2
    exit 3
  fi
  ZIP="$PAPER3_DIR/upload.zip"
  python3 - <<PY
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED

latex_dir = Path(r"$LATEX_DIR")
main_tex = Path(r"$MAIN_TEX")
out_zip = Path(r"$ZIP")
keep_suffixes = {".tex", ".bib", ".cls", ".bst", ".sty"}

if not main_tex.is_file():
    raise SystemExit(f"missing main tex: {main_tex}")

files = [p for p in latex_dir.rglob("*") if p.is_file() and p.suffix.lower() in keep_suffixes]
if not files:
    raise SystemExit(f"no tex assets found under: {latex_dir}")

out_zip.parent.mkdir(parents=True, exist_ok=True)
with ZipFile(out_zip, "w", ZIP_DEFLATED) as zf:
    for p in sorted(files):
        zf.write(p, p.relative_to(latex_dir).as_posix())

print(f"[paper3-zip] wrote {out_zip}")
print(f"[paper3-zip] entries = {len(files)}")
PY
fi

if [[ ! -x "$DOCX_TOOL" ]]; then
  echo "=== building doc-engine (release) ==="
  (cd "$ROOT" && cargo build --release -p doc-engine)
fi

if [[ ! -x "$DOCX_TOOL" ]]; then
  echo "❌ missing tool: $DOCX_TOOL" >&2
  exit 3
fi

echo "=== generating sh DOCX ==="
"$DOCX_TOOL" convert \
  --zip "$ZIP" \
  --main-tex main-jos.tex \
  --page-setup letter \
  --out "$SH_DOCX"

echo "=== generating rust DOCX ==="
"$DOCX_TOOL" convert \
  --zip "$ZIP" \
  --main-tex main-jos.tex \
  --page-setup jos-paper3 \
  --out "$RUST_DOCX"

for f in "$SH_DOCX" "$RUST_DOCX"; do
  if [[ ! -f "$f" ]]; then
    echo "❌ DOCX not generated: $f" >&2
    exit 1
  fi
  echo "✓ $(basename "$f")"
done

python3 - <<PY
from pathlib import Path
sh = Path(r"$SH_DOCX")
rust = Path(r"$RUST_DOCX")
print(f"sh  : {sh} ({sh.stat().st_size} bytes)")
print(f"rust: {rust} ({rust.stat().st_size} bytes)")
PY

echo "=== done ==="
