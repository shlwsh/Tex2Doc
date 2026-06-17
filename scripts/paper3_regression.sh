#!/usr/bin/env bash
# End-to-end paper3 regression: DOCX + AST dump + render dump + verify + traceability report.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${ROOT}/examples/paper3/output"
DOCX="${OUT}/main-jos-rust.docx"
PDF="${ROOT}/examples/paper3/latex/main-jos.pdf"
AST_JSON="${OUT}/main-jos.ast.json"
AST_MD="${OUT}/main-jos.ast.md"
RENDER_JSON="${OUT}/main-jos.render.json"
RENDER_MD="${OUT}/main-jos.render.md"
VERIFY_MD="${OUT}/main-jos.verify.md"
VERIFY_JSON="${OUT}/main-jos.verify.json"
TRACE_MD="${OUT}/main-jos.traceability.md"
TRACE_JSON="${OUT}/main-jos.traceability.json"

mkdir -p "${OUT}"

echo "=== Build Rust DOCX fixture ==="
cargo test -p doc-core --test paper3_e2e paper3_main_jos_to_docx -- --nocapture

echo "=== Dump Standard AST ==="
cargo run -p doc-engine -- ast-dump \
  --root "${ROOT}/examples/paper3/latex" \
  --main-tex main-jos.tex \
  --format json \
  --out "${AST_JSON}"
cargo run -p doc-engine -- ast-dump \
  --root "${ROOT}/examples/paper3/latex" \
  --main-tex main-jos.tex \
  --format md \
  --out "${AST_MD}"

echo "=== Dump DOCX render tree ==="
cargo run -p doc-engine -- render-dump \
  --root "${ROOT}/examples/paper3/latex" \
  --main-tex main-jos.tex \
  --format json \
  --out "${RENDER_JSON}"
cargo run -p doc-engine -- render-dump \
  --root "${ROOT}/examples/paper3/latex" \
  --main-tex main-jos.tex \
  --format md \
  --out "${RENDER_MD}"

echo "=== Verify DOCX ==="
VERIFY_STATUS=0
python3 "${ROOT}/scripts/verify_jos_docx.py" \
  --docx "${DOCX}" \
  --pdf "${PDF}" \
  --tex-root "${ROOT}/examples/paper3" \
  --format "${ROOT}/docs/format/jos_2025_docx_format_definitions.json" \
  --report "${VERIFY_MD}" \
  --json-report "${VERIFY_JSON}" || VERIFY_STATUS=$?

python3 "${ROOT}/scripts/validate_verify_report_schema.py" "${VERIFY_JSON}"

echo "=== Cross-layer traceability ==="
TRACE_STATUS=0
python3 "${ROOT}/scripts/quality_traceability_report.py" \
  --ast "${AST_JSON}" \
  --render "${RENDER_JSON}" \
  --docx "${DOCX}" \
  --pdf "${PDF}" \
  --verify "${VERIFY_JSON}" \
  --out "${TRACE_MD}" \
  --json-report "${TRACE_JSON}" || TRACE_STATUS=$?

echo "=== Outputs ==="
printf 'DOCX: %s\nAST: %s\nRender: %s\nVerify: %s\nTraceability: %s\n' \
  "${DOCX}" "${AST_JSON}" "${RENDER_JSON}" "${VERIFY_JSON}" "${TRACE_JSON}"

if [[ "${VERIFY_STATUS}" -ne 0 || "${TRACE_STATUS}" -ne 0 ]]; then
  echo "Regression generated reports but quality checks are not fully passing." >&2
  exit 1
fi
