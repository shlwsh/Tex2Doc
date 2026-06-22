#!/usr/bin/env python3
"""Plot Fig.3 IQA histogram from calibration_val.csv."""

from __future__ import annotations

import csv
from pathlib import Path

PAPER1_ROOT = Path(__file__).resolve().parents[1]
CSV_PATH = PAPER1_ROOT / 'experiments' / 'results' / 'calibration_val.csv'
OUT_PDF = PAPER1_ROOT / 'figures' / 'fig_iqa_hist.pdf'
OUT_SVG = PAPER1_ROOT / 'figures' / 'fig_iqa_hist.svg'


def main() -> None:
    if not CSV_PATH.is_file():
        raise SystemExit(f'run calibrate_tau.py first; missing {CSV_PATH}')

    clear_q, blur_q = [], []
    with CSV_PATH.open(encoding='utf-8') as f:
        for row in csv.DictReader(f):
            q = float(row['q_img'])
            if row['label'] == 'clear':
                clear_q.append(q)
            elif row['label'] == 'blur':
                blur_q.append(q)

    try:
        import matplotlib

        matplotlib.use('Agg')
        import matplotlib.pyplot as plt
    except ImportError:
        _write_svg_fallback(clear_q, blur_q)
        print(f'matplotlib missing; wrote {OUT_SVG}')
        return

    fig, ax = plt.subplots(figsize=(6, 4))
    bins = 20
    ax.hist(clear_q, bins=bins, alpha=0.6, label='clear', color='#2563eb')
    ax.hist(blur_q, bins=bins, alpha=0.6, label='blur', color='#dc2626')
    ax.set_xlabel('$Q_{img}$')
    ax.set_ylabel('count')
    ax.set_title('Edge-IQA score distribution (val)')
    ax.legend()
    fig.tight_layout()
    OUT_PDF.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(OUT_PDF)
    fig.savefig(OUT_SVG)
    print(f'Wrote {OUT_PDF} and {OUT_SVG}')


def _write_svg_fallback(clear_q: list, blur_q: list) -> None:
    """Minimal SVG bar representation without matplotlib."""
    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        '<svg xmlns="http://www.w3.org/2000/svg" width="600" height="360">',
        '<text x="20" y="30" font-size="16">Fig.3 Edge-IQA histogram (clear vs blur)</text>',
        f'<text x="20" y="55" font-size="12">clear n={len(clear_q)} mean={sum(clear_q)/max(len(clear_q),1):.3f}</text>',
        f'<text x="20" y="75" font-size="12">blur n={len(blur_q)} mean={sum(blur_q)/max(len(blur_q),1):.3f}</text>',
        '</svg>',
    ]
    OUT_SVG.write_text('\n'.join(lines), encoding='utf-8')


if __name__ == '__main__':
    main()
