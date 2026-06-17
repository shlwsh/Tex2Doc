#!/usr/bin/env python3
"""Build a deterministic lock file for standards and profile rule files."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
LOCK_PATH = ROOT / "standards.lock.json"
INCLUDED_ROOTS = ("standards", "profiles")
INCLUDED_SUFFIXES = {".yaml", ".yml", ".json"}


def digest_file(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    rel = path.relative_to(ROOT).as_posix()
    return {
        "path": rel,
        "bytes": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
    }


def iter_rule_files() -> list[Path]:
    files: list[Path] = []
    for root_name in INCLUDED_ROOTS:
        root = ROOT / root_name
        if not root.exists():
            continue
        files.extend(
            path
            for path in root.rglob("*")
            if path.is_file() and path.suffix.lower() in INCLUDED_SUFFIXES
        )
    return sorted(files, key=lambda path: path.relative_to(ROOT).as_posix())


def main() -> None:
    files = [digest_file(path) for path in iter_rule_files()]
    payload = {
        "schema_version": "0.1",
        "algorithm": "sha256",
        "scope": list(INCLUDED_ROOTS),
        "files": files,
    }
    LOCK_PATH.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(f"wrote {LOCK_PATH.relative_to(ROOT)} with {len(files)} files")


if __name__ == "__main__":
    main()
