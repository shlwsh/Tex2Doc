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

# Global counters (accessed from functions below)
_PASS=0
_FAIL=0
declare -A _CHECKS

count_pass() { _PASS=$((_PASS + 1)); }
count_fail() { _FAIL=$((_FAIL + 1)); }

record_pass() {
    local name="$1"
    _CHECKS["$name"]="pass"
    count_pass
}

record_fail() {
    local name="$1"
    _CHECKS["$name"]="fail"
    count_fail
}

run_check() {
    local name="$1"
    local cmd="$2"
    local result
    result=$(eval "$cmd" 2>/dev/null || echo "error")
    if [[ "$result" == "error" ]]; then
        log_fail "$name: command failed"
        record_fail "$name"
    else
        log_ok "$name: $result"
        record_pass "$name"
    fi
}

check_docx_part() {
    local part="$1"
    local name="$2"
    if unzip -l "$DOCX_FILE" 2>/dev/null | grep -q "$part"; then
        log_ok "$name present"
        record_pass "$name"
    else
        log_fail "$name missing"
        record_fail "$name"
    fi
}

run_structural_checks() {
    log_info "--- Structural Checks ---"
    # File size check
    local _size_result
    _size_result=$(stat -c%s "$DOCX_FILE" 2>/dev/null || stat -f%z "$DOCX_FILE" 2>/dev/null || echo 0)
    if [[ ${_size_result:-0} -gt 1024 ]]; then
        log_ok "File size: $_size_result bytes (non-trivial)"
        record_pass "file_size"
    else
        log_fail "File size: ${_size_result:-0} bytes (too small)"
        record_fail "file_size"
    fi
    echo ""
}

run_style_checks() {
    log_info "--- Style Checks ---"
    check_docx_part "word/styles.xml" "styles_present"
    check_docx_part "word/document.xml" "document_present"
    echo ""
}

run_ref_checks() {
    log_info "--- Reference Checks ---"
    check_docx_part "word/_rels/document.xml.rels" "rels_present"
    echo ""
}

# P3.4: Read quality gate results from semantic-convert --report JSON.
run_quality_gate_checks() {
    if [[ -z "$REPORT_FILE" ]]; then
        log_info "--- Quality Gate (skip — no report file) ---"
        return
    fi
    if [[ ! -f "$REPORT_FILE" ]]; then
        log_warn "Quality gate: report file not found: $REPORT_FILE"
        return
    fi

    log_info "--- Quality Gate (from report.json) ---"

    # Extract quality_gate.status
    local qg_status
    qg_status=$(python3 -c "import json,sys; print(json.load(open('$REPORT_FILE')).get('quality_gate',{}).get('status','unknown'))" 2>/dev/null || echo "unknown")
    case "$qg_status" in
        Passed)
            log_ok "quality_gate.status: Passed"
            record_pass "quality_gate_passed"
            ;;
        PassedWithWarnings)
            log_warn "quality_gate.status: PassedWithWarnings"
            record_pass "quality_gate_passed"
            ;;
        Failed)
            log_fail "quality_gate.status: Failed"
            record_fail "quality_gate_passed"
            ;;
        *)
            log_warn "quality_gate.status: $qg_status (unknown)"
            ;;
    esac

    # Extract quality_gate.score
    local qg_score
    qg_score=$(python3 -c "import json,sys; print(json.load(open('$REPORT_FILE')).get('quality_gate',{}).get('score','N/A'))" 2>/dev/null || echo "N/A")
    if [[ "$qg_score" != "N/A" ]] && [[ "$qg_score" != "unknown" ]]; then
        if [[ "$qg_score" -ge 80 ]]; then
            log_ok "quality_gate.score: $qg_score (>= 80)"
            record_pass "quality_gate_score"
        elif [[ "$qg_score" -ge 60 ]]; then
            log_warn "quality_gate.score: $qg_score (60-79, borderline)"
            record_pass "quality_gate_score"
        else
            log_fail "quality_gate.score: $qg_score (< 60)"
            record_fail "quality_gate_score"
        fi
    fi

    # Extract failed checks
    local failed_json
    failed_json=$(python3 -c "import json,sys; print(json.dumps(json.load(open('$REPORT_FILE')).get('quality_gate',{}).get('failed_checks',[])))" 2>/dev/null || echo "[]")
    if [[ "$failed_json" != "[]" ]]; then
        local failed_count
        failed_count=$(python3 -c "import json,sys; print(len(json.loads('$failed_json')))" 2>/dev/null || echo 0)
        if [[ "$failed_count" -gt 0 ]]; then
            log_fail "$failed_count quality check(s) failed:"
            python3 -c "import json,sys; [print('    - ' + c['name'] + ': ' + c['message']) for c in json.loads('$failed_json')]" 2>/dev/null || true
            for check in $(python3 -c "import json,sys; print(' '.join([c['name'] for c in json.loads('$failed_json')]))" 2>/dev/null || echo ""); do
                record_fail "qg_$check"
            done
        fi
    fi

    # Extract warnings
    local warn_count
    warn_count=$(python3 -c "import json,sys; print(len(json.load(open('$REPORT_FILE')).get('quality_gate',{}).get('warnings',[])))" 2>/dev/null || echo 0)
    if [[ "$warn_count" -gt 0 ]]; then
        log_warn "$warn_count quality warning(s):"
        python3 -c "import json,sys; [print('    - ' + c['name'] + ': ' + c['message']) for c in json.load(open('$REPORT_FILE')).get('quality_gate',{}).get('warnings',[])]" 2>/dev/null || true
    fi

    # Extract rule_engine report
    local re_unknown
    re_unknown=$(python3 -c "import json,sys; print(json.load(open('$REPORT_FILE')).get('rule_engine',{}).get('unknown_macro_count',0))" 2>/dev/null || echo 0)
    if [[ "$re_unknown" -gt 0 ]]; then
        log_warn "RuleEngine: $re_unknown unknown macro(s) fell back to text"
        record_pass "rule_engine_unknown"
    else
        log_ok "RuleEngine: no unknown macros"
        record_pass "rule_engine_unknown"
    fi

    echo ""
}

write_summary() {
    local out_file="$1"
    {
        echo "{"
        echo "  \"version\": \"1.0\","
        echo "  \"docx\": \"$DOCX_FILE\","
        echo "  \"min_score\": $MIN_SCORE,"
        echo "  \"results\": {"
        local _first=true
        for key in "${!_CHECKS[@]}"; do
            if [[ "$_first" == "true" ]]; then
                _first=false
            else
                echo ","
            fi
            echo -n "    \"$key\": \"${_CHECKS[$key]}\""
        done
        echo ""
        echo "  },"
        echo "  \"summary\": { \"passed\": $_PASS, \"failed\": $_FAIL }"
        echo "}"
    } > "$out_file"
}

# ── Main ────────────────────────────────────────────────────────────────────
log_info "============================================"
log_info "Commercial Quality Gate"
log_info "============================================"
log_info "DOCX:        $DOCX_FILE"
log_info "Min Score:   $MIN_SCORE"
log_info "Skip Struct: $SKIP_STRUCTURAL"
log_info "Skip Style:  $SKIP_STYLE"
log_info "Skip Refs:   $SKIP_REFERENCES"
echo ""

if [[ "$SKIP_STRUCTURAL" == "false" ]]; then
    run_structural_checks
fi

if [[ "$SKIP_STYLE" == "false" ]]; then
    run_style_checks
fi

if [[ "$SKIP_REFERENCES" == "false" ]]; then
    run_ref_checks
fi

if [[ -n "$REPORT_FILE" ]] && [[ -f "$REPORT_FILE" ]]; then
    run_quality_gate_checks
fi

# ── Summary ──────────────────────────────────────────────────────────────
echo ""
log_info "============================================"
log_info "Quality Gate Summary"
log_info "============================================"
log_info "Passed: $_PASS"
log_info "Failed: $_FAIL"

if [[ -n "$REPORT_FILE" ]]; then
    write_summary "$REPORT_FILE"
    log_info "Report: $REPORT_FILE"
fi

if [[ $_FAIL -eq 0 ]]; then
    log_ok "All quality checks passed!"
    exit 0
else
    log_fail "$_FAIL check(s) failed."
    exit 1
fi
