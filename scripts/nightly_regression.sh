#!/usr/bin/env bash
set -euo pipefail

# P8 nightly regression for journal profiles.
#
# Runs semantic-convert over each profile fixture and writes:
# - conversion_stats.json
# - conversion_stats.md
# - results.jsonl
# - per-fixture DOCX/report/log files
#
# Environment:
#   NIGHTLY_PROFILES="generic tacl"      limit profile list
#   NIGHTLY_OUTPUT_DIR=/path/to/output   override output base directory
#   ALLOW_FAILURES=true                  keep exit code 0 even with failures
#   NIGHTLY_WORD_OPEN_CHECK=true         verify DOCX by LibreOffice headless conversion
#   NIGHTLY_WORD_OPEN_REQUIRED=true      fail when LibreOffice verification is unavailable
#   NIGHTLY_WORD_OPEN_TIMEOUT_SECONDS=60 timeout for each LibreOffice check

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE_LIST="${NIGHTLY_PROFILES:-generic chinese-academic jos-paper tacl cvpr nature springer}"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
OUTPUT_BASE="${NIGHTLY_OUTPUT_DIR:-$ROOT_DIR/examples/journals/output/nightly}"
OUTPUT_DIR="$OUTPUT_BASE/$RUN_ID"
RESULTS_JSONL="$OUTPUT_DIR/results.jsonl"
SUMMARY_JSON="$OUTPUT_DIR/conversion_stats.json"
SUMMARY_MD="$OUTPUT_DIR/conversion_stats.md"
WORD_OPEN_CHECK="${NIGHTLY_WORD_OPEN_CHECK:-false}"
WORD_OPEN_REQUIRED="${NIGHTLY_WORD_OPEN_REQUIRED:-false}"
WORD_OPEN_TIMEOUT_SECONDS="${NIGHTLY_WORD_OPEN_TIMEOUT_SECONDS:-60}"

mkdir -p "$OUTPUT_DIR"
: > "$RESULTS_JSONL"

json_escape() {
    local value="$1"
    value="${value//\\/\\\\}"
    value="${value//\"/\\\"}"
    value="${value//$'\n'/\\n}"
    value="${value//$'\r'/\\r}"
    printf '%s' "$value"
}

is_enabled() {
    local value="${1:-false}"
    case "${value,,}" in
        true | 1 | yes | y | on) return 0 ;;
        *) return 1 ;;
    esac
}

is_docx_zip_openable() {
    local file="$1"
    if [[ ! -s "$file" ]]; then
        return 1
    fi
    local magic
    magic="$(od -An -tx1 -N4 "$file" | tr -d ' \n')"
    [[ "$magic" == "504b0304" ]]
}

docx_required_parts() {
    printf '%s\n' \
        '[Content_Types].xml' \
        '_rels/.rels' \
        'word/_rels/document.xml.rels' \
        'word/styles.xml' \
        'word/document.xml'
}

unzip_part_name() {
    local part="$1"
    if [[ "$part" == "[Content_Types].xml" ]]; then
        printf '%s' '\[Content_Types\].xml'
    else
        printf '%s' "$part"
    fi
}

validate_docx_structure() {
    local file="$1"
    local verify_log="$2"
    : > "$verify_log"

    if ! is_docx_zip_openable "$file"; then
        printf 'invalid ZIP header or empty file: %s\n' "$file" >> "$verify_log"
        return 1
    fi

    local listing
    if ! listing="$(unzip -Z1 "$file" 2>>"$verify_log")"; then
        printf 'cannot list DOCX package entries: %s\n' "$file" >> "$verify_log"
        return 1
    fi

    local missing=0
    local part
    while IFS= read -r part; do
        if ! printf '%s\n' "$listing" | grep -Fxq "$part"; then
            printf 'missing required DOCX part: %s\n' "$part" >> "$verify_log"
            missing=1
        fi
    done < <(docx_required_parts)

    [[ "$missing" -eq 0 ]]
}

validate_docx_xml_parts() {
    local file="$1"
    local verify_log="$2"
    : > "$verify_log"

    if ! command -v xmllint >/dev/null 2>&1; then
        printf 'xmllint not found; XML well-formed validation skipped\n' >> "$verify_log"
        return 2
    fi

    local failed=0
    local part
    while IFS= read -r part; do
        local unzip_part
        unzip_part="$(unzip_part_name "$part")"
        if ! unzip -p "$file" "$unzip_part" 2>>"$verify_log" | xmllint --noout - >>"$verify_log" 2>&1; then
            printf 'XML validation failed for DOCX part: %s\n' "$part" >> "$verify_log"
            failed=1
        fi
    done < <(docx_required_parts)

    [[ "$failed" -eq 0 ]]
}

verify_file_with_soffice_pdf() {
    local file="$1"
    local verify_dir="$2"
    local verify_log="$3"

    : > "$verify_log"
    if ! command -v soffice >/dev/null 2>&1; then
        printf 'soffice not found; install LibreOffice to enable Word-open validation\n' >> "$verify_log"
        return 1
    fi

    mkdir -p "$verify_dir"
    local base_name
    base_name="$(basename "${file%.*}")"
    local pdf_file="$verify_dir/$base_name.pdf"
    local lo_home="$verify_dir/home"
    local lo_config="$verify_dir/config"
    local lo_cache="$verify_dir/cache"
    local lo_runtime="$verify_dir/runtime"
    local lo_profile="$verify_dir/libreoffice-profile"
    mkdir -p "$lo_home" "$lo_config" "$lo_cache" "$lo_runtime" "$lo_profile"

    local -a soffice_cmd=(
        env
        "HOME=$lo_home"
        "XDG_CONFIG_HOME=$lo_config"
        "XDG_CACHE_HOME=$lo_cache"
        "XDG_RUNTIME_DIR=$lo_runtime"
        soffice
        --headless
        --nologo
        --nofirststartwizard
        --nodefault
        --nolockcheck
        --norestore
        "-env:UserInstallation=file://$lo_profile"
        --convert-to
        pdf
        --outdir
        "$verify_dir"
        "$file"
    )

    if command -v timeout >/dev/null 2>&1; then
        timeout "$WORD_OPEN_TIMEOUT_SECONDS" "${soffice_cmd[@]}" >> "$verify_log" 2>&1
    else
        "${soffice_cmd[@]}" >> "$verify_log" 2>&1
    fi

    local status=$?
    if [[ "$status" -ne 0 ]]; then
        printf 'LibreOffice conversion failed with status %s\n' "$status" >> "$verify_log"
        return 1
    fi

    if [[ ! -s "$pdf_file" ]]; then
        printf 'LibreOffice conversion did not create expected PDF: %s\n' "$pdf_file" >> "$verify_log"
        return 1
    fi

    return 0
}

preflight_word_open_check() {
    local verify_dir="$OUTPUT_DIR/word-open-selftest"
    local verify_log="$OUTPUT_DIR/word-open-selftest.log"
    local input_file="$verify_dir/input.txt"

    mkdir -p "$verify_dir"
    printf 'LibreOffice word-open selftest\n' > "$input_file"
    verify_file_with_soffice_pdf "$input_file" "$verify_dir" "$verify_log"
}

expected_profile_pattern() {
    local profile="$1"
    case "$profile" in
        generic) printf 'generic|generic-article|generic-article-toml' ;;
        *) printf '%s|%s-toml' "$profile" "$profile" ;;
    esac
}

profile_matches_report() {
    local profile="$1"
    local report="$2"
    if [[ ! -f "$report" ]]; then
        return 1
    fi
    local pattern
    pattern="$(expected_profile_pattern "$profile")"
    grep -Eq "\"id\"[[:space:]]*:[[:space:]]*\"($pattern)\"" "$report" \
        || grep -Eq "\"detected_profile\"[[:space:]]*:[[:space:]]*\"($pattern)\"" "$report" \
        || grep -Eq "\"profile\"[[:space:]]*:[[:space:]]*\"($pattern)\"" "$report"
}

write_result() {
    local profile="$1"
    local fixture="$2"
    local status="$3"
    local duration_ms="$4"
    local docx="$5"
    local report="$6"
    local log_file="$7"
    local docx_openable="$8"
    local docx_zip_openable="$9"
    local word_open_check="${10}"
    local word_openable="${11}"
    local word_open_skipped="${12}"
    local docx_structure_valid="${13}"
    local docx_xml_valid="${14}"
    local docx_xml_skipped="${15}"
    local structure_log_file="${16}"
    local xml_log_file="${17}"
    local word_log_file="${18}"
    local report_generated="${19}"
    local profile_matched="${20}"
    local panic_detected="${21}"

    local docx_bytes=0
    if [[ -f "$docx" ]]; then
        docx_bytes="$(wc -c < "$docx" | tr -d ' ')"
    fi

    printf '{"profile":"%s","fixture":"%s","exit_code":%s,"duration_ms":%s,"docx_path":"%s","docx_bytes":%s,"report_path":"%s","log_path":"%s","docx_openable":%s,"docx_zip_openable":%s,"docx_structure_valid":%s,"docx_xml_valid":%s,"docx_xml_skipped":%s,"docx_structure_log_path":"%s","docx_xml_log_path":"%s","word_open_check":%s,"word_openable":%s,"word_open_skipped":%s,"word_log_path":"%s","report_generated":%s,"profile_matched":%s,"panic_detected":%s}\n' \
        "$(json_escape "$profile")" \
        "$(json_escape "$fixture")" \
        "$status" \
        "$duration_ms" \
        "$(json_escape "$docx")" \
        "$docx_bytes" \
        "$(json_escape "$report")" \
        "$(json_escape "$log_file")" \
        "$docx_openable" \
        "$docx_zip_openable" \
        "$docx_structure_valid" \
        "$docx_xml_valid" \
        "$docx_xml_skipped" \
        "$(json_escape "$structure_log_file")" \
        "$(json_escape "$xml_log_file")" \
        "$word_open_check" \
        "$word_openable" \
        "$word_open_skipped" \
        "$(json_escape "$word_log_file")" \
        "$report_generated" \
        "$profile_matched" \
        "$panic_detected" >> "$RESULTS_JSONL"
}

total=0
succeeded=0
failed=0
docx_openable_count=0
docx_zip_openable_count=0
docx_structure_valid_count=0
docx_xml_valid_count=0
docx_xml_skipped_count=0
word_openable_count=0
word_open_skipped_count=0
report_generated_count=0
profile_matched_count=0
panic_count=0
word_open_check_enabled=false
word_open_check_required=false
word_open_check_available=false
if is_enabled "$WORD_OPEN_CHECK"; then
    word_open_check_enabled=true
fi
if is_enabled "$WORD_OPEN_REQUIRED"; then
    word_open_check_required=true
fi

if [[ "$word_open_check_enabled" == "true" ]]; then
    if preflight_word_open_check; then
        word_open_check_available=true
    else
        word_open_check_available=false
    fi
fi

echo "P8 nightly regression run: $RUN_ID"
echo "Output: $OUTPUT_DIR"
echo "Word-open check: $word_open_check_enabled"
echo "Word-open available: $word_open_check_available"

for profile in $PROFILE_LIST; do
    profile_dir="$ROOT_DIR/examples/journals/$profile"
    if [[ ! -d "$profile_dir" ]]; then
        echo "WARN missing profile directory: $profile_dir" >&2
        continue
    fi

    profile_output="$OUTPUT_DIR/$profile"
    mkdir -p "$profile_output"

    while IFS= read -r tex_file; do
        fixture="$(basename "$tex_file" .tex)"
        main_tex="$(basename "$tex_file")"
        docx="$profile_output/$fixture.docx"
        report="$profile_output/$fixture.report.json"
        log_file="$profile_output/$fixture.log"
        structure_log_file="$profile_output/$fixture.docx-structure.log"
        xml_log_file="$profile_output/$fixture.docx-xml.log"
        word_log_file="$profile_output/$fixture.word-open.log"
        word_verify_dir="$profile_output/$fixture.word-open"

        total=$((total + 1))
        echo "[$total] $profile/$main_tex"

        started_ms="$(date +%s%3N)"
        status=0
        output="$(
            cargo run -q -p doc-engine -- semantic-convert \
                --project-root "$profile_dir" \
                --main-tex "$main_tex" \
                --profile auto \
                --backend auto \
                --quality preview \
                --out "$docx" \
                --report "$report" \
                --json 2>&1
        )" || status=$?
        finished_ms="$(date +%s%3N)"
        duration_ms=$((finished_ms - started_ms))
        printf '%s\n' "$output" > "$log_file"

        docx_zip_openable=false
        if is_docx_zip_openable "$docx"; then
            docx_zip_openable=true
            docx_zip_openable_count=$((docx_zip_openable_count + 1))
        fi

        docx_structure_valid=false
        if validate_docx_structure "$docx" "$structure_log_file"; then
            docx_structure_valid=true
            docx_structure_valid_count=$((docx_structure_valid_count + 1))
        fi

        docx_xml_valid=false
        docx_xml_skipped=false
        xml_status=0
        validate_docx_xml_parts "$docx" "$xml_log_file" || xml_status=$?
        if [[ "$xml_status" -eq 0 ]]; then
            docx_xml_valid=true
            docx_xml_valid_count=$((docx_xml_valid_count + 1))
        elif [[ "$xml_status" -eq 2 ]]; then
            docx_xml_skipped=true
            docx_xml_skipped_count=$((docx_xml_skipped_count + 1))
        fi

        word_openable=false
        word_open_skipped=false
        if [[ "$word_open_check_enabled" == "true" && "$word_open_check_available" != "true" ]]; then
            word_open_skipped=true
            word_open_skipped_count=$((word_open_skipped_count + 1))
            : > "$word_log_file"
            printf 'skipped: LibreOffice verifier is unavailable; see %s\n' "$OUTPUT_DIR/word-open-selftest.log" > "$word_log_file"
        elif [[ "$word_open_check_enabled" == "true" && "$docx_zip_openable" == "true" ]]; then
            if verify_file_with_soffice_pdf "$docx" "$word_verify_dir" "$word_log_file"; then
                word_openable=true
                word_openable_count=$((word_openable_count + 1))
            fi
        else
            : > "$word_log_file"
            if [[ "$word_open_check_enabled" != "true" ]]; then
                printf 'skipped: NIGHTLY_WORD_OPEN_CHECK is not enabled\n' > "$word_log_file"
            else
                word_open_skipped=true
                word_open_skipped_count=$((word_open_skipped_count + 1))
                printf 'skipped: DOCX ZIP header validation failed\n' > "$word_log_file"
            fi
        fi

        docx_openable="$docx_zip_openable"
        if [[ "$docx_structure_valid" != "true" ]]; then
            docx_openable=false
        fi
        if [[ "$docx_xml_skipped" != "true" && "$docx_xml_valid" != "true" ]]; then
            docx_openable=false
        fi
        if [[ "$word_open_check_enabled" == "true" && "$word_open_check_available" == "true" ]]; then
            docx_openable="$word_openable"
        fi
        if [[ "$docx_openable" == "true" ]]; then
            docx_openable_count=$((docx_openable_count + 1))
        fi

        report_generated=false
        if [[ -s "$report" ]]; then
            report_generated=true
            report_generated_count=$((report_generated_count + 1))
        fi

        profile_matched=false
        if profile_matches_report "$profile" "$report"; then
            profile_matched=true
            profile_matched_count=$((profile_matched_count + 1))
        fi

        panic_detected=false
        if printf '%s' "$output" | grep -qiE 'panic|panicked'; then
            panic_detected=true
            panic_count=$((panic_count + 1))
        fi

        if [[ "$status" -eq 0 && "$docx_openable" == "true" && "$report_generated" == "true" ]]; then
            succeeded=$((succeeded + 1))
        else
            failed=$((failed + 1))
        fi

        write_result \
            "$profile" \
            "$fixture" \
            "$status" \
            "$duration_ms" \
            "$docx" \
            "$report" \
            "$log_file" \
            "$docx_openable" \
            "$docx_zip_openable" \
            "$word_open_check_enabled" \
            "$word_openable" \
            "$word_open_skipped" \
            "$docx_structure_valid" \
            "$docx_xml_valid" \
            "$docx_xml_skipped" \
            "$structure_log_file" \
            "$xml_log_file" \
            "$word_log_file" \
            "$report_generated" \
            "$profile_matched" \
            "$panic_detected"
    done < <(find "$profile_dir" -maxdepth 1 -type f -name '*.tex' | sort)
done

{
    printf '{\n'
    printf '  "version": "p8-nightly-v3",\n'
    printf '  "run_id": "%s",\n' "$(json_escape "$RUN_ID")"
    printf '  "output_dir": "%s",\n' "$(json_escape "$OUTPUT_DIR")"
    printf '  "profiles": "%s",\n' "$(json_escape "$PROFILE_LIST")"
    printf '  "word_open_check": %s,\n' "$word_open_check_enabled"
    printf '  "word_open_check_required": %s,\n' "$word_open_check_required"
    printf '  "word_open_check_available": %s,\n' "$word_open_check_available"
    printf '  "word_open_timeout_seconds": %s,\n' "$WORD_OPEN_TIMEOUT_SECONDS"
    printf '  "total": %s,\n' "$total"
    printf '  "succeeded": %s,\n' "$succeeded"
    printf '  "failed": %s,\n' "$failed"
    printf '  "docx_openable": %s,\n' "$docx_openable_count"
    printf '  "docx_zip_openable": %s,\n' "$docx_zip_openable_count"
    printf '  "docx_structure_valid": %s,\n' "$docx_structure_valid_count"
    printf '  "docx_xml_valid": %s,\n' "$docx_xml_valid_count"
    printf '  "docx_xml_skipped": %s,\n' "$docx_xml_skipped_count"
    printf '  "word_openable": %s,\n' "$word_openable_count"
    printf '  "word_open_skipped": %s,\n' "$word_open_skipped_count"
    printf '  "quality_report_generated": %s,\n' "$report_generated_count"
    printf '  "profile_detection_matched": %s,\n' "$profile_matched_count"
    printf '  "unhandled_panic": %s,\n' "$panic_count"
    printf '  "results": [\n'
    line_no=0
    while IFS= read -r line; do
        if [[ "$line_no" -gt 0 ]]; then
            printf ',\n'
        fi
        printf '    %s' "$line"
        line_no=$((line_no + 1))
    done < "$RESULTS_JSONL"
    printf '\n  ]\n'
    printf '}\n'
} > "$SUMMARY_JSON"

{
    printf '# P8 Nightly Regression Report\n\n'
    printf '%s\n' "- Run ID: \`$RUN_ID\`"
    printf '%s\n' "- Output: \`$OUTPUT_DIR\`"
    printf '%s\n\n' "- Profiles: \`$PROFILE_LIST\`"
    printf '%s\n\n' "- Word-open check: \`$word_open_check_enabled\`"
    printf '%s\n\n' "- Word-open available: \`$word_open_check_available\`"
    printf '| Metric | Value |\n'
    printf '|---|---:|\n'
    printf '| Total fixtures | %s |\n' "$total"
    printf '| Succeeded | %s |\n' "$succeeded"
    printf '| Failed | %s |\n' "$failed"
    printf '| DOCX openable | %s |\n' "$docx_openable_count"
    printf '| DOCX ZIP openable | %s |\n' "$docx_zip_openable_count"
    printf '| DOCX structure valid | %s |\n' "$docx_structure_valid_count"
    printf '| DOCX XML valid | %s |\n' "$docx_xml_valid_count"
    printf '| DOCX XML skipped | %s |\n' "$docx_xml_skipped_count"
    printf '| Word/LibreOffice openable | %s |\n' "$word_openable_count"
    printf '| Word/LibreOffice skipped | %s |\n' "$word_open_skipped_count"
    printf '| Reports generated | %s |\n' "$report_generated_count"
    printf '| Profile detection matched | %s |\n' "$profile_matched_count"
    printf '| Panic detected | %s |\n\n' "$panic_count"
    printf '## Results\n\n'
    printf '| Profile | Fixture | Exit | DOCX | ZIP | Structure | XML | XML skipped | Word | Word skipped | Report | Profile | Panic |\n'
    printf '|---|---|---:|---|---|---|---|---|---|---|---|---|---|\n'
    while IFS= read -r line; do
        profile="$(printf '%s' "$line" | sed -n 's/.*"profile":"\([^"]*\)".*/\1/p')"
        fixture="$(printf '%s' "$line" | sed -n 's/.*"fixture":"\([^"]*\)".*/\1/p')"
        exit_code="$(printf '%s' "$line" | sed -n 's/.*"exit_code":\([0-9]*\).*/\1/p')"
        docx_ok="$(printf '%s' "$line" | sed -n 's/.*"docx_openable":\(true\|false\).*/\1/p')"
        zip_ok="$(printf '%s' "$line" | sed -n 's/.*"docx_zip_openable":\(true\|false\).*/\1/p')"
        structure_ok="$(printf '%s' "$line" | sed -n 's/.*"docx_structure_valid":\(true\|false\).*/\1/p')"
        xml_ok="$(printf '%s' "$line" | sed -n 's/.*"docx_xml_valid":\(true\|false\).*/\1/p')"
        xml_skipped="$(printf '%s' "$line" | sed -n 's/.*"docx_xml_skipped":\(true\|false\).*/\1/p')"
        word_ok="$(printf '%s' "$line" | sed -n 's/.*"word_openable":\(true\|false\).*/\1/p')"
        word_skipped="$(printf '%s' "$line" | sed -n 's/.*"word_open_skipped":\(true\|false\).*/\1/p')"
        report_ok="$(printf '%s' "$line" | sed -n 's/.*"report_generated":\(true\|false\).*/\1/p')"
        profile_ok="$(printf '%s' "$line" | sed -n 's/.*"profile_matched":\(true\|false\).*/\1/p')"
        panic_ok="$(printf '%s' "$line" | sed -n 's/.*"panic_detected":\(true\|false\).*/\1/p')"
        printf '| `%s` | `%s` | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |\n' \
            "$profile" "$fixture" "$exit_code" "$docx_ok" "$zip_ok" "$structure_ok" "$xml_ok" "$xml_skipped" "$word_ok" "$word_skipped" "$report_ok" "$profile_ok" "$panic_ok"
    done < "$RESULTS_JSONL"
} > "$SUMMARY_MD"

echo "Summary JSON: $SUMMARY_JSON"
echo "Summary MD:   $SUMMARY_MD"

if [[ "$failed" -gt 0 && "${ALLOW_FAILURES:-false}" != "true" ]]; then
    echo "Nightly regression failed: $failed/$total fixtures failed" >&2
    exit 1
fi

if [[ "$word_open_check_enabled" == "true" \
    && "$word_open_check_required" == "true" \
    && "$word_open_check_available" != "true" \
    && "${ALLOW_FAILURES:-false}" != "true" ]]; then
    echo "Nightly regression failed: LibreOffice word-open verifier is unavailable" >&2
    exit 1
fi

exit 0
