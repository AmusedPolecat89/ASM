#!/usr/bin/env python3
"""Render lightweight Markdown dashboards from replication and registry data."""
from __future__ import annotations

import argparse
import csv
import json
import pathlib
import sqlite3
from collections import defaultdict
from typing import Dict, Iterable, List, Tuple


def _load_json(*paths: pathlib.Path) -> Dict:
    for path in paths:
        if path and path.exists():
            with path.open() as handle:
                return json.load(handle)
    return {}


def _render_table(headers: Iterable[str], rows: Iterable[Iterable[str]]) -> str:
    header_list = list(headers)
    header_line = "| " + " | ".join(header_list) + " |"
    divider = "| " + " | ".join(["---"] * len(header_list)) + " |"
    body = "\n".join("| " + " | ".join(row) + " |" for row in rows)
    return "\n".join([header_line, divider, body]) if body else "\n".join([header_line, divider])


def _write_markdown(path: pathlib.Path, title: str, table: str, notes: str = "") -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    content = [f"# {title}", "", table]
    if notes:
        content.extend(["", notes])
    path.write_text("\n".join(content) + "\n")


def _vacua_dashboard(replication: pathlib.Path, expected: pathlib.Path, fixtures: pathlib.Path) -> Tuple[str, str]:
    digest = _load_json(
        replication / "metrics_digest.json",
        expected / "metrics_digest.json",
        fixtures / "metrics_digest.json",
    )
    rows = []
    for run, stats in sorted(digest.items()):
        energy = stats.get("energy", {})
        rows.append([
            run,
            f"{energy.get('min', 0):.3f}",
            f"{energy.get('mean', 0):.3f}",
            f"{energy.get('max', 0):.3f}",
            f"{stats.get('last_energy', 0):.3f}",
            str(stats.get("samples", 0)),
        ])
    table = _render_table([
        "Run",
        "Min",
        "Mean",
        "Max",
        "Last",
        "Samples",
    ], rows)
    notes = "Energy statistics aggregated from replication metrics."
    return table, notes


def _rg_dashboard(replication: pathlib.Path, expected: pathlib.Path, fixtures: pathlib.Path) -> Tuple[str, str]:
    couplings = _load_json(
        replication / "rg_couplings.json",
        expected / "rg_couplings.json",
        fixtures / "rg_couplings.json",
    )
    rows = []
    for key, value in sorted(couplings.items()):
        if isinstance(value, (int, float)):
            rows.append([key, f"{float(value):.4f}"])
        else:
            rows.append([key, json.dumps(value, sort_keys=True)])
    table = _render_table(["Metric", "Value"], rows)
    return table, "Coupling deltas recorded during the single-step RG run."


def _gaps_dashboard(replication: pathlib.Path, expected: pathlib.Path, fixtures: pathlib.Path) -> Tuple[str, str]:
    dispersion = _load_json(
        replication / "gaps_dispersion.json",
        expected / "gaps_dispersion.json",
        fixtures / "gaps_dispersion.json",
    )
    spectral = _load_json(
        replication / "gaps_spectral.json",
        expected / "gaps_spectral.json",
        fixtures / "gaps_spectral.json",
    )
    rows = []
    if dispersion:
        rows.append([
            "dispersion",
            f"{dispersion.get('gap_value', 0):.4f}",
            f"[{dispersion.get('ci', [0, 0])[0]:.4f}, {dispersion.get('ci', [0, 0])[1]:.4f}]",
            "yes" if dispersion.get("passes") else "no",
        ])
    if spectral:
        rows.append([
            "spectral",
            f"{spectral.get('gap_value', 0):.4f}",
            f"[{spectral.get('ci', [0, 0])[0]:.4f}, {spectral.get('ci', [0, 0])[1]:.4f}]",
            "yes" if spectral.get("passes") else "no",
        ])
    table = _render_table(["Method", "Gap", "CI", "Passes"], rows)
    return table, "Gap estimators executed against the release vacuum snapshot."


def _load_registry(registry: pathlib.Path, fixtures: pathlib.Path) -> List[Tuple[str, Dict[str, float]]]:
    sqlite_path = registry / "asm.sqlite"
    csv_path = registry / "asm.csv"
    rows: List[Tuple[str, Dict[str, float]]] = []
    if sqlite_path.exists():
        conn = sqlite3.connect(sqlite_path)
        try:
            cur = conn.execute("SELECT plan_name, metrics FROM runs")
            for plan, metrics_json in cur.fetchall():
                metrics = json.loads(metrics_json)
                rows.append((plan, metrics))
        finally:
            conn.close()
    elif csv_path.exists():
        with csv_path.open() as handle:
            reader = csv.DictReader(handle)
            for record in reader:
                rows.append((record["plan_name"], json.loads(record["metrics"])))
    else:
        fixture = fixtures / "registry.csv"
        if fixture.exists():
            with fixture.open() as handle:
                reader = csv.DictReader(handle)
                for record in reader:
                    rows.append((record["plan_name"], json.loads(record["metrics"])))
    return rows


def _ablations_dashboard(registry: pathlib.Path, fixtures: pathlib.Path) -> Tuple[str, str]:
    rows = _load_registry(registry, fixtures)
    aggregates: Dict[str, Dict[str, List[float]]] = defaultdict(lambda: defaultdict(list))
    for plan, metrics in rows:
        for key, value in metrics.items():
            if isinstance(value, (int, float)):
                aggregates[plan][key].append(float(value))
    table_rows: List[List[str]] = []
    for plan in sorted(aggregates):
        for key in sorted(aggregates[plan]):
            values = aggregates[plan][key]
            if not values:
                continue
            mean = sum(values) / len(values)
            table_rows.append([
                plan,
                key,
                f"{mean:.4f}",
                f"{min(values):.4f}",
                f"{max(values):.4f}",
            ])
    table = _render_table(["Plan", "Metric", "Mean", "Min", "Max"], table_rows)
    notes = "Ablation registry averages computed from the deterministic runs."
    return table, notes


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--replication", type=pathlib.Path, default=pathlib.Path("replication/out"))
    parser.add_argument("--expected", type=pathlib.Path, default=pathlib.Path("replication/expected"))
    parser.add_argument("--registry", type=pathlib.Path, default=pathlib.Path("registry"))
    parser.add_argument("--fixtures", type=pathlib.Path, default=pathlib.Path("fixtures/phase10"))
    parser.add_argument("--out", type=pathlib.Path, default=pathlib.Path("dashboards"))
    args = parser.parse_args()

    tables = {
        "vacua.md": _vacua_dashboard(args.replication, args.expected, args.fixtures),
        "rg_covariance.md": _rg_dashboard(args.replication, args.expected, args.fixtures),
        "gaps.md": _gaps_dashboard(args.replication, args.expected, args.fixtures),
        "ablations.md": _ablations_dashboard(args.registry, args.fixtures),
    }

    for name, (table, notes) in tables.items():
        _write_markdown(args.out / name, name.split(".")[0].replace("_", " ").title(), table, notes)


if __name__ == "__main__":
    main()
