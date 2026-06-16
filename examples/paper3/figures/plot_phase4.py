#!/usr/bin/env python3
"""四期实验图表：规模扩展曲线与基线对照"""

import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
PHASE4 = ROOT / "experiments/results/phase4/phase4_latest.json"
OUT = ROOT / "figures"

def setup_cn():
    from matplotlib import font_manager
    font_paths = [
        "/usr/share/fonts/truetype/arphic/uming.ttc",
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    ]
    cn_font_name = None
    for fp in font_paths:
        if Path(fp).exists():
            font_manager.fontManager.addfont(fp)
            prop = font_manager.FontProperties(fname=fp)
            cn_font_name = prop.get_name()
            break
    if cn_font_name:
        plt.rcParams["font.family"] = "sans-serif"
        plt.rcParams["font.sans-serif"] = [cn_font_name] + plt.rcParams.get("font.sans-serif", [])
    else:
        plt.rcParams["font.sans-serif"] = ["WenQuanYi Micro Hei", "Noto Sans CJK SC", "DejaVu Sans"]
    plt.rcParams["axes.unicode_minus"] = False


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
    scale = data.get("scale", {})
    if not scale:
        return
    proj = project_scale(scale)
    all_pts = {**scale, **{k: v for k, v in proj.items()}}
    xs = sorted(int(k) for k in all_pts.keys())
    loki = [all_pts[str(x)].get("loki_stored_mean", 0) for x in xs]
    gw = [all_pts[str(x)].get("gateway_redis_mean", 0) for x in xs]
    ctr = [all_pts[str(x)].get("center_cpu_mean", 0) for x in xs]

    fig, axes = plt.subplots(1, 3, figsize=(12, 3.8))
    colors = ["#2ecc71" if str(x) in scale else "#95a5a6" for x in xs]

    axes[0].bar([str(x) for x in xs], loki, color=colors)
    axes[0].set_title("Loki 入库量（定向模式）")
    axes[0].set_xlabel("节点数")
    axes[0].set_ylabel("条/轮")

    axes[1].bar([str(x) for x in xs], gw, color=colors)
    axes[1].set_title("网关 Redis 增量")
    axes[1].set_xlabel("节点数")

    axes[2].bar([str(x) for x in xs], ctr, color=colors)
    axes[2].set_title("Center CPU (%)")
    axes[2].set_xlabel("节点数")


    fig.tight_layout()
    fig.savefig(OUT / "fig7_scale_bars.pdf", bbox_inches="tight")
    fig.savefig(OUT / "fig7_scale_bars.png", dpi=150, bbox_inches="tight")
    plt.close(fig)


def plot_baselines(data: dict):
    bl = data.get("baselines")
    if not bl:
        return
    names = ["p3_directed\n(实测)", "promtail_static", "otel_tail", "ebpf_probe"]
    keys = ["p3_directed_measured", "promtail_static_filter", "opentelemetry_tail_sampling", "ebpf_probe"]
    logs = []
    cpus = []
    for k in keys:
        v = bl[k]
        logs.append(v.get("loki_stored") or v.get("estimated_loki_stored", 0))
        cpus.append(v.get("cpu_percent") or v.get("estimated_cpu_percent", 0))

    x = np.arange(len(names))
    w = 0.35
    fig, ax = plt.subplots(figsize=(8, 4))
    ax.bar(x - w / 2, logs, w, label="Loki 入库 (条)", color="#3498db")
    ax2 = ax.twinx()
    ax2.bar(x + w / 2, cpus, w, label="CPU (%)", color="#e74c3c", alpha=0.8)
    ax.set_xticks(x)
    ax.set_xticklabels(names, fontsize=9)
    ax.set_ylabel("Loki 入库条数")
    ax2.set_ylabel("CPU (%)")

    fig.tight_layout()
    fig.savefig(OUT / "fig8_baseline_bars.pdf", bbox_inches="tight")
    fig.savefig(OUT / "fig8_baseline_bars.png", dpi=150, bbox_inches="tight")
    plt.close(fig)


def main():
    if not PHASE4.exists():
        print(f"[plot_phase4] skip: {PHASE4} not found")
        return
    setup_cn()
    data = json.loads(PHASE4.read_text(encoding="utf-8"))
    OUT.mkdir(parents=True, exist_ok=True)
    plot_scale(data)
    plot_baselines(data)
    proj = project_scale(data.get("scale", {}))
    if proj:
        data["scale_projected"] = proj
        PHASE4.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
    print("[plot_phase4] fig7_scale_bars, fig8_baseline_bars")


if __name__ == "__main__":
    main()
