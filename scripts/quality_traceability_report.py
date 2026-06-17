#!/usr/bin/env python3
"""Build a cross-layer quality traceability report.

The report links the standard AST, DOCX render tree, DOCX package XML, PDF text,
and verify_jos_docx JSON report so a failed check can be followed back to the
semantic node and mapping layer that produced the visible output.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import zipfile
from collections import Counter
from pathlib import Path
from xml.etree import ElementTree as ET


NS = {
    "w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    "r": "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
}


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def docx_snapshot(path: Path) -> dict:
    with zipfile.ZipFile(path) as zf:
        names = zf.namelist()
        doc_xml = zf.read("word/document.xml")
        root = ET.fromstring(doc_xml)
        rel_count = 0
        if "word/_rels/document.xml.rels" in names:
            rel_root = ET.fromstring(zf.read("word/_rels/document.xml.rels"))
            rel_count = len(list(rel_root))
        text = "".join(t.text or "" for t in root.findall(".//w:t", NS))
        return {
            "parts": len(names),
            "paragraphs": len(root.findall(".//w:body/w:p", NS)),
            "tables": len(root.findall(".//w:tbl", NS)),
            "media": len([n for n in names if n.startswith("word/media/")]),
            "relationships": rel_count,
            "text_chars": len(text),
            "document_xml_bytes": len(doc_xml),
        }


def pdf_text_chars(path: Path) -> int:
    try:
        out = subprocess.run(
            ["pdftotext", str(path), "-"],
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        return len("".join(out.stdout.split()))
    except Exception:
        return 0


def ast_snapshot(ast: dict) -> dict:
    blocks = ast.get("blocks", [])
    kinds: Counter[str] = Counter()
    rules: Counter[str] = Counter()
    raw = 0
    for block in blocks:
        kind = block.get("kind")
        if isinstance(kind, str):
            kind_name = kind
        elif isinstance(kind, dict):
            kind_name = next(iter(kind.keys()), "unknown")
        else:
            kind_name = "unknown"
        kinds[kind_name] += 1
        if kind_name in {"raw_fallback", "RawFallback"}:
            raw += 1
        metadata = block.get("metadata", {})
        for rule in metadata.get("rule_ids", []) if isinstance(metadata, dict) else []:
            rules[rule] += 1
    return {
        "blocks": len(blocks),
        "block_kinds": dict(sorted(kinds.items())),
        "rule_ids": dict(sorted(rules.items())),
        "raw_fallback": raw,
    }


def render_snapshot(render: dict) -> dict:
    nodes = render.get("nodes") or render.get("document", [])
    mapping_rules: Counter[str] = Counter()
    node_kinds: Counter[str] = Counter()
    for node in nodes:
        kind = str(node.get("kind", "unknown"))
        node_kinds[kind] += 1
        rules = node.get("mapping_rule_ids") or node.get("mapping_rules") or []
        if not rules and node.get("mapping_rule_id"):
            rules = [node.get("mapping_rule_id")]
        metadata = node.get("metadata", {})
        if not rules and isinstance(metadata, dict):
            rules = metadata.get("mapping_rule_ids", []) or []
        for rule in rules:
            mapping_rules[rule] += 1
    package = render.get("package", {})
    return {
        "nodes": len(nodes),
        "node_kinds": dict(sorted(node_kinds.items())),
        "mapping_rule_ids": dict(sorted(mapping_rules.items())),
        "parts": len(package.get("parts", [])) if isinstance(package, dict) else 0,
        "relationships": len(render.get("relationships", [])),
        "media": len(render.get("media", [])),
    }


def build_report(args: argparse.Namespace) -> dict:
    ast = load_json(args.ast)
    render = load_json(args.render)
    verify = load_json(args.verify)
    failed = [item for item in verify.get("checks", []) if not item.get("ok")]
    return {
        "schema_version": "0.1",
        "inputs": {
            "ast": str(args.ast),
            "render": str(args.render),
            "docx": str(args.docx),
            "pdf": str(args.pdf),
            "verify": str(args.verify),
        },
        "passed": verify.get("passed", False) and not failed,
        "ast": ast_snapshot(ast),
        "render": render_snapshot(render),
        "docx": docx_snapshot(args.docx),
        "pdf": {"text_chars_normalized": pdf_text_chars(args.pdf)},
        "verify": {
            "passed": verify.get("passed", False),
            "failed_checks": failed,
            "char_ratio": verify.get("char_ratio"),
            "docx_chars": verify.get("docx_chars"),
            "pdf_chars": verify.get("pdf_chars"),
        },
        "traceability": {
            "ast_to_render": "StandardDocument block metadata.rule_ids -> DocxRenderTree nodes mapping_rule_ids",
            "render_to_docx": "DocxRenderTree package parts/media/relationships -> word/document.xml and word/media/*",
            "docx_to_pdf": "DOCX XML text/media/layout -> pdftotext/PDF text and verify_jos_docx checks",
        },
    }


def write_markdown(report: dict, out: Path) -> None:
    lines = [
        "# Paper3 Cross-Layer Quality Traceability",
        "",
        f"- passed: `{report['passed']}`",
        f"- AST blocks: `{report['ast']['blocks']}`",
        f"- Render nodes: `{report['render']['nodes']}`",
        f"- DOCX paragraphs/tables/media: `{report['docx']['paragraphs']}` / `{report['docx']['tables']}` / `{report['docx']['media']}`",
        f"- Verify char ratio: `{report['verify']['char_ratio']}`",
        "",
        "## Failed Checks",
        "",
    ]
    failed = report["verify"]["failed_checks"]
    if failed:
        lines.append("| name | actual | expected |")
        lines.append("|---|---|---|")
        for item in failed:
            lines.append(f"| {item.get('name')} | `{item.get('actual')}` | `{item.get('expected')}` |")
    else:
        lines.append("No failed verify checks.")
    lines.extend(
        [
            "",
            "## AST Rule Coverage",
            "",
            "```json",
            json.dumps(report["ast"]["rule_ids"], ensure_ascii=False, indent=2),
            "```",
            "",
            "## Render Mapping Coverage",
            "",
            "```json",
            json.dumps(report["render"]["mapping_rule_ids"], ensure_ascii=False, indent=2),
            "```",
            "",
            "## Traceability",
            "",
        ]
    )
    for key, value in report["traceability"].items():
        lines.append(f"- `{key}`: {value}")
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ast", required=True, type=Path)
    parser.add_argument("--render", required=True, type=Path)
    parser.add_argument("--docx", required=True, type=Path)
    parser.add_argument("--pdf", required=True, type=Path)
    parser.add_argument("--verify", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--json-report", required=True, type=Path)
    args = parser.parse_args()
    report = build_report(args)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    write_markdown(report, args.out)
    args.json_report.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"traceability_report={args.out}")
    print(f"traceability_json={args.json_report}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
