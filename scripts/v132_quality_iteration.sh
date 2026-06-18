#!/bin/bash
# v132 质量迭代自动化脚本
# 用法: bash scripts/v132_quality_iteration.sh <VERSION> <RUST_DOCX> [ORACLE_DOCX]
#
# 示例:
#   bash scripts/v132_quality_iteration.sh v1321
#   bash scripts/v132_quality_iteration.sh v1321 examples/paper3/output/to-docx/v1321-论文稿件-jos-rust.docx
set -e

VERSION="${1:-v132}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

# 定位 RUST DOCX
if [ -n "$2" ]; then
    RUST_DOCX="$2"
else
    # 自动查找最新的 v132 版本 docx
    RUST_DOCX=$(ls -t examples/paper3/output/to-docx/v132*-论文稿件-jos-rust.docx 2>/dev/null | head -1 || echo "")
    if [ -z "$RUST_DOCX" ]; then
        echo "错误: 未找到 v132 docx 文件，请手动指定 RUST_DOCX"
        exit 1
    fi
fi

# Oracle 固定为 v12 baseline
ORACLE="examples/paper3/output/to-docx/v12-论文稿件-jos-sh-20260618-070357.docx"

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

# Step 1: docx-diff (Markdown 报告)
echo "--- Step 1: 生成 Markdown 报告 ---"
cargo run -p doc-engine -- docx-diff \
  --left "$RUST_DOCX" \
  --right "$ORACLE" \
  --format md --out "${OUT_DIR}/docx-compare.md" 2>/dev/null

# Step 2: docx-diff (JSON 指标提取)
echo "--- Step 2: 生成 JSON 指标 ---"
cargo run -p doc-engine -- docx-diff \
  --left "$RUST_DOCX" \
  --right "$ORACLE" \
  --format json --out "${OUT_DIR}/docx-compare.json" 2>/dev/null

# Step 3: 提取并显示关键指标
echo ""
echo "--- Step 3: 关键指标摘要 ---"
python3 - << 'PYEOF'
import json, sys, os

json_path = sys.argv[1]
version = os.environ.get("VERSION", "v132")

try:
    with open(json_path) as f:
        r = json.load(f)

    s = r.get("summary", {})

    para_delta   = s.get('paragraph_delta', -999)
    equal_para   = s.get('equal_paragraphs', -1)
    mod_para     = s.get('modified_paragraphs', -1)
    ins_para     = s.get('inserted_paragraphs', -1)
    del_para     = s.get('deleted_paragraphs', -1)
    real_diff    = s.get('format_changed_real_paragraphs', -1)
    split_only   = s.get('format_changed_split_only_paragraphs', -1)
    table_delta  = s.get('table_delta', -999)
    draw_delta   = s.get('drawing_delta', -999)
    doc_xml_eq   = s.get('document_xml_equal', None)
    style_eq     = s.get('styles_xml_equal', None)

    print(f"  段落数差:      {para_delta:+d}")
    print(f"  相同段落:      {equal_para}")
    print(f"  修改段落:      {mod_para}")
    print(f"  新增段落:      {ins_para}")
    print(f"  删除段落:      {del_para}")
    print(f"  真实格式差:    {real_diff}")
    print(f"  run分割差:    {split_only}")
    print(f"  表格数差:      {table_delta:+d}")
    print(f"  图片数差:      {draw_delta:+d}")
    print(f"  document.xml:  {'相同' if doc_xml_eq else '不同'}")
    print(f"  styles.xml:    {'相同' if style_eq else '不同'}")

    print(f"\n  === 达标判定 ===")
    ok_real  = "PASS" if real_diff <= 5 else "FAIL"
    ok_para  = "PASS" if abs(para_delta) <= 5 else "FAIL"
    ok_split = "PASS" if split_only <= 10 else "FAIL"
    print(f"  格式一致:  [{ok_real}]  ({real_diff} ≤ 5)")
    print(f"  结构一致:  [{ok_para}]  (|{para_delta}| ≤ 5)")
    print(f"  run一致:   [{ok_split}]  ({split_only} ≤ 10)")

    # 导出供 shell 后续使用
    with open("/tmp/v132_metrics.txt", "w") as mf:
        mf.write(f"PARA_DELTA={para_delta}\n")
        mf.write(f"REAL_DIFF={real_diff}\n")
        mf.write(f"SPLIT_ONLY={split_only}\n")
        mf.write(f"TABLE_DELTA={table_delta}\n")
        mf.write(f"OK_REAL={ok_real}\n")
        mf.write(f"OK_PARA={ok_para}\n")
        mf.write(f"OK_SPLIT={ok_split}\n")

except Exception as e:
    print(f"  JSON 解析失败: {e}", file=sys.stderr)
    # 写空指标
    with open("/tmp/v132_metrics.txt", "w") as mf:
        mf.write("PARA_DELTA=999\nREAL_DIFF=999\nSPLIT_ONLY=999\nTABLE_DELTA=999\nOK_REAL=FAIL\nOK_PARA=FAIL\nOK_SPLIT=FAIL\n")
    sys.exit(0)
PYEOF

# Step 4: 更新迭代记录
echo ""
echo "--- Step 4: 更新迭代记录 ---"
RECORD_FILE="docs/verify/迭代记录.md"
if [ -f "scripts/update_iteration_record.py" ]; then
    python3 scripts/update_iteration_record.py \
        "${VERSION}" \
        "${OUT_DIR}/docx-compare.json" \
        "${RECORD_FILE}" 2>/dev/null || echo "  记录更新失败（可能首次运行）"
else
    echo "  脚本不存在，跳过自动记录"
fi

echo ""
echo "=== 质量迭代完成 ==="
echo "Markdown 报告: ${OUT_DIR}/docx-compare.md"
echo "JSON 指标:     ${OUT_DIR}/docx-compare.json"
echo "记录文件:      ${RECORD_FILE}"
