#!/usr/bin/env python3
"""三期实测图表：优先 phase3，回退 phase2"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "figures"))

PHASE3 = ROOT / "experiments/results/phase3/phase3_latest.json"
PHASE2 = ROOT / "experiments/results/phase2/phase2_latest.json"
PHASE1 = ROOT / "experiments/results/phase1/phase1_latest.json"


def merge_data() -> dict:
    base = json.loads(PHASE1.read_text(encoding="utf-8"))
    src = PHASE3 if PHASE3.exists() else PHASE2
    if src.exists():
        px = json.loads(src.read_text(encoding="utf-8"))
        base["comparison"] = px["comparison"]
        base["phase"] = px.get("phase", "phase3_measured")
        if "directed" in px:
            base["directed_run"] = px["directed"]
            base["full_run"] = px["full"]
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
    print(f"[phase3] charts from {data.get('phase')} comparison")


if __name__ == "__main__":
    main()
