#!/usr/bin/env python3
"""Persist a golden AblationReport for future comparisons."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from compare_to_golden import load_plan  # reuse parser helpers


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", required=True, help="Path to the ablation plan YAML")
    parser.add_argument("--report", required=True, help="Path to the AblationReport JSON")
    parser.add_argument("--out", help="Optional output path for the golden JSON")
    return parser.parse_args()


def canonicalise_report(report_path: Path) -> str:
    data = json.loads(report_path.read_text())
    return json.dumps(data, indent=2, sort_keys=True) + "\n"


def main() -> int:
    args = parse_args()
    plan_path = Path(args.plan)
    report_path = Path(args.report)
    plan = load_plan(plan_path)
    plan_name = plan.get("name")
    if not plan_name:
        raise SystemExit("plan missing name field")
    if args.out:
        out_path = Path(args.out)
    else:
        out_path = Path("ablation/goldens") / f"{plan_name}.gold.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(canonicalise_report(report_path))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
