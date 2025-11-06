# Phase 11 — Spectrum Analysis API

Phase 11 introduces the `asm-spec` crate which bundles deterministic utilities for
constructing effective operators, seeding excitations, running linear response probes,
and aggregating dispersion/correlation measurements into canonical `SpectrumReport`
artifacts.

## Public Rust API

The crate exposes high level helpers that orchestrate the entire pipeline:

- `build_operators(graph, code, opts)` constructs sparse operator bundles from a
  `HypergraphImpl` and matching `CSSCode`. The resulting `Operators` payload includes
  node-level degree summaries together with a canonical hash that is stable across
  platforms.
- `excite_and_propagate(ops, spec, opts)` seeds an excitation according to the
  provided `ExcitationSpec` and computes a deterministic linear response profile using
  `PropOpts` (iterations, tolerance, seed).
- `dispersion_scan(ops, spec, seed)` evaluates a momentum grid, extracts per-mode
  frequencies, and returns a `DispersionReport` with rounded floats (1e-9 granularity).
- `correlation_scan(ops, spec, seed)` measures two-point correlators, estimating
  correlation lengths with reproducible residuals stored in `CorrelationReport`.
- `analyze_spectrum(graph, code, opts)` executes the full workflow and returns a
  `SpectrumReport` combining operator metadata, dispersion and correlation outputs, and
  a provenance record describing the seeds and fit tolerances that were used.

All reports and intermediate structs derive `Serialize`/`Deserialize` and round-trip
through `to_canonical_json_bytes` / `from_json_slice` without reordering. Hashes are
computed over canonical JSON, so identical inputs plus identical seeds produce
byte-identical artifacts.

## CLI integration

`asm-sim` now exposes two subcommands powered by `asm-spec`:

```bash
asm-sim spectrum --input fixtures/validation_vacua/t1_seed0/end_state/ \
  --out analysis/spectrum/t1_seed0 --k-points 64 --modes 3 --seed 11001

asm-sim spectrum-batch --inputs fixtures/validation_vacua/**/end_state/ \
  --out analysis/spectra --k-points 64 --modes 3 --seed 12001
```

Both commands emit deterministic JSON artifacts (`operators.json`, `dispersion.json`,
`correlation.json`, `spectrum_report.json`). The batch variant writes one subdirectory
per input plus a top-level `index.json` summarising analysis hashes and relative paths.

## Determinism and tolerances

- All floating-point outputs are rounded to 1e-9 before serialization.
- Random choices (random low-weight excitations, dispersion jitter, correlation
  residuals) derive seeds via `asm_core::rng::derive_substream_seed` from the supplied
  master seed or propagation seed.
- Reports include hashes (`OperatorsInfo.hash`, `SpectrumReport.analysis_hash`) to make
  reproducibility checks straightforward.

## Performance notes

`spectrum_throughput.rs` records a baseline report under `repro/phase11/` and benchmarks
how quickly deterministic spectra can be computed on the bundled validation fixtures.
The pipeline intentionally uses CPU-only code paths so it can run inside CI and codespace
workflows; upgrading to GPU backends in the future would only require updating the
operator/excitation internals while preserving the same public API.

