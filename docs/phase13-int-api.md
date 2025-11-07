# Phase 13 — Interaction & Running API

Phase 13 introduces the `asm-int` crate which provides deterministic building
blocks for few-body interaction experiments, scattering diagnostics and running
coupling summaries. The public API mirrors the contracts shared during the
phase kickoff and preserves deterministic ordering and rounding semantics.

## Public Rust API

The crate exports the following entry points:

```rust
use asm_int::{
    prepare_state, evolve, measure, fit_couplings, fit_running, interact,
    interact_full,
    PrepSpec, KernelOpts, MeasureOpts, FitOpts, RunningOpts,
    PreparedState, Trajectory, ObsReport, CouplingsFit, RunningReport,
};
```

* `prepare_state` selects and validates participants using Phase 11 spectrum
  artefacts and Phase 12 gauge metadata. The resulting `PreparedState` records a
  canonical `prep_hash`.
* `evolve` applies a reversible interaction kernel controlled via `KernelOpts`.
  The `Trajectory` summary includes the total simulated time, final norm and
  a deterministic `traj_hash`.
* `measure` converts a trajectory into deterministic observables (`ObsReport`)
  with canonical ordering, rounding to `1e-9` and confidence bands.
* `fit_couplings` produces `CouplingsFit` bundles using the measured
  observables. Confidence intervals and residuals follow fixed heuristics and
  serialise with canonical hashes.
* `fit_running` evaluates running couplings across an RG chain described by a
  slice of `StateRef` instances and returns a `RunningReport` with β-like
  summaries and validation flags.
* `interact` (and the convenience wrapper `interact_full`) orchestrate an
  entire experiment and produce an `InteractionReport` alongside the raw
  artefacts.

All error conditions are reported via `AsmError` without panics. Deterministic
rounding uses the helper `round_f64` (1e-9 precision) and every artefact
exposes a canonical SHA-256 hash.

## CLI extensions (`asm-sim`)

Four new commands are available:

* `asm-sim interact` — executes a single interaction experiment and persists
  `prepared_state.json`, `observables.json`, `couplings_fit.json` and
  `interaction_report.json`. `trajectory.json` is written when
  `KernelOpts::save_trajectory` is enabled.
* `asm-sim interact-batch` — evaluates a grid or LHS of experiments using glob
  selectors. Each job receives its own directory and `index.json` lists the
  emitted `interaction_report.json` files.
* `asm-sim fit-couplings` — runs the deterministic preparation/measurement fit
  pipeline without saving trajectories, focusing on coupling extraction.
* `asm-sim fit-running` — traverses an RG directory (`step_*` folders with
  `graph.json`/`code.json`) and emits `running_report.json` summarising
  couplings and β-estimates.

Configuration knobs are provided as YAML files matching the Phase 13 contract
(`PrepSpec`, `KernelOpts`, `MeasureOpts`, `FitOpts`, `RunningOpts`). Seeds are
propagated deterministically through `derive_substream_seed` and reported in the
interaction provenance bundle.

## JSON Schemas & Determinism

All artefacts serialise via `to_canonical_json_bytes`, guaranteeing deterministic
key ordering and float formatting. Canonical hashes are computed with SHA-256
from the canonical JSON representation and recorded for:

* `PreparedState` (`prep_hash`)
* `Trajectory` (`meta.traj_hash`)
* `ObsReport` (`obs_hash`)
* `CouplingsFit` (`fit_hash`)
* `InteractionReport` (`analysis_hash`)
* `RunningReport` (`running_hash`)

Confidence intervals and β-estimates use stable rounding to `1e-9`. The running
report includes `pass` flags comparing β-norms against
`RunningThresholds::beta_tolerance`.

## Performance & Modes

`KernelMode` controls the evolution workload:

* `Light` (default) caps steps at 128 for CI/Codespaces.
* `Fast` favours exploratory runs (≤64 steps, trajectory omitted by default).
* `Full` honours the configured step count for HPC/production runs.

The Criterion benchmark `benches/interact_throughput.rs` tracks interactions per
second and emits `repro/phase13/bench_interact.json` for reproducibility.

## Documentation & Cross-links

Rustdoc for `asm-int` documents every struct and option. This file is linked
from the top-level README so that Phase 13 users can discover the new API and
CLI contracts quickly.
