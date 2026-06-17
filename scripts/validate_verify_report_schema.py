#!/usr/bin/env python3
"""Lightweight schema guard for verify_jos_docx.py JSON reports.

This intentionally avoids a jsonschema dependency so the check can run in the
same minimal environments as the build scripts.  The canonical schema lives at
docs/schema/verify_jos_docx_report.schema.json; this script enforces the
required top-level contract and the key nested arrays used by downstream tools.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_TOP = {
    "docx": str,
    "pdf": str,
    "passed": bool,
    "checks": list,
    "marker_coverage": list,
    "page_setup": dict,
    "figures": list,
    "table_captions": list,
    "table_borders": list,
    "formulas": list,
    "docx_chars": int,
    "pdf_chars": int,
    "char_ratio": (int, float),
    "paragraphs": int,
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def validate(report: dict) -> None:
    for key, typ in REQUIRED_TOP.items():
        require(key in report, f"missing top-level key: {key}")
        require(isinstance(report[key], typ), f"{key} has unexpected type")

    for idx, item in enumerate(report["checks"]):
        require(isinstance(item, dict), f"checks[{idx}] is not an object")
        for key in ["name", "actual", "expected", "ok", "status"]:
            require(key in item, f"checks[{idx}] missing {key}")
        require(isinstance(item["name"], str), f"checks[{idx}].name must be string")
        require(isinstance(item["ok"], bool), f"checks[{idx}].ok must be bool")
        require(item["status"] in {"通过", "失败"}, f"checks[{idx}].status invalid")

    for idx, item in enumerate(report["marker_coverage"]):
        require(isinstance(item, dict), f"marker_coverage[{idx}] is not an object")
        for key in ["marker", "in_docx", "in_pdf"]:
            require(key in item, f"marker_coverage[{idx}] missing {key}")

    for idx, item in enumerate(report["formulas"]):
        require(isinstance(item, dict), f"formulas[{idx}] is not an object")
        for key in ["paragraph", "text", "superscripts", "subscripts", "has_latex_residue"]:
            require(key in item, f"formulas[{idx}] missing {key}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path)
    args = parser.parse_args()
    data = json.loads(args.report.read_text(encoding="utf-8"))
    try:
        validate(data)
    except ValueError as exc:
        print(f"schema validation failed: {exc}", file=sys.stderr)
        return 1
    print(f"schema validation ok: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
