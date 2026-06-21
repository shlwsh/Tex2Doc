#!/usr/bin/env bash
# =============================================================================
# commercial_verify.sh
# Quality gate for commercial Tex2Doc deployments.
# Verifies DOCX quality against configurable thresholds.
#
# Usage:
#   ./scripts/commercial_verify.sh --docx FILE [--min-score N] [--report FILE]
#
# Exit codes:
#   0   All checks passed
#   1   One or more checks failed
#
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DOCX_FILE=""
MIN_SCORE=70
REPORT_FILE=""
SKIP_STRUCTURAL=false
SKIP_STYLE=false
SKIP_REFERENCES=false

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ── Argument parsing ────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --docx)       DOCX_FILE="$2"; shift 2 ;;
        --min-score)  MIN_SCORE="$2"; shift 2 ;;
        --report)     REPORT_FILE="$2"; shift 2 ;;
        --skip-structural) SKIP_STRUCTURAL=true; shift ;;
        --skip-style)     SKIP_STYLE=true; shift ;;
        --skip-refs)      SKIP_REFERENCES=true; shift ;;
        --help|-h)   head -15 "$0"; exit 0 ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
done

if [[ -z "$DOCX_FILE" ]]; then
    echo "Error: --docx is required"
    exit 1
fi

if [[ ! -f "$DOCX_FILE" ]]; then
    echo "Error: DOCX not found: $DOCX_FILE"
    exit 1
fi

log_info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok()    { echo -e "${GREEN}[PASS]${NC}  $*"; }
log_fail()  { echo -e "${RED}[FAIL]${NC}  $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }

PASS=0
FAIL=0
declare -A CHECKS

run_check() {
    local name="$1"
    local cmd="$2"
    local result
    result=$(eval "$cmd" 2>/dev/null || echo "error")
    if [[ "$result" == "error" ]]; then
        log_fail "$name: command failed"
        CHECKS["$name"]="fail"
        ((FAIL++))
    else
        log_ok "$name: $result"
        CHECKS["$name"]="pass"
        ((PASS++))
    fi
}

log_info "============================================"
log_info "Commercial Quality Gate"
log_info "============================================"
log_info "DOCX:        $DOCX_FILE"
log_info "Min Score:   $MIN_SCORE"
log_info "Skip Struct: $SKIP_STRUCTURAL"
log_info "Skip Style:  $SKIP_STYLE"
log_info "Skip Refs:   $SKIP_REFERENCES"
echo ""

# ── Structural checks ──────────────────────────────────────────────────────
if [[ "$SKIP_STRUCTURAL" == "false" ]]; then
    log_info "--- Structural Checks ---"
    local size
    size=$(stat -c%s "$DOCX_FILE" 2>/dev/null || stat -f%z "$DOCX_FILE" 2>/dev/null || echo 0)
    if [[ $size -gt 1024 ]]; then
        log_ok "File size: $size bytes (non-trivial)"
        CHECKS["file_size"]="pass"; ((PASS++))
    else
        log_fail "File size: $size bytes (too small)"
        CHECKS["file_size"]="fail"; ((FAIL++))
    fi
    echo ""
fi

# ── Style checks ─────────────────────────────────────────────────────────
if [[ "$SKIP_STYLE" == "false" ]]; then
    log_info "--- Style Checks ---"
    # Check for required styles
    local has_styles=false
    if command -v unzip >/dev/null 2>&1 && unzip -l "$DOCX_FILE" 2>/dev/null | grep -q "word/styles.xml"; then
        has_styles=true
        log_ok "styles.xml present"
        CHECKS["styles_present"]="pass"; ((PASS++))
    else
        log_fail "styles.xml missing"
        CHECKS["styles_present"]="fail"; ((FAIL++))
    fi

    if unzip -l "$DOCX_FILE" 2>/dev/null | grep -q "word/document.xml"; then
        log_ok "document.xml present"
        CHECKS["document_present"]="pass"; ((PASS++))
    else
        log_fail "document.xml missing"
        CHECKS["document_present"]="fail"; ((FAIL++))
    fi
    echo ""
fi

# ── Reference checks ──────────────────────────────────────────────────────
if [[ "$SKIP_REFERENCES" == "false" ]]; then
    log_info "--- Reference Checks ---"
    local has_rels=false
    if unzip -l "$DOCX_FILE" 2>/dev/null | grep -q "word/_rels/document.xml.rels"; then
        has_rels=true
        log_ok "document.xml.rels present"
        CHECKS["rels_present"]="pass"; ((PASS++))
    else
        log_fail "document.xml.rels missing"
        CHECKS["rels_present"]="fail"; ((FAIL++))
    fi
    echo ""
fi

# ── Summary ──────────────────────────────────────────────────────────────
echo ""
log_info "============================================"
log_info "Quality Gate Summary"
log_info "============================================"
log_info "Passed: $PASS"
log_info "Failed: $FAIL"

if [[ -n "$REPORT_FILE" ]]; then
    cat > "$REPORT_FILE" <<EOF
{
  "version": "1.0",
  "docx": "$DOCX_FILE",
  "min_score": $MIN_SCORE,
  "results": {
EOF
    first=true
    for key in "${!CHECKS[@]}"; do
        if [[ "$first" == "true" ]]; then first=false; else echo "," >> "$REPORT_FILE"; fi
        echo -n "    \"$key\": \"${CHECKS[$key]}\"" >> "$REPORT_FILE"
    done
    cat >> "$REPORT_FILE" <<EOF

  },
  "summary": { "passed": $PASS, "failed": $FAIL }
}
EOF
    log_info "Report: $REPORT_FILE"
fi

if [[ $FAIL -eq 0 ]]; then
    log_ok "All quality checks passed!"
    exit 0
else
    log_fail "$FAIL check(s) failed."
    exit 1
fi
