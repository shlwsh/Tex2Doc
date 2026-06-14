#!/usr/bin/env python3
"""Concatenate every Markdown file under docs/study/ into a single styled HTML.

The output HTML is written next to this script as `combined.html` and is
designed to be loaded by `file://` in a headless browser, where the
Chrome DevTools Protocol's `Page.printToPDF` will turn it into a PDF.
"""
from __future__ import annotations

import datetime as _dt
import os
import re
import sys
from pathlib import Path

import markdown

ROOT = Path(__file__).resolve().parents[1]
STUDY = ROOT / "docs" / "study"
SCRIPTS = ROOT / "scripts"
DOCS = ROOT / "docs"
OUT_HTML = SCRIPTS / "combined.html"
OUT_PDF = DOCS / "Tex2Doc_项目说明文档_Study合集_v1.3.pdf"

# Reading order = directory order. Each entry is (subdir, list of files).
SECTIONS: list[tuple[str, list[str]]] = [
    ("01-overview", ["01-features.md", "02-quick-tour.md"]),
    ("02-tech-stack", [
        "01-rust-stack.md",
        "02-flutter-dart-stack.md",
        "03-web-extension-stack.md",
    ]),
    ("03-project-structure", [
        "01-top-level.md",
        "02-rust-crates.md",
        "03-flutter-app.md",
        "04-extension-scripts-tests.md",
    ]),
    ("04-architecture", [
        "01-end-to-end-pipeline.md",
        "02-layered-architecture.md",
        "03-frontend-bridges.md",
    ]),
    ("05-key-tech", [
        "01-include-topology.md",
        "02-lexer-and-cst.md",
        "03-semantic-lowering.md",
        "04-docx-serialization.md",
        "05-math-pipeline.md",
        "06-vfs-and-fonts.md",
    ]),
    ("06-user-guide", [
        "01-cli-and-script.md",
        "02-pwa-web.md",
        "03-desktop.md",
        "04-chrome-extension.md",
        "05-http-server.md",
    ]),
    ("07-deployment", [
        "01-rust-build.md",
        "02-flutter-build.md",
        "03-wasm-publish.md",
        "04-server-deploy.md",
        "05-extension-pack.md",
        "06-ci-and-hooks.md",
    ]),
]

TITLE_PREFIX = "Tex2Doc 项目说明文档"

# 1-based per-page offset so anchors rendered in the combined PDF do not
# collide with the same anchor text appearing in multiple source files.
PAGE_OFFSET_BASE = 1000

CSS = r"""
/* xhtml2pdf understands the Adobe CIDFont names; we lean on STSong-Light
   so Chinese (and most other CJK) glyphs render without bundling a 10 MB
   TTF subset into the PDF. */
html { font-size: 11pt; }
body {
  font-family: "STSong-Light", "Helvetica", "Arial", sans-serif;
  color: #1d1f23;
  line-height: 1.65;
  max-width: 880px;
  margin: 0 auto;
  padding: 12mm 8mm 18mm;
  word-wrap: break-word;
}

.cover {
  text-align: center;
  padding: 80px 0 60px;
  page-break-after: always;
  border-bottom: 1px solid #e5e7eb;
}
.cover h1 {
  font-size: 32pt;
  margin: 0 0 16px;
  letter-spacing: 2px;
}
.cover .meta { color: #6b7280; font-size: 11pt; }
.cover .toc-link {
  display: inline-block;
  margin-top: 32px;
  color: #2563eb;
  text-decoration: none;
  font-size: 10.5pt;
  border: 1px solid #93c5fd;
  padding: 6px 14px;
  border-radius: 4px;
}

.toc { page-break-after: always; }
.toc h1 { font-size: 22pt; border-bottom: 2px solid #1d1f23; padding-bottom: 6px; }
.toc ol { list-style: none; padding-left: 0; }
.toc li { margin: 4px 0; }
.toc .sec { font-weight: 600; margin-top: 14px; color: #1d1f23; }
.toc a { color: #2563eb; text-decoration: none; }
.toc a:hover { text-decoration: underline; }
.toc .pages { color: #9ca3af; font-size: 9.5pt; margin-left: 6px; }

.chapter { page-break-before: always; }
.chapter:first-of-type { page-break-before: auto; }
.chapter-header {
  font-size: 10pt;
  letter-spacing: 4px;
  color: #9ca3af;
  text-transform: uppercase;
  border-bottom: 1px solid #e5e7eb;
  padding-bottom: 6px;
  margin-bottom: 18px;
}

h1, h2, h3, h4, h5, h6 { color: #111827; line-height: 1.3; }
h1 { font-size: 22pt; border-bottom: 2px solid #1d1f23; padding-bottom: 6px; margin-top: 0; }
h2 { font-size: 16pt; border-bottom: 1px solid #e5e7eb; padding-bottom: 4px; margin-top: 28px; }
h3 { font-size: 13pt; margin-top: 24px; }
h4 { font-size: 12pt; margin-top: 20px; }
h5, h6 { font-size: 11.5pt; margin-top: 16px; }

p { margin: 0.6em 0; }

a { color: #2563eb; text-decoration: none; }
a:hover { text-decoration: underline; }

ul, ol { padding-left: 1.6em; margin: 0.6em 0; }
li { margin: 0.15em 0; }

blockquote {
  margin: 1em 0;
  padding: 8px 14px;
  border-left: 4px solid #cbd5e1;
  color: #475569;
  background: #f8fafc;
  border-radius: 0 4px 4px 0;
}

code {
  font-family: "JetBrains Mono", "Cascadia Code", "Fira Code", Consolas,
    "SFMono-Regular", monospace;
  font-size: 0.9em;
  background: #f1f5f9;
  padding: 1px 5px;
  border-radius: 3px;
  color: #b91c1c;
}
pre {
  font-family: "JetBrains Mono", "Cascadia Code", "Fira Code", Consolas,
    "SFMono-Regular", monospace;
  font-size: 9.5pt;
  line-height: 1.5;
  background: #0f172a;
  color: #e2e8f0;
  padding: 12px 16px;
  border-radius: 6px;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-word;
}
pre code {
  background: transparent;
  color: inherit;
  padding: 0;
  font-size: inherit;
}

table {
  border-collapse: collapse;
  width: 100%;
  margin: 1em 0;
  font-size: 10pt;
  page-break-inside: avoid;
}
th, td {
  border: 1px solid #d1d5db;
  padding: 6px 10px;
  text-align: left;
  vertical-align: top;
}
th { background: #f3f4f6; font-weight: 600; }
tr:nth-child(even) td { background: #fafafa; }

hr { border: none; border-top: 1px solid #e5e7eb; margin: 1.5em 0; }

img { max-width: 100%; height: auto; }
"""

SLUG_RE = re.compile(r"[\s\u3000]+")


def _slugify(text: str, used: dict[str, int]) -> str:
    base = re.sub(r"[^\w\u4e00-\u9fff\- ]+", "", text).strip().lower()
    base = SLUG_RE.sub("-", base) or "section"
    count = used.get(base, 0)
    used[base] = count + 1
    return base if count == 0 else f"{base}-{count}"


def rewrite_anchors(html: str, used: dict[str, int]) -> str:
    """Make Markdown auto-anchors unique across the combined document.

    The default Python-Markdown TOC extension produces ids like `h1`/`h2`.
    When we concatenate many documents those ids collide; we rewrite the
    `id="..."` and matching `href="#..."` pairs in deterministic fashion.
    """
    heading_re = re.compile(
        r'(<h[1-6]\b[^>]*\bid=")([^"]+)(")', re.IGNORECASE
    )

    def repl(match: re.Match[str]) -> str:
        old = match.group(2)
        new = _slugify(old, used)
        return f"{match.group(1)}{new}{match.group(3)}"

    html = heading_re.sub(repl, html)
    # update href references to those renamed ids
    for old, new in list(used.items())[-200:]:
        html = html.replace(f'href="#{old}"', f'href="#{new}"')
    # last pass: rebuild href mapping
    return html


def render_section(md_text: str, *, slug_used: dict[str, int]) -> str:
    html = markdown.markdown(
        md_text,
        extensions=[
            "extra",
            "sane_lists",
            "tables",
            "fenced_code",
            "codehilite",
            "toc",
        ],
        extension_configs={
            "codehilite": {"guess_lang": False, "css_class": "codehilite"},
            "toc": {"permalink": False},
        },
    )
    return rewrite_anchors(html, slug_used)


def main() -> int:
    md = markdown.Markdown(
        extensions=[
            "extra",
            "sane_lists",
            "tables",
            "fenced_code",
            "codehilite",
            "toc",
        ],
        extension_configs={
            "codehilite": {"guess_lang": False, "css_class": "codehilite"},
            "toc": {"permalink": False},
        },
    )

    slug_used: dict[str, int] = {}
    sections_html: list[str] = []
    toc_entries: list[tuple[str, str, str, int]] = []  # (chapter, sub, anchor, depth)

    for chapter, files in SECTIONS:
        chapter_path = STUDY / chapter
        if not chapter_path.is_dir():
            print(f"[warn] missing chapter dir: {chapter_path}", file=sys.stderr)
            continue
        for fname in files:
            fpath = chapter_path / fname
            if not fpath.is_file():
                print(f"[warn] missing file: {fpath}", file=sys.stderr)
                continue
            text = fpath.read_text(encoding="utf-8")
            body = render_section(text, slug_used=slug_used)
            title = fname
            # use the first h1 as the title if available
            m = re.search(r"<h1[^>]*>(.*?)</h1>", body, re.IGNORECASE | re.DOTALL)
            if m:
                clean = re.sub(r"<[^>]+>", "", m.group(1)).strip()
                if clean:
                    title = clean
            section_html = (
                f'<section class="chapter" id="sec-{chapter}-{fname[:-3]}">\n'
                f'  <div class="chapter-header">第 {chapter.split("-")[0]} 章 · '
                f'{chapter.split("-", 1)[1]}</div>\n'
                f'  {body}\n'
                f"</section>\n"
            )
            sections_html.append(section_html)
            toc_entries.append((chapter, fname, title, 1))

    # Build cover + TOC
    now = _dt.datetime.now().strftime("%Y-%m-%d %H:%M")
    cover = f"""
<section class="cover">
  <h1>{TITLE_PREFIX}</h1>
  <p class="meta">Tex2Doc / Doc-engine · V1.3 学习手册</p>
  <p class="meta">由 docs/study/ 下 29 篇 Markdown 自动合并生成</p>
  <p class="meta">生成时间：{now}</p>
  <a class="toc-link" href="#toc">↓ 查看目录</a>
</section>
"""

    toc_items: list[str] = []
    last_chapter = None
    for chapter, fname, title, _depth in toc_entries:
        if chapter != last_chapter:
            ch_title = chapter.split("-", 1)[1] if "-" in chapter else chapter
            toc_items.append(
                f'<li class="sec">{chapter} · {ch_title}</li>'
            )
            last_chapter = chapter
        anchor = f"sec-{chapter}-{fname[:-3]}"
        toc_items.append(
            f'<li>　└ <a href="#{anchor}">{fname} — {title}</a></li>'
        )
    toc = (
        '<section class="toc" id="toc">\n'
        '  <h1>目录</h1>\n'
        '  <ol>\n    ' + "\n    ".join(toc_items) + "\n  </ol>\n"
        "</section>\n"
    )

    html = (
        "<!DOCTYPE html>\n"
        '<html lang="zh-CN">\n'
        "<head>\n"
        '  <meta charset="utf-8">\n'
        f"  <title>{TITLE_PREFIX}</title>\n"
        f"  <style>{CSS}</style>\n"
        "</head>\n"
        "<body>\n"
        + cover
        + toc
        + "\n".join(sections_html)
        + "\n</body>\n</html>\n"
    )

    OUT_HTML.write_text(html, encoding="utf-8")
    print(f"[ok] wrote {OUT_HTML} ({len(html):,} bytes)")

    # ---- Render to PDF -----------------------------------------------------
    # xhtml2pdf is pure-Python and renders the HTML above straight to PDF.
    # It does not understand modern CSS like @page, flex, or grid, so we
    # strip those rules from the embedded stylesheet first to keep the
    # output tidy. Long lines and code blocks still wrap because we set
    # `word-wrap: break-word` on the body and `pre` white-space: pre-wrap
    # survives the simplification.
    pdf_html = _html_for_xhtml2pdf(html)
    render_pdf(pdf_html, OUT_PDF)
    print(f"[ok] wrote {OUT_PDF} ({OUT_PDF.stat().st_size:,} bytes)")
    return 0


def _html_for_xhtml2pdf(html: str) -> str:
    """Return a copy of *html* with @page / flex / grid rules stripped.

    xhtml2pdf raises on `@page` blocks and silently ignores modern layout
    primitives, so we leave a printable subset behind. Everything else
    (typography, tables, code blocks, links) is preserved as-is.
    """
    style_re = re.compile(r"<style>(.*?)</style>", re.IGNORECASE | re.DOTALL)

    def simplify(match: re.Match[str]) -> str:
        css = match.group(1)
        # Drop entire @page / @media / @keyframes / @font-face blocks.
        css = re.sub(r"@page\b[^{]*\{[^}]*\}\s*", "", css)
        css = re.sub(r"@media\b[^{]*\{(?:[^{}]|\{[^{}]*\})*\}\s*", "", css)
        css = re.sub(r"@(?:keyframes|font-face)\b[^{]*\{(?:[^{}]|\{[^{}]*\})*\}\s*", "", css)
        return f"<style>{css}</style>"

    return style_re.sub(simplify, html)


def render_pdf(html: str, dest: Path) -> None:
    """Render *html* to *dest* using xhtml2pdf, surfacing any failures."""
    from xhtml2pdf import pisa  # imported lazily so the script still runs
                                 # if xhtml2pdf is not yet installed.

    dest.parent.mkdir(parents=True, exist_ok=True)
    with dest.open("wb") as fh:
        result = pisa.CreatePDF(html, dest=fh, encoding="utf-8")
    if result.err:
        raise SystemExit(f"[fail] xhtml2pdf reported {result.err} error(s)")


if __name__ == "__main__":
    raise SystemExit(main())
