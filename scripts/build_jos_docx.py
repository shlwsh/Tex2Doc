#!/usr/bin/env python3
"""Build a DOCX directly from the LaTeX manuscript using a template profile.

The old pipeline asked Pandoc to infer a heavily customized rjthesis document.
That lost front matter and several custom environments.  This generator keeps
the conversion deterministic: it parses the manuscript structure and writes
WordprocessingML using format values extracted under docs/format plus a small
profile for template-specific headers, footers, and front-matter spacing.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import zipfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable
from xml.sax.saxutils import escape

try:
    from PIL import Image
except ImportError:  # pragma: no cover - build script reports a clear error.
    Image = None


EMU_PER_CM = 360000
CITATION_RE = re.compile(r"\[[0-9][0-9,\-\s]*\]")
SECTION_FILES = [
    "latex/sections/zh/01_intro.tex",
    "latex/sections/zh/02_related.tex",
    "latex/sections/zh/03_system.tex",
    "latex/sections/zh/04_algorithms.tex",
    "latex/sections/zh/05_implementation.tex",
    "latex/sections/zh/06_experiments.tex",
    "latex/sections/zh/07_conclusion.tex",
]


@dataclass
class Block:
    kind: str
    text: str = ""
    level: int = 0
    caption: str = ""
    label: str = ""
    rows: list[list[str]] = field(default_factory=list)
    image_path: Path | None = None
    width_factor: float = 0.9
    lines: list[str] = field(default_factory=list)
    algorithm_io: list[tuple[str, str]] = field(default_factory=list)
    algorithm_rows: list[dict[str, object]] = field(default_factory=list)


@dataclass
class Manuscript:
    title_zh: str
    authors_zh: str
    institute_lines: list[str]
    abstract_zh: str
    keywords_zh: str
    category: str
    citation_zh: str
    citation_en: str
    title_en: str
    authors_en: str
    institute_en: str
    abstract_en: str
    keywords_en: str
    running_header: str
    first_footer_text: str
    blocks: list[Block]
    references: list[str]
    cn_references: list[str]
    author_bio: list[str]


@dataclass(frozen=True)
class DocxProfile:
    first_header_rows: tuple[tuple[str, str], ...]
    even_header_text: str
    first_footer_text: str
    first_footer_indent_twips: int = 330
    footer_distance_twips: int = 1260
    after_institute_twips: int = 300
    before_citation_twips: int = 300
    before_english_title_twips: int = 220
    before_english_abstract_twips: int = 340
    citation_wrap_units: float = 52.0
    zh_abstract_label: str = "摘   要:"
    zh_keywords_label: str = "关键词:"
    category_label: str = "中图法分类号:"
    en_abstract_label: str = "Abstract:"
    en_keywords_label: str = "Key words:"


JOS_PROFILE = DocxProfile(
    first_header_rows=(
        ("软件学报 ISSN 1000-9825, CODEN RUXUEW", "E-mail: jos@iscas.ac.cn"),
        ("Journal of Software, [doi: 10.13328/j.cnki.jos.000000]", "http://www.jos.org.cn"),
        ("© 中国科学院软件研究所版权所有.", "Tel: +86-10-62562563"),
    ),
    even_header_text="Journal of Software 软件学报",
    first_footer_text="收稿时间: XXXX-XX-XX; 修改时间: XXXX-XX-XX; 采用时间: XXXX-XX-XX",
)


def xml(text: str) -> str:
    return escape(text, {"\u00a0": " "})


def run_xml(text: str, superscript: bool = False, subscript: bool = False) -> str:
    if text == "":
        return ""
    if superscript:
        rpr = '<w:rPr><w:vertAlign w:val="superscript"/></w:rPr>'
    elif subscript:
        rpr = '<w:rPr><w:vertAlign w:val="subscript"/></w:rPr>'
    else:
        rpr = ""
    return f'<w:r>{rpr}<w:t xml:space="preserve">{xml(text)}</w:t></w:r>'


def inline_runs_xml(text: str, enable_superscript: bool = True, enable_subscript: bool = False) -> str:
    if not enable_superscript:
        return run_xml(text)
    runs: list[str] = []
    buf: list[str] = []
    i = 0

    def flush() -> None:
        if buf:
            runs.append(run_xml("".join(buf)))
            buf.clear()

    while i < len(text):
        citation = CITATION_RE.match(text, i)
        if citation:
            flush()
            runs.append(run_xml(citation.group(0), superscript=True))
            i = citation.end()
            continue
        if text[i] == "^" and i + 1 < len(text):
            flush()
            if text[i + 1] == "{":
                end = text.find("}", i + 2)
                if end > i + 2:
                    runs.append(run_xml(text[i + 2 : end], superscript=True))
                    i = end + 1
                    continue
            m = re.match(r"\^([A-Za-z0-9*]+)", text[i:])
            if m:
                runs.append(run_xml(m.group(1), superscript=True))
                i += len(m.group(0))
                continue
        if enable_subscript and text[i] == "_" and i + 1 < len(text):
            flush()
            if text[i + 1] == "{":
                end = text.find("}", i + 2)
                if end > i + 2:
                    runs.append(run_xml(text[i + 2 : end], subscript=True))
                    i = end + 1
                    continue
            m = re.match(r"_([A-Za-z0-9]+)", text[i:])
            if m:
                runs.append(run_xml(m.group(1), subscript=True))
                i += len(m.group(0))
                continue
        buf.append(text[i])
        i += 1
    flush()
    return "".join(runs)


def direct_run_xml(
    text: str,
    *,
    size_half_points: int,
    east: str,
    ascii_font: str = "Times New Roman",
    bold: bool = False,
    superscript: bool = False,
    subscript: bool = False,
) -> str:
    if text == "":
        return ""
    rpr = [
        f'<w:rFonts w:ascii="{ascii_font}" w:hAnsi="{ascii_font}" w:eastAsia="{east}" w:cs="{ascii_font}"/>',
        f'<w:sz w:val="{size_half_points}"/><w:szCs w:val="{size_half_points}"/>',
    ]
    if bold:
        rpr.append("<w:b/><w:bCs/>")
    if superscript:
        rpr.append('<w:vertAlign w:val="superscript"/>')
    elif subscript:
        rpr.append('<w:vertAlign w:val="subscript"/>')
    return f'<w:r><w:rPr>{"".join(rpr)}</w:rPr><w:t xml:space="preserve">{xml(text)}</w:t></w:r>'


def table_inline_runs_xml(text: str, *, bold: bool = False) -> str:
    runs: list[str] = []
    buf: list[str] = []
    i = 0

    def emit(value: str, superscript: bool = False, subscript: bool = False) -> None:
        runs.append(
            direct_run_xml(
                value,
                size_half_points=15,
                east="宋体",
                bold=bold,
                superscript=superscript,
                subscript=subscript,
            )
        )

    def flush() -> None:
        if buf:
            emit("".join(buf))
            buf.clear()

    while i < len(text):
        citation = CITATION_RE.match(text, i)
        if citation:
            flush()
            emit(citation.group(0), superscript=True)
            i = citation.end()
            continue
        if text[i] == "^" and i + 1 < len(text):
            flush()
            if text[i + 1] == "{":
                end = text.find("}", i + 2)
                if end > i + 2:
                    emit(text[i + 2 : end], superscript=True)
                    i = end + 1
                    continue
            m = re.match(r"\^([A-Za-z0-9*]+)", text[i:])
            if m:
                emit(m.group(1), superscript=True)
                i += len(m.group(0))
                continue
        if text[i] == "_" and i + 1 < len(text):
            flush()
            if text[i + 1] == "{":
                end = text.find("}", i + 2)
                if end > i + 2:
                    emit(text[i + 2 : end], subscript=True)
                    i = end + 1
                    continue
            m = re.match(r"_([A-Za-z0-9]+)", text[i:])
            if m:
                emit(m.group(1), subscript=True)
                i += len(m.group(0))
                continue
        buf.append(text[i])
        i += 1
    flush()
    return "".join(runs)


def algorithm_inline_runs_xml(text: str, *, bold: bool = False) -> str:
    runs: list[str] = []
    buf: list[str] = []
    i = 0

    def emit(value: str, superscript: bool = False, subscript: bool = False, is_bold: bool | None = None) -> None:
        runs.append(
            direct_run_xml(
                value,
                size_half_points=18,
                east="宋体",
                bold=bold if is_bold is None else is_bold,
                superscript=superscript,
                subscript=subscript,
            )
        )

    def flush() -> None:
        if buf:
            emit("".join(buf))
            buf.clear()

    while i < len(text):
        if text[i] == "^" and i + 1 < len(text):
            flush()
            if text[i + 1] == "{":
                end = text.find("}", i + 2)
                if end > i + 2:
                    emit(text[i + 2 : end], superscript=True)
                    i = end + 1
                    continue
            m = re.match(r"\^([A-Za-z0-9*]+)", text[i:])
            if m:
                emit(m.group(1), superscript=True)
                i += len(m.group(0))
                continue
        if text[i] == "_" and i + 1 < len(text):
            flush()
            if text[i + 1] == "{":
                end = text.find("}", i + 2)
                if end > i + 2:
                    emit(text[i + 2 : end], subscript=True)
                    i = end + 1
                    continue
            m = re.match(r"_([A-Za-z0-9]+)", text[i:])
            if m:
                emit(m.group(1), subscript=True)
                i += len(m.group(0))
                continue
        buf.append(text[i])
        i += 1
    flush()
    return "".join(runs)


def algorithm_code_runs_xml(text: str) -> str:
    if text.startswith("foreach ") and text.endswith(" do"):
        body = text[len("foreach ") : -len(" do")]
        return (
            algorithm_inline_runs_xml("foreach", bold=True)
            + algorithm_inline_runs_xml(f" {body} ")
            + algorithm_inline_runs_xml("do", bold=True)
        )
    if text.startswith("if ") and text.endswith(" then"):
        body = text[len("if ") : -len(" then")]
        return (
            algorithm_inline_runs_xml("if", bold=True)
            + algorithm_inline_runs_xml(f" {body} ")
            + algorithm_inline_runs_xml("then", bold=True)
        )
    if text.startswith("return "):
        return algorithm_inline_runs_xml("return", bold=True) + algorithm_inline_runs_xml(text[len("return") :])
    return algorithm_inline_runs_xml(text)


def algorithm_title_runs_xml(caption: str) -> str:
    match = re.match(r"算法\s*(\d+)\s*(.+)", caption)
    title = f"Algorithm {match.group(1)}: " if match else ""
    body = match.group(2).strip() if match else caption
    return algorithm_inline_runs_xml(title, bold=True) + algorithm_inline_runs_xml(body)


def clean_formula_display_text(text: str) -> str:
    if not ("bigl" in text or "bigr" in text or re.search(r"\bd_(?:0|n|max)\b", text)):
        return text
    text = text.replace("minbigl", "min")
    text = text.replace("bigl", "").replace("bigr", "")
    text = re.sub(r"\s*([=+])\s*", r" \1 ", text)
    text = re.sub(r"\s*,\s*", ", ", text)
    text = re.sub(r"\(\s+", "(", text)
    text = re.sub(r"\s+\)", ")", text)
    text = re.sub(r"\s{2,}", " ", text)
    text = text.replace(" .", ".")
    return text.strip()


def fix_display_text(text: str) -> str:
    text = re.sub(r"\btheta\b", "θ", text)
    text = text.replace("数字→id，UUID→uuid", "数字→{id}，UUID→{uuid}")
    text = text.replace("映射为 id，UUID 或长哈希段映射为 uuid", "映射为 {id}，UUID 或长哈希段映射为 {uuid}")
    text = text.replace("L=l_1,…,l_N", "L={l_1,…,l_N}")
    text = text.replace("A=(p_i,w_i)", "A={(p_i,w_i)}")
    return text


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def strip_comments(text: str) -> str:
    lines: list[str] = []
    for line in text.splitlines():
        if line.lstrip().startswith("%"):
            continue
        line = re.sub(r"(?<!\\)%.*$", "", line)
        lines.append(line.rstrip())
    return "\n".join(lines)


def find_matching_brace(text: str, open_index: int) -> int:
    depth = 0
    i = open_index
    while i < len(text):
        ch = text[i]
        prev = text[i - 1] if i else ""
        if ch == "{" and prev != "\\":
            depth += 1
        elif ch == "}" and prev != "\\":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    raise ValueError(f"Unmatched brace at offset {open_index}")


def command_arg(text: str, command: str, start: int = 0) -> tuple[str, int, int] | None:
    token = f"\\{command}"
    pos = text.find(token, start)
    if pos < 0:
        return None
    brace = text.find("{", pos + len(token))
    if brace < 0:
        return None
    end = find_matching_brace(text, brace)
    return text[brace + 1 : end], pos, end + 1


def replace_command_arg(text: str, command: str, repl) -> str:
    start = 0
    token = f"\\{command}"
    while True:
        pos = text.find(token, start)
        if pos < 0:
            break
        brace = text.find("{", pos + len(token))
        if brace < 0:
            start = pos + len(token)
            continue
        try:
            end = find_matching_brace(text, brace)
        except ValueError:
            break
        inner = text[brace + 1 : end]
        text = text[:pos] + repl(inner) + text[end + 1 :]
        start = pos
    return text


def parse_newcommands(tex: str) -> dict[str, str]:
    macros: dict[str, str] = {}
    block_pattern = r"\\newcommand\{\\(\w+)\}\{%\n(.*?)%\n\}"
    for match in re.finditer(block_pattern, tex, re.DOTALL):
        macros[match.group(1)] = match.group(2).strip()
    line_pattern = r"\\newcommand\{\\(\w+)\}\{([^{}\n]+)\}"
    for match in re.finditer(line_pattern, tex):
        macros.setdefault(match.group(1), match.group(2).strip())
    return macros


def parse_bbl(bbl_path: Path) -> tuple[dict[str, int], list[str]]:
    if not bbl_path.exists():
        return {}, []
    raw = read_text(bbl_path)
    parts = re.split(r"\\bibitem(?:\[[^\]]*\])?\{([^}]+)\}", raw)
    key_to_num: dict[str, int] = {}
    entries: list[str] = []
    for i in range(1, len(parts) - 1, 2):
        key = parts[i].strip()
        body = parts[i + 1]
        key_to_num[key] = len(entries) + 1
        entries.append(clean_bibitem(body))
    return key_to_num, entries


def clean_bibitem(text: str) -> str:
    text = re.sub(r"\\begin\{thebibliography\}\{[^}]*\}", "", text)
    text = re.sub(r"\\end\{thebibliography\}", "", text)
    text = text.replace("\\newblock", " ")
    text = re.sub(r"\{\\em\s+([^{}]+)\}", r"\1", text)
    return latex_to_text(text, {}, {})


def compress_numbers(numbers: list[int]) -> str:
    if not numbers:
        return ""
    numbers = sorted(dict.fromkeys(numbers))
    ranges: list[str] = []
    start = prev = numbers[0]
    for num in numbers[1:]:
        if num == prev + 1:
            prev = num
            continue
        ranges.append(f"{start}" if start == prev else f"{start}-{prev}")
        start = prev = num
    ranges.append(f"{start}" if start == prev else f"{start}-{prev}")
    return ",".join(ranges)


def clean_math(text: str) -> str:
    replacements = {
        r"\pm": "±",
        r"\%": "%",
        r"\rightarrow": "→",
        r"\leftarrow": "←",
        r"\infty": "∞",
        r"\leq": "≤",
        r"\geq": "≥",
        r"\ll": "≪",
        r"\times": "×",
        r"\cdot": "·",
        r"\emptyset": "∅",
        r"\alpha": "α",
        r"\beta": "β",
        r"\rho": "ρ",
        r"\xi": "ξ",
        r"\ldots": "…",
        r"\log": " log ",
        r"\min": "min",
        r"\max": "max",
        r"\in": "∈",
    }
    lbrace = "\uFFF0"
    rbrace = "\uFFF1"
    text = text.replace(r"\{", lbrace).replace(r"\}", rbrace)
    text = text.replace(r"\,", " ").replace("~", " ")
    text = replace_command_arg(text, "mathrm", lambda s: s)
    text = replace_command_arg(text, "textbf", lambda s: s)
    text = replace_command_arg(text, "textit", lambda s: s)
    for src, dst in replacements.items():
        text = text.replace(src, dst)
    for _ in range(6):
        text = re.sub(r"\{([^{}]*)\}", r"\1", text)
    text = re.sub(r"\\([A-Za-z]+)", r"\1", text)
    text = text.replace(lbrace, "{").replace(rbrace, "}")
    text = text.replace("_", "_")
    return re.sub(r"\s+", " ", text).strip()


def latex_to_text(text: str, cite_map: dict[str, int], label_map: dict[str, str]) -> str:
    text = strip_comments(text)
    text = text.replace("\r", "\n")
    text = re.sub(r"\\\\\s*", " ", text)
    text = text.replace(r"\,", " ")
    lbrace = "\uFFF0"
    rbrace = "\uFFF1"
    text = text.replace(r"\{", lbrace).replace(r"\}", rbrace)

    def cite_repl(match: re.Match[str]) -> str:
        keys = [k.strip() for k in match.group(1).split(",")]
        nums = [cite_map[k] for k in keys if k in cite_map]
        return f"[{compress_numbers(nums)}]" if nums else "[?]"

    text = re.sub(r"\\cite[tp]?\{([^}]+)\}", cite_repl, text)
    text = re.sub(r"\\ref\{([^}]+)\}", lambda m: label_map.get(m.group(1), "??"), text)
    text = re.sub(r"\\label\{[^}]+\}", "", text)
    text = re.sub(r"\$(.+?)\$", lambda m: clean_math(m.group(1)), text)
    text = re.sub(r"\\\((.+?)\\\)", lambda m: clean_math(m.group(1)), text)
    text = replace_command_arg(text, "footnote", lambda s: f"（注：{latex_to_text(s, cite_map, label_map)}）")
    for cmd in ["textbf", "textit", "emph", "url", "nolinkurl", "texttt", "mathrm", "rjrare"]:
        text = replace_command_arg(text, cmd, lambda s: latex_to_text(s, cite_map, label_map))
    text = re.sub(r"\\item\[\{?([^{}\]]+)\}?\]", r"\1 ", text)
    text = re.sub(r"\\item\s*", "", text)
    text = text.replace("``", "“").replace("''", "”")
    text = text.replace("---", "—").replace("--", "–")
    text = text.replace(r"\%", "%").replace(r"\&", "&").replace(r"\_", "_")
    text = text.replace(r"\#", "#").replace(r"\$", "$")
    text = text.replace("~", " ")
    text = re.sub(r"\\(xiaowuhao|wuhao|small|centering|noindent|song|kai|hei|fs|par|allowbreak)\b", "", text)
    text = re.sub(r"\\fontsize\{[^}]*\}\{[^}]*\}\\selectfont", "", text)
    text = re.sub(r"\\hspace\{[^}]*\}", " ", text)
    text = re.sub(r"\\vspace\{[^}]*\}", " ", text)
    text = re.sub(r"\\[A-Za-z]+\*?(?:\[[^\]]*\])?", "", text)
    text = text.replace("{", "").replace("}", "")
    text = text.replace(lbrace, "{").replace(rbrace, "}")
    text = re.sub(r"[ \t]+", " ", text)
    text = re.sub(r"\s*\n\s*", " ", text)
    return text.strip()


def expand_inputs(tex: str, base_dir: Path) -> str:
    def repl(match: re.Match[str]) -> str:
        name = match.group(1)
        rel = Path(name if name.endswith(".tex") else f"{name}.tex")
        path = base_dir / rel
        if not path.exists():
            return ""
        return expand_inputs(read_text(path), path.parent)

    return re.sub(r"\\input\{([^}]+)\}", repl, tex)


def find_envs(text: str, env: str) -> Iterable[str]:
    pattern = re.compile(rf"\\begin\{{{env}\}}(?:\[[^\]]*\])?(.*?)\\end\{{{env}\}}", re.DOTALL)
    for match in pattern.finditer(text):
        yield match.group(1)


def collect_labels(section_texts: list[str]) -> dict[str, str]:
    joined = "\n".join(section_texts)
    labels: dict[str, str] = {}
    for env, prefix, fmt in [
        ("table", "tab", "{}"),
        ("figure", "fig", "{}"),
        ("algorithm", "alg", "{}"),
        ("equation", "eq", "({})"),
    ]:
        count = 0
        for body in find_envs(joined, env):
            count += 1
            label_match = re.search(r"\\label\{([^}]+)\}", body)
            if label_match:
                label = label_match.group(1)
                labels[label] = fmt.format(count)
                if ":" not in label:
                    labels[f"{prefix}:{label}"] = fmt.format(count)
    return labels


def extract_tabular(env_text: str) -> str:
    begin = re.search(r"\\begin\{tabular\*?\}", env_text)
    if not begin:
        return ""
    pos = begin.end()
    while pos < len(env_text) and env_text[pos].isspace():
        pos += 1
    if begin.group(0).endswith("*}") and pos < len(env_text) and env_text[pos] == "{":
        pos = find_matching_brace(env_text, pos) + 1
        while pos < len(env_text) and env_text[pos].isspace():
            pos += 1
    if pos >= len(env_text) or env_text[pos] != "{":
        return ""
    spec_end = find_matching_brace(env_text, pos)
    end = env_text.find(r"\end{tabular*}" if begin.group(0).endswith("*}") else r"\end{tabular}", spec_end)
    if end < 0:
        return ""
    return env_text[spec_end + 1 : end]


def split_cells(row: str) -> list[str]:
    cells: list[str] = []
    depth = 0
    start = 0
    for i, ch in enumerate(row):
        prev = row[i - 1] if i else ""
        if ch == "{" and prev != "\\":
            depth += 1
        elif ch == "}" and prev != "\\" and depth:
            depth -= 1
        elif ch == "&" and depth == 0:
            cells.append(row[start:i])
            start = i + 1
    cells.append(row[start:])
    return cells


def parse_table(env_text: str, cite_map: dict[str, int], label_map: dict[str, str]) -> Block:
    caption_arg = command_arg(env_text, "caption")
    caption = latex_to_text(caption_arg[0], cite_map, label_map) if caption_arg else ""
    label_match = re.search(r"\\label\{([^}]+)\}", env_text)
    label = label_match.group(1) if label_match else ""
    tabular = extract_tabular(env_text)
    tabular = re.sub(r"\\(toprule|midrule|bottomrule|hline)\b", "", tabular)
    rows: list[list[str]] = []
    for raw in re.split(r"\\\\(?:\[[^\]]*\])?", tabular):
        raw = raw.strip()
        if not raw:
            continue
        cells = [latex_to_text(cell, cite_map, label_map) for cell in split_cells(raw)]
        if any(cells):
            rows.append(cells)
    return Block(kind="table", caption=caption, label=label, rows=rows)


def parse_figure(env_text: str, figures_dir: Path, cite_map: dict[str, int], label_map: dict[str, str]) -> Block:
    caption_arg = command_arg(env_text, "caption")
    caption = latex_to_text(caption_arg[0], cite_map, label_map) if caption_arg else ""
    label_match = re.search(r"\\label\{([^}]+)\}", env_text)
    label = label_match.group(1) if label_match else ""
    image_match = re.search(r"\\includegraphics(?:\[([^\]]*)\])?\{([^}]+)\}", env_text)
    image_path: Path | None = None
    width_factor = 0.9
    if image_match:
        options = image_match.group(1) or ""
        name = image_match.group(2)
        base = Path(name)
        if base.suffix.lower() == ".pdf":
            candidate = figures_dir / f"{base.stem}.png"
        elif base.suffix:
            candidate = figures_dir / base.name
        else:
            candidate = figures_dir / f"{base.name}.png"
        if not candidate.exists() and base.suffix.lower() == ".pdf":
            pdf = figures_dir / base.name
            maybe_convert_pdf(pdf, candidate)
        image_path = candidate if candidate.exists() else None
        width_match = re.search(r"width=([0-9.]+)\\textwidth", options)
        if width_match:
            width_factor = float(width_match.group(1))
    return Block(kind="figure", caption=caption, label=label, image_path=image_path, width_factor=width_factor)


def maybe_convert_pdf(pdf_path: Path, png_path: Path) -> None:
    if not pdf_path.exists() or png_path.exists():
        return
    if not shutil.which("pdftoppm"):
        return
    prefix = str(png_path.with_suffix(""))
    subprocess.run(
        ["pdftoppm", "-png", "-singlefile", "-r", "220", str(pdf_path), prefix],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def command_args_at(text: str, pos: int, command: str, argc: int) -> tuple[list[str], int] | None:
    token = f"\\{command}"
    if not text.startswith(token, pos):
        return None
    args: list[str] = []
    cur = pos + len(token)
    for _ in range(argc):
        while cur < len(text) and text[cur].isspace():
            cur += 1
        if cur >= len(text) or text[cur] != "{":
            return None
        end = find_matching_brace(text, cur)
        args.append(text[cur + 1 : end])
        cur = end + 1
    return args, cur


def strip_algorithm_metadata(env_text: str) -> str:
    text = env_text
    for command in ["caption", "label", "KwIn", "KwOut"]:
        start = 0
        while True:
            arg = command_arg(text, command, start)
            if not arg:
                break
            _, begin, end = arg
            text = text[:begin] + "\n" + text[end:]
            start = begin
    return text


def algorithm_statement_text(raw: str, cite_map: dict[str, int], label_map: dict[str, str]) -> tuple[str, str]:
    raw = raw.strip()
    has_semicolon = r"\;" in raw or r"\tcp*" in raw
    comment = ""
    tcp = command_arg(raw, "tcp*")
    if tcp:
        comment = latex_to_text(tcp[0], cite_map, label_map)
        raw = raw[: tcp[1]] + raw[tcp[2] :]
    raw = raw.replace(r"\;", " ")
    raw = normalize_inline_text(raw)
    text = latex_to_text(raw, cite_map, label_map)
    if has_semicolon and text and not text.endswith(";"):
        text += ";"
    return text, comment


def parse_algorithm_rows(
    source: str,
    cite_map: dict[str, int],
    label_map: dict[str, str],
    *,
    pos: int = 0,
    indent: int = 0,
    active_guides: tuple[int, ...] = (),
    counter: list[int] | None = None,
) -> tuple[list[dict[str, object]], int]:
    if counter is None:
        counter = [0]
    rows: list[dict[str, object]] = []
    cur = pos
    statement_start = cur

    def emit_statement(raw: str) -> None:
        text, comment = algorithm_statement_text(raw, cite_map, label_map)
        if not text:
            return
        counter[0] += 1
        rows.append(
            {
                "line_no": counter[0],
                "indent": indent,
                "guides": list(active_guides),
                "end_guides": [],
                "code": text,
                "comment": comment,
            }
        )

    while cur < len(source):
        if source.startswith(r"\tcp*", cur):
            parsed = command_args_at(source, cur, "tcp*", 1)
            if parsed:
                _, end = parsed
                emit_statement(source[statement_start:end])
                cur = end
                statement_start = cur
                continue
        if source.startswith(r"\ForEach", cur) or source.startswith(r"\If", cur):
            emit_statement(source[statement_start:cur])
            command = "ForEach" if source.startswith(r"\ForEach", cur) else "If"
            parsed = command_args_at(source, cur, command, 2)
            if not parsed:
                cur += 1
                continue
            args, end = parsed
            condition = latex_to_text(args[0], cite_map, label_map)
            code = f"foreach {condition} do" if command == "ForEach" else f"if {condition} then"
            counter[0] += 1
            rows.append(
                {
                    "line_no": counter[0],
                    "indent": indent,
                    "guides": list(active_guides),
                    "end_guides": [],
                    "code": code,
                    "comment": "",
                    "keyword": command,
                }
            )
            child_rows, _ = parse_algorithm_rows(
                args[1],
                cite_map,
                label_map,
                indent=indent + 1,
                active_guides=active_guides + (indent,),
                counter=counter,
            )
            if child_rows:
                child_rows[-1].setdefault("end_guides", [])
                child_rows[-1]["end_guides"].append(indent)
            rows.extend(child_rows)
            cur = end
            statement_start = cur
            continue
        if source.startswith(r"\Return", cur):
            emit_statement(source[statement_start:cur])
            parsed = command_args_at(source, cur, "Return", 1)
            if parsed:
                args, end = parsed
                tail = source[end : source.find("\n", end) if "\n" in source[end:] else len(source)]
                has_semicolon = r"\;" in tail
                text = f"return {latex_to_text(args[0], cite_map, label_map)}"
                if has_semicolon:
                    text += ";"
                    end += tail.find(r"\;") + 2
                counter[0] += 1
                rows.append(
                    {
                        "line_no": counter[0],
                        "indent": indent,
                        "guides": list(active_guides),
                        "end_guides": [],
                        "code": text,
                        "comment": "",
                        "keyword": "Return",
                    }
                )
                cur = end
                statement_start = cur
                continue
        if source.startswith(r"\;", cur):
            emit_statement(source[statement_start : cur + 2])
            cur += 2
            statement_start = cur
            continue
        cur += 1
    emit_statement(source[statement_start:])
    return rows, cur


def parse_algorithm(env_text: str, cite_map: dict[str, int], label_map: dict[str, str]) -> Block:
    caption_arg = command_arg(env_text, "caption")
    caption = latex_to_text(caption_arg[0], cite_map, label_map) if caption_arg else "算法"
    label_match = re.search(r"\\label\{([^}]+)\}", env_text)
    label = label_match.group(1) if label_match else ""
    algorithm_io: list[tuple[str, str]] = []
    for command, label_text in [("KwIn", "Input"), ("KwOut", "Output")]:
        arg = command_arg(env_text, command)
        if arg:
            algorithm_io.append((label_text, latex_to_text(arg[0], cite_map, label_map)))
    body = strip_algorithm_metadata(env_text)
    rows, _ = parse_algorithm_rows(body, cite_map, label_map)
    legacy_lines = [
        *(f"{label}: {text}" for label, text in algorithm_io),
        *(str(row["code"]) for row in rows),
    ]
    return Block(
        kind="algorithm",
        caption=caption,
        label=label,
        lines=legacy_lines,
        algorithm_io=algorithm_io,
        algorithm_rows=rows,
    )


def parse_equation(env_text: str, label_map: dict[str, str]) -> Block:
    label_match = re.search(r"\\label\{([^}]+)\}", env_text)
    label = label_match.group(1) if label_match else ""
    content = re.sub(r"\\label\{[^}]+\}", "", env_text).strip()
    eq_no = label_map.get(label, "")
    return Block(kind="equation", text=clean_math(content), label=label, caption=eq_no)


def add_text_blocks(blocks: list[Block], chunk: str, cite_map: dict[str, int], label_map: dict[str, str]) -> None:
    chunk = re.sub(r"\\vspace\{[^}]+\}", "\n\n", chunk)
    chunk = re.sub(r"\\noindent", "", chunk)
    for paragraph in re.split(r"\n\s*\n", chunk):
        paragraph = paragraph.strip()
        if not paragraph:
            continue
        cleaned = latex_to_text(paragraph, cite_map, label_map)
        if cleaned:
            blocks.append(Block(kind="paragraph", text=cleaned))


def parse_enumerate(env_text: str, cite_map: dict[str, int], label_map: dict[str, str]) -> list[Block]:
    result: list[Block] = []
    parts = re.split(r"\\item\s*", env_text)
    for idx, item in enumerate(parts[1:], 1):
        text = latex_to_text(item, cite_map, label_map)
        if text:
            result.append(Block(kind="list_item", text=f"{idx}. {text}"))
    return result


def parse_sections(root: Path, cite_map: dict[str, int], label_map: dict[str, str]) -> list[Block]:
    blocks: list[Block] = []
    section_no = 0
    subsection_no = 0
    subsubsection_no = 0
    figure_no = table_no = algorithm_no = equation_no = 0
    token_re = re.compile(
        r"\\(section|subsection|subsubsection)\{"
        r"|\\begin\{(table|figure|algorithm|equation|enumerate|center)\}(?:\[[^\]]*\])?",
        re.DOTALL,
    )
    for rel in SECTION_FILES:
        text = strip_comments(read_text(root / rel))
        pos = 0
        while True:
            match = token_re.search(text, pos)
            if not match:
                add_text_blocks(blocks, text[pos:], cite_map, label_map)
                break
            add_text_blocks(blocks, text[pos : match.start()], cite_map, label_map)
            if match.group(1):
                cmd = match.group(1)
                brace = match.end() - 1
                end = find_matching_brace(text, brace)
                title = latex_to_text(text[brace + 1 : end], cite_map, label_map)
                if cmd == "section":
                    section_no += 1
                    subsection_no = 0
                    subsubsection_no = 0
                    blocks.append(Block(kind="heading", level=1, text=f"{section_no} {title}"))
                elif cmd == "subsection":
                    subsection_no += 1
                    subsubsection_no = 0
                    blocks.append(Block(kind="heading", level=2, text=f"{section_no}.{subsection_no} {title}"))
                else:
                    subsubsection_no += 1
                    blocks.append(
                        Block(kind="heading", level=3, text=f"{section_no}.{subsection_no}.{subsubsection_no} {title}")
                    )
                pos = end + 1
                continue

            env = match.group(2)
            end_token = rf"\end{{{env}}}"
            end = text.find(end_token, match.end())
            if end < 0:
                add_text_blocks(blocks, text[match.start() :], cite_map, label_map)
                break
            env_body = text[match.end() : end]
            if env == "table":
                table_no += 1
                block = parse_table(env_body, cite_map, label_map)
                block.caption = f"表 {table_no}  {block.caption}" if block.caption else f"表 {table_no}"
                blocks.append(block)
            elif env == "figure":
                figure_no += 1
                block = parse_figure(env_body, root / "figures", cite_map, label_map)
                block.caption = f"图 {figure_no}  {block.caption}" if block.caption else f"图 {figure_no}"
                blocks.append(block)
            elif env == "algorithm":
                algorithm_no += 1
                block = parse_algorithm(env_body, cite_map, label_map)
                block.caption = f"算法 {algorithm_no}  {block.caption}" if block.caption else f"算法 {algorithm_no}"
                blocks.append(block)
            elif env == "equation":
                equation_no += 1
                block = parse_equation(env_body, label_map)
                block.caption = f"({equation_no})"
                blocks.append(block)
            elif env == "enumerate":
                blocks.extend(parse_enumerate(env_body, cite_map, label_map))
            elif env == "center" and r"\begin{tabular}" in env_body:
                blocks.append(parse_table(env_body, cite_map, label_map))
            else:
                add_text_blocks(blocks, env_body, cite_map, label_map)
            pos = end + len(end_token)
    return blocks


def extract_cn_references(main_tex: str, cite_map: dict[str, int], label_map: dict[str, str]) -> list[str]:
    match = re.search(r"\\begin\{description\}.*?(.*?)\\end\{description\}", main_tex, re.DOTALL)
    if not match:
        return []
    body = match.group(1)
    item_match = re.search(r"\\item(?=\s|\[|\{|$)", body)
    if item_match is None:
        return []
    body = body[item_match.start():]
    refs: list[str] = []
    for item in re.split(r"\\item(?=\s|\[|\{|$)", body):
        item = item.strip()
        if not item:
            continue
        text = latex_to_text(r"\item" + item, cite_map, label_map)
        text = re.sub(r"\[(\d+)\s+\]", r"[\1]", text)
        if text:
            refs.append(text)
    return refs


def extract_author_bio(main_tex: str, cite_map: dict[str, int], label_map: dict[str, str]) -> list[str]:
    match = re.search(r"\\begin\{list\}.*?(.*?)\\end\{list\}", main_tex, re.DOTALL)
    if not match:
        return []
    body = match.group(1)
    item_match = re.search(r"\\item(?=\s|\[|\{|$)", body)
    if item_match is None:
        return []
    body = body[item_match.start():]
    bios: list[str] = []
    for item in re.split(r"\\item(?=\s|\[|\{|$)", body):
        item = item.strip()
        if not item:
            continue
        text = latex_to_text(r"\item" + item, cite_map, label_map)
        if text:
            bios.append(text)
    return bios


def extract_line_with(main_tex: str, needle: str, cite_map: dict[str, int], label_map: dict[str, str]) -> str:
    for line in main_tex.splitlines():
        if needle in line:
            return latex_to_text(line, cite_map, label_map)
    return ""


def normalize_inline_text(text: str) -> str:
    text = text.replace("\n", " ")
    text = text.replace("~", " ")
    text = re.sub(r"\s+", " ", text)
    return text.strip()


def normalize_institute_line(text: str) -> str:
    text = normalize_inline_text(text)
    text = re.sub(r"([\u4e00-\u9fff])\s+([\u4e00-\u9fff]+)\s+([0-9]{6})", r"\1\2 \3", text)
    return text


def extract_command_text(tex: str, command: str, cite_map: dict[str, int], label_map: dict[str, str]) -> str:
    value = command_arg(tex, command)
    if not value:
        return ""
    return normalize_inline_text(latex_to_text(value[0], cite_map, label_map))


def extract_footnote_text(tex: str, cite_map: dict[str, int], label_map: dict[str, str]) -> str:
    value = command_arg(tex, "footnotetext")
    if not value:
        return ""
    text = re.sub(r"\\(?:xiaowuhao|song|kai|hei)\b(?:\{\})?", "", value[0])
    return normalize_inline_text(latex_to_text(text, cite_map, label_map))


def extract_english_front_matter(tex: str, cite_map: dict[str, int], label_map: dict[str, str]) -> tuple[str, str, str]:
    marker = "% ---------- 英文标题/作者/机构"
    start = tex.find(marker)
    block = tex[start:] if start >= 0 else tex
    next_marker = block.find("% ---------- 英文摘要")
    if next_marker >= 0:
        block = block[:next_marker]

    title_arg = command_arg(block, "textbf")
    title = latex_to_text(title_arg[0], cite_map, label_map) if title_arg else ""

    authors = ""
    author_match = re.search(r"\\vspace\{[^}]+\}\s*\{\\xiaowuhao\s*(.*?)\}", block, re.DOTALL)
    if author_match:
        authors = latex_to_text(author_match.group(1), cite_map, label_map)

    institute = ""
    institute_match = re.search(r"\([^()]*?(?:China|中国)[^()]*?\)", block, re.DOTALL)
    if institute_match:
        institute = latex_to_text(institute_match.group(0), cite_map, label_map)

    return tuple(normalize_inline_text(x) for x in (title, authors, institute))


def first_author_name(authors: str) -> str:
    first = re.split(r"[,，;；]", authors, maxsplit=1)[0]
    return re.sub(r"\s+", "", first).strip()


def derived_running_header(ms: Manuscript) -> str:
    author = first_author_name(ms.authors_zh)
    return f"{author} 等: {ms.title_zh}" if author and ms.title_zh else ms.title_zh


def build_manuscript(root: Path) -> Manuscript:
    main_tex = read_text(root / "latex/main-jos.tex")
    expanded = expand_inputs(main_tex, root / "latex")
    macros = parse_newcommands(read_text(root / "latex/sections/zh/00_abstract.tex"))
    cite_map, references = parse_bbl(root / "latex/main-jos.bbl")
    section_texts = [strip_comments(read_text(root / rel)) for rel in SECTION_FILES]
    label_map = collect_labels(section_texts)

    title = command_arg(main_tex, "rjtitle")
    authors = command_arg(main_tex, "rjauthor")
    info = command_arg(main_tex, "rjinfor")
    institute_lines = []
    if info:
        institute_lines = [
            normalize_institute_line(latex_to_text(x, cite_map, label_map))
            for x in re.split(r"\\\\", info[0])
            if x.strip()
        ]

    title_en, authors_en, institute_en = extract_english_front_matter(main_tex, cite_map, label_map)
    running_header = extract_command_text(main_tex, "rjhead", cite_map, label_map)

    return Manuscript(
        title_zh=latex_to_text(title[0], cite_map, label_map) if title else "",
        authors_zh=latex_to_text(authors[0], cite_map, label_map) if authors else "",
        institute_lines=institute_lines,
        abstract_zh=latex_to_text(macros.get("AbstractContentZh", ""), cite_map, label_map),
        keywords_zh=latex_to_text(macros.get("KeywordsZh", ""), cite_map, label_map),
        category=extract_command_text(main_tex, "rjcategory", cite_map, label_map),
        citation_zh=extract_line_with(main_tex, "中文引用格式", cite_map, label_map),
        citation_en=extract_line_with(main_tex, "英文引用格式", cite_map, label_map),
        title_en=title_en,
        authors_en=authors_en,
        institute_en=institute_en,
        abstract_en=latex_to_text(macros.get("AbstractContentEn", ""), cite_map, label_map),
        keywords_en=latex_to_text(macros.get("KeywordsEn", ""), cite_map, label_map),
        running_header=running_header,
        first_footer_text=extract_footnote_text(main_tex, cite_map, label_map),
        blocks=parse_sections(root, cite_map, label_map),
        references=references,
        cn_references=extract_cn_references(expanded, cite_map, label_map),
        author_bio=extract_author_bio(expanded, cite_map, label_map),
    )


class DocxBuilder:
    def __init__(self, format_data: dict, title: str, profile: DocxProfile = JOS_PROFILE):
        page = format_data["page_setup"]
        self.paper = page["paper_twips"]
        self.margins = page["margins_twips"]
        self.columns = page.get("columns", {"space": "720", "num": "1"})
        self.text_width_cm = page["paper_cm"]["width"] - page["margins_cm"]["left"] - page["margins_cm"]["right"]
        self.title = title
        self.profile = profile
        self.parts: list[str] = []
        self.image_rels: list[tuple[str, str]] = []
        self.media: list[tuple[Path, str]] = []
        self.next_rid = 20
        self.next_docpr = 1

    @property
    def text_width_twips(self) -> int:
        return int(self.paper["w"]) - int(self.margins["left"]) - int(self.margins["right"])

    def add_paragraph(self, text: str, style: str = "JOSBody", align: str | None = None) -> None:
        if not text:
            return
        text = fix_display_text(text)
        ppr = [f'<w:pStyle w:val="{style}"/>']
        if align:
            ppr.append(f'<w:jc w:val="{align}"/>')
        enable_superscript = not style.startswith("JOSReference")
        enable_subscript = style == "JOSCode" or bool(re.search(r"\b[A-Za-z]_[A-Za-z0-9]+", text))
        if style == "JOSCode":
            text = clean_formula_display_text(text)
        self.parts.append(
            "<w:p><w:pPr>"
            + "".join(ppr)
            + "</w:pPr>"
            + inline_runs_xml(
                text,
                enable_superscript=enable_superscript,
                enable_subscript=enable_subscript,
            )
            + "</w:p>"
        )

    def add_kept_paragraph(
        self,
        text: str,
        style: str = "JOSBody",
        align: str | None = None,
        *,
        keep_next: bool = False,
        keep_lines: bool = False,
    ) -> None:
        if not text:
            return
        text = fix_display_text(text)
        ppr = [f'<w:pStyle w:val="{style}"/>']
        if keep_next:
            ppr.append("<w:keepNext/>")
        if keep_lines:
            ppr.append("<w:keepLines/>")
        if align:
            ppr.append(f'<w:jc w:val="{align}"/>')
        enable_superscript = not style.startswith("JOSReference")
        enable_subscript = style == "JOSCode" or bool(re.search(r"\b[A-Za-z]_[A-Za-z0-9]+", text))
        if style == "JOSCode":
            text = clean_formula_display_text(text)
        self.parts.append(
            "<w:p><w:pPr>"
            + "".join(ppr)
            + "</w:pPr>"
            + inline_runs_xml(
                text,
                enable_superscript=enable_superscript,
                enable_subscript=enable_subscript,
            )
            + "</w:p>"
        )

    def add_tabbed_paragraph(self, left: str, right: str, style: str = "JOSMasthead") -> None:
        self.parts.append(
            "<w:p><w:pPr>"
            f'<w:pStyle w:val="{style}"/>'
            f'<w:tabs><w:tab w:val="right" w:pos="{self.text_width_twips}"/></w:tabs>'
            "</w:pPr>"
            f'<w:r><w:t xml:space="preserve">{xml(left)}</w:t></w:r>'
            "<w:r><w:tab/></w:r>"
            f'<w:r><w:t xml:space="preserve">{xml(right)}</w:t></w:r>'
            "</w:p>"
        )

    def add_spacer(self, height_twips: int) -> None:
        self.parts.append(f'<w:p><w:pPr><w:spacing w:line="{height_twips}" w:lineRule="exact"/></w:pPr></w:p>')

    def add_table(self, rows: list[list[str]]) -> None:
        if not rows:
            return
        max_cols = max(len(r) for r in rows)
        cell_width = max(1, int(self.text_width_twips / max_cols))
        table_rows = []
        row_count = len(rows)
        for row_idx, row in enumerate(rows):
            cells = []
            for cell in row + [""] * (max_cols - len(row)):
                cell = fix_display_text(cell)
                ppr = '<w:pStyle w:val="JOSTableText"/><w:keepLines/>'
                if row_idx < row_count - 1:
                    ppr += "<w:keepNext/>"
                cells.append(
                    f'<w:tc><w:tcPr><w:tcW w:w="{cell_width}" w:type="dxa"/></w:tcPr>'
                    f"<w:p><w:pPr>{ppr}</w:pPr>"
                    f"{table_inline_runs_xml(cell, bold=row_idx == 0)}</w:p></w:tc>"
                )
            table_rows.append("<w:tr><w:trPr><w:cantSplit/></w:trPr>" + "".join(cells) + "</w:tr>")
        self.parts.append(
            '<w:tbl><w:tblPr><w:tblW w:w="5000" w:type="pct"/>'
            '<w:tblBorders><w:top w:val="single" w:sz="6" w:space="0" w:color="000000"/>'
            '<w:left w:val="nil"/><w:bottom w:val="single" w:sz="6" w:space="0" w:color="000000"/>'
            '<w:right w:val="nil"/><w:insideH w:val="single" w:sz="3" w:space="0" w:color="666666"/>'
            '<w:insideV w:val="single" w:sz="3" w:space="0" w:color="666666"/></w:tblBorders></w:tblPr>'
            + "".join(table_rows)
            + "</w:tbl>"
        )

    def add_algorithm_table(
        self,
        caption: str,
        io_lines: list[tuple[str, str]],
        rows: list[dict[str, object]],
    ) -> None:
        if not rows:
            self.add_kept_paragraph(caption, "JOSCaption", "center", keep_next=True, keep_lines=True)
            return
        max_depth = max((int(row.get("indent", 0)) for row in rows), default=0)
        line_width = 560
        guide_width = 280
        comment_width = 3050
        code_width = max(2200, self.text_width_twips - line_width - guide_width * max_depth - comment_width)
        widths = [line_width] + [guide_width] * max_depth + [code_width, comment_width]
        total_cols = len(widths)

        def border_xml(*, top: bool = False, bottom: bool = False, left: bool = False) -> str:
            parts = []
            if top:
                parts.append('<w:top w:val="single" w:sz="8" w:space="0" w:color="000000"/>')
            if bottom:
                parts.append('<w:bottom w:val="single" w:sz="8" w:space="0" w:color="000000"/>')
            if left:
                parts.append('<w:left w:val="single" w:sz="4" w:space="0" w:color="000000"/>')
            return "<w:tcBorders>" + "".join(parts) + "</w:tcBorders>" if parts else ""

        def cell(
            runs: str = "",
            *,
            width: int,
            grid_span: int = 1,
            align: str | None = None,
            borders: str = "",
            keep_next: bool = True,
            keep_lines: bool = True,
            margins: str = "",
            no_wrap: bool = False,
        ) -> str:
            tc_pr = [f'<w:tcW w:w="{width}" w:type="dxa"/>']
            if grid_span > 1:
                tc_pr.append(f'<w:gridSpan w:val="{grid_span}"/>')
            if no_wrap:
                tc_pr.append("<w:noWrap/>")
            if borders:
                tc_pr.append(borders)
            if margins:
                tc_pr.append(margins)
            ppr = []
            if keep_next:
                ppr.append("<w:keepNext/>")
            if keep_lines:
                ppr.append("<w:keepLines/>")
            if align:
                ppr.append(f'<w:jc w:val="{align}"/>')
            return (
                "<w:tc><w:tcPr>"
                + "".join(tc_pr)
                + "</w:tcPr><w:p><w:pPr>"
                + "".join(ppr)
                + "</w:pPr>"
                + runs
                + "</w:p></w:tc>"
            )

        def row_xml(cells: list[str]) -> str:
            return "<w:tr><w:trPr><w:cantSplit/></w:trPr>" + "".join(cells) + "</w:tr>"

        table_rows: list[str] = []
        title_width = sum(widths)
        table_rows.append(
            row_xml(
                [
                    cell(
                        algorithm_title_runs_xml(caption),
                        width=title_width,
                        grid_span=total_cols,
                        borders=border_xml(top=True, bottom=True),
                        margins='<w:tcMar><w:left w:w="80" w:type="dxa"/></w:tcMar>',
                    )
                ]
            )
        )
        for label, text in io_lines:
            table_rows.append(
                row_xml(
                    [
                        cell("", width=line_width, align="right"),
                        cell(
                            algorithm_inline_runs_xml(f"{label}: ", bold=True) + algorithm_inline_runs_xml(text),
                            width=sum(widths[1:]),
                            grid_span=total_cols - 1,
                        ),
                    ]
                )
            )

        for idx, row in enumerate(rows):
            indent = int(row.get("indent", 0))
            guides = set(int(x) for x in row.get("guides", []))
            end_guides = set(int(x) for x in row.get("end_guides", []))
            keep_next = idx < len(rows) - 1
            row_cells = [
                cell(
                    algorithm_inline_runs_xml(str(row.get("line_no", ""))),
                    width=line_width,
                    align="right",
                    keep_next=keep_next,
                    no_wrap=True,
                )
            ]
            for level in range(indent):
                row_cells.append(
                    cell(
                        "",
                        width=guide_width,
                        borders=border_xml(left=level in guides, bottom=level in end_guides),
                        keep_next=keep_next,
                    )
                )
            span = max_depth - indent + 1
            code_runs = algorithm_code_runs_xml(str(row.get("code", "")))
            row_cells.append(cell(code_runs, width=sum(widths[1 + indent : 1 + indent + span]), grid_span=span, keep_next=keep_next))
            comment = str(row.get("comment", ""))
            comment_runs = algorithm_inline_runs_xml(f"// {comment}") if comment else ""
            row_cells.append(cell(comment_runs, width=comment_width, align="right", keep_next=keep_next))
            table_rows.append(row_xml(row_cells))

        self.parts.append(
            '<w:tbl><w:tblPr><w:tblW w:w="5000" w:type="pct"/>'
            '<w:tblBorders><w:top w:val="single" w:sz="8" w:space="0" w:color="000000"/>'
            '<w:left w:val="nil"/><w:bottom w:val="single" w:sz="8" w:space="0" w:color="000000"/>'
            '<w:right w:val="nil"/><w:insideH w:val="nil"/><w:insideV w:val="nil"/></w:tblBorders>'
            '<w:tblLayout w:type="fixed"/></w:tblPr>'
            "<w:tblGrid>"
            + "".join(f'<w:gridCol w:w="{width}"/>' for width in widths)
            + "</w:tblGrid>"
            + "".join(table_rows)
            + "</w:tbl>"
        )

    def add_image(self, path: Path | None, width_factor: float, caption: str) -> None:
        if path is None or not path.exists():
            self.add_paragraph(f"[缺图] {caption}", "JOSCaption", "center")
            return
        if Image is None:
            raise RuntimeError("PIL is required to read image dimensions")
        suffix = path.suffix.lower().lstrip(".") or "png"
        media_name = f"image{len(self.media) + 1}.{suffix}"
        rid = f"rId{self.next_rid}"
        self.next_rid += 1
        self.media.append((path, f"word/media/{media_name}"))
        self.image_rels.append((rid, f"media/{media_name}"))
        with Image.open(path) as img:
            px_w, px_h = img.size
        width_cm = min(self.text_width_cm * width_factor, self.text_width_cm)
        height_cm = width_cm * px_h / max(px_w, 1)
        cx = int(width_cm * EMU_PER_CM)
        cy = int(height_cm * EMU_PER_CM)
        docpr = self.next_docpr
        self.next_docpr += 1
        drawing = f"""
<w:p><w:pPr><w:pStyle w:val="JOSImage"/><w:keepNext/><w:keepLines/><w:jc w:val="center"/></w:pPr><w:r><w:drawing>
<wp:inline distT="0" distB="0" distL="0" distR="0">
<wp:extent cx="{cx}" cy="{cy}"/><wp:effectExtent l="0" t="0" r="0" b="0"/>
<wp:docPr id="{docpr}" name="{xml(path.stem)}"/><wp:cNvGraphicFramePr/>
<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
<pic:pic><pic:nvPicPr><pic:cNvPr id="{docpr}" name="{xml(path.name)}"/><pic:cNvPicPr/></pic:nvPicPr>
<pic:blipFill><a:blip r:embed="{rid}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>
<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm>
<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic>
</a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>
"""
        self.parts.append(drawing)

    def add_section_break(self) -> None:
        self.parts.append("<w:p/>")

    def document_xml(self) -> str:
        sect = (
            '<w:sectPr><w:headerReference w:type="first" r:id="rId8"/>'
            '<w:headerReference w:type="default" r:id="rId3"/>'
            '<w:headerReference w:type="even" r:id="rId4"/>'
            '<w:footerReference w:type="first" r:id="rId5"/>'
            '<w:footerReference w:type="default" r:id="rId6"/>'
            '<w:footerReference w:type="even" r:id="rId7"/>'
            '<w:titlePg/>'
            f'<w:pgSz w:w="{self.paper["w"]}" w:h="{self.paper["h"]}"/>'
            f'<w:pgMar w:top="{self.margins["top"]}" w:right="{self.margins["right"]}" '
            f'w:bottom="{self.margins["bottom"]}" w:left="{self.margins["left"]}" '
            f'w:header="{self.margins["header"]}" w:footer="{self.profile.footer_distance_twips}" '
            f'w:gutter="{self.margins.get("gutter", "0")}"/>'
            f'<w:cols w:space="{self.columns.get("space", "720")}" w:num="{self.columns.get("num", "1")}"/>'
            "</w:sectPr>"
        )
        return (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" '
            'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" '
            'xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" '
            'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" '
            'xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">'
            "<w:body>"
            + "".join(self.parts)
            + sect
            + "</w:body></w:document>"
        )

    def rels_xml(self) -> str:
        rels = [
            '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>',
            '<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>',
            '<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>',
            '<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header2.xml"/>',
            '<Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/>',
            '<Relationship Id="rId6" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer2.xml"/>',
            '<Relationship Id="rId7" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer3.xml"/>',
            '<Relationship Id="rId8" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header0.xml"/>',
        ]
        rels.extend(
            f'<Relationship Id="{rid}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="{target}"/>'
            for rid, target in self.image_rels
        )
        return (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            + "".join(rels)
            + "</Relationships>"
        )


def style(style_id: str, name: str, size: float, east: str, ascii_font: str = "Times New Roman",
          bold: bool = False, jc: str | None = None, first_line: int | None = None,
          left: int | None = None, hanging: int | None = None, before: int | None = None,
          after: int | None = None, line: int | None = None) -> str:
    ppr = [f"<w:jc w:val=\"{jc}\"/>"] if jc else []
    spacing_attrs = []
    if before is not None:
        spacing_attrs.append(f'w:before="{before}"')
    if after is not None:
        spacing_attrs.append(f'w:after="{after}"')
    if line is not None:
        spacing_attrs.append(f'w:line="{line}" w:lineRule="exact"')
    if spacing_attrs:
        ppr.append("<w:spacing " + " ".join(spacing_attrs) + "/>")
    ind_attrs = []
    if first_line is not None:
        ind_attrs.append(f'w:firstLine="{first_line}"')
    if left is not None:
        ind_attrs.append(f'w:left="{left}"')
    if hanging is not None:
        ind_attrs.append(f'w:hanging="{hanging}"')
    if ind_attrs:
        ppr.append("<w:ind " + " ".join(ind_attrs) + "/>")
    rpr = (
        f'<w:rFonts w:ascii="{ascii_font}" w:hAnsi="{ascii_font}" w:eastAsia="{east}" w:cs="{ascii_font}"/>'
        f'<w:sz w:val="{int(size * 2)}"/><w:szCs w:val="{int(size * 2)}"/>'
        + ("<w:b/>" if bold else "")
    )
    return (
        f'<w:style w:type="paragraph" w:styleId="{style_id}"><w:name w:val="{xml(name)}"/>'
        + ("<w:pPr>" + "".join(ppr) + "</w:pPr>" if ppr else "")
        + f"<w:rPr>{rpr}</w:rPr></w:style>"
    )


def styles_xml() -> str:
    styles = [
        style("Normal", "Normal", 9, "宋体", jc="both", line=260),
        style("JOSMasthead", "JOS masthead from sample body style 4", 7.5, "宋体", line=180),
        style("JOSTitleZh", "JOS Chinese title from sample style 64", 14, "黑体", bold=True, jc="center", before=0, after=120),
        style("JOSAuthorZh", "JOS Chinese author from sample style 65", 12, "仿宋_GB2312", jc="center", before=120, after=120),
        style("JOSInstituteZh", "JOS institute from sample style 66", 8, "宋体", jc="center", line=220),
        style("JOSAbstractZh", "JOS abstract from sample style 117", 9, "楷体_GB2312", jc="both", line=240),
        style("JOSAbstractEn", "JOS English abstract from sample first page", 10, "宋体", jc="left", line=240),
        style("JOSKeywords", "JOS keywords from sample style 118", 9, "宋体", left=430, hanging=430, line=240),
        style("JOSCitation", "JOS citation from sample style 121", 9, "宋体", jc="both", line=220),
        style("JOSEnglishTitle", "JOS English title from sample style 120", 12, "黑体", bold=True, before=120, after=100),
        style("JOSBody", "JOS body from sample style 145", 9, "宋体", jc="both", first_line=420, line=260),
        style("JOSBodyNoIndent", "JOS body without first-line indent", 9, "宋体", jc="both", line=260),
        style("JOSHeading1", "JOS heading 1 from sample style 213", 10.5, "黑体", bold=True, before=160, after=160),
        style("JOSHeading2", "JOS heading 2 from sample style 215", 9, "黑体", bold=True, before=25, after=25),
        style("JOSHeading3", "JOS heading 3 from sample style 217", 9, "黑体", bold=True, before=20, after=20),
        style("JOSCaption", "JOS caption from sample figure/table captions", 9, "宋体", jc="center", after=120),
        style("JOSImage", "JOS image paragraph with automatic line height", 9, "宋体", jc="center", before=80, after=80),
        style("JOSTableText", "JOS table text", 7.5, "宋体", jc="center", line=220),
        style("JOSCode", "JOS algorithm/code text", 8, "宋体", ascii_font="Courier New", line=220),
        style("JOSReferenceHeading", "JOS reference heading from sample style 126", 9, "黑体", bold=True, before=280),
        style("JOSReference", "JOS reference text from sample style 129", 7.5, "宋体", jc="both", left=420, hanging=420, line=260),
    ]
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        + "".join(styles)
        + "</w:styles>"
    )


def content_types_xml() -> str:
    overrides = [
        ("word/document.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"),
        ("word/styles.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"),
        ("word/settings.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"),
        ("word/header0.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"),
        ("word/header1.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"),
        ("word/header2.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"),
        ("word/footer1.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"),
        ("word/footer2.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"),
        ("word/footer3.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"),
    ]
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        '<Default Extension="xml" ContentType="application/xml"/>'
        '<Default Extension="png" ContentType="image/png"/>'
        '<Default Extension="jpg" ContentType="image/jpeg"/>'
        '<Default Extension="jpeg" ContentType="image/jpeg"/>'
        + "".join(f'<Override PartName="/{name}" ContentType="{ctype}"/>' for name, ctype in overrides)
        + "</Types>"
    )


def package_rels_xml() -> str:
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>'
        "</Relationships>"
    )


def settings_xml() -> str:
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        "<w:evenAndOddHeaders/><w:characterSpacingControl w:val=\"doNotCompress\"/>"
        "</w:settings>"
    )


def page_field_xml() -> str:
    return (
        '<w:fldSimple w:instr=" PAGE ">'
        '<w:r><w:t>1</w:t></w:r>'
        "</w:fldSimple>"
    )


def header_xml(text: str, text_width_twips: int) -> str:
    runs = (
        f'<w:r><w:t xml:space="preserve">{xml(text)}</w:t></w:r>'
        "<w:r><w:tab/></w:r>"
        f"{page_field_xml()}"
    )
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        '<w:p><w:pPr><w:pStyle w:val="JOSMasthead"/>'
        f'<w:tabs><w:tab w:val="right" w:pos="{text_width_twips}"/></w:tabs>'
        "</w:pPr>"
        f"{runs}</w:p></w:hdr>"
    )


def first_header_xml(text_width_twips: int, rows: Iterable[tuple[str, str]]) -> str:
    paras = []
    for left, right in rows:
        paras.append(
            "<w:p><w:pPr>"
            '<w:pStyle w:val="JOSMasthead"/>'
            f'<w:tabs><w:tab w:val="right" w:pos="{text_width_twips}"/></w:tabs>'
            "</w:pPr>"
            f'<w:r><w:t xml:space="preserve">{xml(left)}</w:t></w:r>'
            "<w:r><w:tab/></w:r>"
            f'<w:r><w:t xml:space="preserve">{xml(right)}</w:t></w:r>'
            "</w:p>"
        )
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        + "".join(paras)
        + "</w:hdr>"
    )


def footer_xml(text: str = "", indent_twips: int = 0) -> str:
    if text:
        body = (
            '<w:p><w:pPr><w:pStyle w:val="JOSMasthead"/><w:jc w:val="left"/>'
            f'<w:ind w:left="{indent_twips}"/>'
            '<w:pBdr><w:top w:val="single" w:sz="4" w:space="1" w:color="auto"/></w:pBdr>'
            "</w:pPr>"
            f'<w:r><w:t xml:space="preserve">{xml(text)}</w:t></w:r></w:p>'
        )
    else:
        body = "<w:p/>"
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        f"{body}</w:ftr>"
    )


def write_docx(builder: DocxBuilder, output: Path, manuscript: Manuscript, profile: DocxProfile = JOS_PROFILE) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("[Content_Types].xml", content_types_xml())
        zf.writestr("_rels/.rels", package_rels_xml())
        zf.writestr("word/document.xml", builder.document_xml())
        zf.writestr("word/_rels/document.xml.rels", builder.rels_xml())
        zf.writestr("word/styles.xml", styles_xml())
        zf.writestr("word/settings.xml", settings_xml())
        running_header = manuscript.running_header or derived_running_header(manuscript)
        first_footer = manuscript.first_footer_text or profile.first_footer_text
        zf.writestr("word/header0.xml", first_header_xml(builder.text_width_twips, profile.first_header_rows))
        zf.writestr("word/header1.xml", header_xml(running_header, builder.text_width_twips))
        zf.writestr("word/header2.xml", header_xml(profile.even_header_text, builder.text_width_twips))
        zf.writestr("word/footer1.xml", footer_xml(first_footer, profile.first_footer_indent_twips))
        zf.writestr("word/footer2.xml", footer_xml())
        zf.writestr("word/footer3.xml", footer_xml())
        for src, arcname in builder.media:
            zf.write(src, arcname)


def spaced_keywords(text: str) -> str:
    return re.sub(r";\s*", "; ", text).strip()


def token_width_units(token: str) -> float:
    total = 0.0
    for ch in token:
        code = ord(ch)
        if ch.isspace():
            total += 0.35
        elif 0x4E00 <= code <= 0x9FFF:
            total += 1.0
        elif ch.isupper():
            total += 0.62
        elif ch.islower() or ch.isdigit():
            total += 0.52
        elif ch in "-/.":
            total += 0.28
        else:
            total += 0.35
    return total


def wrap_text_units(text: str, max_units: float) -> list[str]:
    tokens = re.findall(r"https?://\S+|\s+|[A-Za-z0-9]+(?:[-/][A-Za-z0-9]+)*|[\u4e00-\u9fff]|.", text)
    lines: list[str] = []
    current: list[str] = []
    width = 0.0
    for token in tokens:
        if token.isspace():
            token = " "
        token_width = token_width_units(token)
        if current and width + token_width > max_units:
            lines.append("".join(current).strip())
            current = []
            width = 0.0
            token = token.lstrip()
            token_width = token_width_units(token)
        if token or current:
            current.append(token)
            width += token_width
    if current:
        lines.append("".join(current).strip())
    return [line for line in lines if line]


def split_citation_text(text: str, max_units: float) -> list[str]:
    text = text.strip()
    return wrap_text_units(text, max_units) if text else []


def populate(builder: DocxBuilder, ms: Manuscript, profile: DocxProfile = JOS_PROFILE) -> None:
    builder.add_paragraph(ms.title_zh, "JOSTitleZh", "left")
    builder.add_paragraph(ms.authors_zh, "JOSAuthorZh", "left")
    for line in ms.institute_lines:
        builder.add_paragraph(normalize_institute_line(line), "JOSInstituteZh", "left")
    builder.add_spacer(profile.after_institute_twips)
    builder.add_paragraph(f"{profile.zh_abstract_label} {ms.abstract_zh}", "JOSAbstractZh")
    builder.add_paragraph(f"{profile.zh_keywords_label} {spaced_keywords(ms.keywords_zh)}", "JOSKeywords")
    builder.add_paragraph(f"{profile.category_label} {ms.category}", "JOSBodyNoIndent")
    builder.add_spacer(profile.before_citation_twips)
    for line in split_citation_text(ms.citation_zh, profile.citation_wrap_units):
        builder.add_paragraph(line, "JOSCitation")
    for line in split_citation_text(ms.citation_en, profile.citation_wrap_units):
        builder.add_paragraph(line, "JOSCitation")
    builder.add_spacer(profile.before_english_title_twips)
    builder.add_paragraph(ms.title_en, "JOSEnglishTitle")
    builder.add_paragraph(ms.authors_en, "JOSCitation")
    builder.add_paragraph(ms.institute_en, "JOSCitation")
    builder.add_spacer(profile.before_english_abstract_twips)
    builder.add_paragraph(f"{profile.en_abstract_label}   {ms.abstract_en}", "JOSAbstractEn")
    builder.add_paragraph(f"{profile.en_keywords_label} {spaced_keywords(ms.keywords_en)}", "JOSKeywords")

    for block in ms.blocks:
        if block.kind == "heading":
            style_id = {1: "JOSHeading1", 2: "JOSHeading2"}.get(block.level, "JOSHeading3")
            builder.add_paragraph(block.text, style_id)
        elif block.kind in {"paragraph", "list_item"}:
            builder.add_paragraph(block.text, "JOSBody")
        elif block.kind == "table":
            if block.caption:
                builder.add_kept_paragraph(block.caption, "JOSCaption", "center", keep_next=True, keep_lines=True)
            builder.add_table(block.rows)
        elif block.kind == "figure":
            builder.add_image(block.image_path, block.width_factor, block.caption)
            builder.add_paragraph(block.caption, "JOSCaption", "center")
        elif block.kind == "algorithm":
            builder.add_algorithm_table(block.caption, block.algorithm_io, block.algorithm_rows)
        elif block.kind == "equation":
            suffix = f"    {block.caption}" if block.caption else ""
            builder.add_paragraph(f"{block.text}{suffix}", "JOSCode", "center")

    builder.add_paragraph("本文撰写与实验脚本生成过程中使用了大语言模型辅助，作者对全部内容与数据负责。", "JOSBody")
    builder.add_paragraph("References", "JOSReferenceHeading")
    for idx, ref in enumerate(ms.references, 1):
        builder.add_paragraph(f"[{idx}] {ref}", "JOSReference")
    builder.add_paragraph("附中文参考文献:", "JOSReferenceHeading")
    for ref in ms.cn_references:
        builder.add_paragraph(ref, "JOSReference")
    builder.add_paragraph("作者简介", "JOSReferenceHeading")
    for bio in ms.author_bio:
        builder.add_paragraph(bio, "JOSReference")


def main() -> int:
    parser = argparse.ArgumentParser(description="Build JOS-format DOCX from latex/main-jos.tex")
    parser.add_argument("--root", default=".", help="Repository root")
    parser.add_argument("--format", default="docs/format/jos_2025_docx_format_definitions.json")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    root = Path(args.root).resolve()
    format_arg = Path(args.format)
    if format_arg.is_absolute():
        format_path = format_arg
    else:
        format_path = root / format_arg
        if not format_path.exists():
            format_path = Path.cwd() / format_arg
    output = Path(args.output)
    if not output.is_absolute():
        output = root / output
    if Image is None:
        raise SystemExit("PIL/Pillow is required for image sizing")
    with format_path.open(encoding="utf-8") as f:
        format_data = json.load(f)

    manuscript = build_manuscript(root)
    profile = JOS_PROFILE
    builder = DocxBuilder(format_data, manuscript.title_zh, profile)
    populate(builder, manuscript, profile)
    write_docx(builder, output, manuscript, profile)
    print(f"DOCX written: {output}")
    print(f"paragraph_blocks={len(builder.parts)} images={len(builder.media)} references={len(manuscript.references)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
