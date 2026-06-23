#!/usr/bin/env python3
"""Fig.7 — τ ablation (M4): valid rate and M1 p50 vs τ."""

from __future__ import annotations

import csv
from pathlib import Path

import numpy as np

PAPER1_ROOT = Path(__file__).resolve().parents[1]
CSV_PATH = PAPER1_ROOT / 'experiments/results/ablation_tau.csv'
OUT = PAPER1_ROOT / 'figures/fig7_ablation_tau.pdf'


def main() -> None:
    import matplotlib

    matplotlib.use('Agg')
    import matplotlib.pyplot as plt

    taus, p50s, valids = [], [], []
    with CSV_PATH.open(encoding='utf-8') as f:
        for row in csv.DictReader(f):
            taus.append(float(row['tau']))
            p50s.append(float(row['m1_p50_ms']))
            valids.append(float(row['m2_valid_rate']))

    fig, ax1 = plt.subplots(figsize=(6, 4))
    ax2 = ax1.twinx()
    ax1.plot(taus, valids, 'o-', color='#2563eb', linewidth=2, label='M2 valid rate')
    ax2.plot(taus, p50s, 's--', color='#dc2626', linewidth=2, label='M1 p50 (ms)')
    ax1.axvline(0.505, color='#64748b', linestyle=':', label=r'$\tau^*=0.505$')
    ax1.set_xlabel(r'Threshold $\tau$')
    ax1.set_ylabel('Valid frame rate', color='#2563eb')
    ax2.set_ylabel('RTT p50 (ms)', color='#dc2626')
    ax1.set_ylim(0, 1.05)
    ax1.grid(True, alpha=0.3)
    lines1, labels1 = ax1.get_legend_handles_labels()
    lines2, labels2 = ax2.get_legend_handles_labels()
    ax1.legend(lines1 + lines2, labels1 + labels2, loc='center right')
    fig.tight_layout()
    fig.savefig(OUT)
    print('Wrote', OUT)


if __name__ == '__main__':
    main()
