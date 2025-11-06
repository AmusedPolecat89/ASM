# Phase 12 — Gauge Algebra Extraction API

Phase 12 introduces the `asm-gauge` crate and associated CLI commands that turn
Phase 11 spectrum artefacts and Phase 5 automorphism reports into deterministic
gauge algebra summaries. This document captures the public API, CLI surfaces,
JSON schemas, and determinism guarantees for the phase.

## Library API

The crate exposes the following entry points:

```rust
fn build_rep(spectrum: &SpectrumReport,
             aut: &AnalysisReport,
             opts: &RepOpts) -> Result<RepMatrices, AsmError>;
fn check_closure(rep: &RepMatrices, opts: &ClosureOpts) -> Result<ClosureReport, AsmError>;
fn decompose(rep: &RepMatrices, opts: &DecompOpts) -> Result<DecompReport, AsmError>;
fn ward_check(rep: &RepMatrices,
              ops: &OperatorsInfo,
              opts: &WardOpts) -> Result<WardReport, AsmError>;
fn analyze_gauge(spectrum: &SpectrumReport,
                 aut: &AnalysisReport,
                 ops: &OperatorsInfo,
                 opts: &GaugeOpts) -> Result<GaugeReport, AsmError>;
```

`RepOpts` controls the basis label, generator budget, and deterministic seed.
`ClosureOpts` and `WardOpts` expose the `tolerance` and `relative_tol` knobs
respectively, both defaulting to the Phase contract values (`1e-6` and `1e-5`).

### JSON Schemas

* `RepMatrices` — `{ basis: "modes", dim, gens: [{ id, matrix, norm }] }`
* `ClosureReport` — `{ closed, max_dev, structure_tensors: [{ i, j, k, value }] }`
* `DecompReport` — `{ factors: [{ type, dim, rank, invariants }], residual_norm }`
* `WardReport` — `{ max_comm_norm, pass, thresholds: { rel_tol } }`
* `GaugeReport` — `{ analysis_hash, graph_hash, code_hash, rep_hash, closure, decomp, ward, provenance }`

Floats are rounded to `1e-9` before serialisation and all payloads are emitted
through canonical JSON writers so byte-level comparisons are stable.

### Determinism

Identical `(spectrum_report, analysis_report, opts)` tuples (including seeds)
produce byte-identical JSON artefacts. The representation basis is diagonal,
commutators collapse exactly, and Ward residuals depend only on the operator
metadata, so repeated runs are deterministic across platforms.

## CLI Commands

* `asm-sim gauge` — Single-state analysis emitting `rep.json`, `closure.json`,
  `decomp.json`, `ward.json`, and `gauge_report.json`.
* `asm-sim gauge-batch` — Batch driver that pairs spectrum reports with
  automorphism reports (using graph/code hashes) and writes one gauge bundle per
  input plus an `index.json` manifest.
* `asm-sim gauge-compare` — Deterministic diff between two gauge reports.

Each command accepts `--closure-tol`, `--ward-tol`, and `--seed` flags mirroring
library options. The batch runner derives per-entry seeds via
`derive_substream_seed` to guarantee reproducible ordering and provenance.

## Artefacts & Provenance

* `fixtures/phase12/analysis/.../analysis_report.json` — Phase 5 analysis
  fixtures aligned with Phase 11 spectrum reports.
* `fixtures/phase12/t1_seed{0,1}/gauge_report.json` — Canonical outputs used by
  tests and upcoming phases.
* `repro/phase12/bench_gauge.json` — Throughput baseline captured during the
  Criterion benchmark.

`GaugeReport.provenance` records the commit hash (or crate version), seed, and
closure/Ward tolerances to make regression comparisons straightforward.

## Deterministic Workflows

1. Generate spectra via `asm-sim spectrum` (Phase 11).
2. Pair with Phase 5 analysis reports and run `asm-sim gauge` or
   `asm-sim gauge-batch`.
3. Optionally compare against existing reports using `asm-sim gauge-compare`.

Tests cover determinism, decomposition labelling, Ward tolerances, and JSON
round-trips. CI ensures gauge artefacts stay reproducible and the benchmark
baseline is refreshed whenever the implementation changes.
