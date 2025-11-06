#!/usr/bin/env python3
"""Summarise the ablation registry into CSV and Markdown dashboards."""

from __future__ import annotations

import argparse
import csv
import json
import sqlite3
from collections import defaultdict
from pathlib import Path
from statistics import mean, pstdev
from typing import Any, Dict, Iterable, List, Tuple


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry", required=True, help="Path to the registry CSV or SQLite file")
    parser.add_argument("--out", required=True, help="Directory for dashboard outputs")
    return parser.parse_args()


def load_rows(path: Path) -> List[Dict[str, Any]]:
    if path.suffix == ".sqlite":
        return load_sqlite_rows(path)
    return load_csv_rows(path)


def load_csv_rows(path: Path) -> List[Dict[str, Any]]:
    if not path.exists():
        return []
    rows: List[Dict[str, Any]] = []
    with path.open() as fh:
        reader = csv.DictReader(fh)
        for row in reader:
            rows.append(row)
    return rows


def load_sqlite_rows(path: Path) -> List[Dict[str, Any]]:
    if not path.exists():
        return []
    rows: List[Dict[str, Any]] = []
    conn = sqlite3.connect(path)
    try:
        for row in conn.execute(
            "SELECT date, \"commit\", plan_name, plan_hash, job_id, params, metrics FROM runs ORDER BY date, plan_name, job_id"
        ):
            rows.append(
                {
                    "date": row[0],
                    "commit": row[1],
                    "plan_name": row[2],
                    "plan_hash": row[3],
                    "job_id": row[4],
                    "params": row[5],
                    "metrics": row[6],
                }
            )
    finally:
        conn.close()
    return rows


def aggregate(rows: Iterable[Dict[str, Any]]) -> List[Dict[str, Any]]:
    buckets: Dict[Tuple[str, str], Dict[str, Any]] = defaultdict(lambda: {"values": [], "passes": []})
    provenance: Dict[str, Tuple[str, str]] = {}
    for row in rows:
        plan = row["plan_name"]
        data = json.loads(row["metrics"])
        kpis = data.get("kpis", {})
        provenance.setdefault(plan, (row["date"], row["commit"]))
        for name, payload in kpis.items():
            buckets[(plan, name)]["values"].append(float(payload.get("value", 0.0)))
            buckets[(plan, name)]["passes"].append(bool(payload.get("pass", False)))
    summaries: List[Dict[str, Any]] = []
    for (plan, kpi), payload in buckets.items():
        values = payload["values"]
        passes = payload["passes"]
        if not values:
            continue
        plan_date, plan_commit = provenance.get(plan, ("n/a", "n/a"))
        summaries.append(
            {
                "date": plan_date,
                "commit": plan_commit,
                "plan": plan,
                "kpi": kpi,
                "count": len(values),
                "mean": mean(values),
                "std": pstdev(values) if len(values) > 1 else 0.0,
                "pass_rate": sum(1 for v in passes if v) / len(passes),
            }
        )
    summaries.sort(key=lambda item: (item["plan"], item["kpi"]))
    return summaries


def write_csv(rows: List[Dict[str, Any]], path: Path) -> None:
    if not rows:
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("w", newline="") as fh:
            writer = csv.writer(fh)
            writer.writerow(["date", "commit", "plan", "kpi", "count", "mean", "std", "pass_rate"])
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as fh:
        writer = csv.DictWriter(
            fh,
            fieldnames=["date", "commit", "plan", "kpi", "count", "mean", "std", "pass_rate"],
        )
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def write_markdown(rows: List[Dict[str, Any]], path: Path) -> None:
    lines = ["# Ablation KPI Trends", ""]
    if not rows:
        lines.append("No registry entries were found.")
    else:
        current_plan = None
        for row in rows:
            if row["plan"] != current_plan:
                current_plan = row["plan"]
                lines.append(f"## {current_plan}")
                lines.append("| KPI | Count | Mean | Std | Pass rate |")
                lines.append("| --- | ---: | ---: | ---: | ---: |")
            lines.append(
                f"| {row['kpi']} | {row['count']} | {row['mean']:.6f} | {row['std']:.6f} | {row['pass_rate']:.2%} |"
            )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n")


def main() -> int:
    args = parse_args()
    registry_path = Path(args.registry)
    out_dir = Path(args.out)
    rows = load_rows(registry_path)
    summaries = aggregate(rows)
    write_csv(summaries, out_dir / "kpi_trends.csv")
    write_markdown(summaries, out_dir / "kpi_trends.md")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
