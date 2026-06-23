#!/usr/bin/env python3
"""Fig.2 routing flow (SVG/PDF)."""

from pathlib import Path

OUT_SVG = Path(__file__).parent / 'fig2_routing_flow.svg'
OUT_PDF = Path(__file__).parent / 'fig2_routing_flow.pdf'

SVG = """<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="720" height="200" viewBox="0 0 720 200">
  <style>
    .box { fill:#f8fafc; stroke:#334155; stroke-width:2; }
    .t { font: 12px sans-serif; fill:#0f172a; }
    .arrow { stroke:#2563eb; stroke-width:2; fill:none; marker-end:url(#m); }
  </style>
  <defs><marker id="m" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto">
    <path d="M0,0 L6,3 L0,6 Z" fill="#2563eb"/></marker></defs>
  <rect class="box" x="10" y="70" width="100" height="50" rx="6"/>
  <text class="t" x="35" y="100">capture</text>
  <rect class="box" x="140" y="70" width="100" height="50" rx="6"/>
  <text class="t" x="155" y="100">edge_iqa</text>
  <rect class="box" x="270" y="70" width="80" height="50" rx="6"/>
  <text class="t" x="285" y="100">route</text>
  <rect class="box" x="390" y="30" width="110" height="40" rx="6"/>
  <text class="t" x="405" y="55">upload_cloud</text>
  <rect class="box" x="390" y="90" width="110" height="40" rx="6"/>
  <text class="t" x="400" y="115">resample_edge</text>
  <rect class="box" x="390" y="150" width="110" height="40" rx="6"/>
  <text class="t" x="410" y="175">fail_safe</text>
  <line class="arrow" x1="110" y1="95" x2="140" y2="95"/>
  <line class="arrow" x1="240" y1="95" x2="270" y2="95"/>
  <line class="arrow" x1="350" y1="85" x2="390" y2="50"/>
  <line class="arrow" x1="350" y1="95" x2="390" y2="110"/>
  <line class="arrow" x1="350" y1="105" x2="390" y2="170"/>
</svg>
"""


def main() -> None:
    OUT_SVG.write_text(SVG, encoding='utf-8')
    try:
        import matplotlib

        matplotlib.use('Agg')
        import matplotlib.pyplot as plt
        from matplotlib.patches import FancyBboxPatch

        fig, ax = plt.subplots(figsize=(7.2, 2))
        ax.set_xlim(0, 7.2)
        ax.set_ylim(0, 2)
        ax.axis('off')
        labels = [
            (0.2, 1, 'capture'),
            (1.5, 1, 'edge_iqa'),
            (2.8, 1, 'route'),
            (4.5, 1.4, 'upload'),
            (4.5, 1, 'resample'),
            (4.5, 0.6, 'fail_safe'),
        ]
        for x, y, t in labels:
            ax.add_patch(FancyBboxPatch((x, y - 0.15), 0.9, 0.3, boxstyle='round', fc='#f8fafc', ec='#334155'))
            ax.text(x + 0.1, y, t, fontsize=9)
        fig.savefig(OUT_PDF, bbox_inches='tight')
        print('Wrote', OUT_PDF)
    except ImportError:
        print('matplotlib missing; SVG only')
    print('Wrote', OUT_SVG)


if __name__ == '__main__':
    main()
