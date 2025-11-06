#!/usr/bin/env python3
"""Aggregate per-run validation summaries into a compact index."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Dict


def load_summary(run_dir: Path) -> Dict[str, Any] | None:
    summary_path = run_dir / "summary.json"
    if not summary_path.exists():
        return None
    try:
        return json.loads(summary_path.read_text())
    except json.JSONDecodeError:
        return None


def compute_metric(summary: Dict[str, Any]) -> float | None:
    coverage = summary.get("coverage")
    if isinstance(coverage, dict):
        unique = coverage.get("unique_state_hashes")
        if isinstance(unique, (int, float)):
            return float(unique)
    ess = summary.get("effective_sample_size")
    if isinstance(ess, (int, float)):
        return float(ess)
    return None


def build_index(runs_dir: Path) -> Dict[str, Dict[str, Any]]:
    index: Dict[str, Dict[str, Any]] = {}
    if not runs_dir.exists():
        return index
    for entry in sorted(runs_dir.iterdir()):
        if not entry.is_dir():
            continue
        summary = load_summary(entry)
        if summary is None:
            index[entry.name] = {"status": "MISSING", "metric": None}
            continue
        metric = compute_metric(summary)
        index[entry.name] = {"status": "PASS", "metric": metric}
    return index


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print("usage: make_validation_index.py RUNS_DIR OUTPUT.json", file=sys.stderr)
        return 1
    runs_dir = Path(argv[1])
    output = Path(argv[2])
    index = build_index(runs_dir)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(index, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
