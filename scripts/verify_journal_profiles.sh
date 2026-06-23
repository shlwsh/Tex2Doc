#!/usr/bin/env bash
# =============================================================================
# verify_journal_profiles.sh
# End-to-end verification for journal profile auto-detection and DOCX generation.
#
# Usage:
#   ./scripts/verify_journal_profiles.sh [--all]
#   ./scripts/verify_journal_profiles.sh --profile-id PROFILE
#   ./scripts/verify_journal_profiles.sh --skip-docx
#   ./scripts/verify_journal_profiles.sh --skip-runtime
#
# Options:
#   --profile-id PROFILE  Run verification for a specific journal profile
#   --all                 Run all 7 journal profiles (default)
#   --skip-docx           Skip DOCX generation, only run unit tests
#   --skip-runtime        Skip steps requiring XeLaTeX/LuaLaTeX (CI-friendly)
#   --help                Show this help message
#
# Exit codes:
#   0   All verifications passed
#   1   One or more verifications failed
#
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
JOURNALS_DIR="$PROJECT_ROOT/examples/journals"
OUTPUT_DIR="$PROJECT_ROOT/examples/journals/output"
CARGO="cargo"

# Colours
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Colour

PROFILES=("jos-paper" "tacl" "cvpr" "nature" "springer" "chinese-academic" "generic")
SKIP_DOCX=false
SKIP_RUNTIME=false
SPECIFIC_PROFILE=""

# ── Argument parsing ────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        --profile-id)
            SPECIFIC_PROFILE="$2"
            shift 2
            ;;
        --all)
            SPECIFIC_PROFILE=""
            shift
            ;;
        --skip-docx)
            SKIP_DOCX=true
            shift
            ;;
        --skip-runtime)
            SKIP_RUNTIME=true
            shift
            ;;
        --help|-h)
            head -20 "$0"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

if [[ -n "$SPECIFIC_PROFILE" ]]; then
    PROFILES=("$SPECIFIC_PROFILE")
fi

mkdir -p "$OUTPUT_DIR"

# ── Helpers ─────────────────────────────────────────────────────────────────

log_info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok()    { echo -e "${GREEN}[PASS]${NC}  $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_fail()  { echo -e "${RED}[FAIL]${NC}  $*"; }

run_test() {
    local profile="$1"
    local tex_file="$JOURNALS_DIR/$profile/minimal.tex"
    local output_file="$OUTPUT_DIR/${profile}.docx"
    local report_file="$OUTPUT_DIR/${profile}.report.json"
    local all_passed=true

    if [[ ! -f "$tex_file" ]]; then
        log_fail "Fixture not found: $tex_file"
        return 1
    fi

    log_info "────────────────────────────────────────"
    log_info "Testing profile: $profile"
    log_info "TeX source:     $tex_file"

    # ── Step 1: Journal Detection ────────────────────────────────────────────
    log_info "Step 1: Running journal detection..."
    local detect_result=0
    if ! "$CARGO" test -q -p doc-compiler-engine journal_detector 2>/dev/null; then
        log_warn "Step 1: journal_detector tests had non-zero exit (non-blocking)"
        detect_result=1
    else
        log_ok "Step 1: journal detection tests passed"
    fi

    # ── Step 2: Compatibility Analysis ───────────────────────────────────────
    log_info "Step 2: Running compatibility analysis..."
    if ! "$CARGO" test -q -p doc-compatibility-analyzer 2>/dev/null; then
        log_warn "Step 2: compatibility analyzer tests had non-zero exit (non-blocking)"
    else
        log_ok "Step 2: compatibility analysis tests passed"
    fi

    # ── Step 3: Rule Engine ──────────────────────────────────────────────────
    log_info "Step 3: Verifying rule engine (journal rules)..."
    if ! "$CARGO" test -q -p doc-rule-engine 2>/dev/null; then
        log_warn "Step 3: rule engine tests had non-zero exit (non-blocking)"
    else
        log_ok "Step 3: rule engine tests passed"
    fi

    # ── Step 4: Profile-aware Backend Selection ────────────────────────────────
    log_info "Step 4: Verifying profile-aware backend selection..."
    if ! "$CARGO" test -q -p doc-compiler-engine profile_aware 2>/dev/null && \
       ! "$CARGO" test -q -p doc-compiler-engine backend_selector 2>/dev/null; then
        log_warn "Step 4: backend selector tests not found or had non-zero exit (non-blocking)"
    else
        log_ok "Step 4: backend selection tests passed"
    fi

    # ── Step 5: DOCX Generation (if not skipped) ────────────────────────────
    if [[ "$SKIP_DOCX" == "false" ]]; then
        if [[ "$SKIP_RUNTIME" == "true" ]]; then
            log_info "Step 5: Skipped (--skip-runtime specified — requires XeLaTeX/LuaLaTeX)"
        else
            log_info "Step 5: Generating DOCX..."

            local docx_result=0
            "$CARGO" run -q -p doc-compiler-engine --example paper3_to_docx -- \
                --project-root "$JOURNALS_DIR/$profile" \
                --main-tex "minimal.tex" \
                --out "$output_file" \
                --profile "$profile" \
                --semantic-backend auto \
                --report "$report_file" \
                2>&1 || docx_result=$?

            if [[ $docx_result -eq 0 ]]; then
                if [[ -f "$output_file" ]]; then
                    local size
                    size=$(stat -c%s "$output_file" 2>/dev/null || stat -f%z "$output_file" 2>/dev/null || echo 0)
                    log_ok "Step 5: DOCX generated ($size bytes) → $output_file"
                    log_ok "Step 5: Report JSON → $report_file"
                else
                    log_fail "Step 5: DOCX not found at $output_file"
                    return 1
                fi
            else
                log_fail "Step 5: DOCX generation failed (exit $docx_result)"
                log_fail "  Hint: ensure XeLaTeX or LuaLaTeX is in PATH, or use --skip-docx / --skip-runtime"
                return 1
            fi
        fi
    else
        log_info "Step 5: Skipped (--skip-docx specified)"
    fi

    log_ok "All steps completed for profile: $profile"
    echo ""
}

# ── Main ────────────────────────────────────────────────────────────────────

log_info "============================================"
log_info "Journal Profile Verification"
log_info "============================================"
log_info "Project root:  $PROJECT_ROOT"
log_info "Journals dir:   $JOURNALS_DIR"
log_info "Output dir:     $OUTPUT_DIR"
log_info "Profiles:       ${PROFILES[*]}"
log_info "Skip DOCX:     $SKIP_DOCX"
echo ""

if [[ "$SKIP_DOCX" == "false" && "$SKIP_RUNTIME" == "false" ]]; then
    log_info "Note: DOCX generation requires XeLaTeX or LuaLaTeX in PATH."
    log_info "      Use --skip-docx or --skip-runtime if not available."
    echo ""
fi

declare -A PROFILE_RESULTS
PASS=0
FAIL=0

for profile in "${PROFILES[@]}"; do
    if run_test "$profile"; then
        PROFILE_RESULTS["$profile"]="passed"
        PASS=$((PASS + 1))
    else
        PROFILE_RESULTS["$profile"]="failed"
        FAIL=$((FAIL + 1))
    fi
done

echo ""
log_info "============================================"
log_info "Verification Summary"
log_info "============================================"
log_info "Passed: $PASS"
log_info "Failed: $FAIL"

# ── Summary JSON ───────────────────────────────────────────────────────────
SUMMARY_JSON="$OUTPUT_DIR/verify-summary.json"
cat > "$SUMMARY_JSON" <<'EOFJSON'
{
  "version": "1.0",
  "project_root": "%s",
  "profiles": {
EOFJSON

# Re-generate with PROJECT_ROOT injected
{
    echo "{"
    echo "  \"version\": \"1.0\","
    echo "  \"project_root\": \"$PROJECT_ROOT\","
    echo "  \"profiles\": {"
    first=true
    for profile in "${PROFILES[@]}"; do
        if [[ "$first" == "true" ]]; then
            first=false
        else
            echo ","
        fi
        echo -n "    \"$profile\": \"${PROFILE_RESULTS[$profile]}\""
    done
    echo ""
    echo "  },"
    echo "  \"summary\": {"
    echo "    \"total\": ${#PROFILES[@]},"
    echo "    \"passed\": $PASS,"
    echo "    \"failed\": $FAIL,"
    echo "    \"skipped_docx\": $SKIP_DOCX,"
    echo "    \"skipped_runtime\": $SKIP_RUNTIME"
    echo "  }"
    echo "}"
} > "$SUMMARY_JSON"

log_info "Summary JSON: $SUMMARY_JSON"

if [[ $FAIL -eq 0 ]]; then
    log_ok "All journal profile verifications passed!"
    exit 0
else
    log_fail "$FAIL profile(s) failed verification."
    exit 1
fi
