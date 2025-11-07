# Phase 14 — Landscape Enumeration & Anthropic Scan

The `asm-land` crate orchestrates deterministic landscape explorations by combining existing
Phase 1–13 stages into reproducible multi-job pipelines. This document summarises the public API,
CLI entry points, canonical artefacts, and reproducibility guarantees introduced in Phase 14.

## Public API

The crate exposes four primary entry points:

```rust
use asm_land::{load_plan, run_plan, build_atlas, summarize};
```

- `load_plan<P: AsRef<Path>>(path: P) -> Result<Plan>` parses a YAML landscape plan, normalises
  the seed and rule ordering, records the base directory for relative paths, and computes the plan
  hash using canonical JSON serialisation.
- `run_plan(plan: &Plan, out: &Path, opts: &RunOpts) -> Result<LandscapeReport>` executes the
  plan deterministically, synthesising stage artefacts (`mcmc/`, `spectrum/`, `gauge/`, `interact/`)
  alongside canonical KPIs, hashes, and the aggregate `landscape_report.json`. Resume semantics are
  honoured when `RunOpts::resume` is enabled.
- `build_atlas(root: &Path, opts: &AtlasOpts) -> Result<Atlas>` walks an existing run directory and
  produces a compact atlas manifest capturing the universes discovered. Entries are ordered and
  hashed canonically to guarantee byte-stable JSON.
- `summarize(root: &Path, filt: &FilterSpec) -> Result<SummaryReport>` replays anthropic filters
  against the stored KPIs, generating deterministic histograms, quantiles, and correlation summaries.

Supporting modules provide deterministic hashing (`hash`), canonical JSON helpers (`serde`),
statistical aggregation (`stat`), anthropic filters (`filters`), and stage synthesis (`stages`).

## Canonical Artefacts

Running a plan produces the following directory structure for each `(seed, rule_id)` pair:

```
<root>/<seed>_<rule_id>/
  mcmc/manifest.json
  spectrum/spectrum_report.json
  gauge/gauge_report.json
  interact/interaction_report.json
  kpi.json
  hashes.json
```

The root directory also receives `landscape_report.json`. Subsequent summarisation populates:

```
summary/summary_report.json
atlas/atlas.json
```

All JSON payloads are emitted via `asm_land::serde::to_canonical_json_bytes`, ensuring stable field
ordering and pretty-printed output with 1e-9 rounding inherited from earlier phases. Benchmarks store
results in `repro/phase14/bench_landscape.json` for reproducibility.

## CLI Integration (`asm-sim landscape`)

The `asm-sim` binary now exposes a `landscape` command with four subcommands:

- `plan` — synthesise a deterministic plan YAML based on CLI knobs (seed count, graph size, sampler
  sweeps, interaction steps, etc.).
- `run` — execute a plan with optional resume support and emit canonical artefacts under the chosen
  output directory.
- `summarize` — apply an anthropic filter specification and export `summary_report.json`.
- `atlas` — build a compact atlas manifest with optional inclusion of failed jobs.

Example workflows:

```bash
asm-sim landscape plan --out landscape/plans/smoke.yaml \
  --seeds 3 --size 128 --degree 3 --k 3 --sweeps 200 --worm 0.3 \
  --kpoints 32 --modes 2 --steps 64 --dt 0.02

asm-sim landscape run --plan landscape/plans/smoke.yaml \
  --out runs/landscape/smoke/ --resume

asm-sim landscape summarize --root runs/landscape/smoke/ \
  --filters landscape/filters/default.yaml --out runs/landscape/smoke/summary/

asm-sim landscape atlas --root runs/landscape/smoke/ \
  --out runs/landscape/smoke/atlas/
```

## Determinism and Resume Semantics

- Stage artefacts are synthesised from `(seed, rule_id)` pairs with fixed formulas, ensuring
  repeatable KPIs, hashes, and filter outcomes.
- Resuming a partially completed run reuses existing `kpi.json` and `hashes.json` files, only
  recomputing missing artefacts while yielding byte-identical reports.
- Filter predicates (`closure`, `ward`, `c_range`, `gap_ok`, `factors`) are pure functions of the
  stored KPIs and therefore stable across repeated evaluations.

## Statistical Summaries

`StatsSummary::from_kpis` produces:

- Fixed-bin histograms for `c_est` and `gap_proxy`.
- Deterministic quantiles (`q05`, `q50`, `q95`).
- Pearson and Spearman correlations for the `(c_est, gap_proxy)` pair.

These aggregates underpin the `SummaryReport`, which also tracks total job counts and anthropic pass
rates.

## Benchmarks

`cargo bench -p asm-land --bench landscape_throughput` runs a light-weight smoke plan, records the
wall-clock baseline in `repro/phase14/bench_landscape.json`, and exercises the full pipeline
(run → summarize → atlas) to ensure end-to-end coverage.
