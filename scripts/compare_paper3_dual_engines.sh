#!/usr/bin/env bash
# Compare the existing Rust rule engine and the new Semantic TeX Engine on
# examples/paper3 without coupling the two implementations.
#
# Usage:
#   bash scripts/compare_paper3_dual_engines.sh [VERSION] [ZIP]
#
# Environment:
#   SEMANTIC_BACKEND=auto|xelatex-hook|luatex-node|rule-based
#   STRICT_SEMANTIC=1|0
#   KEY_PHRASES='phrase1|phrase2|...'
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION_RAW="${1:-15}"
VERSION_NUM="${VERSION_RAW#v}"
VERSION_TAG="v${VERSION_NUM}"
PAPER3_DIR="$ROOT/examples/paper3"
LATEX_DIR="$PAPER3_DIR/latex"
MAIN_TEX="$LATEX_DIR/main-jos.tex"
OUT_DIR="$PAPER3_DIR/output/to-docx"
STAMP="$(date +%Y%m%d-%H%M%S)"
BASE="${VERSION_TAG}-论文稿件-jos-${STAMP}-dual-engines"
ZIP="${2:-$OUT_DIR/${BASE}-paper3-upload.zip}"
SEMANTIC_BACKEND="${SEMANTIC_BACKEND:-auto}"
STRICT_SEMANTIC="${STRICT_SEMANTIC:-0}"
SEMANTIC_SLUG="${SEMANTIC_BACKEND//-/_}"

RUST_RULE_DOCX="$OUT_DIR/${BASE}-rust-rule.docx"
SEMANTIC_DOCX="$OUT_DIR/${BASE}-semantic-engine-${SEMANTIC_SLUG}.docx"
RUST_RULE_LOG="$OUT_DIR/${BASE}-rust-rule.log"
SEMANTIC_LOG="$OUT_DIR/${BASE}-semantic-engine-${SEMANTIC_SLUG}.log"
REPORT="$OUT_DIR/${BASE}-comparison-report.md"
RUST_TEXT="$OUT_DIR/${BASE}-rust-rule-document.txt"
SEMANTIC_TEXT="$OUT_DIR/${BASE}-semantic-engine-${SEMANTIC_SLUG}-document.txt"
TEXT_DIFF="$OUT_DIR/${BASE}-document-text.diff"
DOCX_TOOL="$ROOT/target/release/doc-engine"
KEY_PHRASES="${KEY_PHRASES:-基于动态关注清单|微服务日志|Dynamic Attention List|DASM|Loki|DSB-Lite|系统总体设计|实验与分析}"

mkdir -p "$OUT_DIR"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing dependency: $1" >&2
    exit 2
  }
}

build_zip() {
  export LATEX_DIR MAIN_TEX PAPER3_DIR ZIP
  python3 - <<'PY'
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile
import os

latex_dir = Path(os.environ["LATEX_DIR"])
main_tex = Path(os.environ["MAIN_TEX"])
paper3_dir = Path(os.environ["PAPER3_DIR"])
out_zip = Path(os.environ["ZIP"])
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

analyze_docx_pair() {
  export RUST_RULE_DOCX SEMANTIC_DOCX RUST_TEXT SEMANTIC_TEXT TEXT_DIFF REPORT
  export RUST_RULE_LOG SEMANTIC_LOG SEMANTIC_BACKEND STRICT_SEMANTIC KEY_PHRASES
  export MAIN_TEX ZIP OUT_DIR STAMP VERSION_TAG
  python3 - <<'PY'
from pathlib import Path
from zipfile import ZipFile
import difflib
import os
import re
import xml.etree.ElementTree as ET

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
WP = "{http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing}"

rust_docx = Path(os.environ["RUST_RULE_DOCX"])
semantic_docx = Path(os.environ["SEMANTIC_DOCX"])
rust_text_path = Path(os.environ["RUST_TEXT"])
semantic_text_path = Path(os.environ["SEMANTIC_TEXT"])
diff_path = Path(os.environ["TEXT_DIFF"])
report_path = Path(os.environ["REPORT"])
phrases = [p for p in os.environ["KEY_PHRASES"].split("|") if p]


def read_backend_lines(path: Path) -> list[str]:
    if not path.is_file():
        return []
    wanted = (
        "backend-requested:",
        "backend-selected:",
        "backend-fallback-from:",
        "backend-reason:",
        "profile-id:",
        "profile-page-setup:",
        "reference-labels:",
        "reference-edges:",
        "citations:",
        "unresolved-references:",
        "bookmarks:",
        "hyperlinks:",
    )
    return [
        line.strip()
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines()
        if line.startswith(wanted)
    ]


def docx_metrics(path: Path) -> dict:
    with ZipFile(path) as zf:
        names = zf.namelist()
        document_xml = zf.read("word/document.xml")

    root = ET.fromstring(document_xml)
    paragraphs = []
    for para in root.iter(f"{W}p"):
        parts = []
        for node in para.iter():
            if node.tag == f"{W}t" and node.text:
                parts.append(node.text)
            elif node.tag == f"{W}tab":
                parts.append("\t")
            elif node.tag in {f"{W}br", f"{W}cr"}:
                parts.append("\n")
        text = "".join(parts)
        text = re.sub(r"[ \t]+", " ", text).strip()
        if text:
            paragraphs.append(text)

    full_text = "\n".join(paragraphs)
    plain_lines = [line for line in paragraphs if line.strip()]
    return {
        "bytes": path.stat().st_size,
        "parts": len(names),
        "media": sum(1 for name in names if name.startswith("word/media/")),
        "paragraphs": len(paragraphs),
        "tables": sum(1 for _ in root.iter(f"{W}tbl")),
        "drawings": sum(1 for _ in root.iter(f"{W}drawing")) + sum(1 for _ in root.iter(f"{WP}inline")),
        "chars": len(full_text),
        "text": full_text,
        "lines": plain_lines,
        "summary": plain_lines[:12],
    }


def count_phrase(text: str, phrase: str) -> int:
    return text.count(phrase)


rust = docx_metrics(rust_docx)
semantic = docx_metrics(semantic_docx)

rust_text_path.write_text(rust["text"] + "\n", encoding="utf-8")
semantic_text_path.write_text(semantic["text"] + "\n", encoding="utf-8")

diff_lines = list(
    difflib.unified_diff(
        rust["lines"],
        semantic["lines"],
        fromfile=rust_docx.name,
        tofile=semantic_docx.name,
        lineterm="",
        n=3,
    )
)
diff_path.write_text("\n".join(diff_lines) + ("\n" if diff_lines else ""), encoding="utf-8")
changed_lines = sum(
    1
    for line in diff_lines
    if line and line[0] in {"+", "-"} and not line.startswith(("+++", "---"))
)
hunks = sum(1 for line in diff_lines if line.startswith("@@"))


def row(label: str, docx: Path, metrics: dict, log: Path) -> str:
    return (
        f"| `{label}` | [{docx.name}](./{docx.name}) | {metrics['bytes']} | "
        f"{metrics['parts']} | {metrics['media']} | {metrics['paragraphs']} | "
        f"{metrics['tables']} | {metrics['drawings']} | {metrics['chars']} | "
        f"[{log.name}](./{log.name}) |"
    )


def phrase_row(phrase: str) -> str:
    rust_count = count_phrase(rust["text"], phrase)
    semantic_count = count_phrase(semantic["text"], phrase)
    status = "ok" if rust_count > 0 and semantic_count > 0 else "missing"
    return f"| `{phrase}` | {rust_count} | {semantic_count} | `{status}` |"


with report_path.open("w", encoding="utf-8") as f:
    f.write("# paper3 双引擎 DOCX 对比报告\n\n")
    f.write(f"- timestamp: {os.environ['STAMP']}\n")
    f.write(f"- version: {os.environ['VERSION_TAG']}\n")
    f.write(f"- main_tex: {os.environ['MAIN_TEX']}\n")
    f.write(f"- zip: {os.environ['ZIP']}\n")
    f.write(f"- output_dir: {os.environ['OUT_DIR']}\n")
    f.write(f"- semantic_backend: {os.environ['SEMANTIC_BACKEND']}\n")
    f.write(f"- strict_semantic: {os.environ['STRICT_SEMANTIC']}\n\n")

    f.write("## DOCX 结构摘要\n\n")
    f.write("| engine | docx | bytes | zip parts | media | paragraphs | tables | drawings | text chars | log |\n")
    f.write("|---|---|---:|---:|---:|---:|---:|---:|---:|---|\n")
    f.write(row("rust-rule", rust_docx, rust, Path(os.environ["RUST_RULE_LOG"])) + "\n")
    f.write(row("semantic-engine", semantic_docx, semantic, Path(os.environ["SEMANTIC_LOG"])) + "\n\n")

    f.write("## Semantic Backend\n\n")
    backend_lines = read_backend_lines(Path(os.environ["SEMANTIC_LOG"]))
    if backend_lines:
        f.write("```text\n")
        f.write("\n".join(backend_lines))
        f.write("\n```\n\n")
    else:
        f.write("未在 semantic log 中发现 backend 报告行。\n\n")

    f.write("## 关键短语命中\n\n")
    f.write("| phrase | rust-rule | semantic-engine | status |\n")
    f.write("|---|---:|---:|---|\n")
    for phrase in phrases:
        f.write(phrase_row(phrase) + "\n")
    f.write("\n")

    f.write("## document.xml 文本摘要\n\n")
    f.write("### rust-rule\n\n")
    for line in rust["summary"]:
        f.write(f"- {line[:220]}\n")
    f.write("\n### semantic-engine\n\n")
    for line in semantic["summary"]:
        f.write(f"- {line[:220]}\n")
    f.write("\n")

    f.write("## 文本差异\n\n")
    f.write(f"- diff_file: [{diff_path.name}](./{diff_path.name})\n")
    f.write(f"- rust_text: [{rust_text_path.name}](./{rust_text_path.name})\n")
    f.write(f"- semantic_text: [{semantic_text_path.name}](./{semantic_text_path.name})\n")
    f.write(f"- hunks: {hunks}\n")
    f.write(f"- changed_lines: {changed_lines}\n")
PY
}

need python3
need cargo

if [[ ! -f "$MAIN_TEX" ]]; then
  echo "missing main tex: $MAIN_TEX" >&2
  exit 2
fi

echo "=== prepare paper3 zip ==="
build_zip

echo "=== build rust-rule DOCX ==="
if [[ ! -x "$DOCX_TOOL" ]]; then
  (cd "$ROOT" && cargo build --release -p doc-engine) >"$RUST_RULE_LOG" 2>&1
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

echo "=== analyze DOCX pair ==="
analyze_docx_pair

echo "=== done ==="
echo "rust-rule       : $RUST_RULE_DOCX"
echo "semantic-engine : $SEMANTIC_DOCX"
echo "report          : $REPORT"
echo "text diff       : $TEXT_DIFF"
