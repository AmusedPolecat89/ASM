#!/usr/bin/env python3
"""Render a Markdown validation summary from an index JSON file."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Dict


def format_metric(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, float):
        return f"{value:.6g}"
    return str(value)


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: emit_report_md.py INDEX.json", file=sys.stderr)
        return 1
    index_path = Path(argv[1])
    data: Dict[str, Dict[str, Any]] = json.loads(index_path.read_text())

    lines = ["# Validation Summary", "", "| Test | Status | Metric |", "|------|--------|--------|"]
    for name in sorted(data):
        entry = data[name]
        status = entry.get("status", "")
        metric = format_metric(entry.get("metric"))
        lines.append(f"| {name} | {status} | {metric} |")
    sys.stdout.write("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
