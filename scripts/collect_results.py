#!/usr/bin/env python3
"""Aggregate heavy run artefacts into figures and a Markdown report."""

from __future__ import annotations

import argparse
import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Tuple

try:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
except ModuleNotFoundError as exc:
    raise SystemExit("Matplotlib is required to build the heavy run report. Install it via 'pip install matplotlib'.") from exc

try:
    import numpy as np
except ModuleNotFoundError as exc:
    raise SystemExit("NumPy is required to build the heavy run report. Install it via 'pip install numpy'.") from exc

args: argparse.Namespace = argparse.Namespace(figures=Path("."))


@dataclass
class BenchResult:
    name: str
    path: Path
    throughput: Optional[float]
    throughput_unit: Optional[str]
    runtime_seconds: Optional[float]
    notes: Optional[str]


def load_json(path: Path) -> Optional[Any]:
    try:
        with path.open("r", encoding="utf8") as handle:
            return json.load(handle)
    except FileNotFoundError:
        return None
    except json.JSONDecodeError:
        return None


def _find_numeric(data: Any, keys: Iterable[str]) -> Optional[Tuple[str, float]]:
    if isinstance(data, dict):
        for key in keys:
            value = data.get(key)
            if isinstance(value, (int, float)) and not math.isnan(float(value)):
                return key, float(value)
        for value in data.values():
            result = _find_numeric(value, keys)
            if result is not None:
                return result
    return None


def extract_bench(name: str, path: Path, payload: Any) -> BenchResult:
    throughput = None
    unit = None
    runtime = None
    notes = None

    if isinstance(payload, dict):
        if "notes" in payload and isinstance(payload["notes"], str):
            notes = payload["notes"].strip()
        if {
            "jobs",
            "seconds",
        }.issubset(payload.keys()):
            seconds = float(payload.get("seconds", 0.0) or 0.0)
            jobs = float(payload.get("jobs", 0.0) or 0.0)
            if seconds > 0.0:
                throughput = jobs / seconds
                unit = "jobs/s"
                runtime = seconds
        if throughput is None:
            result = _find_numeric(payload, [
                "samples_per_second",
                "jobs_per_second",
                "throughput",
                "items_per_second",
                "per_second",
            ])
            if result is not None:
                _, throughput = result
                unit = "ops/s"
        if runtime is None:
            result = _find_numeric(payload, [
                "seconds",
                "duration_seconds",
                "total_seconds",
            ])
            if result is not None:
                _, runtime = result
    return BenchResult(
        name=name,
        path=path,
        throughput=throughput,
        throughput_unit=unit,
        runtime_seconds=runtime,
        notes=notes,
    )


def ensure_directory(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)


def save_figure(fig: plt.Figure, out_dir: Path, stem: str) -> Tuple[Path, Path]:
    ensure_directory(out_dir)
    png_path = out_dir / f"{stem}.png"
    svg_path = out_dir / f"{stem}.svg"
    fig.savefig(png_path, bbox_inches="tight", dpi=160)
    fig.savefig(svg_path, bbox_inches="tight")
    plt.close(fig)
    return png_path, svg_path


def histogram_midpoints(edges: List[float]) -> List[float]:
    return [0.5 * (edges[idx] + edges[idx + 1]) for idx in range(len(edges) - 1)]


def build_cest_fig(histogram: Dict[str, Any]) -> Optional[Tuple[Path, Path]]:
    edges = histogram.get("edges", []) if isinstance(histogram, dict) else []
    counts = histogram.get("counts", []) if isinstance(histogram, dict) else []
    if not edges or not counts:
        return None
    total = float(sum(counts)) or 1.0
    mids = histogram_midpoints(edges)
    cumulative = np.cumsum(counts) / total
    fig, (ax_hist, ax_ecdf) = plt.subplots(1, 2, figsize=(10, 4))
    widths = [edges[i + 1] - edges[i] for i in range(len(edges) - 1)]
    ax_hist.bar(edges[:-1], counts, width=widths, align="edge", edgecolor="#333333", color="#4472c4")
    ax_hist.set_xlabel("c_est")
    ax_hist.set_ylabel("count")
    ax_hist.set_title("c_est histogram")
    ax_ecdf.step(mids, cumulative, where="post", color="#ed7d31")
    ax_ecdf.set_xlabel("c_est")
    ax_ecdf.set_ylabel("ECDF")
    ax_ecdf.set_ylim(0.0, 1.0)
    ax_ecdf.set_title("c_est ECDF")
    return save_figure(fig, args.figures, "c_est_distribution")


def build_hist_fig(name: str, histogram: Dict[str, Any]) -> Optional[Tuple[Path, Path]]:
    edges = histogram.get("edges", []) if isinstance(histogram, dict) else []
    counts = histogram.get("counts", []) if isinstance(histogram, dict) else []
    if not edges or not counts:
        return None
    fig, ax = plt.subplots(figsize=(6, 4))
    widths = [edges[i + 1] - edges[i] for i in range(len(edges) - 1)]
    ax.bar(edges[:-1], counts, width=widths, align="edge", edgecolor="#333333", color="#6aabd2")
    ax.set_xlabel(name)
    ax.set_ylabel("count")
    ax.set_title(f"{name} histogram")
    return save_figure(fig, args.figures, f"{name}_histogram")


def build_pass_rate_fig(rate: Optional[float]) -> Optional[Tuple[Path, Path]]:
    if rate is None:
        return None
    rate = max(0.0, min(1.0, rate))
    fig, ax = plt.subplots(figsize=(4, 4))
    ax.bar(["pass"], [rate * 100.0], color="#70ad47")
    ax.set_ylim(0.0, 100.0)
    ax.set_ylabel("Pass rate (%)")
    ax.set_title("Anthropic filter pass rate")
    return save_figure(fig, args.figures, "pass_rate")


def build_bench_fig(results: List[BenchResult]) -> Optional[Tuple[Path, Path]]:
    entries = [res for res in results if res.throughput is not None]
    if not entries:
        return None
    entries.sort(key=lambda item: item.throughput or 0.0, reverse=True)
    names = [res.name for res in entries]
    values = [res.throughput for res in entries]
    fig, ax = plt.subplots(figsize=(8, 4 + 0.3 * len(entries)))
    ax.barh(names, values, color="#5b9bd5")
    ax.invert_yaxis()
    ax.set_xlabel(entries[0].throughput_unit or "ops/s")
    ax.set_title("Benchmark throughput")
    return save_figure(fig, args.figures, "benchmark_throughput")


def build_runtime_fig(results: List[BenchResult]) -> Optional[Tuple[Path, Path]]:
    entries = [res for res in results if res.runtime_seconds is not None]
    if not entries:
        return None
    entries.sort(key=lambda item: item.runtime_seconds or 0.0, reverse=True)
    names = [res.name for res in entries]
    values = [res.runtime_seconds for res in entries]
    fig, ax = plt.subplots(figsize=(8, 4 + 0.3 * len(entries)))
    ax.barh(names, values, color="#ffc000")
    ax.invert_yaxis()
    ax.set_xlabel("seconds")
    ax.set_title("Benchmark runtime contributions")
    return save_figure(fig, args.figures, "benchmark_runtime")


def build_assertion_fig(summary: Dict[str, Dict[str, Any]]) -> Optional[Tuple[Path, Path]]:
    if not summary:
        return None
    names = sorted(summary.keys())
    passes = [summary[name]["pass"] for name in names]
    fails = [summary[name]["fail"] for name in names]
    fig, ax = plt.subplots(figsize=(8, 4 + 0.35 * len(names)))
    ax.barh(names, passes, color="#70ad47", label="pass")
    ax.barh(names, fails, left=passes, color="#c00000", label="fail")
    ax.invert_yaxis()
    ax.set_xlabel("count")
    ax.set_title("Assertion outcomes")
    ax.legend()
    return save_figure(fig, args.figures, "assertions_outcome")


def fmt_float(value: Optional[float], digits: int = 3) -> str:
    if value is None:
        return "—"
    if math.isnan(value):
        return "NaN"
    return f"{value:.{digits}f}"


def build_env_table(env: Dict[str, Any]) -> str:
    rows = [
        ("Timestamp", env.get("timestamp_utc", "unknown")),
        ("Git commit", env.get("git", {}).get("head")),
        ("Git describe", env.get("git", {}).get("describe")),
        ("Workspace dirty", str(env.get("git", {}).get("dirty"))),
        ("rustc", env.get("rustc_version")),
        ("cargo", env.get("cargo_version")),
        ("asm-sim", env.get("asm_sim_version")),
        ("CPU", env.get("cpu", {}).get("model")),
        ("Cores", env.get("cpu", {}).get("cores")),
        (
            "Memory (GB)",
            f"{env.get('memory_bytes') / 1e9:.2f}" if env.get("memory_bytes") else "—",
        ),
        ("GPU", "; ".join(env.get("gpu", {}).get("devices", [])) if env.get("gpu") else "—"),
        ("OS", env.get("os", {}).get("platform")),
        ("Kernel", env.get("os", {}).get("kernel")),
        ("Container", env.get("container")),
        ("Plan", env.get("plan_path")),
        ("Light mode", str(env.get("light_mode"))),
        ("Concurrency", env.get("concurrency")),
    ]
    header = "| Field | Value |\n| --- | --- |"
    body = "\n".join(f"| {field} | {value if value is not None else '—'} |" for field, value in rows)
    return f"{header}\n{body}"


def summarize_assertions(index_path: Path) -> Tuple[Dict[str, Dict[str, Any]], int]:
    summary: Dict[str, Dict[str, Any]] = {}
    total_reports = 0
    index_data = load_json(index_path)
    if not isinstance(index_data, list):
        return summary, total_reports
    for entry in index_data:
        if not isinstance(entry, dict):
            continue
        rel = entry.get("report")
        if not isinstance(rel, str):
            continue
        report_path = index_path.parent / rel
        payload = load_json(report_path)
        if not isinstance(payload, dict):
            continue
        checks = payload.get("checks", [])
        if not isinstance(checks, list):
            continue
        total_reports += 1
        for check in checks:
            if not isinstance(check, dict):
                continue
            name = check.get("name")
            if not isinstance(name, str):
                continue
            passed = bool(check.get("pass"))
            metric = check.get("metric")
            threshold = check.get("threshold") or check.get("range")
            record = summary.setdefault(name, {"pass": 0, "fail": 0, "metrics": []})
            if passed:
                record["pass"] += 1
            else:
                record["fail"] += 1
            if isinstance(metric, (int, float)):
                record["metrics"].append(float(metric))
            if threshold is not None:
                record.setdefault("threshold", threshold)
    return summary, total_reports


def build_bench_table(results: List[BenchResult]) -> str:
    header = "| Benchmark | Throughput | Runtime (s) | Notes |\n| --- | --- | --- | --- |"
    lines = []
    for res in sorted(results, key=lambda item: item.name):
        throughput = (
            f"{res.throughput:.3f} {res.throughput_unit}" if res.throughput is not None and res.throughput_unit else
            (f"{res.throughput:.3f}" if res.throughput is not None else "—")
        )
        runtime = f"{res.runtime_seconds:.3f}" if res.runtime_seconds is not None else "—"
        note = res.notes or ""
        lines.append(f"| {res.name} | {throughput} | {runtime} | {note} |")
    return "\n".join([header] + lines) if lines else "No benchmark artefacts were detected."


def summarize_landscape(summary_path: Path) -> Tuple[Dict[str, Any], Dict[str, Any]]:
    data = load_json(summary_path)
    distributions: Dict[str, Any] = {}
    metrics: Dict[str, Any] = {}
    if not isinstance(data, dict):
        return distributions, metrics
    distributions = data.get("distributions", {}) if isinstance(data.get("distributions"), dict) else {}
    quantiles = data.get("quantiles", {}) if isinstance(data.get("quantiles"), dict) else {}
    totals = data.get("totals", {}) if isinstance(data.get("totals"), dict) else {}
    pass_rates = data.get("pass_rates", {}) if isinstance(data.get("pass_rates"), dict) else {}
    metrics = {
        "quantiles": quantiles,
        "totals": totals,
        "pass_rate": pass_rates.get("anthropic"),
    }
    return distributions, metrics


def summarize_optional(path: Path) -> str:
    if path.is_file():
        return f"✅ present ({path})"
    if path.is_dir():
        return f"✅ present ({path})"
    return "⚠️ skipped"


def main() -> None:
    parser = argparse.ArgumentParser(description="Collate heavy run results")
    parser.add_argument("--in", dest="artifacts", required=True, help="Directory containing collected artefacts")
    parser.add_argument("--out", dest="out", required=True, help="Run output directory")
    args_parsed = parser.parse_args()

    artifacts = Path(args_parsed.artifacts).resolve()
    out_dir = Path(args_parsed.out).resolve()
    figures_dir = out_dir / "figures"
    ensure_directory(figures_dir)

    global args
    args = argparse.Namespace(figures=figures_dir)

    env = load_json(out_dir / "env.json") or {}
    status = load_json(artifacts / "status.json") or {}

    bench_results: List[BenchResult] = []
    for bench_file in sorted(artifacts.glob("repro/**/*.json")):
        rel = bench_file.relative_to(artifacts)
        name = str(rel.with_suffix(""))
        payload = load_json(bench_file)
        bench_results.append(extract_bench(name, rel, payload))

    bench_fig = build_bench_fig(bench_results)
    runtime_fig = build_runtime_fig(bench_results)
    bench_table = build_bench_table(bench_results)

    summary_path = artifacts / "runs" / "landscape" / "full" / "summary" / "SummaryReport.json"
    distributions, landscape_metrics = summarize_landscape(summary_path)

    cest_fig = build_cest_fig(distributions.get("c_est", {})) if distributions else None
    gap_fig = build_hist_fig("gap_proxy", distributions.get("gap_proxy", {})) if distributions else None
    xi_fig = build_hist_fig("xi", distributions.get("xi", {})) if distributions and "xi" in distributions else None
    pass_fig = build_pass_rate_fig(landscape_metrics.get("pass_rate"))

    assertions_index = artifacts / "runs" / "landscape" / "full" / "assertions" / "index.json"
    assertion_summary, assertion_jobs = summarize_assertions(assertions_index)
    assertion_fig = build_assertion_fig(assertion_summary)

    optional_status = {
        "paper": summarize_optional(artifacts / "paper" / "build" / "main.pdf"),
        "paper_figures": summarize_optional(artifacts / "paper" / "figures"),
        "site": summarize_optional(artifacts / "site" / "dist"),
    }

    report_lines: List[str] = []
    commit = env.get("git", {}).get("head", "unknown")
    timestamp = env.get("timestamp_utc", "unknown")
    report_lines.append(f"# ASM Heavy Run Report ({commit[:8]})")
    report_lines.append("")
    report_lines.append(f"_Generated at {timestamp}_")
    report_lines.append("")

    tests_ok = status.get("tests_passed")
    repl_ok = status.get("replication_passed")
    report_lines.append("## Status")
    status_items = [
        ("Tests", "✅" if tests_ok else "❌"),
        ("Replication", "✅" if repl_ok else "❌"),
    ]
    for label, emoji in status_items:
        report_lines.append(f"- {emoji} {label}")
    report_lines.append("")

    report_lines.append("## Environment")
    report_lines.append(build_env_table(env))
    report_lines.append("")

    report_lines.append("## Benchmarks")
    report_lines.append(bench_table)
    if bench_fig:
        png_path, _ = bench_fig
        report_lines.append("")
        report_lines.append(f"![Benchmark throughput]({png_path.relative_to(out_dir)})")
    if runtime_fig:
        png_path, _ = runtime_fig
        report_lines.append("")
        report_lines.append(f"![Benchmark runtimes]({png_path.relative_to(out_dir)})")
    report_lines.append("")

    report_lines.append("## Landscape Summary")
    totals = landscape_metrics.get("totals", {}) if landscape_metrics else {}
    if totals:
        jobs = totals.get("jobs", 0)
        passing = totals.get("passing", 0)
        report_lines.append(f"- Jobs analysed: **{jobs}**")
        report_lines.append(f"- Passing anthropic filters: **{passing}**")
    else:
        report_lines.append("Landscape summary unavailable (summary report missing).")
    quantiles = landscape_metrics.get("quantiles", {}) if landscape_metrics else {}
    for metric_name in sorted(quantiles.keys()):
        metric = quantiles.get(metric_name, {})
        report_lines.append(
            f"- {metric_name} quantiles (Q05/Q50/Q95): {fmt_float(metric.get('q05'))} / {fmt_float(metric.get('q50'))} / {fmt_float(metric.get('q95'))}"
        )
    if cest_fig:
        png_path, _ = cest_fig
        report_lines.append("")
        report_lines.append(f"![c_est distribution]({png_path.relative_to(out_dir)})")
    if gap_fig:
        png_path, _ = gap_fig
        report_lines.append("")
        report_lines.append(f"![gap proxy distribution]({png_path.relative_to(out_dir)})")
    if xi_fig:
        png_path, _ = xi_fig
        report_lines.append("")
        report_lines.append(f"![xi distribution]({png_path.relative_to(out_dir)})")
    if pass_fig:
        png_path, _ = pass_fig
        report_lines.append("")
        report_lines.append(f"![Anthropic pass rate]({png_path.relative_to(out_dir)})")
    report_lines.append("")

    report_lines.append("## Assertions")
    if assertion_summary:
        if assertion_jobs:
            report_lines.append(f"- Jobs evaluated: **{assertion_jobs}**")
        for name in sorted(assertion_summary.keys()):
            data = assertion_summary[name]
            total = data["pass"] + data["fail"]
            rate = data["pass"] / total if total else 0.0
            worst = max(data["metrics"]) if data.get("metrics") else None
            threshold = data.get("threshold")
            threshold_fmt = json.dumps(threshold) if threshold is not None else "n/a"
            report_lines.append(
                f"- **{name}**: pass rate {rate:.1%} (pass {data['pass']}, fail {data['fail']}), worst metric {fmt_float(worst)} vs {threshold_fmt}"
            )
        if assertion_fig:
            png_path, _ = assertion_fig
            report_lines.append("")
            report_lines.append(f"![Assertion outcomes]({png_path.relative_to(out_dir)})")
    else:
        report_lines.append("Assertions summary unavailable (index missing).")
    report_lines.append("")

    report_lines.append("## Optional Outputs")
    for key, message in optional_status.items():
        report_lines.append(f"- {key.replace('_', ' ').title()}: {message}")
    report_lines.append("")

    report_path = out_dir / "report.md"
    report_path.write_text("\n".join(report_lines), encoding="utf8")


if __name__ == "__main__":
    main()
