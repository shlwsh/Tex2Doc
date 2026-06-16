#!/usr/bin/env python3
"""首期科研图表生成（论文用 PDF + PNG）"""

import json
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
from matplotlib.patches import FancyBboxPatch, FancyArrowPatch
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
FIGURES_DIR = ROOT / "figures"
RESULTS_FILE = ROOT / "experiments" / "results" / "phase1" / "phase1_latest.json"


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
    plt.rcParams.update({
        "axes.unicode_minus": False,
        "figure.dpi": 150,
        "savefig.dpi": 300,
        "font.size": 11,
    })


def load_data() -> dict:
    if not RESULTS_FILE.exists():
        raise FileNotFoundError(f"请先运行 phase1_collect.py，缺少 {RESULTS_FILE}")
    return json.loads(RESULTS_FILE.read_text(encoding="utf-8"))


def save(fig, name: str):
    for ext in ("pdf", "png"):
        fig.savefig(FIGURES_DIR / f"{name}.{ext}", bbox_inches="tight")
    plt.close(fig)
    print(f"[OK] {name}.pdf / .png")


def fig1_system_overview():
    """图1 系统总体架构"""
    fig, ax = plt.subplots(figsize=(10, 6))
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 6)
    ax.axis("off")

    boxes = [
        (0.5, 3.8, 2.2, 1.2, "网关节点\nOpenResty + Agent", "#E3F2FD"),
        (3.8, 3.8, 2.2, 1.2, "微服务节点\nApp + Agent", "#E8F5E9"),
        (7.1, 3.8, 2.2, 1.2, "微服务节点\nApp + Agent", "#E8F5E9"),
        (3.8, 1.5, 2.4, 1.4, "日志中心 Center\n策略生成 + 二次过滤", "#FFF3E0"),
        (0.8, 0.2, 1.8, 0.9, "Redis\n流量缓存", "#FCE4EC"),
        (4.1, 0.2, 1.8, 0.9, "Loki\n日志存储", "#F3E5F5"),
        (7.2, 0.2, 1.8, 0.9, "Grafana\n可视化", "#E0F7FA"),
    ]
    for x, y, w, h, text, color in boxes:
        ax.add_patch(FancyBboxPatch((x, y), w, h, boxstyle="round,pad=0.05",
                                    facecolor=color, edgecolor="#455A64", linewidth=1.2))
        ax.text(x + w / 2, y + h / 2, text, ha="center", va="center", fontsize=10)

    # --- 顶部三个节点之间的水平箭头 ---
    arrows = [
        ((2.7, 4.4), (3.8, 4.4), "流量"),           # 网关 → 微服务1
        ((6.0, 4.4), (7.1, 4.4), ""),                # 微服务1 → 微服务2
    ]
    # --- 节点到 Center 的垂直/斜向箭头 ---
    arrows += [
        ((4.9, 3.8), (4.9, 2.9), "gRPC 上传"),       # 微服务1 底部中心 → Center 顶部
        ((1.6, 3.8), (4.2, 2.9), "流量日志"),         # 网关底部中心 → Center 左上
    ]
    # --- Center 到底部存储的箭头 ---
    arrows += [
        ((5.0, 1.5), (5.0, 1.1), ""),                # Center → Loki
        ((4.1, 0.65), (2.6, 0.65), ""),              # Loki → Redis
        ((5.9, 0.65), (7.2, 0.65), ""),              # Loki → Grafana（查询方向）
    ]
    for src, dst, label in arrows:
        ax.annotate("", xy=dst, xytext=src,
                    arrowprops=dict(arrowstyle="->", color="#37474F", lw=1.5))
        if label:
            mx, my = (src[0] + dst[0]) / 2, (src[1] + dst[1]) / 2
            ax.text(mx, my + 0.18, label, ha="center", fontsize=9, color="#546E7A")


    save(fig, "fig1_system_overview")


def fig2_triple_transform():
    """图2 定向策略三次转换流程"""
    fig, ax = plt.subplots(figsize=(10, 2.4))
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 2.0)
    ax.axis("off")

    box_h = 1.2
    box_y = 0.4  # 垂直居中: (2.0 - 1.2) / 2 = 0.4
    stages = [
        (0.3, box_y, "定向策略\n(阈值T, 错误码)", "#BBDEFB"),
        (2.5, box_y, "第一次转换\n网关采集规则", "#90CAF9"),
        (4.7, box_y, "第二次转换\n关注清单→Agent规则", "#64B5F6"),
        (6.9, box_y, "第三次转换\n存储规则→Loki", "#42A5F5"),
    ]
    arrow_y = box_y + box_h / 2  # 箭头在方块垂直中心
    for i, (x, y, text, color) in enumerate(stages):
        ax.add_patch(FancyBboxPatch((x, y), 1.8, box_h, boxstyle="round,pad=0.04",
                                    facecolor=color, edgecolor="#1565C0"))
        ax.text(x + 0.9, y + box_h / 2, text, ha="center", va="center", fontsize=9)
        if i < len(stages) - 1:
            ax.annotate("", xy=(stages[i + 1][0], arrow_y), xytext=(x + 1.8, arrow_y),
                        arrowprops=dict(arrowstyle="->", lw=2, color="#0D47A1"))


    save(fig, "fig2_triple_transform")


def fig3_comparison(data: dict):
    """图3 定向采集 vs 全量采集对比"""
    comp = data["comparison"]
    d, f = comp["directed"], comp["full_collect"]
    metrics = ["CPU\n(%)", "内存\n(MB)", "带宽\n(KB/s)", "日志量\n(条)"]
    d_mem = d.get("memory_mb", d.get("memory_mb", 0))
    f_mem = f.get("memory_mb", f.get("memory_mb", 0))
    directed = [d["cpu_percent"], d_mem, d["bandwidth_mbps"], d["log_volume_k"] * 1000]
    full = [f["cpu_percent"], f_mem, f["bandwidth_mbps"], f["log_volume_k"] * 1000]

    x = np.arange(len(metrics))
    w = 0.35
    fig, ax = plt.subplots(figsize=(8, 5))
    b1 = ax.bar(x - w / 2, directed, w, label="定向采集", color="#1976D2", alpha=0.88)
    b2 = ax.bar(x + w / 2, full, w, label="全量采集", color="#E64A19", alpha=0.88)
    ax.set_ylabel("数值")
    note = "实测" if data.get("phase") == "phase2_measured" else "首期仿真"

    ax.set_xticks(x)
    ax.set_xticklabels(metrics)
    ax.legend()
    for bars in (b1, b2):
        for bar in bars:
            h = bar.get_height()
            ax.text(bar.get_x() + bar.get_width() / 2, h, f"{h:.1f}", ha="center", va="bottom", fontsize=9)
    save(fig, "fig3_comparison_bars")


def fig4_ablation(data: dict):
    """图4 消融实验"""
    groups = data["ablation"]["groups"]
    names = [g["name"] for g in groups]
    x = np.arange(len(names))
    fig, axes = plt.subplots(1, 3, figsize=(12, 4))
    colors = ["#43A047", "#FB8C00", "#E53935", "#8E24AA", "#00ACC1"]

    axes[0].bar(names, [g["log_k"] for g in groups], color=colors, alpha=0.88)
    axes[0].set_title("日志量 (K条)")
    axes[0].tick_params(axis="x", rotation=25)

    axes[1].bar(names, [g["cpu"] for g in groups], color=colors, alpha=0.88)
    axes[1].set_title("CPU (%)")
    axes[1].tick_params(axis="x", rotation=25)

    axes[2].bar(names, [g["loss_rate"] for g in groups], color=colors, alpha=0.88)
    axes[2].set_title("丢失率 (%)")
    axes[2].tick_params(axis="x", rotation=25)


    fig.tight_layout()
    save(fig, "fig4_ablation_bars")


def fig5_backoff(data: dict):
    """图5 指数退避延迟分布"""
    curve = data["backoff_curve"]
    attempts = [r["attempt"] for r in curve]
    base = [r["base_ms"] for r in curve]
    mn = [r["min_ms"] for r in curve]
    mx = [r["max_ms"] for r in curve]

    fig, ax = plt.subplots(figsize=(7, 4.5))
    ax.fill_between(attempts, mn, mx, alpha=0.25, color="#1976D2", label="抖动区间")
    ax.plot(attempts, base, "o-", color="#0D47A1", lw=2, markersize=7, label="基础延迟")
    ax.axhline(30000, color="#C62828", ls="--", lw=1.2, label="上限 30s")
    ax.set_xlabel("重试次数")
    ax.set_ylabel("延迟 (ms)")

    ax.set_yscale("log")
    ax.legend()
    ax.grid(True, alpha=0.3)
    save(fig, "fig5_backoff_distribution")


def fig6_attention_bench(data: dict):
    """图6 关注清单生成微基准"""
    bench = data["attention_list_bench"]
    labels = ["输入日志", "高价值日志", "泛化模式", "Top-K 清单"]
    values = [bench["input_logs"], bench["high_value_logs"],
              bench["unique_patterns"], bench["top_k"]]

    fig, ax = plt.subplots(figsize=(7, 4.5))
    bars = ax.bar(labels, values, color=["#5C6BC0", "#FFA726", "#66BB6A", "#26C6DA"], alpha=0.9)
    ax.set_ylabel("数量")

    for bar in bars:
        ax.text(bar.get_x() + bar.get_width() / 2, bar.get_height(),
                str(int(bar.get_height())), ha="center", va="bottom")
    save(fig, "fig6_attention_bench")


def main():
    setup_cn()
    FIGURES_DIR.mkdir(parents=True, exist_ok=True)
    data = load_data()
    fig1_system_overview()
    fig2_triple_transform()
    fig3_comparison(data)
    fig4_ablation(data)
    fig5_backoff(data)
    fig6_attention_bench(data)
    print(f"\n全部图表已输出至 {FIGURES_DIR}")


if __name__ == "__main__":
    main()
