#!/usr/bin/env python3
"""Verify generated JOS DOCX against the source PDF and format definition."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET


NS = {
    "w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    "wp": "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
}

SECTION_FILES = [
    "latex/sections/zh/01_intro.tex",
    "latex/sections/zh/02_related.tex",
    "latex/sections/zh/03_system.tex",
    "latex/sections/zh/04_algorithms.tex",
    "latex/sections/zh/05_implementation.tex",
    "latex/sections/zh/06_experiments.tex",
    "latex/sections/zh/07_conclusion.tex",
]


def w(name: str) -> str:
    return f"{{{NS['w']}}}{name}"


def read_docx_xml(docx: Path, name: str) -> ET.Element:
    with zipfile.ZipFile(docx) as zf:
        return ET.fromstring(zf.read(name))


def docx_text(docx: Path) -> str:
    root = read_docx_xml(docx, "word/document.xml")
    return "\n".join(text_of(p) for p in root.findall(".//w:p", NS))


def text_of(el: ET.Element) -> str:
    return "".join(t.text or "" for t in el.findall(".//w:t", NS)).strip()


def wattrs(el: ET.Element | None) -> dict[str, str]:
    if el is None:
        return {}
    return {k.split("}")[-1]: v for k, v in el.attrib.items()}


def normalize(text: str) -> str:
    text = text.replace("\u00a0", " ")
    text = re.sub(r"\s+", "", text)
    text = text.replace("–", "-").replace("—", "-")
    text = text.replace("“", '"').replace("”", '"')
    return text


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def find_matching_brace(text: str, open_index: int) -> int:
    depth = 0
    for i in range(open_index, len(text)):
        ch = text[i]
        prev = text[i - 1] if i else ""
        if ch == "{" and prev != "\\":
            depth += 1
        elif ch == "}" and prev != "\\":
            depth -= 1
            if depth == 0:
                return i
    raise ValueError(f"Unmatched brace at offset {open_index}")


def command_arg(text: str, command: str) -> str:
    token = f"\\{command}"
    pos = text.find(token)
    if pos < 0:
        return ""
    brace = text.find("{", pos + len(token))
    if brace < 0:
        return ""
    try:
        end = find_matching_brace(text, brace)
    except ValueError:
        return ""
    return text[brace + 1 : end]


def tex_to_plain(text: str) -> str:
    text = re.sub(r"\\hspace\{[^}]*\}", " ", text)
    text = re.sub(r"\\(?:xiaowuhao|wuhao|song|kai|hei|textit|textbf)\b(?:\{\})?", "", text)
    text = re.sub(r"\\url\{([^}]*)\}", r"\1", text)
    text = re.sub(r"\\nolinkurl\{([^}]*)\}", r"\1", text)
    text = text.replace("~", " ")
    text = text.replace("\\%", "%").replace("\\&", "&")
    text = re.sub(r"\\[A-Za-z]+\*?(?:\[[^\]]*\])?", "", text)
    text = text.replace("{", "").replace("}", "")
    return re.sub(r"\s+", " ", text).strip()


def count_env(text: str, env: str) -> int:
    return len(re.findall(rf"\\begin\{{{re.escape(env)}\}}", text))


def citation_key_count(text: str) -> int:
    keys: set[str] = set()
    for match in re.finditer(r"\\cite[tp]?\{([^}]+)\}", text):
        keys.update(key.strip() for key in match.group(1).split(",") if key.strip())
    return len(keys)


def source_profile(tex_root: Path) -> dict[str, object]:
    main_tex = read_text(tex_root / "latex/main-jos.tex")
    section_texts = [read_text(tex_root / rel) for rel in SECTION_FILES if (tex_root / rel).exists()]
    joined_sections = "\n".join(section_texts)

    title = tex_to_plain(command_arg(main_tex, "rjtitle"))
    running_header = tex_to_plain(command_arg(main_tex, "rjhead"))
    section_markers = []
    for idx, title_match in enumerate(re.finditer(r"\\section\{([^}]*)\}", joined_sections), 1):
        section_markers.append(f"{idx} {tex_to_plain(title_match.group(1))}")

    markers = [
        title,
        "摘  要",
        "关键词",
        "Abstract",
        "Key words",
        *section_markers,
        "References",
        "附中文参考文献",
        "作者简介",
        "shihonglei0042@link.tyut.edu.cn",
        "zh_juanjuan@126.com",
    ]

    return {
        "title": title,
        "running_header": running_header,
        "markers": [m for m in markers if m],
        "figures": count_env(joined_sections, "figure"),
        "tables": count_env(joined_sections, "table"),
        "algorithms": count_env(joined_sections, "algorithm"),
        "citations": citation_key_count(main_tex + "\n" + joined_sections),
    }


def pdf_text(pdf: Path) -> str:
    result = subprocess.run(
        ["pdftotext", str(pdf), "-"],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return result.stdout


def count_paragraphs(root: ET.Element) -> int:
    return len(root.findall(".//w:body/w:p", NS))


def count_tables(root: ET.Element) -> int:
    return len(root.findall(".//w:tbl", NS))


def count_images(root: ET.Element) -> int:
    return len(root.findall(".//wp:inline", NS))


def sect_values(root: ET.Element) -> dict[str, dict[str, str]]:
    sect = root.find(".//w:sectPr", NS)
    if sect is None:
        return {}
    pg_sz = sect.find("w:pgSz", NS)
    pg_mar = sect.find("w:pgMar", NS)
    cols = sect.find("w:cols", NS)
    return {
        "paper_twips": {k.split("}")[-1]: v for k, v in (pg_sz.attrib if pg_sz is not None else {}).items()},
        "margins_twips": {k.split("}")[-1]: v for k, v in (pg_mar.attrib if pg_mar is not None else {}).items()},
        "columns": {k.split("}")[-1]: v for k, v in (cols.attrib if cols is not None else {}).items()},
    }


def reference_count(text: str) -> int:
    return len(re.findall(r"^\[\d+\]", text, re.MULTILINE))


def paragraph_style(p: ET.Element) -> str:
    st = p.find("w:pPr/w:pStyle", NS)
    return st.get(w("val")) if st is not None else ""


def figure_records(root: ET.Element) -> list[dict[str, object]]:
    paras = root.findall(".//w:body/w:p", NS)
    records: list[dict[str, object]] = []
    for idx, para in enumerate(paras):
        inline = para.find(".//wp:inline", NS)
        if inline is None:
            continue
        extent = wattrs(inline.find("wp:extent", NS))
        docpr = wattrs(inline.find("wp:docPr", NS))
        caption = ""
        for nxt in paras[idx + 1 : idx + 4]:
            txt = text_of(nxt)
            if txt.startswith("图"):
                caption = txt
                break
        records.append(
            {
                "index": len(records) + 1,
                "paragraph": idx + 1,
                "style": paragraph_style(para),
                "name": docpr.get("name", ""),
                "cx_cm": round(int(extent.get("cx", "0")) / 360000, 2),
                "cy_cm": round(int(extent.get("cy", "0")) / 360000, 2),
                "caption": caption,
            }
        )
    return records


def table_caption_records(root: ET.Element) -> list[str]:
    return [
        text_of(p)
        for p in root.findall(".//w:body/w:p", NS)
        if paragraph_style(p) == "JOSCaption" and re.match(r"^表\s*\d+", text_of(p))
    ]


def is_algorithm_table(tbl: ET.Element) -> bool:
    text = text_of(tbl)
    return "Algorithm " in text and "Input:" in text and "Output:" in text


def table_border_records(root: ET.Element) -> list[dict[str, object]]:
    records: list[dict[str, object]] = []
    regular_tables = [tbl for tbl in root.findall(".//w:tbl", NS) if not is_algorithm_table(tbl)]
    for idx, tbl in enumerate(regular_tables, 1):
        borders = tbl.find(".//w:tblBorders", NS)
        vals = {}
        if borders is not None:
            for child in borders:
                vals[child.tag.split("}")[-1]] = wattrs(child)
        records.append(
            {
                "index": idx,
                "top": vals.get("top", {}).get("val"),
                "bottom": vals.get("bottom", {}).get("val"),
                "left": vals.get("left", {}).get("val"),
                "right": vals.get("right", {}).get("val"),
                "insideH": vals.get("insideH", {}).get("val"),
                "insideV": vals.get("insideV", {}).get("val"),
            }
        )
    return records


def table_font_stats(root: ET.Element) -> dict[str, int]:
    total = 0
    direct = 0
    header_runs = 0
    header_bold = 0
    body_runs = 0
    body_not_bold = 0
    for tbl in root.findall(".//w:tbl", NS):
        if is_algorithm_table(tbl):
            continue
        for row_idx, row in enumerate(tbl.findall("w:tr", NS)):
            for run in row.findall(".//w:r", NS):
                txt = text_of(run)
                if not txt:
                    continue
                total += 1
                rpr = run.find("w:rPr", NS)
                fonts = rpr.find("w:rFonts", NS) if rpr is not None else None
                size = rpr.find("w:sz", NS) if rpr is not None else None
                bold = rpr.find("w:b", NS) is not None if rpr is not None else False
                if (
                    fonts is not None
                    and size is not None
                    and fonts.get(w("eastAsia")) == "宋体"
                    and fonts.get(w("ascii")) == "Times New Roman"
                    and size.get(w("val")) == "15"
                ):
                    direct += 1
                if row_idx == 0:
                    header_runs += 1
                    if bold:
                        header_bold += 1
                else:
                    body_runs += 1
                    if not bold:
                        body_not_bold += 1
    return {
        "total": total,
        "direct": direct,
        "header_runs": header_runs,
        "header_bold": header_bold,
        "body_runs": body_runs,
        "body_not_bold": body_not_bold,
    }


def header_page_field_count(docx: Path) -> int:
    count = 0
    with zipfile.ZipFile(docx) as zf:
        for name in ["word/header1.xml", "word/header2.xml"]:
            if b" PAGE " in zf.read(name):
                count += 1
    return count


def header_texts(docx: Path) -> dict[str, str]:
    headers: dict[str, str] = {}
    for name in ["word/header1.xml", "word/header2.xml"]:
        headers[name] = text_of(read_docx_xml(docx, name))
    return headers


def superscript_run_count(root: ET.Element) -> int:
    return len(root.findall(".//w:vertAlign[@w:val='superscript']", NS))


def subscript_run_count(root: ET.Element) -> int:
    return len(root.findall(".//w:vertAlign[@w:val='subscript']", NS))


def formula_paragraph_records(root: ET.Element) -> list[dict[str, object]]:
    records: list[dict[str, object]] = []
    for idx, p in enumerate(root.findall(".//w:body/w:p", NS), 1):
        if paragraph_style(p) != "JOSCode":
            continue
        text = text_of(p)
        if not re.search(r"\(\d+\)$", text.strip()):
            continue
        records.append(
            {
                "paragraph": idx,
                "text": text,
                "superscripts": len(p.findall(".//w:vertAlign[@w:val='superscript']", NS)),
                "subscripts": len(p.findall(".//w:vertAlign[@w:val='subscript']", NS)),
                "has_latex_residue": bool(re.search(r"\b(?:bigl|bigr)\b|[_^]", text)),
            }
        )
    return records


def body_citation_superscript_stats(root: ET.Element) -> dict[str, int]:
    total = 0
    superscript = 0
    for p in root.findall(".//w:body/w:p", NS):
        if paragraph_style(p).startswith("JOSReference"):
            continue
        for r in p.findall("w:r", NS):
            txt = text_of(r)
            if re.fullmatch(r"\[[0-9][0-9,\-\s]*\]", txt):
                total += 1
                if r.find("w:rPr/w:vertAlign[@w:val='superscript']", NS) is not None:
                    superscript += 1
    for tbl in root.findall(".//w:tbl", NS):
        for r in tbl.findall(".//w:r", NS):
            txt = text_of(r)
            if re.fullmatch(r"\[[0-9][0-9,\-\s]*\]", txt):
                total += 1
                if r.find("w:rPr/w:vertAlign[@w:val='superscript']", NS) is not None:
                    superscript += 1
    return {"total": total, "superscript": superscript}


def masthead_tab_count(root: ET.Element, docx: Path | None = None) -> int:
    count = 0
    roots = [root]
    if docx is not None:
        with zipfile.ZipFile(docx) as zf:
            if "word/header0.xml" in zf.namelist():
                roots.insert(0, ET.fromstring(zf.read("word/header0.xml")))
    for idx, candidate in enumerate(roots):
        paras = candidate.findall(".//w:body/w:p", NS)[:3] if idx else candidate.findall(".//w:p", NS)
        local_count = 0
        for p in paras:
            if p.find("w:pPr/w:tabs/w:tab", NS) is not None and p.find("w:r/w:tab", NS) is not None:
                local_count += 1
        count = max(count, local_count)
    return count


def pagination_keep_stats(root: ET.Element) -> dict[str, int]:
    paras = root.findall(".//w:body/w:p", NS)
    table_caption_total = 0
    table_caption_keep = 0
    image_total = 0
    image_keep = 0
    algorithm_caption_total = 0
    algorithm_caption_keep = 0
    algorithm_code_total = 0
    algorithm_code_keep_lines = 0
    algorithm_code_keep_next_expected = 0
    algorithm_code_keep_next = 0

    for idx, para in enumerate(paras):
        style_id = paragraph_style(para)
        text = text_of(para)
        keep_next = para.find("w:pPr/w:keepNext", NS) is not None
        keep_lines = para.find("w:pPr/w:keepLines", NS) is not None
        if style_id == "JOSCaption" and re.match(r"^表\s*\d+", text):
            table_caption_total += 1
            table_caption_keep += int(keep_next)
        if style_id == "JOSImage":
            image_total += 1
            image_keep += int(keep_next)
        if style_id == "JOSCaption" and text.startswith("算法"):
            algorithm_caption_total += 1
            algorithm_caption_keep += int(keep_next)
            code_indices = []
            for next_idx in range(idx + 1, len(paras)):
                if paragraph_style(paras[next_idx]) != "JOSCode":
                    break
                code_indices.append(next_idx)
            for pos, code_idx in enumerate(code_indices):
                code_para = paras[code_idx]
                algorithm_code_total += 1
                algorithm_code_keep_lines += int(code_para.find("w:pPr/w:keepLines", NS) is not None)
                if pos < len(code_indices) - 1:
                    algorithm_code_keep_next_expected += 1
                    algorithm_code_keep_next += int(code_para.find("w:pPr/w:keepNext", NS) is not None)

    table_rows = root.findall(".//w:tr", NS)
    return {
        "table_rows": len(table_rows),
        "table_rows_cant_split": sum(1 for tr in table_rows if tr.find("w:trPr/w:cantSplit", NS) is not None),
        "table_caption_total": table_caption_total,
        "table_caption_keep": table_caption_keep,
        "image_total": image_total,
        "image_keep": image_keep,
        "algorithm_caption_total": algorithm_caption_total,
        "algorithm_caption_keep": algorithm_caption_keep,
        "algorithm_code_total": algorithm_code_total,
        "algorithm_code_keep_lines": algorithm_code_keep_lines,
        "algorithm_code_keep_next_expected": algorithm_code_keep_next_expected,
        "algorithm_code_keep_next": algorithm_code_keep_next,
    }


def algorithm_table_stats(root: ET.Element) -> dict[str, object]:
    for tbl in root.findall(".//w:tbl", NS):
        if not is_algorithm_table(tbl):
            continue
        rows = tbl.findall("w:tr", NS)
        line_numbers: list[str] = []
        comment_rows = 0
        left_borders = 0
        bottom_hooks = 0
        cant_split = 0
        for row in rows:
            if row.find("w:trPr/w:cantSplit", NS) is not None:
                cant_split += 1
            cells = row.findall("w:tc", NS)
            first = text_of(cells[0]) if cells else ""
            if re.fullmatch(r"\d+", first):
                line_numbers.append(first)
            if "//" in text_of(row):
                comment_rows += 1
            for cell in cells:
                borders = cell.find("w:tcPr/w:tcBorders", NS)
                if borders is None:
                    continue
                if borders.find("w:left", NS) is not None:
                    left_borders += 1
                if borders.find("w:bottom", NS) is not None:
                    bottom_hooks += 1
        return {
            "found": True,
            "rows": len(rows),
            "cant_split": cant_split,
            "line_numbers": line_numbers,
            "comment_rows": comment_rows,
            "left_borders": left_borders,
            "bottom_hooks": bottom_hooks,
            "text": text_of(tbl),
        }
    return {
        "found": False,
        "rows": 0,
        "cant_split": 0,
        "line_numbers": [],
        "comment_rows": 0,
        "left_borders": 0,
        "bottom_hooks": 0,
        "text": "",
    }


def split_header_stats(docx: Path) -> dict[str, int]:
    tabbed_headers = 0
    compact_separator_headers = 0
    with zipfile.ZipFile(docx) as zf:
        for name in ["word/header1.xml", "word/header2.xml"]:
            root = ET.fromstring(zf.read(name))
            if root.find(".//w:tabs/w:tab", NS) is not None and root.find(".//w:r/w:tab", NS) is not None:
                tabbed_headers += 1
            if " | " in text_of(root):
                compact_separator_headers += 1
    return {
        "tabbed_headers": tabbed_headers,
        "compact_separator_headers": compact_separator_headers,
    }


def reference_indent(docx: Path) -> dict[str, str]:
    styles = read_docx_xml(docx, "word/styles.xml")
    st = styles.find(".//w:style[@w:styleId='JOSReference']", NS)
    return wattrs(st.find("w:pPr/w:ind", NS)) if st is not None else {}


def coverage(markers: list[str], docx_norm: str, pdf_norm: str) -> list[dict[str, object]]:
    rows = []
    for marker in markers:
        m = normalize(marker)
        rows.append(
            {
                "marker": marker,
                "in_docx": m in docx_norm,
                "in_pdf": m in pdf_norm,
            }
        )
    return rows


def make_report(result: dict[str, object]) -> str:
    lines = [
        "# DOCX 与 PDF 一致性校验报告",
        "",
        f"- DOCX: `{result['docx']}`",
        f"- PDF: `{result['pdf']}`",
        f"- 结论: {'通过' if result['passed'] else '未通过'}",
        "",
        "## 结构计数",
        "",
        "| 项 | 值 | 要求 | 状态 |",
        "|---|---:|---:|---|",
    ]
    for item in result["checks"]:
        lines.append(f"| {item['name']} | {item['actual']} | {item['expected']} | {item['status']} |")
    lines.extend(
        [
            "",
            "## 关键内容覆盖",
            "",
            "| 标记 | DOCX | PDF |",
            "|---|---|---|",
        ]
    )
    for row in result["marker_coverage"]:
        lines.append(
            f"| {row['marker']} | {'是' if row['in_docx'] else '否'} | {'是' if row['in_pdf'] else '否'} |"
        )
    lines.extend(
        [
            "",
            "## 页面 XML",
            "",
            f"- 纸张: `{result['page_setup']['paper_twips']}`",
            f"- 页边距: `{result['page_setup']['margins_twips']}`",
            f"- 分栏: `{result['page_setup']['columns']}`",
            "",
            "## 文本量",
            "",
            f"- DOCX 提取文本字符数: {result['docx_chars']}",
            f"- PDF 提取文本字符数: {result['pdf_chars']}",
            f"- DOCX/PDF 字符比例: {result['char_ratio']:.3f}",
        ]
    )
    lines.extend(
        [
            "",
            "## 图像插入",
            "",
            "| # | 段落 | 图片 | 尺寸 cm | 段落样式 | 图题 |",
            "|---:|---:|---|---:|---|---|",
        ]
    )
    for row in result["figures"]:
        lines.append(
            f"| {row['index']} | {row['paragraph']} | {row['name']} | {row['cx_cm']}×{row['cy_cm']} | {row['style']} | {row['caption']} |"
        )
    lines.extend(
        [
            "",
            "## 表格与边框",
            "",
            "编号表题：",
        ]
    )
    for caption in result["table_captions"]:
        lines.append(f"- {caption}")
    lines.extend(
        [
            "",
            "| 表格对象 | 上边 | 下边 | 左边 | 右边 | 横向内线 | 纵向内线 |",
            "|---:|---|---|---|---|---|---|",
        ]
    )
    for row in result["table_borders"]:
        lines.append(
            f"| {row['index']} | {row['top']} | {row['bottom']} | {row['left']} | {row['right']} | {row['insideH']} | {row['insideV']} |"
        )
    lines.extend(
        [
            "",
            "## 公式输出",
            "",
            "| 段落 | 文本 | 上标 run | 下标 run | LaTeX 残留 |",
            "|---:|---|---:|---:|---|",
        ]
    )
    for row in result["formulas"]:
        lines.append(
            f"| {row['paragraph']} | {row['text']} | {row['superscripts']} | {row['subscripts']} | "
            f"{'有' if row['has_latex_residue'] else '无'} |"
        )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify generated JOS DOCX")
    parser.add_argument("--docx", required=True)
    parser.add_argument("--pdf", default="latex/main-jos.pdf")
    parser.add_argument("--tex-root", default=None, help="Paper root containing latex/main-jos.tex")
    parser.add_argument("--format", default="docs/format/jos_2025_docx_format_definitions.json")
    parser.add_argument("--allowed-footer", default="1260", help="Additional allowed footer distance in twips")
    parser.add_argument("--report", required=True)
    parser.add_argument("--json-report", default=None)
    args = parser.parse_args()

    docx = Path(args.docx).resolve()
    pdf = Path(args.pdf).resolve()
    tex_root = Path(args.tex_root).resolve() if args.tex_root else pdf.parent.parent
    fmt = Path(args.format).resolve()
    report = Path(args.report).resolve()
    json_report = Path(args.json_report).resolve() if args.json_report else report.with_suffix(".json")

    if not docx.exists():
        raise SystemExit(f"DOCX not found: {docx}")
    if not pdf.exists():
        raise SystemExit(f"PDF not found: {pdf}")
    if not (tex_root / "latex/main-jos.tex").exists():
        raise SystemExit(f"TeX root not found or invalid: {tex_root}")
    if not fmt.exists():
        raise SystemExit(f"format definition not found: {fmt}")

    src = source_profile(tex_root)
    fmt_data = json.loads(fmt.read_text(encoding="utf-8"))
    doc_root = read_docx_xml(docx, "word/document.xml")
    dtext = docx_text(docx)
    ptext = pdf_text(pdf)
    dnorm = normalize(dtext)
    pnorm = normalize(ptext)
    page_setup = sect_values(doc_root)
    expected_page = fmt_data["page_setup"]

    markers = list(src["markers"])
    marker_rows = coverage(markers, dnorm, pnorm)
    figures = figure_records(doc_root)
    table_captions = table_caption_records(doc_root)
    table_borders = table_border_records(doc_root)
    table_fonts = table_font_stats(doc_root)
    page_fields = header_page_field_count(docx)
    headers = header_texts(docx)
    header_markers = [str(src["running_header"]), "Journal of Software 软件学报"]
    normalized_header_values = [normalize(text) for text in headers.values()]
    normalized_pdf_text = normalize(ptext)
    docx_header_hits = sum(
        1 for marker in header_markers if any(normalize(marker) in text for text in normalized_header_values)
    )
    pdf_header_hits = sum(1 for marker in header_markers if normalize(marker) in normalized_pdf_text)
    masthead_tabs = masthead_tab_count(doc_root, docx)
    keep_stats = pagination_keep_stats(doc_root)
    algorithm_stats = algorithm_table_stats(doc_root)
    header_split = split_header_stats(docx)
    actual_margins = page_setup.get("margins_twips", {})
    expected_margins = expected_page["margins_twips"]
    allowed_footers = {expected_margins.get("footer"), args.allowed_footer}
    margins_without_footer_ok = all(
        actual_margins.get(key) == expected
        for key, expected in expected_margins.items()
        if key != "footer"
    )
    margins_ok = margins_without_footer_ok and actual_margins.get("footer") in allowed_footers
    ref_indent = reference_indent(docx)
    leaked_config = any(
        x in dtext
        for x in ["leftmargin", "labelwidth", "itemindent", "nosep", "indent=-", "sep=4pt", "=0pt"]
    )
    citation_superscripts = body_citation_superscript_stats(doc_root)
    superscripts = superscript_run_count(doc_root)
    subscripts = subscript_run_count(doc_root)
    formulas = formula_paragraph_records(doc_root)
    formula_superscripts = sum(int(row["superscripts"]) for row in formulas)
    formula_subscripts = sum(int(row["subscripts"]) for row in formulas)
    formula_residue = [str(row["text"]) for row in formulas if row["has_latex_residue"]]
    caret_leaks = re.findall(r"\^[A-Za-z0-9*]+", dtext)
    docx_chars = len(normalize(dtext))
    pdf_chars = len(normalize(ptext))
    ratio = docx_chars / max(pdf_chars, 1)
    expected_figures = int(src["figures"])
    expected_tables = int(src["tables"])
    expected_algorithms = int(src["algorithms"])
    expected_references = max(1, int(src["citations"]))
    algorithm_line_numbers = list(algorithm_stats["line_numbers"])
    expected_algorithm_lines = [str(i) for i in range(1, len(algorithm_line_numbers) + 1)]
    checks = [
        {
            "name": "表格对象数",
            "actual": count_tables(doc_root),
            "expected": f">={expected_tables}",
            "ok": count_tables(doc_root) >= expected_tables,
        },
        {
            "name": "图片数",
            "actual": count_images(doc_root),
            "expected": expected_figures,
            "ok": count_images(doc_root) == expected_figures,
        },
        {
            "name": "图题与图片一一对应",
            "actual": f"{sum(1 for r in figures if r['caption'])}/{len(figures)}",
            "expected": f"{expected_figures}/{expected_figures}",
            "ok": len(figures) == expected_figures and all(r["caption"] for r in figures),
        },
        {
            "name": "图片段落非固定行距样式",
            "actual": f"{sum(1 for r in figures if r['style'] == 'JOSImage')}/{len(figures)}",
            "expected": f"{expected_figures}/{expected_figures}",
            "ok": len(figures) == expected_figures and all(r["style"] == "JOSImage" for r in figures),
        },
        {
            "name": "编号表题数",
            "actual": len(table_captions),
            "expected": expected_tables,
            "ok": len(table_captions) == expected_tables,
        },
        {
            "name": "表格中间竖线",
            "actual": f"{sum(1 for r in table_borders if r['insideV'] == 'single')}/{len(table_borders)}",
            "expected": f"{len(table_borders)}/{len(table_borders)}",
            "ok": bool(table_borders) and all(r["insideV"] == "single" for r in table_borders),
        },
        {
            "name": "表格左右开口",
            "actual": f"{sum(1 for r in table_borders if r['left'] == 'nil' and r['right'] == 'nil')}/{len(table_borders)}",
            "expected": f"{len(table_borders)}/{len(table_borders)}",
            "ok": bool(table_borders) and all(r["left"] == "nil" and r["right"] == "nil" for r in table_borders),
        },
        {
            "name": "表格单元格直接字体",
            "actual": f"{table_fonts['direct']}/{table_fonts['total']}",
            "expected": f"{table_fonts['total']}/{table_fonts['total']}",
            "ok": table_fonts["total"] > 0 and table_fonts["direct"] == table_fonts["total"],
        },
        {
            "name": "表头加粗正文常规",
            "actual": f"表头 {table_fonts['header_bold']}/{table_fonts['header_runs']}；正文 {table_fonts['body_not_bold']}/{table_fonts['body_runs']}",
            "expected": "全部符合",
            "ok": table_fonts["header_runs"] > 0
            and table_fonts["header_bold"] == table_fonts["header_runs"]
            and table_fonts["body_not_bold"] == table_fonts["body_runs"],
        },
        {
            "name": "页眉页码字段",
            "actual": page_fields,
            "expected": 2,
            "ok": page_fields == 2,
        },
        {
            "name": "页眉内容",
            "actual": headers,
            "expected": {"word/header1.xml": header_markers[0], "word/header2.xml": header_markers[1]},
            "ok": normalize(header_markers[0]) in normalize(headers.get("word/header1.xml", ""))
            and normalize(header_markers[1]) in normalize(headers.get("word/header2.xml", "")),
        },
        {
            "name": "页眉与 main-jos.pdf 对照",
            "actual": f"DOCX {docx_header_hits}/2；PDF {pdf_header_hits}/2",
            "expected": "DOCX 2/2；PDF 2/2",
            "ok": docx_header_hits == 2 and pdf_header_hits == 2,
        },
        {
            "name": "页眉标题页码左右分开",
            "actual": f"制表位 {header_split['tabbed_headers']}/2；紧挤分隔符 {header_split['compact_separator_headers']}",
            "expected": "制表位 2/2；紧挤分隔符 0",
            "ok": header_split["tabbed_headers"] == 2 and header_split["compact_separator_headers"] == 0,
        },
        {
            "name": "首页期刊信息右对齐制表位",
            "actual": masthead_tabs,
            "expected": 3,
            "ok": masthead_tabs == 3,
        },
        {
            "name": "表格行禁止跨页拆分",
            "actual": f"{keep_stats['table_rows_cant_split']}/{keep_stats['table_rows']}",
            "expected": f"{keep_stats['table_rows']}/{keep_stats['table_rows']}",
            "ok": keep_stats["table_rows"] > 0
            and keep_stats["table_rows_cant_split"] == keep_stats["table_rows"],
        },
        {
            "name": "表题与表格同页",
            "actual": f"{keep_stats['table_caption_keep']}/{keep_stats['table_caption_total']}",
            "expected": f"{keep_stats['table_caption_total']}/{keep_stats['table_caption_total']}",
            "ok": keep_stats["table_caption_total"] > 0
            and keep_stats["table_caption_keep"] == keep_stats["table_caption_total"],
        },
        {
            "name": "图片与图题同页",
            "actual": f"{keep_stats['image_keep']}/{keep_stats['image_total']}",
            "expected": f"{keep_stats['image_total']}/{keep_stats['image_total']}",
            "ok": keep_stats["image_total"] > 0
            and keep_stats["image_keep"] == keep_stats["image_total"],
        },
        {
            "name": "算法清单同页保持",
            "actual": f"表格行 cantSplit {algorithm_stats['cant_split']}/{algorithm_stats['rows']}",
            "expected": "算法表格全部行 cantSplit" if expected_algorithms else "无算法",
            "ok": expected_algorithms == 0
            or (
                bool(algorithm_stats["found"])
                and algorithm_stats["rows"] > 0
                and algorithm_stats["cant_split"] == algorithm_stats["rows"]
            ),
        },
        {
            "name": "算法表格结构",
            "actual": (
                f"found={algorithm_stats['found']}；"
                f"行号={','.join(algorithm_stats['line_numbers'])}；"
                f"注释行={algorithm_stats['comment_rows']}；"
                f"竖线={algorithm_stats['left_borders']}；"
                f"收口线={algorithm_stats['bottom_hooks']}"
            ),
            "expected": "行号连续，含注释列和块连接线" if expected_algorithms else "无算法",
            "ok": expected_algorithms == 0
            or (
                bool(algorithm_stats["found"])
                and algorithm_line_numbers == expected_algorithm_lines
                and algorithm_stats["comment_rows"] >= 2
                and algorithm_stats["left_borders"] > 0
                and algorithm_stats["bottom_hooks"] > 0
            ),
        },
        {
            "name": "参考文献悬挂缩进",
            "actual": ref_indent,
            "expected": {"left": "420", "hanging": "420"},
            "ok": ref_indent.get("left") == "420" and ref_indent.get("hanging") == "420",
        },
        {
            "name": "LaTeX 环境参数泄漏",
            "actual": "有" if leaked_config else "无",
            "expected": "无",
            "ok": not leaked_config,
        },
        {
            "name": "正文数字引用上标",
            "actual": f"{citation_superscripts['superscript']}/{citation_superscripts['total']}",
            "expected": f"{citation_superscripts['total']}/{citation_superscripts['total']}",
            "ok": citation_superscripts["total"] > 0
            and citation_superscripts["superscript"] == citation_superscripts["total"],
        },
        {
            "name": "Word 上标 run 数",
            "actual": superscripts,
            "expected": ">0",
            "ok": superscripts > 0,
        },
        {
            "name": "上标源码残留",
            "actual": ", ".join(caret_leaks[:10]) if caret_leaks else "无",
            "expected": "无",
            "ok": not caret_leaks,
        },
        {
            "name": "公式段落识别",
            "actual": len(formulas),
            "expected": ">=1",
            "ok": len(formulas) >= 1,
        },
        {
            "name": "公式上标 run",
            "actual": formula_superscripts,
            "expected": ">0",
            "ok": formula_superscripts > 0,
        },
        {
            "name": "公式下标 run",
            "actual": formula_subscripts,
            "expected": ">0",
            "ok": formula_subscripts > 0 and subscripts > 0,
        },
        {
            "name": "公式 LaTeX 残留",
            "actual": "; ".join(formula_residue[:3]) if formula_residue else "无",
            "expected": "无",
            "ok": not formula_residue,
        },
        {
            "name": "参考文献条目数",
            "actual": reference_count(dtext),
            "expected": f">={expected_references}",
            "ok": reference_count(dtext) >= expected_references,
        },
        {"name": "DOCX/PDF 字符比例", "actual": f"{ratio:.3f}", "expected": ">=0.75", "ok": ratio >= 0.75},
        {
            "name": "页面尺寸",
            "actual": page_setup.get("paper_twips"),
            "expected": expected_page["paper_twips"],
            "ok": page_setup.get("paper_twips") == expected_page["paper_twips"],
        },
        {
            "name": "页边距",
            "actual": actual_margins,
            "expected": {**expected_margins, "footer": f"{expected_margins.get('footer')} 或 {args.allowed_footer}"},
            "ok": margins_ok,
        },
        {
            "name": "分栏",
            "actual": page_setup.get("columns"),
            "expected": expected_page["columns"],
            "ok": page_setup.get("columns") == expected_page["columns"],
        },
        {
            "name": "关键标记覆盖",
            "actual": f"{sum(1 for r in marker_rows if r['in_docx'])}/{len(marker_rows)}",
            "expected": f"{len(marker_rows)}/{len(marker_rows)}",
            "ok": all(r["in_docx"] for r in marker_rows),
        },
    ]
    for item in checks:
        item["status"] = "通过" if item["ok"] else "失败"
    result = {
        "docx": str(docx),
        "pdf": str(pdf),
        "passed": all(item["ok"] for item in checks),
        "checks": checks,
        "marker_coverage": marker_rows,
        "page_setup": page_setup,
        "figures": figures,
        "table_captions": table_captions,
        "table_borders": table_borders,
        "formulas": formulas,
        "docx_chars": docx_chars,
        "pdf_chars": pdf_chars,
        "char_ratio": ratio,
        "paragraphs": count_paragraphs(doc_root),
    }
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(make_report(result), encoding="utf-8")
    json_report.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"report={report}")
    print(f"json={json_report}")
    print(f"passed={result['passed']} tables={count_tables(doc_root)} images={count_images(doc_root)} refs={reference_count(dtext)} ratio={ratio:.3f}")
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
