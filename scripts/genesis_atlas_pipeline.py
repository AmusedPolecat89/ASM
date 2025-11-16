#!/usr/bin/env python3
"""Helper utilities for The Genesis Atlas pipeline."""

from __future__ import annotations

import argparse
import json
import math
import statistics
from pathlib import Path
from typing import Any, Dict, List

import yaml


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def save_json(payload: Any, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, sort_keys=True)


def parse_rule_map(plan_path: Path) -> Dict[int, str]:
    data = yaml.safe_load(plan_path.read_text(encoding="utf-8"))
    rule_entries = data.get("rules", []) or [{"id": 0, "label": "default"}]
    return {int(entry["id"]): str(entry["label"]) for entry in rule_entries}


def build_candidate_record(
    job: Dict[str, Any],
    rule_map: Dict[int, str],
    stage1_root: Path,
) -> Dict[str, Any]:
    seed = job["seed"]
    rule_id = job["rule_id"]
    job_id = f"{seed}_{rule_id}"
    job_dir = stage1_root / job_id
    kpis = job.get("kpis", {})
    record = {
        "job_id": job_id,
        "job_dir": str(job_dir),
        "seed": seed,
        "rule_id": rule_id,
        "rule_label": rule_map.get(rule_id, f"rule_{rule_id}"),
        "gap_proxy": kpis.get("gap_proxy"),
        "energy_final": kpis.get("energy_final"),
        "xi": kpis.get("xi"),
        "c_est": kpis.get("c_est"),
        "factors": kpis.get("factors", []),
        "g": kpis.get("g", []),
        "lambda_h": kpis.get("lambda_h"),
    }
    return record


def command_filter(args: argparse.Namespace) -> None:
    report_path = Path(args.report)
    plan_path = Path(args.plan)
    stage1_root = Path(args.stage1_root)
    report = load_json(report_path)
    rule_map = parse_rule_map(plan_path)
    candidates: List[Dict[str, Any]] = []
    for job in report.get("jobs", []):
        record = build_candidate_record(job, rule_map, stage1_root)
        gap = record.get("gap_proxy") or 0.0
        energy = record.get("energy_final") or 0.0
        if gap < args.min_gap:
            continue
        if energy > args.max_energy:
            continue
        candidates.append(record)
    candidates.sort(key=lambda item: item.get("gap_proxy", 0.0), reverse=True)
    summary = {
        "criteria": {
            "min_gap": args.min_gap,
            "max_energy": args.max_energy,
        },
        "plan": str(plan_path),
        "report": str(report_path),
        "stage1_root": str(stage1_root),
        "total_jobs": len(report.get("jobs", [])),
        "fertile_count": len(candidates),
        "candidates": candidates,
    }
    save_json(summary, Path(args.out))


def copy_file(src: Path, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(src.read_text(encoding="utf-8"), encoding="utf-8")


def command_stage2(args: argparse.Namespace) -> None:
    data = load_json(Path(args.candidates))
    stage1_root = Path(data["stage1_root"])
    candidates = data.get("candidates", [])
    limit = args.limit or len(candidates)
    selected = candidates[:limit]
    summaries: List[Dict[str, Any]] = []
    for record in selected:
        job_dir = Path(record["job_dir"])
        spec_path = job_dir / "spectrum" / "spectrum_report.json"
        gauge_path = job_dir / "gauge" / "gauge_report.json"
        if not spec_path.exists() or not gauge_path.exists():
            continue
        spectrum = load_json(spec_path)
        gauge = load_json(gauge_path)
        dest_dir = Path(args.out) / record["job_id"]
        dest_dir.mkdir(parents=True, exist_ok=True)
        copy_file(spec_path, dest_dir / "spectrum_report.json")
        copy_file(gauge_path, dest_dir / "gauge_report.json")
        summary = {
            "job_id": record["job_id"],
            "seed": record["seed"],
            "rule_id": record["rule_id"],
            "rule_label": record["rule_label"],
            "gap_proxy": record.get("gap_proxy"),
            "xi": record.get("xi"),
            "mass_gap": spectrum.get("spectral_gap"),
            "symmetry_rank": len(gauge.get("factors", [])),
            "factors": gauge.get("factors", []),
            "closure_pass": gauge.get("closure_pass"),
            "ward_pass": gauge.get("ward_pass"),
        }
        standard_model = {
            "metadata": summary,
            "spectrum": spectrum,
            "gauge": gauge,
        }
        save_json(standard_model, dest_dir / "standard_model.json")
        summaries.append(summary)
    save_json({"standard_models": summaries}, Path(args.out) / "standard_models_summary.json")


def command_stage3(args: argparse.Namespace) -> None:
    data = load_json(Path(args.candidates))
    candidates = data.get("candidates", [])
    limit = args.limit or len(candidates)
    selected = candidates[:limit]
    summaries: List[Dict[str, Any]] = []
    for record in selected:
        job_dir = Path(record["job_dir"])
        interact_path = job_dir / "interact" / "interaction_report.json"
        if not interact_path.exists():
            continue
        report = load_json(interact_path)
        dest_dir = Path(args.out) / record["job_id"]
        dest_dir.mkdir(parents=True, exist_ok=True)
        copy_file(interact_path, dest_dir / "interaction_report.json")
        g_values = report.get("g", record.get("g", []))
        mean_g = statistics.fmean(g_values) if g_values else 0.0
        field_theory = {
            "job_id": record["job_id"],
            "seed": record["seed"],
            "rule_id": record["rule_id"],
            "rule_label": record["rule_label"],
            "xi": record.get("xi"),
            "gap_proxy": record.get("gap_proxy"),
            "c_est": report.get("c_est", record.get("c_est")),
            "lambda_h": report.get("lambda_h", record.get("lambda_h")),
            "g": g_values,
            "mean_g": round(mean_g, 6),
        }
        save_json(field_theory, dest_dir / "field_theory_report.json")
        summaries.append(field_theory)
    save_json({"field_theories": summaries}, Path(args.out) / "field_theory_summary.json")


def command_stage4(args: argparse.Namespace) -> None:
    summary = load_json(Path(args.field_summary))
    entries = summary.get("field_theories", [])
    groups: Dict[int, List[Dict[str, Any]]] = {}
    for entry in entries:
        groups.setdefault(entry["rule_id"], []).append(entry)
    reports: List[Dict[str, Any]] = []
    for rule_id, bucket in groups.items():
        if len(bucket) < 2:
            continue
        bucket.sort(key=lambda item: item.get("xi", 0.0))
        slopes: List[List[float]] = []
        for prev, curr in zip(bucket, bucket[1:]):
            prev_xi = max(prev.get("xi", 1.0), 1e-6)
            curr_xi = max(curr.get("xi", prev_xi + 1e-6), 1e-6)
            log_ratio = math.log(curr_xi / prev_xi)
            log_ratio = log_ratio if abs(log_ratio) > 1e-6 else 1e-6
            prev_g = prev.get("g", [])
            curr_g = curr.get("g", [])
            delta = []
            for idx in range(min(len(prev_g), len(curr_g))):
                delta.append(round((curr_g[idx] - prev_g[idx]) / log_ratio, 6))
            slopes.append(delta)
        if not slopes:
            continue
        avg_slopes = [
            round(statistics.fmean([s[idx] for s in slopes if idx < len(s)]), 6)
            for idx in range(max(len(s) for s in slopes))
        ]
        avg_lambda = round(statistics.fmean(entry.get("lambda_h", 0.0) for entry in bucket), 6)
        report = {
            "rule_id": rule_id,
            "rule_label": bucket[0]["rule_label"],
            "entry_count": len(bucket),
            "avg_lambda_h": avg_lambda,
            "dg_dlog_xi": avg_slopes,
        }
        reports.append(report)
        save_json(report, Path(args.out) / f"running_{rule_id}.json")
    aggregate = {
        "reports": reports,
        "total_rules": len(reports),
    }
    save_json(aggregate, Path(args.out) / "running_summary.json")


def main() -> None:
    parser = argparse.ArgumentParser(description="Genesis Atlas helper utilities")
    subparsers = parser.add_subparsers(dest="command", required=True)

    filt = subparsers.add_parser("filter", help="Filter fertile candidates from a landscape report")
    filt.add_argument("--report", required=True)
    filt.add_argument("--plan", required=True)
    filt.add_argument("--stage1-root", required=True)
    filt.add_argument("--out", required=True)
    filt.add_argument("--min-gap", type=float, default=0.05)
    filt.add_argument("--max-energy", type=float, default=0.0)
    filt.set_defaults(func=command_filter)

    stage2 = subparsers.add_parser("stage2", help="Assemble Standard Model summaries")
    stage2.add_argument("--candidates", required=True)
    stage2.add_argument("--out", required=True)
    stage2.add_argument("--limit", type=int, default=4)
    stage2.set_defaults(func=command_stage2)

    stage3 = subparsers.add_parser("stage3", help="Summarize field theory reports")
    stage3.add_argument("--candidates", required=True)
    stage3.add_argument("--out", required=True)
    stage3.add_argument("--limit", type=int, default=4)
    stage3.set_defaults(func=command_stage3)

    stage4 = subparsers.add_parser("stage4", help="Compute running summaries from field theory fits")
    stage4.add_argument("--field-summary", required=True)
    stage4.add_argument("--out", required=True)
    stage4.set_defaults(func=command_stage4)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
