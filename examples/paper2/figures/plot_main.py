#!/usr/bin/env python3
"""Fig.5 RTT CDF and Fig.6 valid rate (from main_seed*.csv)."""

from __future__ import annotations

import csv
from pathlib import Path

import numpy as np

PAPER1_ROOT = Path(__file__).resolve().parents[1]
RESULTS = PAPER1_ROOT / 'experiments' / 'results'
FIG5 = PAPER1_ROOT / 'figures' / 'fig5_rtt_cdf.pdf'
FIG6 = PAPER1_ROOT / 'figures' / 'fig6_valid_rate.pdf'


def load_details() -> dict:
    """baseline -> list of rtt_ms from all seeds."""
    data: dict = {}
    for seed in [0, 1, 2]:
        path = RESULTS / f'main_seed{seed}_detail.csv'
        if not path.is_file():
            continue
        with path.open(encoding='utf-8') as f:
            for row in csv.DictReader(f):
                bl = row['baseline']
                data.setdefault(bl, []).append(float(row['rtt_ms']))
    return data


def load_summary() -> list:
    rows = []
    for seed in [0, 1, 2]:
        path = RESULTS / f'main_seed{seed}.csv'
        with path.open(encoding='utf-8') as f:
            rows.extend(list(csv.DictReader(f)))
    return rows


def main() -> None:
    import matplotlib

    matplotlib.use('Agg')
    import matplotlib.pyplot as plt

    details = load_details()
    summary = load_summary()

    # Fig.5 CDF B0 vs B2
    fig, ax = plt.subplots(figsize=(6, 4))
    for bl, style in [('B0', '--'), ('B1', '-.'), ('B2', '-'), ('B4', ':')]:
        if bl not in details:
            continue
        xs = np.sort(details[bl])
        ys = np.arange(1, len(xs) + 1) / len(xs)
        ax.plot(xs, ys, style, label=bl, linewidth=2)
    ax.set_xlabel('RTT (ms)')
    ax.set_ylabel('CDF')
    ax.set_title('End-to-end RTT (offline main experiment)')
    ax.legend()
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    fig.savefig(FIG5)
    print('Wrote', FIG5)

    # Fig.6 valid rate bar
    baselines = ['B0', 'B1', 'B2', 'B3']
    means, stds = [], []
    for bl in baselines:
        vals = [float(r['m2_valid_rate']) for r in summary if r['baseline'] == bl]
        means.append(np.mean(vals))
        stds.append(np.std(vals))
    fig, ax = plt.subplots(figsize=(6, 4))
    x = np.arange(len(baselines))
    ax.bar(x, means, yerr=stds, capsize=4, color=['#94a3b8', '#60a5fa', '#2563eb', '#1d4ed8'])
    ax.set_xticks(x, baselines)
    ax.set_ylabel('Valid frame rate')
    ax.set_ylim(0, 1.05)
    ax.set_title('M2: valid rate (3 seeds)')
    fig.tight_layout()
    fig.savefig(FIG6)
    print('Wrote', FIG6)


if __name__ == '__main__':
    main()
