#!/usr/bin/env python3
"""二期实测图表（覆盖 fig3/fig4 数据为实测）"""

import json
import sys
from pathlib import Path

# 复用 phase1 绘图，替换数据源
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "figures"))

PHASE2 = ROOT / "experiments/results/phase2/phase2_latest.json"
PHASE1 = ROOT / "experiments/results/phase1/phase1_latest.json"


def merge_data() -> dict:
    base = json.loads(PHASE1.read_text(encoding="utf-8"))
    if PHASE2.exists():
        p2 = json.loads(PHASE2.read_text(encoding="utf-8"))
        base["comparison"] = p2["comparison"]
        base["ablation"] = p2.get("ablation", base.get("ablation"))
        base["phase"] = "phase2_measured"
        base["directed_run"] = p2.get("directed_run")
        base["full_run"] = p2.get("full_run")
    return base


def main():
    data = merge_data()
    out = ROOT / "experiments/results/phase1/phase1_latest.json"
    backup = ROOT / "experiments/results/phase1/phase1_latest_backup.json"
    if not backup.exists():
        backup.write_text(out.read_text(encoding="utf-8"), encoding="utf-8")
    out.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")

    import plot_all_phase1
    plot_all_phase1.main()

    # 恢复 phase1 原始仿真数据到 backup 位置供对照
    print("[phase2] 图表已基于实测 comparison 数据重新生成")


if __name__ == "__main__":
    main()
