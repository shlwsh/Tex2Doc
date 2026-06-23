#!/usr/bin/env python3
"""Fig.S1 — auxiliary franka M6 RTT distribution."""

from __future__ import annotations

import csv
from pathlib import Path

import numpy as np

PAPER1_ROOT = Path(__file__).resolve().parents[1]
CSV_PATH = PAPER1_ROOT / 'experiments/results/franka_m6_rtt.csv'
OUT = PAPER1_ROOT / 'figures/fig_s1_franka_rtt.pdf'


def main() -> None:
    import matplotlib

    matplotlib.use('Agg')
    import matplotlib.pyplot as plt

    lats = []
    with CSV_PATH.open(encoding='utf-8') as f:
        for row in csv.DictReader(f):
            lats.append(float(row['latency_ms']))

    fig, ax = plt.subplots(figsize=(6, 4))
    ax.hist(lats, bins=20, color='#0d9488', alpha=0.85, edgecolor='white')
    ax.set_xlabel('Trial latency (ms)')
    ax.set_ylabel('Count')
    ax.set_title(f'Auxiliary track M6 (n={len(lats)}, franka closed-loop)')
    fig.tight_layout()
    fig.savefig(OUT)
    print('Wrote', OUT)


if __name__ == '__main__':
    main()
