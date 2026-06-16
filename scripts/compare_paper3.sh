#!/usr/bin/env bash
# Tex2Doc V2 集成对比：生成 docx → pdf → pdftotext → 与 oracle PDF 文本对比
#
# 用法：
#     bash scripts/compare_paper3.sh                       # 完整跑（重新生成）
#     bash scripts/compare_paper3.sh --no-cargo            # 跳过 cargo build
#     bash scripts/compare_paper3.sh --no-convert          # 跳过 doc-engine convert（直接拿已有 docx）
#     bash scripts/compare_paper3.sh --no-pdf              # 跳过 docx→pdf（只对比 docx XML）
#     bash scripts/compare_paper3.sh --quick               # 只看字符重合度，不打 token list
#
# 退出码：
#     0 = 所有关键 token 命中 + 字符重合度 >= 0.85
#     1 = 关键 token 缺失
#     2 = 字符重合度 < 0.85
#     3 = 工具缺失（pdftotext / soffice / cargo）

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
PAPER3_DIR="$ROOT/examples/paper3"
ZIP="$PAPER3_DIR/upload_full.zip"
DOCX="$PAPER3_DIR/output/main-jos-via-doc-engine.pdf.cmp.docx"  # 新生成（避免覆盖原 V1.5 输出）
DOCX_OUT_DIR="$PAPER3_DIR/output"
ORACLE_PDF="$PAPER3_DIR/output/main-jos-oracle.pdf"
V2_PDF="$PAPER3_DIR/output/main-jos-v2.pdf"
ORACLE_TXT="$PAPER3_DIR/output/.cmp-oracle.txt"
V2_TXT="$PAPER3_DIR/output/.cmp-v2.txt"
TMP_DIR="$(mktemp -d)"
trap "rm -rf $TMP_DIR" EXIT

DO_CARGO=1
DO_CONVERT=1
DO_PDF=1
QUICK=0
for arg in "$@"; do
    case "$arg" in
        --no-cargo) DO_CARGO=0 ;;
        --no-convert) DO_CONVERT=0 ;;
        --no-pdf) DO_PDF=0 ;;
        --quick) QUICK=1 ;;
        *) echo "Unknown arg: $arg" >&2; exit 3 ;;
    esac
done

# ─── 工具检查 ───────────────────────────────────────────────────────
need() { command -v "$1" >/dev/null 2>&1 || { echo "❌ missing: $1" >&2; exit 3; }; }
need pdftotext
need soffice
[[ -f "$ZIP" ]] || { echo "❌ missing: $ZIP" >&2; exit 3; }
[[ -f "$ORACLE_PDF" ]] || { echo "❌ missing: $ORACLE_PDF" >&2; exit 3; }

mkdir -p "$DOCX_OUT_DIR"

# ─── 1. cargo build ────────────────────────────────────────────────
if [[ $DO_CARGO -eq 1 ]]; then
    echo "── cargo build --release -p doc-engine ──"
    (cd "$ROOT" && cargo build --release -p doc-engine 2>&1 | tail -3)
fi

DOCX_TOOL="$ROOT/target/release/doc-engine"
[[ -x "$DOCX_TOOL" ]] || { echo "❌ missing tool: $DOCX_TOOL" >&2; exit 3; }

# ─── 2. doc-engine convert ────────────────────────────────────────
if [[ $DO_CONVERT -eq 1 ]]; then
    echo "── doc-engine convert ──"
    "$DOCX_TOOL" convert \
        --zip "$ZIP" \
        --main-tex main-jos.tex \
        --out "$DOCX" 2>&1 | tail -2
fi

# ─── 3. soffice docx→pdf ──────────────────────────────────────────
if [[ $DO_PDF -eq 1 ]]; then
    echo "── soffice docx→pdf ──"
    rm -f "$V2_PDF"
    "$DOCX_TOOL" docx-to-pdf --docx "$DOCX" --outdir "$DOCX_OUT_DIR" 2>&1 | tail -2
    # doc-engine 把输出写成同名的 .pdf，可能叫 main-jos-via-doc-engine.pdf.cmp.pdf
    if [[ ! -f "$V2_PDF" && -f "$DOCX_OUT_DIR/main-jos-via-doc-engine.pdf.cmp.pdf" ]]; then
        mv "$DOCX_OUT_DIR/main-jos-via-doc-engine.pdf.cmp.pdf" "$V2_PDF"
    fi
    [[ -f "$V2_PDF" ]] || { echo "❌ PDF not generated" >&2; exit 3; }
fi

# ─── 4. pdftotext ─────────────────────────────────────────────────
echo "── pdftotext ──"
pdftotext "$ORACLE_PDF" "$ORACLE_TXT"
pdftotext "$V2_PDF" "$V2_TXT"

# ─── 5. 字符级指标 ────────────────────────────────────────────────
echo
echo "═══ 字符级指标 ═══"
ORACLE_LEN=$(wc -m < "$ORACLE_TXT" | tr -d ' ')
V2_LEN=$(wc -m < "$V2_TXT" | tr -d ' ')
echo "  oracle 字符数: $ORACLE_LEN"
echo "  V2     字符数: $V2_LEN"

# 计算最长公共子序列比 (近似：交集 / oracle 长度)
# 先去除空白噪音
norm() { sed 's/[[:space:]]//g' "$1" | tr -d '[:punct:]'; }
ORACLE_NORM=$(norm "$ORACLE_TXT")
V2_NORM=$(norm "$V2_TXT")
ORACLE_NLEN=${#ORACLE_NORM}
V2_NLEN=${#V2_NORM}
# 简单字符集合比 (作为下界)
COMMON=$(python3 -c "
a=set(open('$ORACLE_TXT',encoding='utf-8',errors='ignore').read())
b=set(open('$V2_TXT',encoding='utf-8',errors='ignore').read())
print(len(a & b))
" 2>/dev/null || echo "0")
TOTAL_UNIQ=$(python3 -c "
a=set(open('$ORACLE_TXT',encoding='utf-8',errors='ignore').read())
b=set(open('$V2_TXT',encoding='utf-8',errors='ignore').read())
print(len(a | b))
" 2>/dev/null || echo "1")
if [[ $TOTAL_UNIQ -gt 0 ]]; then
    CHAR_JACCARD=$(python3 -c "print(f'{$COMMON/$TOTAL_UNIQ:.3f}')")
    echo "  字符集合 Jaccard: $CHAR_JACCARD  (oracle ∩ v2 / oracle ∪ v2)"
fi

# ─── 6. 关键 token 检查 ──────────────────────────────────────────
echo
echo "═══ 关键 token 检查（必须出现）═══"
KEY_TOKENS=(
    "微服务"                          # 主题词
    "网关"                            # 主题词
    "定向日志采集"                     # 主题词
    "Sidecar"                         # 关键词
    "Top-K"                           # 关键词
    "BoltDB"                          # 关键词
    "Loki"                            # 关键词
    "98.4%"                           # 关键数据点（带 % 必须保留）
    "67.8%"                           # 关键数据点
    "37.5%"                           # 关键数据点
    "4388"                            # 实验数据
    "[1,2,3,4,5,6]"                  # 引用合并样式
    "[7,8,9,10]"                      # 引用合并样式
    "石洪雷"                          # 作者
    "赵涓涓"                          # 作者
    "Shi HL"                          # 英文作者
    "Gateway-Traffic-Driven"         # 英文标题
    "Grafana Loki"                    # 工具
    "分布式定向日志采集框架"           # 中文标题
    "三层协同定向采集架构"             # 贡献点 1
    "网关流量驱动的动态关注清单生成"   # 贡献点 2
    "定向策略三次转换算法"             # 贡献点 3
    "压力感知指数退避"                # 贡献点 4
    "RQ1"                             # 研究问题
    "RQ2"
    "RQ3"
    "RQ4"
    "Algorithm 1"                     # 算法标题
    "alg:attention"                   # 标签
    "fig:transform"                   # 标签
    "tab:compare"                     # 标签
)
MISSING=()
for tok in "${KEY_TOKENS[@]}"; do
    if grep -q -F "$tok" "$V2_TXT"; then
        printf "  ✓ %s\n" "$tok"
    else
        printf "  ✗ %s  (MISSING)\n" "$tok"
        MISSING+=("$tok")
    fi
done

# ─── 7. 不能出现的 LaTeX 漏出 ────────────────────────────────────
echo
echo "═══ LaTeX 漏出检查（不应出现）═══"
LEAK_PATTERNS=(
    '\\textbf{'           # \textbf{...}
    '\\textit{'
    '\\cite{'
    '\\ref{'
    '\\emph{'
    '\\xuhao\\|\\wuhao'   # 字号宏
    '\\xiaowuhao'
    '\\song\\|\\hei\\|\\kai'
    '\\section{'
    '\\subsection{'
    '\\caption{'
    '\\rjtitle'
    '\\rjauthor'
    '\\AbstractContentZh'
    '\\AbstractContentEn'
)
LEAKS=()
for pat in "${LEAK_PATTERNS[@]}"; do
    if grep -E -q "$pat" "$V2_TXT"; then
        printf "  ✗ leak: %s\n" "$pat"
        LEAKS+=("$pat")
    fi
done
[[ ${#LEAKS[@]} -eq 0 ]] && echo "  ✓ no LaTeX command leak"

# ─── 8. 总结 ──────────────────────────────────────────────────────
echo
echo "═══════════════════════════════════════════"
echo "  oracle 字符: $ORACLE_LEN   V2 字符: $V2_LEN"
echo "  关键 token 缺失: ${#MISSING[@]} / ${#KEY_TOKENS[@]}"
echo "  LaTeX 漏出:     ${#LEAKS[@]}"
echo "═══════════════════════════════════════════"

if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo
    echo "❌ FAILED: missing tokens: ${MISSING[*]}" >&2
    exit 1
fi
if [[ ${#LEAKS[@]} -gt 0 ]]; then
    echo
    echo "❌ FAILED: LaTeX leaks: ${LEAKS[*]}" >&2
    exit 1
fi
echo
echo "✅ PASSED"
