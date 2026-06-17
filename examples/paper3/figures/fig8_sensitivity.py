#!/usr/bin/env python3
"""
参数敏感性图表 — B3: fig8_sensitivity

生成 Top-K 和延迟阈值 T 的敏感性分析双面板图，
适用于《软件学报》投稿。

用法:
    cd /home/ros/work/p3-microservice
    python3 figures/fig8_sensitivity.py
"""

import json
import sys
from pathlib import Path

# 确保能导入 plot_style
sys.path.insert(0, str(Path(__file__).resolve().parent))
from plot_style import apply_style, save, COLORS, FONT, style_axis

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

# ── 加载实验数据 ─────────────────────────────────────

DATA_DIR = Path(__file__).resolve().parents[1] / "experiments" / "results" / "phase6"

def load_latest_data():
    """加载最新的敏感性数据文件。"""
    files = sorted(DATA_DIR.glob("sensitivity_*.json"))
    if not files:
        raise FileNotFoundError(f"未找到敏感性数据: {DATA_DIR}")
    with open(files[-1]) as f:
        return json.load(f)

# ── 色板 ─────────────────────────────────────────────

C_INGEST = "#2171B5"     # 深蓝 — 入库量
C_COVER  = "#E6550D"     # 深橙 — 覆盖率
C_PATTERN = "#27AE60"    # 绿色 — 模式数
C_HIGHLIGHT = "#C0392B"  # 红色 — 推荐值

# ── 绘制主图 ─────────────────────────────────────────

def plot_sensitivity():
    apply_style()

    data = load_latest_data()
    topk_data = data["results"]["topk_sensitivity"]
    threshold_data = data["results"]["threshold_sensitivity"]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5.5))

    # ─── Panel (a): Top-K 敏感性 ────────────────────────

    k_vals = [d["param_value"] for d in topk_data]
    ingest = [d["loki_ingested"] for d in topk_data]
    coverage = [d["coverage_rate"] * 100 for d in topk_data]
    patterns = [d["pattern_count"] for d in topk_data]

    # 双 Y 轴
    ax1_twin = ax1.twinx()

    # 入库量折线（蓝）
    line1, = ax1.plot(k_vals, ingest, 'o-', color=C_INGEST, lw=2.5,
                       markersize=8, markeredgecolor='white', markeredgewidth=1.5,
                       label='Loki 入库量', zorder=5)

    # 覆盖率折线（橙）
    line2, = ax1_twin.plot(k_vals, coverage, 's--', color=C_COVER, lw=2.5,
                            markersize=8, markeredgecolor='white', markeredgewidth=1.5,
                            label='规则级覆盖率', zorder=5)

    # 数值标注
    for x, y in zip(k_vals, ingest):
        ax1.annotate(f'{y:.0f}', (x, y), textcoords="offset points",
                    xytext=(0, 12), ha='center', fontsize=FONT["annotation"],
                    fontweight='bold', color=C_INGEST)

    for x, y in zip(k_vals, coverage):
        ax1_twin.annotate(f'{y:.1f}%', (x, y), textcoords="offset points",
                         xytext=(0, -18), ha='center', fontsize=FONT["annotation"],
                         fontweight='bold', color=C_COVER)

    # 推荐区域（K=20~30）
    ax1.axvspan(18, 32, alpha=0.08, color=C_HIGHLIGHT, zorder=0)
    ax1.annotate('推荐区间', xy=(25, max(ingest)*0.95), fontsize=10,
                ha='center', color=C_HIGHLIGHT, fontweight='bold',
                bbox=dict(boxstyle='round,pad=0.3', facecolor='white',
                         edgecolor=C_HIGHLIGHT, alpha=0.9))

    # 95% 覆盖率参考线
    ax1_twin.axhline(y=95, color=C_COVER, ls=':', lw=1.2, alpha=0.5)
    ax1_twin.annotate('95%', xy=(k_vals[-1]+3, 95), fontsize=10,
                     color=C_COVER, va='center', fontweight='bold')

    # 样式
    ax1.set_xlabel('Top-K 取值', fontsize=FONT["label"], fontweight='medium')
    ax1.set_ylabel('Loki 入库量（条）', fontsize=FONT["label"],
                   fontweight='medium', color=C_INGEST)
    ax1_twin.set_ylabel('覆盖率（%）', fontsize=FONT["label"],
                        fontweight='medium', color=C_COVER)
    ax1.set_xticks(k_vals)
    ax1_twin.set_ylim(80, 102)
    ax1.tick_params(axis='y', labelcolor=C_INGEST)
    ax1_twin.tick_params(axis='y', labelcolor=C_COVER)
    ax1_twin.spines['right'].set_visible(True)
    ax1_twin.spines['right'].set_color(C_COVER)
    ax1_twin.spines['right'].set_linewidth(1.2)

    # 合并图例
    lines = [line1, line2]
    labels = [l.get_label() for l in lines]
    ax1.legend(lines, labels, loc='lower right', fontsize=FONT["legend"],
              frameon=True, framealpha=0.9, edgecolor='#CCC')

    ax1.set_title('(a) Top-K 敏感性', fontsize=FONT["title"],
                  fontweight='bold', pad=12)

    # ─── Panel (b): 延迟阈值 T 敏感性 ──────────────────

    t_vals = [d["param_value"] for d in threshold_data]
    t_ingest = [d["loki_ingested"] for d in threshold_data]
    t_hv = [d["high_value_count"] for d in threshold_data]

    ax2_twin = ax2.twinx()

    # 入库量柱状图（蓝）
    bars = ax2.bar(range(len(t_vals)), t_ingest, width=0.6,
                   color=C_INGEST, alpha=0.85, edgecolor='white',
                   linewidth=1.5, label='Loki 入库量', zorder=3)

    # 高价值日志数折线（绿）
    line3, = ax2_twin.plot(range(len(t_vals)), t_hv, 'D-', color=C_PATTERN,
                           lw=2.5, markersize=8, markeredgecolor='white',
                           markeredgewidth=1.5, label='高价值日志数', zorder=5)

    # 数值标注
    for i, (v, hv) in enumerate(zip(t_ingest, t_hv)):
        ax2.annotate(f'{v:.0f}', (i, v), textcoords="offset points",
                    xytext=(0, 8), ha='center', fontsize=FONT["annotation"],
                    fontweight='bold', color=C_INGEST)
        ax2_twin.annotate(f'{hv:.0f}', (i, hv), textcoords="offset points",
                         xytext=(0, 10), ha='center', fontsize=FONT["annotation"],
                         fontweight='bold', color=C_PATTERN)

    # 默认值标记
    default_idx = t_vals.index(500)
    bars[default_idx].set_edgecolor(C_HIGHLIGHT)
    bars[default_idx].set_linewidth(3)
    ax2.annotate('默认值', (default_idx, t_ingest[default_idx]),
                textcoords="offset points", xytext=(25, 25),
                fontsize=10, fontweight='bold', color=C_HIGHLIGHT,
                arrowprops=dict(arrowstyle='->', color=C_HIGHLIGHT, lw=1.5),
                bbox=dict(boxstyle='round,pad=0.3', facecolor='white',
                         edgecolor=C_HIGHLIGHT, alpha=0.9))

    # 样式
    ax2.set_xlabel('延迟阈值 T（ms）', fontsize=FONT["label"], fontweight='medium')
    ax2.set_ylabel('Loki 入库量（条）', fontsize=FONT["label"],
                   fontweight='medium', color=C_INGEST)
    ax2_twin.set_ylabel('高价值日志数', fontsize=FONT["label"],
                        fontweight='medium', color=C_PATTERN)
    ax2.set_xticks(range(len(t_vals)))
    ax2.set_xticklabels([str(t) for t in t_vals])
    ax2.tick_params(axis='y', labelcolor=C_INGEST)
    ax2_twin.tick_params(axis='y', labelcolor=C_PATTERN)
    ax2_twin.spines['right'].set_visible(True)
    ax2_twin.spines['right'].set_color(C_PATTERN)
    ax2_twin.spines['right'].set_linewidth(1.2)

    # 合并图例
    from matplotlib.patches import Patch
    legend_elements = [
        Patch(facecolor=C_INGEST, alpha=0.85, edgecolor='white', label='Loki 入库量'),
        plt.Line2D([0], [0], color=C_PATTERN, marker='D', markersize=8,
                   markeredgecolor='white', lw=2.5, label='高价值日志数'),
    ]
    ax2.legend(handles=legend_elements, loc='upper right', fontsize=FONT["legend"],
              frameon=True, framealpha=0.9, edgecolor='#CCC')

    ax2.set_title('(b) 延迟阈值 T 敏感性', fontsize=FONT["title"],
                  fontweight='bold', pad=12)

    # ── 保存 ─────────────────────────────────────────
    save(fig, "fig8_sensitivity")
    print(f"✅ 敏感性图表已保存至 figures/fig8_sensitivity.pdf/.png")


if __name__ == "__main__":
    plot_sensitivity()
