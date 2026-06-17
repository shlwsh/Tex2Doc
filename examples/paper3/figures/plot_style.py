#!/usr/bin/env python3
"""统一绘图样式模块 — Nature / 高影响力期刊风格

所有绘图脚本应导入此模块，调用 apply_style() 初始化。
提供统一色板、字体、辅助函数。
"""

from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib import font_manager
import numpy as np

# ── 路径 ──────────────────────────────────────────────────────
ROOT = Path(__file__).resolve().parents[1]
FIGURES_DIR = ROOT / "figures"

# ── 统一色板（Nature 风格蓝/橙对比 + 语义色） ─────────────────
COLORS = {
    # 主对比色
    "directed":    "#2171B5",   # 深蓝 — 定向采集
    "full":        "#E6550D",   # 深橙 — 全量采集
    # 消融实验 5 色
    "ablation": [
        "#2B8CBE",  # 完整方案 — 蓝
        "#F4A582",  # 无动态清单 — 暖橙
        "#D6604D",  # 无固定缓存 — 红
        "#8073AC",  # 无指数退避 — 紫
        "#4DAC26",  # 无压力感知 — 绿
    ],
    # 关注清单微基准 4 色
    "bench": ["#4393C3", "#F4A582", "#66C2A5", "#5AB4AC"],
    # 规模扩展
    "measured":    "#2CA02C",   # 实测 — 绿
    "projected":   "#BDBDBD",   # 外推 — 灰
    # 基线对照
    "baseline_loki": "#3182BD",
    "baseline_cpu":  "#E6550D",
    # 退避分布
    "backoff_fill":  "#6BAED6",
    "backoff_line":  "#08519C",
    "backoff_limit": "#CB181D",
    # 架构图
    "arch_gateway":  "#C6DBEF",
    "arch_service":  "#C7E9C0",
    "arch_center":   "#FDDBC7",
    "arch_redis":    "#FCBBA1",
    "arch_loki":     "#DADAEB",
    "arch_grafana":  "#D5E8D4",
    "arch_edge":     "#2F4F4F",
    "arch_arrow":    "#37474F",
    "arch_label":    "#37474F",
    # 流程图
    "flow": ["#6BAED6", "#4292C6", "#2171B5", "#084594"],
    "flow_edge": "#08306B",
}

# ── 字号常量（在最终 LaTeX 0.85–0.95\textwidth 缩放后仍 ≥7pt）──
FONT = {
    "title":       15,   # 图标题 / 子图标题
    "label":       13,   # 轴标签
    "tick":        12,   # 刻度值
    "annotation":  11,   # 数值标注 / 箭头标注
    "legend":      11,   # 图例
    "arch_box":    13,   # 架构图方块内文字
    "arch_arrow":  11,   # 架构图箭头标注
    "flow_box":    12,   # 流程图方块文字
}


def apply_style():
    """应用全局样式，替代各脚本内联的 setup_cn()。"""
    # 中文字体
    cjk_fonts = [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        "/usr/share/fonts/truetype/arphic/uming.ttc",
    ]
    cn_font_name = None
    for fp in cjk_fonts:
        if Path(fp).exists():
            font_manager.fontManager.addfont(fp)
            prop = font_manager.FontProperties(fname=fp)
            cn_font_name = prop.get_name()
            break

    sans_list = [cn_font_name] if cn_font_name else []
    sans_list += ["Noto Sans CJK SC", "WenQuanYi Micro Hei", "DejaVu Sans"]

    plt.rcParams.update({
        # 字体
        "font.family":          "sans-serif",
        "font.sans-serif":      sans_list,
        "font.size":            14,
        "axes.unicode_minus":   False,
        # 标题 / 标签
        "axes.titlesize":       FONT["title"],
        "axes.labelsize":       FONT["label"],
        "xtick.labelsize":      FONT["tick"],
        "ytick.labelsize":      FONT["tick"],
        "legend.fontsize":      FONT["legend"],
        # 线条
        "axes.linewidth":       0.8,
        "axes.edgecolor":       "#333333",
        "axes.grid":            False,
        # Spine — 移除顶 / 右
        "axes.spines.top":      False,
        "axes.spines.right":    False,
        # DPI
        "figure.dpi":           150,
        "savefig.dpi":          300,
        "savefig.bbox":         "tight",
        "savefig.pad_inches":   0.08,
        # 布局
        "figure.constrained_layout.use": True,
    })


def save(fig, name: str, directory: Path = None):
    """统一保存 PDF + PNG，确保 300 DPI。"""
    out = directory or FIGURES_DIR
    out.mkdir(parents=True, exist_ok=True)
    for ext in ("pdf", "png"):
        fig.savefig(out / f"{name}.{ext}", bbox_inches="tight", dpi=300)
    plt.close(fig)
    print(f"  [OK] {name}.pdf / .png")


def add_value_labels(ax, bars, fmt="{:.1f}", fontsize=None, offset=0.02):
    """在柱形图上方添加数值标注。"""
    fs = fontsize or FONT["annotation"]
    ymin, ymax = ax.get_ylim()
    span = ymax - ymin
    for bar in bars:
        h = bar.get_height()
        ax.text(
            bar.get_x() + bar.get_width() / 2,
            h + span * offset,
            fmt.format(h),
            ha="center", va="bottom",
            fontsize=fs, fontweight="medium",
        )


def add_reduction_label(ax, x, y_top, pct_text, fontsize=None):
    """在柱形图上方添加降幅标注（如 '↓98.4%'）。"""
    fs = fontsize or FONT["annotation"]
    ax.text(
        x, y_top * 1.15,
        pct_text,
        ha="center", va="bottom",
        fontsize=fs, fontweight="bold",
        color="#C62828",
    )


def style_axis(ax, xlabel=None, ylabel=None, title=None):
    """统一设置轴标签和标题样式。"""
    if xlabel:
        ax.set_xlabel(xlabel, fontsize=FONT["label"], fontweight="medium")
    if ylabel:
        ax.set_ylabel(ylabel, fontsize=FONT["label"], fontweight="medium")
    if title:
        ax.set_title(title, fontsize=FONT["title"], fontweight="bold", pad=10)
