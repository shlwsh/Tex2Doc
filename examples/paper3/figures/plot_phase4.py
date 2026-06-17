#!/usr/bin/env python3
"""四期实验图表 — Nature 风格

改进要点：
- fig7：折线+面积图展示规模扩展趋势（取代柱状图）
- fig8：哑铃图(Dumbbell)+气泡图展示基线对照差异
"""

import json
import sys
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from plot_style import (
    apply_style, save, COLORS, FONT, style_axis,
    ROOT, FIGURES_DIR,
)

PHASE4 = ROOT / "experiments/results/phase4/phase4_latest.json"
OUT = FIGURES_DIR


def project_scale(measured: dict) -> dict:
    """由已测 8/16 节点外推 32/64（网关 Redis 近似线性，Center CPU 次线性）。"""
    nodes = sorted(int(k) for k in measured.keys())
    if len(nodes) < 2:
        return {}
    n1, n2 = nodes[0], nodes[-1]
    g1 = measured[str(n1)]["gateway_redis_mean"]
    g2 = measured[str(n2)]["gateway_redis_mean"]
    c1 = measured[str(n1)]["center_cpu_mean"]
    c2 = measured[str(n2)]["center_cpu_mean"]
    l1 = measured[str(n1)]["loki_stored_mean"]
    l2 = measured[str(n2)]["loki_stored_mean"]
    proj = {}
    for n in [32, 64]:
        ratio = n / n2
        proj[str(n)] = {
            "nodes": n,
            "gateway_redis_mean": round(g2 * ratio, 1),
            "center_cpu_mean": round(c2 * (ratio ** 0.85), 2),
            "loki_stored_mean": round((l1 + l2) / 2, 1),
            "method": f"projected_from_{n1}_{n2}_nodes",
        }
    return proj


def plot_scale(data: dict):
    """图7 规模扩展 — 折线+面积趋势图 + 实测/外推标记"""
    scale = data.get("scale", {})
    if not scale:
        return
    proj = project_scale(scale)
    all_pts = {**scale, **{k: v for k, v in proj.items()}}
    xs = sorted(int(k) for k in all_pts.keys())

    loki = [all_pts[str(x)].get("loki_stored_mean", 0) for x in xs]
    gw   = [all_pts[str(x)].get("gateway_redis_mean", 0) for x in xs]
    ctr  = [all_pts[str(x)].get("center_cpu_mean", 0) for x in xs]
    is_measured = [str(x) in scale for x in xs]

    fig, axes = plt.subplots(1, 3, figsize=(15, 5))

    sub_data = [
        (loki, "Loki 入库量（定向模式）", "条/轮", "#2980B9", "#AED6F1"),
        (gw,   "网关 Redis 增量",         "条/轮", "#27AE60", "#ABEBC6"),
        (ctr,  "Center CPU",              "%",     "#E67E22", "#FAD7A0"),
    ]

    for ax, (vals, title, ylabel, color_main, color_fill) in zip(axes, sub_data):
        x_arr = np.array(xs)
        v_arr = np.array(vals)

        # 面积填充
        ax.fill_between(x_arr, 0, v_arr, alpha=0.15, color=color_fill, zorder=1)

        # 折线
        ax.plot(x_arr, v_arr, "-", color=color_main, lw=2.5, zorder=3)

        # 实测点 vs 外推点
        for i, (x, v, m) in enumerate(zip(xs, vals, is_measured)):
            if m:
                ax.plot(x, v, "o", color=color_main, markersize=10,
                        markeredgecolor="white", markeredgewidth=2, zorder=5)
            else:
                ax.plot(x, v, "D", color="#BDBDBD", markersize=9,
                        markeredgecolor=color_main, markeredgewidth=1.5,
                        zorder=5, alpha=0.8)

            # 数值标注
            fmt = f"{v:.0f}" if v >= 1 else f"{v:.2f}"
            ax.text(x, v + (max(vals) - min(vals)) * 0.08, fmt,
                    ha="center", va="bottom",
                    fontsize=FONT["annotation"] - 1, fontweight="bold",
                    color=color_main)

        # 分割实测与外推区域
        measured_xs = [x for x, m in zip(xs, is_measured) if m]
        if measured_xs:
            ax.axvspan(measured_xs[-1], xs[-1], alpha=0.06,
                       color="#BDBDBD", zorder=0)
            ax.axvline(measured_xs[-1], color="#BDBDBD", ls=":",
                       lw=1.0, alpha=0.5)

        style_axis(ax, xlabel="节点数", ylabel=ylabel, title=title)
        ax.set_xticks(xs)
        ax.set_xticklabels([str(x) for x in xs], fontsize=FONT["tick"])
        ymax = max(vals) * 1.25
        ax.set_ylim(0, ymax)
        ax.grid(True, axis="y", alpha=0.12)

    # 全局图例（只在第一个子图）
    from matplotlib.lines import Line2D
    legend_elements = [
        Line2D([0], [0], marker="o", color="w", markerfacecolor="#2980B9",
               markersize=9, markeredgecolor="white", markeredgewidth=1.5,
               label="集群实测"),
        Line2D([0], [0], marker="D", color="w", markerfacecolor="#BDBDBD",
               markersize=8, markeredgecolor="#2980B9", markeredgewidth=1.5,
               label="线性外推"),
    ]
    axes[0].legend(handles=legend_elements, loc="upper left",
                   fontsize=FONT["legend"] - 1, frameon=True,
                   framealpha=0.9, edgecolor="#CCC")

    save(fig, "fig7_scale_bars")


def plot_baselines(data: dict):
    """图8 工业基线对照 — 哑铃图(Dumbbell) + 数值标注"""
    bl = data.get("baselines")
    if not bl:
        return

    names = ["本文定向采集", "Promtail 静态过滤", "OTel 尾采样", "eBPF 探针"]
    keys = ["p3_directed_measured", "promtail_static_filter",
            "opentelemetry_tail_sampling", "ebpf_probe"]
    evidence = ["measured", "simulated", "estimated", "estimated"]

    logs = []
    cpus = []
    for k in keys:
        v = bl[k]
        logs.append(v.get("loki_stored") or v.get("estimated_loki_stored", 0))
        cpus.append(v.get("cpu_percent") or v.get("estimated_cpu_percent", 0))

    fig, (ax_loki, ax_cpu) = plt.subplots(1, 2, figsize=(14, 5),
                                           gridspec_kw={"wspace": 0.35})

    y_pos = np.arange(len(names))

    # ── 颜色与标记 ────────────────────────────────────────
    colors_scheme = ["#2980B9", "#E67E22", "#9B59B6", "#1ABC9C"]
    markers = {"measured": "o", "simulated": "s", "estimated": "D"}
    evidence_labels = {"measured": "实测", "simulated": "规则复现", "estimated": "估算"}

    # ── 左图：Loki 入库量（水平棒棒糖图）─────────────────
    for i, (name, val, ev, color) in enumerate(zip(names, logs, evidence, colors_scheme)):
        # 水平茎
        ax_loki.plot([0, val], [i, i], color=color, lw=3.0, alpha=0.6, zorder=3)
        # 头部圆点
        mk = markers[ev]
        ax_loki.scatter(val, i, s=140, color=color, marker=mk,
                        edgecolors="white", linewidth=1.5, zorder=5)
        # 数值标注
        ax_loki.text(val + max(logs) * 0.03, i,
                     f"{val:.0f}",
                     ha="left", va="center",
                     fontsize=FONT["annotation"], fontweight="bold",
                     color=color)
        # 证据等级
        ax_loki.text(val + max(logs) * 0.03, i - 0.25,
                     f"({evidence_labels[ev]})",
                     ha="left", va="center",
                     fontsize=7, color="#888", fontstyle="italic")

    ax_loki.set_yticks(y_pos)
    ax_loki.set_yticklabels(names, fontsize=FONT["tick"])
    ax_loki.invert_yaxis()
    style_axis(ax_loki, xlabel="Loki 入库条数", title="Loki 入库量对比")
    ax_loki.set_xlim(0, max(logs) * 1.3)
    ax_loki.grid(True, axis="x", alpha=0.12)

    # 高亮本文（最小值）
    ax_loki.axhspan(-0.4, 0.4, alpha=0.08, color="#2980B9", zorder=0)

    # ── 右图：CPU 消耗（水平棒棒糖图）───────────────────
    for i, (name, val, ev, color) in enumerate(zip(names, cpus, evidence, colors_scheme)):
        ax_cpu.plot([0, val], [i, i], color=color, lw=3.0, alpha=0.6, zorder=3)
        mk = markers[ev]
        ax_cpu.scatter(val, i, s=140, color=color, marker=mk,
                       edgecolors="white", linewidth=1.5, zorder=5)
        ax_cpu.text(val + max(cpus) * 0.03, i,
                    f"{val:.2f}%",
                    ha="left", va="center",
                    fontsize=FONT["annotation"], fontweight="bold",
                    color=color)
        ax_cpu.text(val + max(cpus) * 0.03, i - 0.25,
                    f"({evidence_labels[ev]})",
                    ha="left", va="center",
                    fontsize=7, color="#888", fontstyle="italic")

    ax_cpu.set_yticks(y_pos)
    ax_cpu.set_yticklabels(names, fontsize=FONT["tick"])
    ax_cpu.invert_yaxis()
    style_axis(ax_cpu, xlabel="Agent CPU (%)", title="Agent CPU 消耗对比")
    ax_cpu.set_xlim(0, max(cpus) * 1.4)
    ax_cpu.grid(True, axis="x", alpha=0.12)

    ax_cpu.axhspan(-0.4, 0.4, alpha=0.08, color="#2980B9", zorder=0)

    # 合并图例
    from matplotlib.lines import Line2D
    legend_elements = [
        Line2D([0], [0], marker="o", color="w", markerfacecolor="#666",
               markersize=9, label="实测"),
        Line2D([0], [0], marker="s", color="w", markerfacecolor="#666",
               markersize=9, label="规则复现"),
        Line2D([0], [0], marker="D", color="w", markerfacecolor="#666",
               markersize=9, label="估算"),
    ]
    ax_cpu.legend(handles=legend_elements, loc="lower right",
                  fontsize=FONT["legend"] - 1, frameon=True,
                  framealpha=0.9, edgecolor="#CCC")

    save(fig, "fig8_baseline_bars")


def main():
    if not PHASE4.exists():
        print(f"[plot_phase4] skip: {PHASE4} not found")
        return
    apply_style()
    data = json.loads(PHASE4.read_text(encoding="utf-8"))
    OUT.mkdir(parents=True, exist_ok=True)
    print("生成改进版图表 (fig7–fig8)...")
    plot_scale(data)
    plot_baselines(data)
    proj = project_scale(data.get("scale", {}))
    if proj:
        data["scale_projected"] = proj
        PHASE4.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
    print("[plot_phase4] fig7_scale_bars, fig8_baseline_bars")


if __name__ == "__main__":
    main()
