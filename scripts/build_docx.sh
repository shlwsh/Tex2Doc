#!/usr/bin/env bash
# ============================================================================
# build_docx.sh - Convert examples/paper3/latex/main-jos.tex to JOS DOCX.
#
# This ports the p3-microservice JOS pipeline into Tex2Doc:
#   1. ensure the LaTeX side products needed by the parser exist (PDF/BBL);
#   2. parse paper3 LaTeX, figures, tables, algorithms, references;
#   3. emit WordprocessingML using docs/format/jos_2025_docx_format_definitions.json;
#   4. verify the generated DOCX against the current TeX/PDF/format profile.
#
# Usage:
#   ./scripts/build_docx.sh [version]
# ============================================================================
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPTS="${ROOT}/scripts"
PAPER_ROOT="${ROOT}/examples/paper3"
LATEX_DIR="${PAPER_ROOT}/latex"
FORMAT_JSON="${ROOT}/docs/format/jos_2025_docx_format_definitions.json"
TEX_SRC="${LATEX_DIR}/main-jos.tex"
PDF_SRC="${LATEX_DIR}/main-jos.pdf"
BBL_SRC="${LATEX_DIR}/main-jos.bbl"
OUTPUT_DIR="${PAPER_ROOT}/output/to-docx"
INPUT_MANIFEST="${LATEX_DIR}/.main-jos.inputs.sha256"

check_deps() {
    local missing=()

    command -v python3 >/dev/null 2>&1 || missing+=("python3")
    command -v pdftotext >/dev/null 2>&1 || missing+=("pdftotext (apt install poppler-utils)")
    command -v pdftoppm >/dev/null 2>&1 || missing+=("pdftoppm (apt install poppler-utils)")
    command -v latexmk >/dev/null 2>&1 || missing+=("latexmk")
    command -v xelatex >/dev/null 2>&1 || missing+=("xelatex")
    command -v bibtex >/dev/null 2>&1 || missing+=("bibtex")
    command -v sha256sum >/dev/null 2>&1 || missing+=("sha256sum")

    if ! python3 -c "from PIL import Image" 2>/dev/null; then
        missing+=("Pillow/PIL (used to read image dimensions)")
    fi
    [[ -f "${FORMAT_JSON}" ]] || missing+=("${FORMAT_JSON}")
    [[ -f "${TEX_SRC}" ]] || missing+=("${TEX_SRC}")

    if [[ ${#missing[@]} -gt 0 ]]; then
        echo "Missing dependencies or inputs:"
        for dep in "${missing[@]}"; do
            echo "  - ${dep}"
        done
        exit 1
    fi
}

write_input_manifest() {
    (
        cd "${ROOT}"
        {
            find "examples/paper3/latex" -type f \
                \( -name '*.tex' -o -name '*.bib' -o -name '*.bst' -o -name '*.cls' -o -name '*.sty' \) \
                -print
            printf '%s\n' "docs/format/jos_2025_docx_format_definitions.json"
        } | sort | while IFS= read -r path; do
            sha256sum "${path}"
        done
    )
}

latex_inputs_unchanged() {
    local tmp_manifest="$1"
    [[ -f "${INPUT_MANIFEST}" ]] && cmp -s "${tmp_manifest}" "${INPUT_MANIFEST}"
}

ensure_latex_outputs() {
    local tmp_manifest
    tmp_manifest="$(mktemp)"
    trap 'rm -f "${tmp_manifest}"' RETURN
    write_input_manifest >"${tmp_manifest}"

    if [[ -f "${PDF_SRC}" && -f "${BBL_SRC}" ]] && latex_inputs_unchanged "${tmp_manifest}"; then
        echo "  - Reusing ${PDF_SRC} and ${BBL_SRC}"
        return
    fi

    echo "  - Building PDF/BBL with latexmk"
    (
        cd "${LATEX_DIR}"
        latexmk -xelatex -bibtex -interaction=nonstopmode -halt-on-error main-jos.tex
    )
    cp "${tmp_manifest}" "${INPUT_MANIFEST}"
    echo "  - Updated input manifest ${INPUT_MANIFEST}"
}

echo "=== Check dependencies and inputs ==="
check_deps
echo "  - OK"

echo "=== Ensure LaTeX PDF/BBL ==="
ensure_latex_outputs

mkdir -p "${OUTPUT_DIR}"

if [[ -n "${1:-}" ]]; then
    VERSION="$1"
else
    MAX_V="$(find "${OUTPUT_DIR}" -type f -name 'v*-论文稿件-jos-*' -printf '%f\n' 2>/dev/null \
        | sed -n 's/^v\([0-9]\+\)-论文稿件-jos-.*/\1/p' \
        | sort -n \
        | tail -1 || true)"
    VERSION=$(( ${MAX_V:-0} + 1 ))
fi

TS="$(date +%Y%m%d-%H%M%S)"
DOCX_DST="${OUTPUT_DIR}/v${VERSION}-论文稿件-jos-sh-${TS}.docx"
REPORT_DST="${OUTPUT_DIR}/v${VERSION}-论文稿件-jos-sh-${TS}-docx校验报告.md"
REPORT_JSON="${OUTPUT_DIR}/v${VERSION}-论文稿件-jos-sh-${TS}-docx校验报告.json"

echo "=== Version: v${VERSION} @ ${TS} ==="
echo "=== Generate DOCX ==="
python3 "${SCRIPTS}/build_jos_docx.py" \
    --root "${PAPER_ROOT}" \
    --format "${FORMAT_JSON}" \
    --output "${DOCX_DST}"

echo "=== Verify DOCX against TeX/PDF/format ==="
python3 "${SCRIPTS}/verify_jos_docx.py" \
    --docx "${DOCX_DST}" \
    --pdf "${PDF_SRC}" \
    --tex-root "${PAPER_ROOT}" \
    --format "${FORMAT_JSON}" \
    --report "${REPORT_DST}" \
    --json-report "${REPORT_JSON}"

FILE_SIZE=$(stat -c%s "${DOCX_DST}" 2>/dev/null || stat -f%z "${DOCX_DST}")

echo ""
echo "=== Done ==="
echo "  DOCX: ${DOCX_DST}"
echo "  Report: ${REPORT_DST}"
echo "  Size: $(numfmt --to=iec "${FILE_SIZE}" 2>/dev/null || echo "${FILE_SIZE} bytes")"
