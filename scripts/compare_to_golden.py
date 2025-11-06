#!/usr/bin/env python3
"""Compare ablation reports against golden references with tolerances."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Tuple

DEFAULT_ABS_TOL = 1e-9
DEFAULT_REL_TOL = 1e-3


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", required=True, help="Path to the ablation plan YAML")
    parser.add_argument("--report", required=True, help="Path to the produced AblationReport JSON")
    parser.add_argument("--golden", required=True, help="Path to the golden AblationReport JSON")
    parser.add_argument(
        "--diff",
        help="Optional path to write diff summary JSON",
    )
    return parser.parse_args()


@dataclass
class Tolerance:
    min: float | None = None
    max: float | None = None
    abs: float = DEFAULT_ABS_TOL
    rel: float = DEFAULT_REL_TOL

    @classmethod
    def from_mapping(cls, mapping: Dict[str, Any]) -> "Tolerance":
        return cls(
            min=mapping.get("min"),
            max=mapping.get("max"),
            abs=mapping.get("abs", DEFAULT_ABS_TOL),
            rel=mapping.get("rel", DEFAULT_REL_TOL),
        )


def load_plan(path: Path) -> Dict[str, Any]:
    text = path.read_text()
    try:
        import yaml  # type: ignore

        return dict(yaml.safe_load(text))  # pragma: no cover - exercised in CI
    except Exception:
        # Fallback: minimal YAML parser for indent-based mappings and lists.
        return simple_yaml(text)


def simple_yaml(text: str) -> Dict[str, Any]:
    lines = [line.rstrip("\n") for line in text.splitlines() if line.strip() and not line.strip().startswith("#")]
    mapping, _ = parse_block(lines, 0, 0)
    return mapping


def parse_block(lines: list[str], start: int, indent: int) -> Tuple[Dict[str, Any], int]:
    result: Dict[str, Any] = {}
    i = start
    while i < len(lines):
        line = lines[i]
        current_indent = len(line) - len(line.lstrip())
        if current_indent < indent:
            break
        if current_indent > indent:
            raise ValueError(f"invalid indentation on line: {line}")
        key, rest = split_key_value(line.strip())
        if rest is None:
            value, next_index = parse_block(lines, i + 1, indent + 2)
            result[key] = value
            i = next_index
            continue
        result[key] = parse_scalar(rest)
        i += 1
    return result, i


def split_key_value(line: str) -> Tuple[str, str | None]:
    if ":" not in line:
        raise ValueError(f"expected key: value pair, got {line}")
    key, rest = line.split(":", 1)
    rest = rest.strip()
    if rest:
        return key, rest
    return key, None


def parse_scalar(value: str) -> Any:
    if value.startswith("[") and value.endswith("]"):
        return json.loads(value.replace("'", '"'))
    if value.lower() in {"true", "false"}:
        return value.lower() == "true"
    try:
        if "." in value or "e" in value or "E" in value:
            return float(value)
        return int(value)
    except ValueError:
        return value.strip('"')


def load_report(path: Path) -> Dict[str, Any]:
    return json.loads(path.read_text())


def collect_tolerances(plan: Dict[str, Any]) -> Dict[str, Tolerance]:
    tolerances: Dict[str, Tolerance] = {}
    raw = plan.get("tolerances", {}) or {}
    for name, spec in raw.items():
        if isinstance(spec, dict):
            tolerances[name] = Tolerance.from_mapping(spec)
        else:
            tolerances[name] = Tolerance()
    return tolerances


def compare(report: Dict[str, Any], golden: Dict[str, Any], tolerances: Dict[str, Tolerance]) -> Tuple[bool, Dict[str, Any]]:
    jobs_report = report.get("jobs", [])
    jobs_golden = golden.get("jobs", [])
    if len(jobs_report) != len(jobs_golden):
        return False, {"error": "job_count_mismatch", "report": len(jobs_report), "golden": len(jobs_golden)}

    diff_summary = {"jobs": []}
    ok = True
    for idx, (job_report, job_golden) in enumerate(zip(jobs_report, jobs_golden)):
        kpis_report = job_report.get("metrics", {}).get("kpis", {})
        kpis_golden = job_golden.get("metrics", {}).get("kpis", {})
        entry = {"job": idx, "metrics": {}}
        for name, golden_payload in kpis_golden.items():
            golden_value = float(golden_payload.get("value"))
            report_value = float(kpis_report.get(name, {}).get("value"))
            tol = tolerances.get(name, Tolerance())
            abs_delta = abs(report_value - golden_value)
            allowed = tol.abs + tol.rel * abs(golden_value)
            within = abs_delta <= allowed
            within_bounds = True
            if tol.min is not None and report_value + tol.abs < tol.min:
                within_bounds = False
            if tol.max is not None and report_value - tol.abs > tol.max:
                within_bounds = False
            status = within and within_bounds
            ok = ok and status
            entry["metrics"][name] = {
                "report": report_value,
                "golden": golden_value,
                "abs_delta": abs_delta,
                "allowed": allowed,
                "within_delta": within,
                "within_bounds": within_bounds,
                "pass": status,
            }
        diff_summary["jobs"].append(entry)
    return ok, diff_summary


def main() -> int:
    args = parse_args()
    plan = load_plan(Path(args.plan))
    tolerances = collect_tolerances(plan)
    report = load_report(Path(args.report))
    golden = load_report(Path(args.golden))
    ok, diff_summary = compare(report, golden, tolerances)
    if args.diff:
        Path(args.diff).parent.mkdir(parents=True, exist_ok=True)
        Path(args.diff).write_text(json.dumps(diff_summary, indent=2) + "\n")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
