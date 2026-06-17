#!/usr/bin/env python3
"""
fig9_dsb_comparison — DSB 与 httpbin 负载对比图

双面板图: (a) 降幅与召回率对比  (b) RPS 扩展性

用法:
    cd /home/ros/work/p3-microservice
    python3 figures/fig9_dsb_comparison.py
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from plot_style import apply_style, save, COLORS, FONT

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

# ── 加载 DSB 实验数据 ────────────────────────────────

DATA_DIR = Path(__file__).resolve().parents[1] / "experiments" / "results" / "phase7"


def load_dsb_data():
    files = sorted(DATA_DIR.glob("dsb_validation_*.json"))
    if not files:
        raise FileNotFoundError(f"未找到 DSB 数据: {DATA_DIR}")
    with open(files[-1]) as f:
        return json.load(f)


# ── 色板 ─────────────────────────────────────────────

C_HTTPBIN = "#2171B5"   # 深蓝 — httpbin
C_DSB = "#E6550D"       # 深橙 — DSB
C_RECALL = "#27AE60"    # 绿色 — 召回率
C_REDUCTION = "#8E44AD" # 紫色 — 降幅


def plot_comparison():
    apply_style()

    data = load_dsb_data()

    # 提取数据
    standard = next(e for e in data["experiments"] if e["name"] == "standard_config")
    rps_data = next(e for e in data["experiments"] if e["name"] == "rps_scaling")
    comparison = next(e for e in data["experiments"] if e["name"] == "workload_comparison")

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5.5))

    # ─── Panel (a): 负载对比条形图 ──────────────────────

    workloads = ['httpbin\n(8 端点)', 'DSB Social Network\n(21 端点)']
    reduction_vals = [
        comparison["comparison"]["httpbin"]["reduction_rate"],
        comparison["comparison"]["dsb"]["reduction_rate"],
    ]
    recall_vals = [
        comparison["comparison"]["httpbin"]["recall"],
        comparison["comparison"]["dsb"]["recall"],
    ]

    x = np.arange(len(workloads))
    w = 0.35

    bars1 = ax1.bar(x - w/2, reduction_vals, w, color=C_REDUCTION, alpha=0.85,
                    edgecolor='white', linewidth=1.5, label='日志降幅', zorder=3)
    bars2 = ax1.bar(x + w/2, recall_vals, w, color=C_RECALL, alpha=0.85,
                    edgecolor='white', linewidth=1.5, label='高价值召回率', zorder=3)

    # 数值标注
    for bar, val in zip(bars1, reduction_vals):
        ax1.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 1.5,
                f'{val:.1f}%', ha='center', va='bottom',
                fontsize=FONT["annotation"], fontweight='bold', color=C_REDUCTION)

    for bar, val in zip(bars2, recall_vals):
        ax1.text(bar.get_x() + bar.get_width()/2, bar.get_height() + 1.5,
                f'{val:.1f}%', ha='center', va='bottom',
                fontsize=FONT["annotation"], fontweight='bold', color=C_RECALL)

    ax1.set_xticks(x)
    ax1.set_xticklabels(workloads, fontsize=FONT["tick"])
    ax1.set_ylabel('比率（%）', fontsize=FONT["label"], fontweight='medium')
    ax1.set_ylim(0, 115)
    ax1.legend(fontsize=FONT["legend"], frameon=True, framealpha=0.9,
              edgecolor='#CCC', loc='upper center')

    # 添加注释
    ax1.annotate('高减量\n低端点数', xy=(0, 50), fontsize=9,
                ha='center', color='#666', style='italic')
    ax1.annotate('低减量\n高端点数', xy=(1, 30), fontsize=9,
                ha='center', color='#666', style='italic')

    ax1.set_title('(a) 不同负载的减量效果', fontsize=FONT["title"],
                  fontweight='bold', pad=12)

    # ─── Panel (b): RPS 扩展性 ─────────────────────────

    rps_vals = [r["rps"] for r in rps_data["results"]]
    rps_reduction = [r["avg_reduction"] for r in rps_data["results"]]
    rps_recall = [r["avg_recall"] for r in rps_data["results"]]

    ax2_twin = ax2.twinx()

    # 降幅面积填充
    ax2.fill_between(rps_vals, rps_reduction, alpha=0.15, color=C_REDUCTION)
    line1, = ax2.plot(rps_vals, rps_reduction, 'o-', color=C_REDUCTION, lw=2.5,
                      markersize=8, markeredgecolor='white', markeredgewidth=1.5,
                      label='日志降幅', zorder=5)

    # 召回率折线
    line2, = ax2_twin.plot(rps_vals, rps_recall, 's--', color=C_RECALL, lw=2.5,
                           markersize=8, markeredgecolor='white', markeredgewidth=1.5,
                           label='高价值召回率', zorder=5)

    # 数值标注
    for x_val, y_val in zip(rps_vals, rps_reduction):
        ax2.annotate(f'{y_val:.1f}%', (x_val, y_val), textcoords="offset points",
                    xytext=(0, 12), ha='center', fontsize=10,
                    fontweight='bold', color=C_REDUCTION)

    for x_val, y_val in zip(rps_vals, rps_recall):
        ax2_twin.annotate(f'{y_val:.1f}%', (x_val, y_val), textcoords="offset points",
                         xytext=(0, -16), ha='center', fontsize=10,
                         fontweight='bold', color=C_RECALL)

    ax2.set_xlabel('请求速率（RPS）', fontsize=FONT["label"], fontweight='medium')
    ax2.set_ylabel('日志降幅（%）', fontsize=FONT["label"],
                   fontweight='medium', color=C_REDUCTION)
    ax2_twin.set_ylabel('高价值召回率（%）', fontsize=FONT["label"],
                        fontweight='medium', color=C_RECALL)

    ax2.set_xscale('log')
    ax2.set_xticks(rps_vals)
    ax2.set_xticklabels([str(r) for r in rps_vals])
    ax2.tick_params(axis='y', labelcolor=C_REDUCTION)
    ax2_twin.tick_params(axis='y', labelcolor=C_RECALL)
    ax2_twin.set_ylim(90, 100)
    ax2_twin.spines['right'].set_visible(True)
    ax2_twin.spines['right'].set_color(C_RECALL)

    lines = [line1, line2]
    labels = [l.get_label() for l in lines]
    ax2.legend(lines, labels, loc='center right', fontsize=FONT["legend"],
              frameon=True, framealpha=0.9, edgecolor='#CCC')

    ax2.set_title('(b) DSB 负载下 RPS 扩展性', fontsize=FONT["title"],
                  fontweight='bold', pad=12)

    # ── 保存 ─────────────────────────────────────────
    save(fig, "fig9_dsb_comparison")
    print("✅ DSB 对比图已保存至 figures/fig9_dsb_comparison.pdf/.png")


if __name__ == "__main__":
    plot_comparison()
