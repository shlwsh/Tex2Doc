#!/usr/bin/env bash
# End-to-end paper3 regression: DOCX + AST dump + render dump + verify + traceability report.
#
# v12: 接受 VERSION 环境变量作为产物前缀,默认 v12-<timestamp>。
# 用法: VERSION=v12-20260618-070000 ./scripts/paper3_regression.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${VERSION:-v12-$(date +%Y%m%d-%H%M%S)}"
OUT="${ROOT}/examples/paper3/output"
TO_DOCX_DIR="${OUT}/to-docx"
VERIFY_DIR="${ROOT}/docs/verify"
DOCX="${TO_DOCX_DIR}/${VERSION}-论文稿件-jos-rust.docx"
PDF="${ROOT}/examples/paper3/latex/main-jos.pdf"
AST_JSON="${OUT}/${VERSION}-main-jos.ast.json"
AST_MD="${OUT}/${VERSION}-main-jos.ast.md"
RENDER_JSON="${OUT}/${VERSION}-main-jos.render.json"
RENDER_MD="${OUT}/${VERSION}-main-jos.render.md"
VERIFY_MD="${VERIFY_DIR}/${VERSION}-docx-compare.md"
VERIFY_JSON="${VERIFY_DIR}/${VERSION}-docx-compare.json"
PER_TABLE_MD="${VERIFY_DIR}/${VERSION}-逐项对比表.md"
TRACE_MD="${OUT}/${VERSION}-main-jos.traceability.md"
TRACE_JSON="${OUT}/${VERSION}-main-jos.traceability.json"

mkdir -p "${OUT}" "${TO_DOCX_DIR}" "${VERIFY_DIR}"

echo "=== Build Rust DOCX fixture (version=${VERSION}) ==="
DOCX_ENV="${DOCX}" cargo test -p doc-core --test paper3_e2e paper3_main_jos_to_docx -- --nocapture

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
