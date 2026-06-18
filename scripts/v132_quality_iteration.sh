#!/bin/bash
# v132 质量迭代自动化脚本
# 用法: bash scripts/v132_quality_iteration.sh <VERSION> <RUST_DOCX> [ORACLE_DOCX]
set -e

VERSION="${1:-v132}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

if [ -n "$2" ]; then
    RUST_DOCX="$2"
else
    RUST_DOCX=$(ls -t examples/paper3/output/to-docx/v132*-论文稿件-jos-rust.docx 2>/dev/null | head -1 || echo "")
    if [ -z "$RUST_DOCX" ]; then
        RUST_DOCX="examples/paper3/output/main-jos-rust.docx"
    fi
fi

if [ -n "$3" ]; then
    ORACLE="$3"
else
    ORACLE="examples/paper3/output/to-docx/v12-论文稿件-jos-sh-20260618-070357.docx"
fi

OUT_DIR="docs/verify/${VERSION}-${TIMESTAMP}"

echo "=== [${VERSION}] 质量自动化迭代 ==="
echo "Rust DOCX: $RUST_DOCX"
echo "Oracle:    $ORACLE"
echo "输出目录:  $OUT_DIR"

if [ ! -f "$RUST_DOCX" ]; then
    echo "错误: 文件不存在: $RUST_DOCX"
    exit 1
fi
if [ ! -f "$ORACLE" ]; then
    echo "错误: Oracle 文件不存在: $ORACLE"
    exit 1
fi

mkdir -p "$OUT_DIR"

echo "--- Step 1: 生成 Markdown 报告 ---"
cargo run -p doc-engine -- docx-diff \
  --left "$RUST_DOCX" \
  --right "$ORACLE" \
  --format md --out "${OUT_DIR}/docx-compare.md"

echo "--- Step 2: 生成 JSON 指标 ---"
cargo run -p doc-engine -- docx-diff \
  --left "$RUST_DOCX" \
  --right "$ORACLE" \
  --format json --out "${OUT_DIR}/docx-compare.json"

echo ""
echo "--- Step 3: 关键指标摘要 ---"
JSON_PATH="${OUT_DIR}/docx-compare.json" VERSION="$VERSION" python3 - <<'PYEOF'
import json, os

json_path = os.environ["JSON_PATH"]
with open(json_path) as f:
    r = json.load(f)
s = r.get("summary", {})

para_delta = s.get("paragraph_delta", -999)
real_diff = s.get("format_changed_real_paragraphs", -1)
split_only = s.get("format_changed_split_only_paragraphs", -1)
table_delta = s.get("table_delta", -999)

print(f"  段落数差:      {para_delta:+d}")
print(f"  相同段落:      {s.get('equal_paragraphs', -1)}")
print(f"  修改段落:      {s.get('modified_paragraphs', -1)}")
print(f"  新增段落:      {s.get('inserted_paragraphs', -1)}")
print(f"  删除段落:      {s.get('deleted_paragraphs', -1)}")
print(f"  真实格式差:    {real_diff}")
print(f"  run分割差:    {split_only}")
print(f"  表格数差:      {table_delta:+d}")
print(f"  图片数差:      {s.get('drawing_delta', -999):+d}")
print(f"  document.xml:  {'相同' if s.get('document_xml_equal') else '不同'}")
print(f"  styles.xml:    {'相同' if s.get('styles_xml_equal') else '不同'}")

ok_real = "PASS" if real_diff <= 5 else "FAIL"
ok_para = "PASS" if abs(para_delta) <= 5 else "FAIL"
ok_split = "PASS" if split_only <= 10 else "FAIL"
print(f"\n  === 达标判定 ===")
print(f"  格式一致:  [{ok_real}]  ({real_diff} ≤ 5)")
print(f"  结构一致:  [{ok_para}]  (|{para_delta}| ≤ 5)")
print(f"  run一致:   [{ok_split}]  ({split_only} ≤ 10)")
PYEOF

echo ""
echo "--- Step 4: 更新迭代记录 ---"
RECORD_FILE="docs/verify/迭代记录.md"
if [ -f "scripts/update_iteration_record.py" ]; then
    python3 scripts/update_iteration_record.py \
        "${VERSION}" \
        "${OUT_DIR}/docx-compare.json" \
        "${RECORD_FILE}" || echo "  记录更新失败"
fi

echo ""
echo "=== 质量迭代完成 ==="
echo "Markdown 报告: ${OUT_DIR}/docx-compare.md"
echo "JSON 指标:     ${OUT_DIR}/docx-compare.json"
echo "记录文件:      ${RECORD_FILE}"
