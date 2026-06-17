#!/usr/bin/env python3
"""科研图表生成 — Nature 风格（论文用 PDF + PNG）

改进要点：
- 统一使用 plot_style 样式基础设施
- 全局字号提升（14pt 基础，标题 15pt，标注 11pt）
- fig3 拆分为 2×2 子图 + 误差棒 + 降幅标注
- fig4 添加基准线和数值标注
- fig6 使用对数 Y 轴解决量级悬殊
- 全部图表移除顶/右 spine，统一色板
"""

import json
import sys
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
from matplotlib.patches import FancyBboxPatch, FancyArrowPatch
import numpy as np

# 导入统一样式
sys.path.insert(0, str(Path(__file__).resolve().parent))
from plot_style import (
    apply_style, save, COLORS, FONT, style_axis,
    add_value_labels, add_reduction_label,
    ROOT, FIGURES_DIR,
)

RESULTS_FILE = ROOT / "experiments" / "results" / "phase1" / "phase1_latest.json"


def load_data() -> dict:
    if not RESULTS_FILE.exists():
        raise FileNotFoundError(f"请先运行 phase1_collect.py，缺少 {RESULTS_FILE}")
    return json.loads(RESULTS_FILE.read_text(encoding="utf-8"))


# ═══════════════════════════════════════════════════════════════
# 图1  系统总体架构（详细版）
# ═══════════════════════════════════════════════════════════════

# ── 架构图专用色板 ──────────────────────────────────────────
_C = {
    # 三层背景
    "band_collect":  "#EBF5FB",   # 采集层 — 浅蓝
    "band_process":  "#FEF9E7",   # 处理层 — 浅黄
    "band_store":    "#F0F4F8",   # 存储层 — 浅灰蓝
    # 外框容器
    "gw_outer":      "#D4E6F1",
    "svc_outer":     "#D5F5E3",
    # 内部小模块
    "lua":           "#85C1E9",
    "shm":           "#AED6F1",
    "gw_agent":      "#5DADE2",
    "app":           "#82E0AA",
    "collector":     "#58D68D",
    "matcher":       "#2ECC71",
    "cache":         "#27AE60",
    "uploader":      "#1E8449",
    "boltdb":        "#F5B041",
    # Center
    "center_outer":  "#FAD7A0",
    "strategy":      "#F0B27A",
    "receiver":      "#EB984E",
    "storage":       "#E67E22",
    "dispatch":      "#CA6F1E",
    # 存储组件
    "redis":         "#F1948A",
    "loki":          "#BB8FCE",
    "grafana":       "#76D7C4",
    # 线条
    "edge":          "#2C3E50",
    "arrow":         "#2C3E50",
    "flow_num":      "#C0392B",
    "gw_flow":       "#2980B9",
    "data_flow":     "#8E44AD",
    "rule_flow":     "#27AE60",
}
_FS = {
    "layer":    16,    # 层级标签 — 大幅增大
    "title":    14,    # 容器标题 — 增大
    "mod":      11,    # 内部模块 — 增大
    "arrow":    11,    # 箭头标注 — 增大
    "step":     13,    # 步骤编号 — 增大
    "detail":   9.5,   # 细节文字 — 增大
}


def _box(ax, x, y, w, h, text, fc, fs=None, ec=None, lw=1.0,
         tc="black", fw="medium", zorder=5, alpha=1.0, style="round,pad=0.05"):
    """绘制一个圆角方块 + 居中文字。"""
    box = FancyBboxPatch((x, y), w, h, boxstyle=style,
                         facecolor=fc, edgecolor=ec or _C["edge"],
                         linewidth=lw, alpha=alpha, zorder=zorder)
    ax.add_patch(box)
    ax.text(x + w / 2, y + h / 2, text,
            ha="center", va="center",
            fontsize=fs or _FS["mod"], fontweight=fw,
            color=tc, zorder=zorder + 1)
    return box


def _arrow_hv(ax, src, dst, color=None, lw=1.8, zorder=8, via=None):
    """绘制正交折线箭头（水平→垂直 或 垂直→水平）。
    via: 中间拐点坐标，如 (mid_x, mid_y)。若 None 则直连。
    """
    c = color or _C["arrow"]
    if via is None:
        ax.annotate("", xy=dst, xytext=src,
                    arrowprops=dict(arrowstyle="-|>", color=c,
                                   lw=lw, mutation_scale=16),
                    zorder=zorder)
    else:
        # 先画折线段（无箭头），再画最后一段带箭头
        ax.plot([src[0], via[0]], [src[1], via[1]], color=c,
                lw=lw, solid_capstyle="round", zorder=zorder)
        ax.annotate("", xy=dst, xytext=via,
                    arrowprops=dict(arrowstyle="-|>", color=c,
                                   lw=lw, mutation_scale=16),
                    zorder=zorder)


def _step_num(ax, x, y, num, zorder=12):
    """绘制红色步骤编号圆圈。"""
    circle = plt.Circle((x, y), 0.3, fc=_C["flow_num"], ec="white",
                         lw=2.0, zorder=zorder)
    ax.add_patch(circle)
    ax.text(x, y, str(num), ha="center", va="center",
            fontsize=_FS["step"], fontweight="bold", color="white",
            zorder=zorder + 1)


def fig1_system_overview():
    """图1 系统总体架构 — 三层协同 + 5 步数据流（弧线箭头 + 无遮挡标签）"""

    fig, ax = plt.subplots(figsize=(18, 12))
    ax.set_xlim(0, 18)
    ax.set_ylim(-0.5, 12)
    ax.axis("off")

    # ── 弧线箭头辅助函数 ──────────────────────────────────
    from matplotlib.patches import FancyArrowPatch
    def _arc(src, dst, color=None, lw=1.8, rad=0.15, zorder=8):
        """绘制带弧度的箭头。rad>0 逆时针弯曲，rad<0 顺时针。"""
        c = color or _C["arrow"]
        arrow = FancyArrowPatch(
            src, dst,
            connectionstyle=f"arc3,rad={rad}",
            arrowstyle="-|>",
            color=c, lw=lw, mutation_scale=18,
            zorder=zorder)
        ax.add_patch(arrow)

    def _straight(src, dst, color=None, lw=1.8, zorder=8):
        """绘制直线箭头。"""
        c = color or _C["arrow"]
        ax.annotate("", xy=dst, xytext=src,
                    arrowprops=dict(arrowstyle="-|>", color=c,
                                   lw=lw, mutation_scale=16),
                    zorder=zorder)

    # ── ① 三层背景带 ──────────────────────────────────────
    bands = [
        (7.2, 4.6, _C["band_collect"]),
        (3.4, 3.8, _C["band_process"]),
        (0.0, 3.4, _C["band_store"]),
    ]
    layer_labels = [
        (7.2, 4.6, "采 集 层", "#2471A3"),
        (3.4, 3.8, "处理与策略层", "#A04000"),
        (0.0, 3.4, "存储与展示层", "#555555"),
    ]
    for y0, h, color in bands:
        ax.add_patch(plt.Rectangle((0, y0), 18, h,
                     facecolor=color, edgecolor="#CCCCCC",
                     linewidth=0.5, zorder=0))

    # 层名称 — 水平显示在右侧上方
    for y0, h, label, color in layer_labels:
        ax.text(17.5, y0 + h - 0.2, label,
                fontsize=17, color=color,
                fontweight="bold", va="top", ha="right",
                zorder=1,
                bbox=dict(boxstyle="round,pad=0.25", facecolor="white",
                          edgecolor=color, alpha=0.9, linewidth=1.8))

    # ── ② 网关节点（采集层左侧）────────────────────────────
    _box(ax, 1.2, 7.5, 5.6, 4.0, "", _C["gw_outer"],
         ec="#2980B9", lw=2.0, zorder=2, alpha=0.4,
         style="round,pad=0.12")
    ax.text(4.0, 11.25, "网关节点（OpenResty）",
            ha="center", fontsize=_FS["title"], fontweight="bold",
            color="#2471A3", zorder=3)

    # 内部模块 — 网关
    _box(ax, 1.5, 10.0, 2.4, 1.0, "Nginx\nlog_by_lua", _C["lua"],
         fs=_FS["mod"], tc="white", fw="bold")
    _box(ax, 1.5, 8.6, 2.4, 1.0, "共享内存\n缓冲区", _C["shm"],
         fs=_FS["mod"])
    _box(ax, 4.2, 8.6, 2.4, 1.0, "Gateway\nAgent", _C["gw_agent"],
         fs=_FS["mod"], tc="white", fw="bold")
    _box(ax, 4.2, 7.7, 2.4, 0.6, "退避重试 + 资源监控", "#AED6F1",
         fs=_FS["detail"], ec="#2980B9", lw=0.6)

    # 网关内部箭头
    # Nginx → 共享内存 (垂直向下)
    _straight((2.7, 10.0), (2.7, 9.6), _C["gw_flow"])
    # 标签放在箭头右侧，不与任何框重叠
    ax.text(2.95, 9.8, "URL/状态码/时延", fontsize=_FS["detail"],
            color="#2471A3", va="center", ha="left")
    # 共享内存 → Gateway Agent (水平向右)
    _straight((3.9, 9.1), (4.2, 9.1), _C["gw_flow"])
    # "拉取"标签放在箭头上方的安全空白区
    ax.text(4.05, 9.55, "拉取", fontsize=_FS["detail"],
            color="#2471A3", ha="center", va="bottom",
            bbox=dict(boxstyle="round,pad=0.08", facecolor="white",
                      edgecolor="#2471A3", alpha=0.85, linewidth=0.5),
            zorder=15)

    # ── ③ 微服务节点 ×N（采集层右侧）─────────────────────
    svc_positions = [(7.5, "服务-1"), (12.8, "服务-N")]
    for ox, label_extra in svc_positions:
        _box(ax, ox, 7.5, 4.6, 4.0, "", _C["svc_outer"],
             ec="#27AE60", lw=2.0, zorder=2, alpha=0.4,
             style="round,pad=0.12")
        ax.text(ox + 2.3, 11.25, f"微服务节点（{label_extra}）",
                ha="center", fontsize=_FS["title"], fontweight="bold",
                color="#1E8449", zorder=3)

        # 内部模块
        _box(ax, ox + 0.2, 10.1, 2.0, 0.8, "App\n应用日志", _C["app"],
             fs=_FS["mod"])
        _box(ax, ox + 2.4, 10.1, 2.0, 0.8, "Collector\n+Matcher", _C["collector"],
             fs=_FS["mod"], tc="white")
        _box(ax, ox + 0.2, 8.9, 2.0, 0.8, "固定缓存块\n(环形队列)", _C["cache"],
             fs=_FS["mod"], tc="white")
        _box(ax, ox + 2.4, 8.9, 2.0, 0.8, "gRPC\nUploader", _C["uploader"],
             fs=_FS["mod"], tc="white")
        _box(ax, ox + 1.3, 7.9, 2.0, 0.6, "BoltDB 兜底", _C["boltdb"],
             fs=_FS["detail"], ec="#D4AC0D", lw=0.8)

        # 内部箭头
        _straight((ox + 2.2, 10.5), (ox + 2.4, 10.5), "#1E8449", lw=1.2)
        _straight((ox + 1.2, 10.1), (ox + 1.2, 9.7), "#1E8449", lw=1.2)
        _straight((ox + 2.2, 9.3), (ox + 2.4, 9.3), "#1E8449", lw=1.2)
        # BoltDB 虚线
        ax.annotate("", xy=(ox + 2.3, 8.2), xytext=(ox + 3.4, 8.9),
                    arrowprops=dict(arrowstyle="-|>", color="#D4AC0D",
                                   lw=1.0, ls="--", mutation_scale=12),
                    zorder=8)

    # ── 省略号 ──────────────────────────────────────────────
    ax.text(12.3, 9.8, "· · ·", fontsize=22, ha="center", va="center",
            color="#888", fontweight="bold", zorder=10)

    # ── ④ 日志中心 Center（处理层）────────────────────────
    _box(ax, 2.0, 3.7, 14.0, 3.3, "", _C["center_outer"],
         ec="#D35400", lw=2.0, zorder=2, alpha=0.35,
         style="round,pad=0.12")
    ax.text(9.0, 6.75, "日志中心 Center（Go + gRPC）",
            ha="center", fontsize=_FS["title"] + 1, fontweight="bold",
            color="#A04000", zorder=3)

    # Center 内部 4 模块
    _box(ax, 2.5, 4.1, 3.0, 1.9, "关注清单生成\n(Strategy)", _C["strategy"],
         fs=_FS["mod"] + 1, fw="bold")
    _box(ax, 6.0, 4.1, 3.0, 1.9, "规则下发\n(Dispatch)\ngRPC → Agent", _C["dispatch"],
         fs=_FS["mod"], tc="white", fw="bold")
    _box(ax, 9.5, 4.1, 3.0, 1.9, "二次过滤\n(Receiver)\n版本+去重", _C["receiver"],
         fs=_FS["mod"], tc="white", fw="bold")
    _box(ax, 13.0, 4.1, 2.7, 1.9, "Loki 推送\n(Storage)\n批量写入", _C["storage"],
         fs=_FS["mod"], tc="white", fw="bold")

    # Center 内部水平箭头（短距直连）
    _straight((5.5, 5.05), (6.0, 5.05), "#A04000", lw=1.8)
    # "Top-K" 放在 Strategy 与 Dispatch 之间上方空白处
    ax.text(5.75, 6.15, "Top-K", fontsize=_FS["detail"] + 2, color="#A04000",
            ha="center", fontstyle="italic", fontweight="bold",
            bbox=dict(boxstyle="round,pad=0.2", facecolor="white",
                      edgecolor="#A04000", alpha=0.95, linewidth=1.2),
            zorder=15)
    _straight((9.0, 5.05), (9.5, 5.05), "#A04000", lw=1.8)
    _straight((12.5, 5.05), (13.0, 5.05), "#A04000", lw=1.8)

    # ── ⑤ 存储层（底部 3 组件）──────────────────────────────
    _box(ax, 0.8, 0.5, 4.0, 2.2, "Redis\n流量日志缓存\n(版本化键)", _C["redis"],
         fs=_FS["mod"] + 1, fw="bold")
    _box(ax, 6.0, 0.5, 4.5, 2.2, "Grafana Loki\n日志存储 + 标签索引", _C["loki"],
         fs=_FS["mod"] + 1, fw="bold")
    _box(ax, 12.5, 0.5, 4.5, 2.2, "Grafana\n可视化仪表盘", _C["grafana"],
         fs=_FS["mod"] + 1, fw="bold")

    # ════════════════════════════════════════════════════════
    # ⑥ 跨层数据流（5 步 — 与论文 §3.2 一一对应）
    #    每个编号放在对应主箭头的弧线中点，确保编号"骑"在箭头上
    # ════════════════════════════════════════════════════════

    # ── 步骤① 网关采集: Nginx → 共享内存 ──
    #    主箭头：Nginx 底部(2.7,10.0) → 共享内存顶部(2.7,9.6)（第210行已绘制）
    #    编号放在箭头左侧，与箭头同高
    _step_num(ax, 1.5, 9.8, 1, zorder=16)

    # ── 步骤② Agent 拉取 + gRPC 上传到 Center → 缓存 Redis ──
    #    主箭头：Agent(5.4,8.6) → Center(7.0,7.0) 蓝色弧线
    #    编号放在弧线中点位置
    # 箭头 A: Gateway Agent 底部 → Center 上沿 (gRPC 上传)
    _arc((5.4, 8.6), (7.0, 7.0), _C["gw_flow"], lw=2.2, rad=0.15)
    _step_num(ax, 5.8, 7.7, 2, zorder=16)
    ax.text(4.6, 7.4, "gRPC 上传", fontsize=_FS["arrow"],
            color=_C["gw_flow"], ha="center", fontweight="bold",
            bbox=dict(boxstyle="round,pad=0.15", facecolor="white",
                      edgecolor=_C["gw_flow"], alpha=0.9, linewidth=0.8),
            zorder=15)
    # 箭头 B: Center(Strategy区) → Redis 缓存
    _arc((3.5, 4.1), (2.8, 2.7), _C["gw_flow"], lw=1.8, rad=0.12)
    ax.text(2.0, 3.45, "缓存流量", fontsize=_FS["arrow"],
            color=_C["gw_flow"], fontweight="bold",
            bbox=dict(boxstyle="round,pad=0.1", facecolor="white",
                      edgecolor=_C["gw_flow"], alpha=0.85, linewidth=0.6),
            zorder=15)

    # ── 步骤③ Center 生成 Top-K 清单 + Dispatch 下发 Agent ──
    #    主箭头：Dispatch(7.5,6.0) → 服务-1(9.8,7.5) 绿色弧线
    #    编号放在绿色弧线中点
    # 箭头 A: Redis → Strategy（读取流量数据）
    _arc((4.0, 2.7), (4.5, 4.1), _C["data_flow"], lw=1.8, rad=-0.12)
    ax.text(5.3, 3.55, "读取", fontsize=_FS["arrow"],
            color=_C["data_flow"], fontweight="bold",
            bbox=dict(boxstyle="round,pad=0.1", facecolor="white",
                      edgecolor=_C["data_flow"], alpha=0.85, linewidth=0.6),
            zorder=15)
    # 箭头 B: Dispatch → 服务-1 底部
    _arc((7.5, 6.0), (9.8, 7.5), _C["rule_flow"], lw=2.2, rad=-0.15)
    # 箭头 C: Dispatch → 服务-N 底部
    _arc((7.5, 6.0), (15.1, 7.5), _C["rule_flow"], lw=2.2, rad=-0.08)
    _step_num(ax, 9.0, 7.6, 3, zorder=16)
    ax.text(10.4, 7.6, "下发清单", fontsize=_FS["arrow"],
            color=_C["rule_flow"], fontweight="bold",
            bbox=dict(boxstyle="round,pad=0.14", facecolor="white",
                      edgecolor=_C["rule_flow"], alpha=0.95, linewidth=1.0),
            zorder=15)

    # ── 步骤④ 微服务 Agent 匹配 + Uploader → Center Receiver ──
    #    主箭头：Uploader(10.9,8.9) → Receiver(10.5,6.0) 紫色弧线
    #    编号放在弧线中点
    # 箭头 A: 服务-1 Uploader → Center Receiver
    _arc((10.9, 8.9), (10.5, 6.0), _C["data_flow"], lw=2.0, rad=0.15)
    # 箭头 B: 服务-N Uploader → Center Receiver
    _arc((16.2, 8.9), (11.0, 6.0), _C["data_flow"], lw=2.0, rad=0.2)
    _step_num(ax, 13.5, 7.6, 4, zorder=16)
    ax.text(12.5, 8.3, "上传命中日志", fontsize=_FS["arrow"],
            color=_C["data_flow"], fontweight="bold", ha="center",
            bbox=dict(boxstyle="round,pad=0.12", facecolor="white",
                      edgecolor=_C["data_flow"], alpha=0.9, linewidth=0.8),
            zorder=15)

    # ── 步骤⑤ Center 二次过滤 → Storage 批量推送 Loki ──
    #    主箭头：Storage(14.3,4.1) → Loki(8.25,2.7) 紫色弧线
    #    编号放在弧线中点
    # 箭头: Storage → Loki
    _arc((14.3, 4.1), (8.25, 2.7), _C["data_flow"], lw=2.2, rad=-0.15)
    _step_num(ax, 12.0, 3.4, 5, zorder=16)
    ax.text(10.5, 2.9, "批量推送", fontsize=_FS["arrow"],
            color=_C["data_flow"], fontweight="bold",
            bbox=dict(boxstyle="round,pad=0.1", facecolor="white",
                      edgecolor=_C["data_flow"], alpha=0.85, linewidth=0.6),
            zorder=15)

    # Loki → Grafana (查询展示，非步骤编号)
    _arc((10.5, 1.6), (12.5, 1.6), "#555", lw=1.5, rad=-0.08)
    ax.text(11.5, 2.05, "查询", fontsize=_FS["arrow"],
            ha="center", color="#555", fontweight="medium")

    # ── ⑦ 图例 ──────────────────────────────────────────
    from matplotlib.patches import Patch
    from matplotlib.lines import Line2D
    legend_items = [
        Patch(facecolor=_C["band_collect"], edgecolor="#CCC",
              label="采集层"),
        Patch(facecolor=_C["band_process"], edgecolor="#CCC",
              label="处理层"),
        Patch(facecolor=_C["band_store"], edgecolor="#CCC",
              label="存储层"),
        Line2D([0], [0], color=_C["gw_flow"], lw=2.5, label="流量日志流"),
        Line2D([0], [0], color=_C["rule_flow"], lw=2.5, label="规则/清单下发"),
        Line2D([0], [0], color=_C["data_flow"], lw=2.5, label="命中日志上传"),
    ]
    ax.legend(handles=legend_items, loc="lower left",
              fontsize=_FS["mod"] + 1, frameon=True, framealpha=0.9,
              edgecolor="#CCC", ncol=3, bbox_to_anchor=(0.02, -0.04))

    save(fig, "fig1_system_overview")


def fig2_triple_transform():
    """图2 定向策略三次转换 — 展示每阶段输入/输出、执行位置和关键机制"""

    fig, ax = plt.subplots(figsize=(17, 9))
    ax.set_xlim(0, 17)
    ax.set_ylim(0, 9)
    ax.axis("off")

    # ── 配色 ──────────────────────────────────────────────
    C = {
        "input":   "#D6EAF8",  # 输入框 浅蓝
        "t1":      "#85C1E9",  # 第一次转换
        "t2":      "#5DADE2",  # 第二次转换
        "t3":      "#2E86C1",  # 第三次转换
        "output":  "#D5F5E3",  # 输出框 浅绿
        "loc":     "#FAD7A0",  # 位置标签 浅橙
        "inv":     "#FADBD8",  # 一致性前提 浅红
        "edge":    "#1B4F72",
        "arrow":   "#1B4F72",
        "detail":  "#7F8C8D",
        "mech":    "#C0392B",  # 关键机制红色
    }

    # ── 辅助函数 ────────────────────────────────────────────
    def rbox(x, y, w, h, text, fc, tc="black", fs=11, fw="medium",
             ec=None, lw=1.0, alpha=1.0, zorder=5):
        box = FancyBboxPatch((x, y), w, h, boxstyle="round,pad=0.08",
                             facecolor=fc, edgecolor=ec or C["edge"],
                             linewidth=lw, alpha=alpha, zorder=zorder)
        ax.add_patch(box)
        ax.text(x + w / 2, y + h / 2, text,
                ha="center", va="center", fontsize=fs, fontweight=fw,
                color=tc, zorder=zorder + 1, linespacing=1.35)
        return box

    def arr(src, dst, color=None, lw=2.2):
        ax.annotate("", xy=dst, xytext=src,
                    arrowprops=dict(arrowstyle="-|>",
                                   color=color or C["arrow"],
                                   lw=lw, mutation_scale=18),
                    zorder=8)

    def step_circle(x, y, text):
        circle = plt.Circle((x, y), 0.32, fc=C["mech"], ec="white",
                             lw=2.0, zorder=12)
        ax.add_patch(circle)
        ax.text(x, y, text, ha="center", va="center",
                fontsize=13, fontweight="bold", color="white", zorder=13)

    # ── 定义各列 x 坐标（增大间距）─────────────────────────
    col_x = [0.3, 3.6, 6.9, 10.2, 13.5]
    bw = 2.8   # 方块宽 — 增大
    bh_main = 2.0  # 主框高 — 增大
    bh_io = 0.85   # 输入/输出小框高 — 增大

    # ── 第一行：执行位置标签 (y=7.9) ─────────────────────
    loc_labels = ["策略定义", "网关层", "Center", "Center", "Loki"]
    for i, lbl in enumerate(loc_labels):
        rbox(col_x[i], 7.9, bw, 0.65, lbl, C["loc"],
             tc="#A04000", fs=12, fw="bold", ec="#E59866", lw=1.0)

    # ── 第二行：输入框 (y=6.65) ────────────────────────────
    inputs = [
        "定向策略 S\n(阈值T, 错误码E, K, TTL)",
        "HTTP access log\n状态码, 响应时延",
        "网关流量日志 + S\n(N 条候选)",
        "关注清单 + 服务日志\n(版本 v, TTL)",
        None,
    ]
    for i, txt in enumerate(inputs):
        if txt:
            rbox(col_x[i], 6.55, bw, bh_io + 0.1, txt, C["input"],
                 tc="#1A5276", fs=10.5, ec="#85C1E9", lw=0.8)

    # ── 第三行：核心转换框 (y=3.8) ─────────────────────────
    transforms = [
        ("定向策略 S\n(阈值T, 错误码E\nTop-K, TTL)", C["input"], "black"),
        ("第一次转换\n\n网关预筛选\n阈值 T/5 宽松过滤",       C["t1"], "white"),
        ("第二次转换\n\nTop-K 关注清单\nURL 泛化 + 权重排序",  C["t2"], "white"),
        ("第三次转换\n\n二次过滤入库\n版本校验 + TTL + 去重",   C["t3"], "white"),
        ("Loki 入库日志\n\n高价值上下文\n可查询标签",          C["output"], "#1A5276"),
    ]
    for i, (txt, fc, tc) in enumerate(transforms):
        rbox(col_x[i], 3.8, bw, bh_main + 0.3, txt, fc,
             tc=tc, fs=12, fw="bold", lw=1.5)

    # ── 第四行：输出框 (y=2.2) ────────────────────────────
    outputs = [
        None,
        "网关采集规则\n(宽松阈值候选集)",
        "Agent 过滤规则\n(Top-K URL 模式清单)",
        "Loki 存储规则\n(过滤过期/重复日志)",
        "72 条 vs 4388 条\n(降幅 98.4%)",
    ]
    for i, txt in enumerate(outputs):
        if txt:
            rbox(col_x[i], 2.0, bw, bh_io + 0.1, txt,
                 C["output"] if i < 4 else "#E8F8F5",
                 tc="#1A5276" if i < 4 else "#C0392B",
                 fs=10.5, ec="#82E0AA" if i < 4 else "#C0392B",
                 lw=0.8, fw="bold" if i == 4 else "medium")

    # ── 数据量递减标注 (y=1.55) ────────────────────────────
    shrink = [
        (col_x[0] + bw/2, "N 条日志"),
        (col_x[1] + bw/2, "N' 条 (T/5)"),
        (col_x[2] + bw/2, "M → K 模式"),
        (col_x[3] + bw/2, "去重后入库"),
        (col_x[4] + bw/2, "≤72 条"),
    ]
    for x, txt in shrink:
        ax.text(x, 1.65, txt, ha="center", va="center",
                fontsize=9.5, color=C["detail"], fontstyle="italic",
                fontweight="medium", zorder=5)

    # ── 箭头连接（水平主流 + 垂直输入/输出）──────────────
    y_mid = 3.8 + (bh_main + 0.3) / 2  # 主框中线
    for i in range(4):
        arr((col_x[i] + bw, y_mid), (col_x[i+1], y_mid))

    # 输入 → 主框 垂直箭头
    for i in range(4):
        arr((col_x[i] + bw/2, 6.55),
            (col_x[i] + bw/2, 3.8 + bh_main + 0.3),
            color="#5DADE2", lw=1.5)

    # 主框 → 输出 垂直箭头
    for i in range(1, 5):
        arr((col_x[i] + bw/2, 3.8),
            (col_x[i] + bw/2, 2.0 + bh_io + 0.1),
            color="#27AE60", lw=1.5)

    # ── 步骤编号 ──────────────────────────────────────────
    for i in range(1, 4):
        mid_x = (col_x[i-1] + bw + col_x[i]) / 2
        step_circle(mid_x, y_mid + 0.75, str(i))

    # ── 箭头标注 ──────────────────────────────────────────
    arrow_labels = ["阈值过滤\n(T/5)", "Top-K 选取\n(清单生成)", "版本校验\n(TTL+去重)"]
    for i in range(3):
        mid_x = (col_x[i] + bw + col_x[i+1]) / 2
        ax.text(mid_x, y_mid - 0.4, arrow_labels[i],
                ha="center", va="top", fontsize=10,
                color=C["mech"], fontstyle="italic", fontweight="bold",
                zorder=10)

    # ── 一致性前提横幅 (底部) ─────────────────────────────
    rbox(2.5, 0.2, 12.0, 0.7,
         "一致性前提:  同一关注清单版本号 v  ×  同一 URL 泛化函数 Generalize(·)"
         "  →  跨层策略语义不漂移",
         C["inv"], tc="#922B21", fs=11, fw="bold",
         ec="#E74C3C", lw=1.5, alpha=0.85)

    # ── 主流标题 ──────────────────────────────────────────
    ax.text(8.5, 8.8, "定向策略三次转换流程",
            ha="center", va="bottom", fontsize=16,
            fontweight="bold", color=C["edge"])

    save(fig, "fig2_triple_transform")


# ═══════════════════════════════════════════════════════════════
# 图3  定向采集 vs 全量采集（雷达图 + 降幅条形图）
# ═══════════════════════════════════════════════════════════════
def fig3_comparison(data: dict):
    """图3 资源对比 — 雷达图多维对比 + 水平降幅条"""
    comp = data["comparison"]
    d, f = comp["directed"], comp["full_collect"]

    # 4 个指标
    labels = ["Loki 入库量", "Agent CPU", "Agent 内存", "网络带宽"]
    d_vals = [d["log_volume_k"] * 1000, d["cpu_percent"], d.get("memory_mb", 0), d["bandwidth_mbps"]]
    f_vals = [f["log_volume_k"] * 1000, f["cpu_percent"], f.get("memory_mb", 0), f["bandwidth_mbps"]]
    reductions = [98.4, 37.5, 2.0, 98.4]

    # 归一化到 0-1（以全量采集为基准 = 1.0）
    d_norm = [dv / fv if fv > 0 else 0 for dv, fv in zip(d_vals, f_vals)]
    f_norm = [1.0] * 4

    # ── 雷达图 ───────────────────────────────────────────
    fig = plt.figure(figsize=(13, 5.5))

    # 左 — 雷达图
    ax_radar = fig.add_subplot(121, polar=True)
    N = len(labels)
    angles = np.linspace(0, 2 * np.pi, N, endpoint=False).tolist()
    angles += angles[:1]

    d_norm_c = d_norm + d_norm[:1]
    f_norm_c = f_norm + f_norm[:1]

    ax_radar.fill(angles, f_norm_c, alpha=0.15, color=COLORS["full"])
    ax_radar.plot(angles, f_norm_c, "o-", color=COLORS["full"],
                  lw=2.0, markersize=7, label="全量采集", zorder=5)

    ax_radar.fill(angles, d_norm_c, alpha=0.25, color=COLORS["directed"])
    ax_radar.plot(angles, d_norm_c, "s-", color=COLORS["directed"],
                  lw=2.5, markersize=7, label="定向采集", zorder=6)

    ax_radar.set_xticks(angles[:-1])
    ax_radar.set_xticklabels(labels, fontsize=FONT["label"])
    ax_radar.set_ylim(0, 1.15)
    ax_radar.set_yticks([0.25, 0.5, 0.75, 1.0])
    ax_radar.set_yticklabels(["25%", "50%", "75%", "100%"],
                              fontsize=FONT["tick"] - 2, color="#888")
    ax_radar.legend(loc="upper right", bbox_to_anchor=(1.25, 1.15),
                    fontsize=FONT["legend"], frameon=True,
                    framealpha=0.9, edgecolor="#CCC")
    ax_radar.set_title("多维资源归一化对比", fontsize=FONT["title"],
                        fontweight="bold", pad=20)

    # 在雷达顶点上标注实际值
    for i, (dv, fv) in enumerate(zip(d_vals, f_vals)):
        angle = angles[i]
        fmt = "{:.0f}" if dv >= 1 else "{:.2f}"
        r_d = d_norm[i]
        ax_radar.text(angle, r_d + 0.12, fmt.format(dv),
                      ha="center", va="center",
                      fontsize=8, color=COLORS["directed"], fontweight="bold")

    # 右 — 降幅水平条形图
    ax_bar = fig.add_subplot(122)
    y_pos = np.arange(len(labels))
    units = ["72 vs 4388 条", "0.05% vs 0.08%", "95.6 vs 97.5 MB", "0.2 vs 12.2 KB/s"]

    bar_colors = ["#C0392B" if r > 50 else "#E67E22" if r > 10 else "#27AE60"
                  for r in reductions]
    bars = ax_bar.barh(y_pos, reductions, height=0.55,
                       color=bar_colors, alpha=0.85,
                       edgecolor="white", linewidth=0.8)

    ax_bar.set_yticks(y_pos)
    ax_bar.set_yticklabels(labels, fontsize=FONT["tick"])
    ax_bar.set_xlabel("降幅 (%)", fontsize=FONT["label"], fontweight="medium")
    ax_bar.set_title("资源消耗降幅", fontsize=FONT["title"], fontweight="bold", pad=10)
    ax_bar.set_xlim(0, 115)
    ax_bar.invert_yaxis()

    # 数值 + 实际值标注
    for i, (bar, r, u) in enumerate(zip(bars, reductions, units)):
        ax_bar.text(bar.get_width() + 1.5, bar.get_y() + bar.get_height() / 2,
                    f"↓{r:.1f}%",
                    ha="left", va="center",
                    fontsize=FONT["annotation"], fontweight="bold",
                    color=bar_colors[i])
        ax_bar.text(bar.get_width() / 2, bar.get_y() + bar.get_height() / 2,
                    u, ha="center", va="center",
                    fontsize=8, color="white", fontweight="medium")

    ax_bar.grid(True, axis="x", alpha=0.15)

    save(fig, "fig3_comparison_bars")


# ═══════════════════════════════════════════════════════════════
# 图4  消融实验
def fig4_ablation(data: dict):
    """图4 消融实验 — 热力图 + 棒棒糖图双面板"""
    groups = data["ablation"]["groups"]
    short = ["完整方案", "无动态清单", "无固定缓存", "无指数退避", "无压力感知"]
    metrics = ["日志量 (K条)", "CPU (%)", "丢失率 (%)"]
    keys = ["log_k", "cpu", "loss_rate"]

    # 构建矩阵
    raw = np.array([[g[k] for k in keys] for g in groups])
    baseline = raw[0]
    # 相对变化倍数（>1 表示恶化）
    ratio = raw / baseline

    fig, (ax_heat, ax_lollipop) = plt.subplots(
        2, 1, figsize=(11, 8), height_ratios=[1, 1.2],
        gridspec_kw={"hspace": 0.4})

    # ── 上：热力图 ─────────────────────────────────────────
    # 使用 imshow 绘制热力图
    im = ax_heat.imshow(ratio.T, aspect="auto", cmap="YlOrRd",
                         vmin=0.8, vmax=4.0)

    ax_heat.set_xticks(range(len(short)))
    ax_heat.set_xticklabels(short, fontsize=FONT["tick"], rotation=0)
    ax_heat.set_yticks(range(len(metrics)))
    ax_heat.set_yticklabels(metrics, fontsize=FONT["tick"])
    ax_heat.set_title("消融实验：各组件移除后指标恶化倍数",
                       fontsize=FONT["title"], fontweight="bold", pad=12)

    # 在每个单元格内标注 原始值 + 倍数
    fmts = ["{:.0f}", "{:.1f}", "{:.1f}"]
    for i in range(len(short)):
        for j in range(len(metrics)):
            val = raw[i, j]
            r = ratio[i, j]
            color = "white" if r > 2.0 else "black"
            text = fmts[j].format(val)
            if i > 0:
                text += f"\n({r:.1f}×)"
            else:
                text += "\n(基准)"
            ax_heat.text(i, j, text, ha="center", va="center",
                         fontsize=9, fontweight="bold", color=color)

    # 颜色条
    cbar = fig.colorbar(im, ax=ax_heat, shrink=0.8, pad=0.02)
    cbar.set_label("恶化倍数", fontsize=FONT["label"] - 1)
    cbar.ax.tick_params(labelsize=FONT["tick"] - 1)

    # 移除 spines for heat
    for sp in ax_heat.spines.values():
        sp.set_visible(False)

    # ── 下：棒棒糖图 ──────────────────────────────────────
    colors_lollipop = COLORS["ablation"]
    x_pos = np.arange(len(metrics))
    width = 0.15
    offsets = np.arange(len(short)) - (len(short) - 1) / 2

    for i, (name, color) in enumerate(zip(short, colors_lollipop)):
        pos = x_pos + offsets[i] * width
        vals = raw[i]
        # 归一化到百分比（以各指标最大值为 100%）
        norm_vals = vals / raw.max(axis=0) * 100

        # 棒棒糖茎
        for p, v in zip(pos, norm_vals):
            ax_lollipop.plot([p, p], [0, v], color=color,
                             lw=2.5, alpha=0.7, zorder=3)
        # 棒棒糖头
        ax_lollipop.scatter(pos, norm_vals, color=color, s=80,
                            zorder=5, edgecolors="white", linewidth=1.0,
                            label=name if x_pos[0] == 0 else None)

        # 数值标注
        for p, v, rv in zip(pos, norm_vals, vals):
            fmt = "{:.0f}" if rv >= 1 else "{:.1f}"
            ax_lollipop.text(p, v + 2.5, fmt.format(rv),
                             ha="center", va="bottom",
                             fontsize=7, fontweight="medium", color=color)

    ax_lollipop.set_xticks(x_pos)
    ax_lollipop.set_xticklabels(metrics, fontsize=FONT["tick"])
    ax_lollipop.set_ylabel("指标值 (归一化%)", fontsize=FONT["label"],
                            fontweight="medium")
    ax_lollipop.set_title("各配置绝对指标值对比",
                           fontsize=FONT["title"], fontweight="bold", pad=10)
    ax_lollipop.set_ylim(0, 115)
    ax_lollipop.legend(loc="upper center", ncol=5,
                       fontsize=FONT["legend"] - 1,
                       frameon=True, framealpha=0.9, edgecolor="#CCC",
                       bbox_to_anchor=(0.5, -0.08))
    ax_lollipop.grid(True, axis="y", alpha=0.15)

    save(fig, "fig4_ablation_bars")


# ═══════════════════════════════════════════════════════════════
# 图5  指数退避延迟分布
# ═══════════════════════════════════════════════════════════════
def fig5_backoff(data: dict):
    """图5 指数退避 — 字号提升，图例优化"""
    curve = data["backoff_curve"]
    attempts = [r["attempt"] for r in curve]
    base = [r["base_ms"] for r in curve]
    mn = [r["min_ms"] for r in curve]
    mx = [r["max_ms"] for r in curve]

    fig, ax = plt.subplots(figsize=(8, 5))
    ax.fill_between(attempts, mn, mx, alpha=0.20,
                    color=COLORS["backoff_fill"], label="抖动区间")
    ax.plot(attempts, base, "o-",
            color=COLORS["backoff_line"], lw=2.5, markersize=8,
            label="基础延迟", zorder=5)
    ax.axhline(30000, color=COLORS["backoff_limit"], ls="--", lw=1.5,
               label="上限 30 s")

    style_axis(ax, xlabel="重试次数", ylabel="延迟 (ms)")
    ax.set_yscale("log")
    ax.legend(loc="upper left", frameon=True, framealpha=0.9,
              edgecolor="#CCCCCC")
    ax.grid(True, alpha=0.15, which="both")

    # 关键点标注
    ax.annotate(f"{base[-1]/1000:.1f} s",
                xy=(attempts[-1], base[-1]),
                xytext=(attempts[-1] - 0.8, base[-1] * 1.8),
                fontsize=FONT["annotation"],
                arrowprops=dict(arrowstyle="->", color="#555"),
                fontweight="medium", color=COLORS["backoff_line"])

    save(fig, "fig5_backoff_distribution")


def fig6_attention_bench(data: dict):
    """图6 微基准 — 可视化漏斗图 + 右侧阶段注释"""
    bench = data["attention_list_bench"]
    labels = ["输入日志", "高价值日志", "泛化模式", "Top-K 清单"]
    values = [bench["input_logs"], bench["high_value_logs"],
              bench["unique_patterns"], bench["top_k"]]
    step_labels = ["阈值/错误码筛选", "URL Generalize(·)", "权重排序 Top-K"]
    colors_funnel = ["#3498DB", "#2ECC71", "#F39C12", "#E74C3C"]

    fig, (ax_funnel, ax_detail) = plt.subplots(
        1, 2, figsize=(13, 5.5), width_ratios=[1.6, 1],
        gridspec_kw={"wspace": 0.3})

    # ── 左：漏斗图 ──────────────────────────────────────────
    ax_funnel.set_xlim(0, 10)
    ax_funnel.set_ylim(-0.5, len(values) * 2 + 0.5)
    ax_funnel.axis("off")
    ax_funnel.set_title("关注清单生成算法漏斗",
                         fontsize=FONT["title"], fontweight="bold", pad=12)

    max_val = values[0]
    center_x = 5.0
    bar_h = 1.2
    gap = 0.5

    for i, (val, label, color) in enumerate(zip(values, labels, colors_funnel)):
        y_bottom = (len(values) - 1 - i) * (bar_h + gap)
        # 梯形宽度 按比例（最小 1.5 保底）
        half_w = max(val / max_val * 4.0, 0.8)
        # 下一层宽度
        if i < len(values) - 1:
            next_half_w = max(values[i + 1] / max_val * 4.0, 0.8)
        else:
            next_half_w = half_w

        # 绘制梯形
        trap = plt.Polygon([
            (center_x - half_w, y_bottom + bar_h),
            (center_x + half_w, y_bottom + bar_h),
            (center_x + next_half_w, y_bottom),
            (center_x - next_half_w, y_bottom),
        ], closed=True, fc=color, ec="white", lw=2, alpha=0.85, zorder=3)
        ax_funnel.add_patch(trap)

        # 标签 + 数值
        ax_funnel.text(center_x, y_bottom + bar_h / 2,
                       f"{label}\n{int(val):,}", ha="center", va="center",
                       fontsize=11, fontweight="bold", color="white", zorder=5)

        # 左侧百分比标注
        pct = val / max_val * 100
        ax_funnel.text(center_x - half_w - 0.3, y_bottom + bar_h / 2,
                       f"{pct:.1f}%", ha="right", va="center",
                       fontsize=9, color=color, fontweight="bold")

        # 阶段箭头标注（两层之间）
        if i < len(values) - 1:
            retention = values[i + 1] / val * 100
            ax_funnel.annotate(
                f"{step_labels[i]}\n保留 {retention:.1f}%",
                xy=(center_x + half_w + 0.2, y_bottom + bar_h / 2),
                fontsize=8, color="#555", fontstyle="italic",
                va="center")

    # ── 右：对数条形图（精确数据参考）─────────────────────
    y_pos = np.arange(len(labels))
    bars = ax_detail.barh(y_pos, values, height=0.55,
                          color=colors_funnel, alpha=0.85,
                          edgecolor="white", linewidth=0.8)
    ax_detail.set_xscale("log")
    ax_detail.set_yticks(y_pos)
    ax_detail.set_yticklabels(labels, fontsize=FONT["tick"])
    ax_detail.invert_yaxis()
    style_axis(ax_detail, xlabel="数量 (对数)", title="各阶段精确数值")

    # 数值标注
    for bar, val in zip(bars, values):
        ax_detail.text(val * 1.5, bar.get_y() + bar.get_height() / 2,
                       f"{int(val):,}",
                       ha="left", va="center",
                       fontsize=FONT["annotation"], fontweight="bold")

    ax_detail.grid(True, axis="x", alpha=0.1, which="both")
    ax_detail.set_xlim(1, max_val * 10)

    save(fig, "fig6_attention_bench")


# ═══════════════════════════════════════════════════════════════
# main
# ═══════════════════════════════════════════════════════════════
def main():
    apply_style()
    FIGURES_DIR.mkdir(parents=True, exist_ok=True)
    data = load_data()
    print("生成改进版图表 (fig1–fig6)...")
    fig1_system_overview()
    fig2_triple_transform()
    fig3_comparison(data)
    fig4_ablation(data)
    fig5_backoff(data)
    fig6_attention_bench(data)
    print(f"\n全部图表已输出至 {FIGURES_DIR}")


if __name__ == "__main__":
    main()
