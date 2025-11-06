#!/usr/bin/env python3
"""Generate deterministic release figures from replication and registry data.

The script favours artefacts produced by the replication pack. When those
outputs are unavailable (for example on a clean checkout) it falls back to the
small fixtures in ``fixtures/phase10``. Figures are rendered using the Matplotlib
Agg backend to ensure byte-for-byte reproducibility across environments.
"""
from __future__ import annotations

import argparse
import csv
import json
import pathlib
from typing import Dict, Iterable, List, Tuple

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

plt.rcParams.update({
    "font.family": "DejaVu Sans",
    "figure.dpi": 150,
})

def _ensure_dir(path: pathlib.Path) -> None:
    path.mkdir(parents=True, exist_ok=True)


def _discover_energy_csv(root: pathlib.Path, fixtures: pathlib.Path) -> List[pathlib.Path]:
    candidates = sorted(root.glob("energy_vs_sweep_*.csv"))
    if candidates:
        return candidates
    return sorted(fixtures.glob("energy_vs_sweep_*.csv"))


def _discover_dispersion_csv(root: pathlib.Path, fixtures: pathlib.Path) -> List[pathlib.Path]:
    candidates = sorted(root.glob("dispersion_*.csv"))
    if candidates:
        return candidates
    fallback = fixtures / "dispersion_seed0.csv"
    return [fallback] if fallback.exists() else []


def _load_csv_series(path: pathlib.Path) -> Tuple[List[float], List[float]]:
    xs: List[float] = []
    ys: List[float] = []
    with path.open() as handle:
        reader = csv.DictReader(handle)
        fields = reader.fieldnames or []
        if {"sweep", "energy"}.issubset(fields):
            key_x, key_y = "sweep", "energy"
        elif {"k", "energy"}.issubset(fields):
            key_x, key_y = "k", "energy"
        elif {"index", "value"}.issubset(fields):
            key_x, key_y = "index", "value"
        else:
            key_x, key_y = fields[:2]  # type: ignore[index]
        for row in reader:
            xs.append(float(row[key_x]))
            ys.append(float(row[key_y]))
    return xs, ys


def _plot_energy(figures_dir: pathlib.Path, sources: Iterable[pathlib.Path]) -> None:
    for source in sources:
        xs, ys = _load_csv_series(source)
        seed = source.stem.split("_")[-1]
        fig, ax = plt.subplots(figsize=(4.8, 3.2))
        ax.plot(xs, ys, marker="o", color="#1b9e77", linewidth=1.5)
        ax.set_xlabel("Sweep")
        ax.set_ylabel("Energy")
        ax.set_title(f"Energy vs sweep ({seed})")
        ax.grid(True, alpha=0.3)
        out_path = figures_dir / f"energy_vs_sweep_{seed}.pdf"
        fig.tight_layout()
        fig.savefig(out_path)
        plt.close(fig)


def _plot_dispersion(figures_dir: pathlib.Path, sources: Iterable[pathlib.Path]) -> None:
    for source in sources:
        xs, ys = _load_csv_series(source)
        fig, ax = plt.subplots(figsize=(4.2, 3.2))
        ax.scatter(xs, ys, color="#d95f02")
        coeffs = _fit_parabola(xs, ys)
        dense_x = _linspace(min(xs), max(xs), 100)
        ax.plot(dense_x, [_eval_parabola(coeffs, x) for x in dense_x], color="#7570b3")
        ax.set_xlabel("k")
        ax.set_ylabel("Energy")
        ax.set_title(source.stem.replace("_", " "))
        ax.grid(True, alpha=0.3)
        out_path = figures_dir / f"{source.stem}.pdf"
        fig.tight_layout()
        fig.savefig(out_path)
        plt.close(fig)


def _fit_parabola(xs: List[float], ys: List[float]) -> Tuple[float, float, float]:
    if len(xs) < 3:
        return (0.0, 0.0, sum(ys) / len(ys))
    s_x2 = sum(x * x for x in xs)
    s_x3 = sum(x ** 3 for x in xs)
    s_x4 = sum(x ** 4 for x in xs)
    s_x = sum(xs)
    s_x2y = sum((x * x) * y for x, y in zip(xs, ys))
    s_xy = sum(x * y for x, y in zip(xs, ys))
    s_y = sum(ys)
    n = float(len(xs))

    matrix = [
        [s_x4, s_x3, s_x2],
        [s_x3, s_x2, s_x],
        [s_x2, s_x, n],
    ]
    rhs = [s_x2y, s_xy, s_y]
    return tuple(_solve_linear_system(matrix, rhs))  # type: ignore[return-value]


def _solve_linear_system(matrix: List[List[float]], rhs: List[float]) -> List[float]:
    mat = [row[:] + [val] for row, val in zip(matrix, rhs)]
    size = len(mat)
    for pivot in range(size):
        max_row = max(range(pivot, size), key=lambda r: abs(mat[r][pivot]))
        if abs(mat[max_row][pivot]) < 1e-12:
            continue
        if max_row != pivot:
            mat[pivot], mat[max_row] = mat[max_row], mat[pivot]
        factor = mat[pivot][pivot]
        mat[pivot] = [value / factor for value in mat[pivot]]
        for row in range(size):
            if row == pivot:
                continue
            scale = mat[row][pivot]
            mat[row] = [val - scale * ref for val, ref in zip(mat[row], mat[pivot])]
    return [row[-1] for row in mat]


def _linspace(start: float, stop: float, count: int) -> List[float]:
    if count <= 1:
        return [start]
    step = (stop - start) / (count - 1)
    return [start + step * i for i in range(count)]


def _eval_parabola(coeffs: Tuple[float, float, float], x: float) -> float:
    a, b, c = coeffs
    return a * x * x + b * x + c


def _plot_covariance(figures_dir: pathlib.Path, source: pathlib.Path | None) -> None:
    if source is None or not source.exists():
        return
    xs, ys = _load_csv_series(source)
    fig, ax = plt.subplots(figsize=(4.2, 3.0))
    ax.plot(xs, ys, marker="s", color="#66a61e")
    ax.set_xlabel("Index")
    ax.set_ylabel("|Δc_kin|")
    ax.set_title("RG covariance summary")
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    fig.savefig(figures_dir / "rg_cov.pdf")
    plt.close(fig)


def _plot_ablations(figures_dir: pathlib.Path, sources: Iterable[pathlib.Path]) -> None:
    items = []
    for source in sources:
        with source.open() as handle:
            data = json.load(handle)
        plan = data.get("plan_name", source.stem)
        kpis: Dict[str, List[float]] = data.get("kpis", {})
        for name, values in kpis.items():
            if not values:
                continue
            mean = sum(values) / len(values)
            lo = min(values)
            hi = max(values)
            items.append((plan, name, mean, lo, hi))
    if not items:
        return
    items.sort(key=lambda item: (item[0], item[1]))
    fig, ax = plt.subplots(figsize=(5.6, 3.4))
    positions = range(len(items))
    means = [item[2] for item in items]
    errors = [[item[2] - item[3] for item in items], [item[4] - item[2] for item in items]]
    ax.errorbar(positions, means, yerr=errors, fmt="o", color="#e7298a", capsize=4)
    ax.set_xticks(list(positions))
    ax.set_xticklabels([f"{plan}\n{name}" for plan, name, *_ in items], rotation=45, ha="right")
    ax.set_ylabel("KPI value")
    ax.set_title("Ablation KPI summary")
    ax.grid(True, axis="y", alpha=0.3)
    fig.tight_layout()
    fig.savefig(figures_dir / "ablations_overview.pdf")
    plt.close(fig)


def build_figures(replication: pathlib.Path, fixtures: pathlib.Path, figures: pathlib.Path) -> None:
    _ensure_dir(figures)
    energy_sources = _discover_energy_csv(replication, fixtures)
    if not energy_sources and replication.exists():
        metrics = sorted(replication.glob("run_*/metrics.csv"))
        energy_sources = metrics
    _plot_energy(figures, energy_sources)

    dispersion_sources = _discover_dispersion_csv(replication, fixtures)
    _plot_dispersion(figures, dispersion_sources)

    cov_source = None
    if (replication / "covariance_summary.csv").exists():
        cov_source = replication / "covariance_summary.csv"
    elif (fixtures / "covariance_summary.csv").exists():
        cov_source = fixtures / "covariance_summary.csv"
    _plot_covariance(figures, cov_source)

    ablation_root = replication / "ablations"
    if not ablation_root.exists():
        ablation_root = fixtures / "ablations"
    ablation_sources = sorted(ablation_root.glob("*.json"))
    _plot_ablations(figures, ablation_sources)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--replication", type=pathlib.Path, default=pathlib.Path("replication/out"))
    parser.add_argument("--fixtures", type=pathlib.Path, default=pathlib.Path("fixtures/phase10"))
    parser.add_argument("--figures", type=pathlib.Path, default=pathlib.Path("paper/figures"))
    args = parser.parse_args()

    build_figures(args.replication, args.fixtures, args.figures)


if __name__ == "__main__":
    main()
