# Phase 6 — Renormalisation & Operator Dictionary

The `asm-rg` crate provides deterministic coarse-graining and coupling
extraction primitives used by the ASM pipeline. The implementation emphasises
reproducibility: every function is pure with respect to its inputs and options,
produces canonical hashes, and reports provenance in the emitted JSON artefacts.

## Public API

```rust
use asm_rg::{
    covariance::covariance_check,
    dictionary::extract_couplings,
    rg_run,
    rg_step,
    CovarianceReport,
    CouplingsReport,
    DictOpts,
    RGOpts,
    StateRef,
};
```

### RG primitives

* `rg_step(graph, code, opts) -> RGStep`
  * Runs a single coarse-graining step.
  * Returns the coarse graph/code together with an `RGStepReport` describing the
    scale factor, retained fraction, and canonical hashes.
* `rg_run(input, steps, opts) -> RGRun`
  * Applies `rg_step` sequentially and collects `RGRunEntry` summaries.
  * The embedded `RGRunReport` records the initial/final hashes, per-step
    metadata, and a deterministic `run_hash`.

### Dictionary extraction

* `extract_couplings(graph, code, opts) -> CouplingsReport`
  * Computes synthetic couplings `(c_kin, g1..g3, λ_h, yukawa[])` from simple
    structural features.
  * Emits confidence intervals, residual diagnostics, provenance, and a
    canonical `dict_hash`.

### Covariance check

* `covariance_check(input, steps, rg_opts, dict_opts) -> CovarianceReport`
  * Compares `extract_couplings(rg_step^K(state))` with a deterministic pushdown
    of `extract_couplings(state)` through the RG metadata.
  * Reports per-component deviations, thresholds, a pass/fail flag, and a
    canonical `covariance_hash`.

## JSON schemas

`serde_io` exposes helpers to serialise/deserialise the reports:

* `serde_io::step_to_json` / `step_from_json`
* `serde_io::run_to_json` / `run_from_json`
* `serde_io::couplings_to_json` / `couplings_from_json`
* `serde_io::covariance_to_json` / `covariance_from_json`

All serialised payloads are deterministic, prettified JSON strings suitable for
artifact storage.

## Determinism & Tolerances

* RG block formation is derived from a pure hash of node identifiers and the
  configured seed; identical inputs therefore produce identical partitions.
* Coarse graphs/codes are cloned via canonical serializers, ensuring stable
  hashes across executions.
* Couplings and covariance diagnostics are algebraic functions of basic counts
  and ranks. The default covariance thresholds are:
  * `|Δc_kin| / c_kin ≤ 5%`
  * `|Δg_i| ≤ 0.1`
  * `|Δλ_h| ≤ 0.1`
  * `|Δyukawa_i| ≤ 0.1`

## CLI integration

The `asm-sim` binary exposes three new commands:

* `asm-sim rg --input VACUUM_DIR --steps K --out OUT_DIR`
* `asm-sim extract --input STATE_DIR --out OUT_DIR`
* `asm-sim rg-covariance --input VACUUM_DIR --steps K --out OUT_DIR`

Each command writes deterministic JSON summaries (`rg_run.json`,
`couplings.json`, `covariance.json`) and auxiliary CSV data where applicable.

