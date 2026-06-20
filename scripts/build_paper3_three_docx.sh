#!/usr/bin/env bash
# Build paper3 DOCX through three independent paths:
#   1. sh pipeline: scripts/build_docx.sh
#   2. rust-rule pipeline: doc-engine convert
#   3. semantic-engine pipeline: doc-compiler-engine example
#
# Usage:
#   bash scripts/build_paper3_three_docx.sh [VERSION] [ZIP]
#
# Environment:
#   SEMANTIC_BACKEND=xelatex-hook|luatex-node|rule-based|auto
#   STRICT_SEMANTIC=1|0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION_RAW="${1:-14}"
VERSION_NUM="${VERSION_RAW#v}"
VERSION_TAG="v${VERSION_NUM}"
PAPER3_DIR="$ROOT/examples/paper3"
LATEX_DIR="$PAPER3_DIR/latex"
MAIN_TEX="$LATEX_DIR/main-jos.tex"
OUT_DIR="$PAPER3_DIR/output/to-docx"
STAMP="$(date +%Y%m%d-%H%M%S)"
BASE="${VERSION_TAG}-论文稿件-jos-${STAMP}"
ZIP="${2:-$OUT_DIR/${BASE}-paper3-upload.zip}"
SEMANTIC_BACKEND="${SEMANTIC_BACKEND:-xelatex-hook}"
STRICT_SEMANTIC="${STRICT_SEMANTIC:-1}"
SEMANTIC_SLUG="${SEMANTIC_BACKEND//-/_}"

RUST_RULE_DOCX="$OUT_DIR/${BASE}-rust-rule.docx"
SEMANTIC_DOCX="$OUT_DIR/${BASE}-semantic-engine-${SEMANTIC_SLUG}.docx"
SH_LOG="$OUT_DIR/${BASE}-sh.log"
RUST_RULE_LOG="$OUT_DIR/${BASE}-rust-rule.log"
SEMANTIC_LOG="$OUT_DIR/${BASE}-semantic-engine-${SEMANTIC_SLUG}.log"
REPORT="$OUT_DIR/${BASE}-three-docx-report.md"
DOCX_TOOL="$ROOT/target/release/doc-engine"

mkdir -p "$OUT_DIR"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing dependency: $1" >&2
    exit 2
  }
}

latest_file() {
  local pattern="$1"
  find "$OUT_DIR" -maxdepth 1 -type f -name "$pattern" -printf '%T@ %p\n' 2>/dev/null \
    | sort -n \
    | tail -1 \
    | sed 's/^[0-9.]* //'
}

file_size() {
  local path="$1"
  stat -c%s "$path" 2>/dev/null || stat -f%z "$path"
}

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

build_zip() {
  local zip_path="$1"
  python3 - <<PY
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED

latex_dir = Path(r"$LATEX_DIR")
main_tex = Path(r"$MAIN_TEX")
paper3_dir = Path(r"$PAPER3_DIR")
out_zip = Path(r"$zip_path")
keep_suffixes = {".tex", ".bib", ".cls", ".bst", ".sty"}
figure_suffixes = {".png", ".jpg", ".jpeg", ".gif", ".pdf", ".eps", ".svg"}

if not main_tex.is_file():
    raise SystemExit(f"missing main tex: {main_tex}")

sources = [
    p for p in latex_dir.rglob("*")
    if p.is_file() and p.suffix.lower() in keep_suffixes
]
figures = [
    p for p in latex_dir.rglob("*")
    if p.is_file() and p.suffix.lower() in figure_suffixes
]
fig_dir = paper3_dir / "figures"
if fig_dir.is_dir():
    figures.extend(
        p for p in fig_dir.rglob("*")
        if p.is_file() and p.suffix.lower() in figure_suffixes
    )

out_zip.parent.mkdir(parents=True, exist_ok=True)
with ZipFile(out_zip, "w", ZIP_DEFLATED) as zf:
    for p in sorted(sources):
        rel = p.name if p.resolve() == main_tex.resolve() else p.relative_to(latex_dir).as_posix()
        zf.write(p, rel)
    for p in sorted(figures):
        try:
            rel = p.relative_to(paper3_dir).as_posix()
        except ValueError:
            rel = p.name
        zf.write(p, rel)

print(f"[paper3-zip] wrote {out_zip}")
print(f"[paper3-zip] sources={len(sources)} figures={len(figures)}")
PY
}

write_report_header() {
  {
    echo "# paper3 三路径 DOCX 验证报告"
    echo
    echo "- timestamp: $STAMP"
    echo "- version: $VERSION_TAG"
    echo "- main_tex: $MAIN_TEX"
    echo "- zip: $ZIP"
    echo "- output_dir: $OUT_DIR"
    echo "- semantic_backend: $SEMANTIC_BACKEND"
    echo "- strict_semantic: $STRICT_SEMANTIC"
    echo
    echo "| path | docx | bytes | media | log |"
    echo "|---|---|---:|---:|---|"
  } >"$REPORT"
}

append_report_row() {
  local label="$1"
  local docx="$2"
  local log="$3"
  local bytes
  local media
  bytes="$(file_size "$docx")"
  media="$(media_count "$docx")"
  printf '| `%s` | [%s](./%s) | %s | %s | [%s](./%s) |\n' \
    "$label" "$(basename "$docx")" "$(basename "$docx")" "$bytes" "$media" \
    "$(basename "$log")" "$(basename "$log")" >>"$REPORT"
}

need python3
need cargo

if [[ ! -f "$MAIN_TEX" ]]; then
  echo "missing main tex: $MAIN_TEX" >&2
  exit 2
fi

echo "=== prepare paper3 zip ==="
build_zip "$ZIP"

write_report_header

echo "=== build sh DOCX ==="
before_sh="$(latest_file "v${VERSION_NUM}-论文稿件-jos-sh-*.docx" || true)"
if bash "$ROOT/scripts/build_docx.sh" "$VERSION_NUM" >"$SH_LOG" 2>&1; then
  sh_docx="$(latest_file "v${VERSION_NUM}-论文稿件-jos-sh-*.docx")"
else
  echo "sh pipeline failed; see $SH_LOG" >&2
  exit 1
fi

if [[ -z "${sh_docx:-}" || ! -f "$sh_docx" || "$sh_docx" == "${before_sh:-}" ]]; then
  echo "sh DOCX was not generated; see $SH_LOG" >&2
  exit 1
fi
append_report_row "sh" "$sh_docx" "$SH_LOG"

echo "=== build rust-rule DOCX ==="
if [[ ! -x "$DOCX_TOOL" ]]; then
  (cd "$ROOT" && cargo build --release -p doc-engine) >"$RUST_RULE_LOG" 2>&1
fi

if [[ ! -x "$DOCX_TOOL" ]]; then
  echo "missing doc-engine binary: $DOCX_TOOL" >&2
  exit 2
fi

if "$DOCX_TOOL" convert \
  --zip "$ZIP" \
  --main-tex main-jos.tex \
  --page-setup jos-paper3 \
  --out "$RUST_RULE_DOCX" >>"$RUST_RULE_LOG" 2>&1; then
  :
else
  echo "rust-rule pipeline failed; see $RUST_RULE_LOG" >&2
  exit 1
fi

if [[ ! -f "$RUST_RULE_DOCX" ]]; then
  echo "rust-rule DOCX was not generated: $RUST_RULE_DOCX" >&2
  exit 1
fi
append_report_row "rust-rule" "$RUST_RULE_DOCX" "$RUST_RULE_LOG"

echo "=== build semantic-engine DOCX ==="
semantic_args=(
  --project-root "$LATEX_DIR"
  --main-tex "$MAIN_TEX"
  --profile jos-paper
  --semantic-backend "$SEMANTIC_BACKEND"
  --out "$SEMANTIC_DOCX"
)
if [[ "$STRICT_SEMANTIC" == "1" ]]; then
  semantic_args+=(--no-backend-fallback)
fi

if (cd "$ROOT" && cargo run -p doc-compiler-engine --example paper3_to_docx -- "${semantic_args[@]}") \
  >"$SEMANTIC_LOG" 2>&1; then
  :
else
  echo "semantic-engine pipeline failed; see $SEMANTIC_LOG" >&2
  exit 1
fi

if [[ ! -f "$SEMANTIC_DOCX" ]]; then
  echo "semantic-engine DOCX was not generated: $SEMANTIC_DOCX" >&2
  exit 1
fi
append_report_row "semantic-engine" "$SEMANTIC_DOCX" "$SEMANTIC_LOG"

{
  echo
  echo "## Semantic Backend"
  echo
  grep -E '^(backend-(requested|selected|fallback-from|reason)|profile-(id|page-setup)|reference-(labels|edges)|citations|unresolved-references|bookmarks|hyperlinks|omml-equations|omml-equation-fallbacks):' "$SEMANTIC_LOG" || true
} >>"$REPORT"

echo "=== done ==="
echo "sh              : $sh_docx"
echo "rust-rule       : $RUST_RULE_DOCX"
echo "semantic-engine : $SEMANTIC_DOCX"
echo "report          : $REPORT"
