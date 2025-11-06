# Phase 4 – Ensemble & Sampler API

This document captures the public contracts for the Phase 4 MCMC sampler.  It
complements the inline documentation of the `asm-mcmc` crate and describes the
configuration schema, scoring proxies, move semantics, determinism guarantees,
metrics, and persistence formats.

## 1. Public surface

The crate exposes the following primary entry points:

- `run(config: &RunConfig, seed: u64, code: &CSSCode, graph: &HypergraphImpl)`
  executes a deterministic ensemble sweep and returns a [`RunSummary`].
- `resume(path: &Path)` resumes from a previously written checkpoint and runs
  using the configuration embedded within that checkpoint.
- `score(code: &CSSCode, graph: &HypergraphImpl, weights: &ScoringWeights)`
  computes the weighted energy and the three proxy components used by the
  sampler.

The `RunSummary` contains:

| Field | Meaning |
| ----- | ------- |
| `acceptance_rates` | Per-move acceptance ratios aggregated across replicas. |
| `replica_temperatures` | Temperatures that define the replica ladder. |
| `exchange_acceptance` | Average acceptance probabilities recorded for each adjacent replica pair. |
| `coverage` | [`CoverageMetrics`] describing exploration quality. |
| `effective_sample_size` | Crude ESS proxy derived from the coverage statistics. |
| `final_code_hash` / `final_graph_hash` | Canonical hashes of the coldest replica at the end of the run. |
| `metrics_path` / `manifest_path` | Absolute paths to artefacts emitted during the run, if enabled. |
| `checkpoints` | Ordered list of checkpoint files written during execution. |
| `samples` | Recorded [`MetricSample`] entries useful for diagnostics and testing. |

All structs derive `Serialize` and `Deserialize`, allowing the caller to write
additional reports if required.

## 2. Configuration schema

The `RunConfig` type is serializable via `serde_yaml`.  Its relevant fields are:

- `sweeps`: number of sweeps executed during `run`.  Burn-in and thinning are
  controlled via `burn_in` and `thinning` fields.
- `ladder`: [`LadderConfig`] describing the base temperature and policy for
  constructing the replica ladder.  Two policies are supported:
  - `Geometric { ratio }`: multiplicative spacing with a deterministic lower
    bound of `ratio ≥ 1.01`.
  - `Manual { temperatures }`: explicit temperatures overriding `replicas`.
- `move_counts`: [`MoveCounts`] controlling how many proposals of each move type
  are attempted within each sweep.
- `checkpoint`: [`CheckpointConfig`] enabling periodic checkpointing.
- `scoring`: [`ScoringWeights`] applied to the energy proxies.
- `seed_policy`: records the master seed and optional label used for manifests.
- `output`: [`OutputConfig`] specifying directories for metrics, manifests,
  checkpoints, and end-state exports.  When omitted, all I/O is suppressed and
  the sampler operates purely in-memory.

Example YAML fragment:

```yaml
sweeps: 64
burn_in: 8
thinning: 2
ladder:
  replicas: 4
  base_temperature: 1.0
  policy:
    type: geometric
    ratio: 1.4
move_counts:
  generator_flips: 2
  row_ops: 2
  graph_rewires: 3
  worm_moves: 1
checkpoint:
  interval: 10
  directory: checkpoints
  max_to_keep: 5
scoring:
  cmdl: 1.0
  spec: 0.5
  curv: 0.25
output:
  run_directory: runs/phase4-demo
```

## 3. Energy proxies

The sampler energy is the weighted sum of three deterministic proxies:

1. **cMDL (`cmdl_proxy`)** – approximates the compressed description length of
   the CSS code by combining the number of generators, the average support size,
   and a logarithmic gap penalty inspired by Lempel–Ziv heuristics.
2. **Spectrum regularity (`spec_proxy`)** – penalises rank deficits and large
   variance in stabiliser supports; sparse codes receive an extra penalty to
   discourage extremely local generators.
3. **Curvature variance (`curv_proxy`)** – averages the variance of Forman edge
   and node curvatures computed via the Phase 2 graph engine.

Each proxy is computed via [`score`] and logged individually.  The total energy
is `weights.cmdl * cmdl + weights.spec * spec + weights.curv * curv`.

## 4. Move semantics

The move set covers three layers:

| Move kind | Behaviour |
| --------- | --------- |
| `GeneratorFlip` | Toggle the presence of a variable within a stabiliser.  The move rebuilds the CSS code and validates orthogonality, rejecting invalid modifications with `AsmError::Code`. |
| `RowOperation` | XOR two generators from the same stabiliser family.  The canonical form is recomputed; invalid moves surface the underlying error. |
| `GraphSwapTargets` | Swap the destination sets of two hyperedges using the Phase 2 rewiring helpers. |
| `GraphRetarget` | Remove one destination from an edge and replace it with another node. |
| `GraphResourceBalance` | Invoke the degree-aware balancing heuristic provided by `asm-graph`. |
| `WormSample` | Generate a logical worm/loop sample used purely for coverage diagnostics. |

Every accepted structural move recomputes the energy using the shared scoring
weights.  Forward and reverse proposal probabilities are recorded in
[`ProposalOutcome`], ensuring detailed balance verification is straightforward in
unit tests.

Invalid proposals never mutate state; they return an `AsmError` with structured
context and are counted as rejections in the move statistics.

## 5. Tempering and scheduling

The sampler implements a Metropolis–Hastings kernel with configurable move
counts followed by a parallel-tempering exchange stage.  For each adjacent pair
of replicas `(i, i+1)`, the exchange acceptance probability is

```text
exp(-(β_i - β_{i+1}) * (E_{i+1} - E_i))
```

clamped to `[0,1]`.  Deterministic substreams derived from the master seed drive
both proposal RNGs and exchange draws, ensuring reproducibility.

Per-sweep acceptance probabilities are accumulated and exposed via
`RunSummary.exchange_acceptance` for downstream diagnostics.

## 6. Metrics and coverage

`MetricsRecorder` collects per-sweep [`MetricSample`] entries comprising the
energy breakdown, acceptance counters, and canonical code/graph hashes.  The
aggregate [`CoverageMetrics`] report includes:

- unique structural hashes visited,
- the number of worm samples recorded,
- mean and variance of the total energy, and
- the average Jaccard similarity between consecutive generator signatures.

These quantities provide lightweight coverage signals without performing
expensive full-state comparisons.

## 7. Determinism guarantees

All randomness is derived from the master seed provided to `run` (or stored in a
checkpoint).  Substreams are deterministically generated via
`asm_core::derive_substream_seed` using the tuple `(replica_index, sweep,
move_slot)` for proposals and `(sweep, pair_index)` for replica exchanges.  This
ensures that replaying the same configuration and seed yields bit-identical
outputs, including metrics, manifests, and checkpoint contents.

Worm samples contribute to coverage metrics but never mutate the state,
preserving the exact trajectory taken by structural moves.

## 8. Persistence formats

### Checkpoints

Checkpoints are JSON payloads written to `output.checkpoint_dir` (default
`checkpoints/`).  Each file contains:

- the sweep number when it was written,
- the full `RunConfig` snapshot,
- the master seed, and
- per-replica serialized CSS codes, graphs, and energy breakdowns.

`resume(path)` loads the payload, reconstructs the ladder, and continues
execution using the embedded configuration.

### Manifest and metrics

When `output.run_directory` is provided, the sampler writes:

- `metrics.csv`: a tidy table of the recorded [`MetricSample`] entries;
- `manifest.json`: a [`RunManifest`] capturing the configuration, hashes, seed,
  and relative paths to generated artefacts;
- `end_state/code.json` and `end_state/graph.json`: canonical JSON exports of the
  coldest replica after the final sweep.

## 9. CLI integration

The Phase 4 CLI lives in the `asm-sim` crate and provides the command:

```text
asm-sim mcmc --config CONFIG.yaml --in STATE.json --out RUN_DIR/
```

where `STATE.json` references serialized code and graph inputs.  The CLI simply
parses the configuration, loads the inputs using the Phase 2/3 serialization
helpers, executes `asm_mcmc::run`, and writes the resulting summary JSON into
`RUN_DIR/summary.json` alongside the manifest, metrics, and checkpoints.

## 10. Testing and validation

The crate ships with unit tests covering detailed balance, exchange acceptance,
coverage improvements due to worm moves, deterministic replay, and checkpoint
round-trips.  The `sweep_throughput` criterion benchmark provides a baseline for
regression tracking of sweep throughput.

[`RunSummary`]: ../src/kernel.rs
[`CoverageMetrics`]: ../src/metrics.rs
[`MetricSample`]: ../src/metrics.rs
[`MoveCounts`]: ../src/config.rs
[`RunConfig`]: ../src/config.rs
[`LadderConfig`]: ../src/config.rs
[`CheckpointConfig`]: ../src/config.rs
[`ScoringWeights`]: ../src/config.rs
[`OutputConfig`]: ../src/config.rs
[`ProposalOutcome`]: ../src/kernel.rs
