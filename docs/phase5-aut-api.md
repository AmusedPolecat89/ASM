# Phase 5 — Automorphisms, Invariants & Attractors

The `asm-aut` crate provides canonical symmetry scans, deterministic invariant
reports, and clustering utilities for analysing ASM checkpoints. This document
summarises the public API, CLI surface, JSON schemas, and determinism
contracts introduced during Phase 5.

## Crate API

The crate exposes three high-level entry points:

```rust
fn analyze_state(graph: &HypergraphImpl, code: &CSSCode, opts: &ScanOpts)
    -> Result<AnalysisReport, AsmError>;

fn compare(a: &AnalysisReport, b: &AnalysisReport) -> SimilarityScore;

fn cluster(reports: &[AnalysisReport], opts: &ClusterOpts) -> ClusterSummary;
```

`ScanOpts` controls spectral truncation (`laplacian_topk`, `stabilizer_topk`) and
allows callers to attach provenance metadata. `ClusterOpts` fixes the number of
clusters (`k`), iteration cap, deterministic seed, and optional representative
emission. All operations are deterministic—identical inputs produce identical
outputs.

`AnalysisReport` aggregates:

- `graph_aut`: group order, truncation flag, and orbit histogram.
- `code_aut`: CSS-preserving automorphism order and truncation flag.
- `logical`: logical ranks and commutation signature derived from
  `LogicalAlgebraSummary`.
- `spectral`: Laplacian and stabiliser Gram eigen spectra (top-k).
- `hashes`: canonical analysis hash plus graph/code structural hashes.
- `provenance`: seed, run identifier, checkpoint id, and commit hash.

`SimilarityScore` encodes a scalar distance in \[0,1] with per-component
contributions (`graph`, `code`, `logical`, `spectral`). `ClusterSummary`
contains `ClusterInfo { cluster_id, size, centroid_report_hash, members,
occupancy }` for each discovered attractor class.

## CLI Extensions

`asm-sim` gains two new analysis modes:

```bash
asm-sim analyze --input <RUN_OR_VACUUM> --symmetry-scan \
  --out <OUT_DIR> \
  --laplacian-topk 16 --stabilizer-topk 16

asm-sim analyze --cluster --inputs runs/**/analysis/ --out analysis/clusters/ \
  [--emit-representatives N]
```

`symmetry-scan` loads the cold checkpoint (or vacuum snapshot), computes an
`analysis_report.json`, writes `spectral.csv` with the eigen spectra, and
updates `index.json` describing analysed states. `cluster` consumes one or more
analysis directories and emits `cluster_summary.json` alongside a deterministic
index manifest. Optional `--emit-representatives` exports the hashes of the
closest members to each centroid for RG experiments.

## JSON Schema Overview

The JSON payloads match the Rust structures exactly. Key excerpts:

```json
{
  "graph_aut": {"order": 6, "gens_truncated": false, "orbit_hist": [4, 4]},
  "code_aut": {"order": 4, "gens_truncated": false, "css_preserving": true},
  "logical": {"rank_x": 2, "rank_z": 2, "comm_signature": "logical:2|..."},
  "spectral": {
    "laplacian_topk": [0.0, 1.5, 1.5],
    "stabilizer_topk": [3.0, 1.0]
  },
  "hashes": {
    "analysis_hash": "…",
    "graph_hash": "…",
    "code_hash": "…"
  },
  "provenance": {
    "seed": 1001,
    "run_id": "runs/t1_seed0/",
    "checkpoint_id": null,
    "commit": "<git sha>"
  }
}
```

`ClusterSummary` serialises as:

```json
{
  "clusters": [
    {
      "cluster_id": 0,
      "size": 12,
      "centroid_report_hash": "…",
      "members": ["…", "…"],
      "occupancy": 0.32
    }
  ]
}
```

## Determinism & Tolerances

- Canonical hashes use SHA-256 over normalised graph/code encodings; reports are
  content-addressed via `analysis_hash`.
- Laplacian and stabiliser eigenvalues are rounded to `1e-9` and sorted in a
  deterministic order before truncation.
- Clustering initialises centroids by sorting analysis hashes; k-means executes
  with a fixed iteration cap and produces stable membership assignments.
- JSON is emitted via `serde_json::to_string_pretty`, yielding byte-for-byte
  stable artefacts.

## Performance & Truncation

The current implementation exhaustively enumerates automorphisms for states with
≤7 nodes (graph) and ≤6 variables (code); larger systems fall back to lower
bounds with `gens_truncated = true`. Spectral computations use dense
`nalgebra` routines, adequate for the provided fixtures and validation vacua.

Benches (`cargo bench -p asm-aut scan_vacuum`) record timings for a small t1
vacuum and write metrics to `repro/phase5/bench_scan.json`.

## Integration Notes

- Reports feed directly into Phase 6 RG sampling via `AnalysisReport` hashes.
- `asm-sim analyze --cluster --emit-representatives N` surfaces per-cluster
  representatives for downstream experiments.
- Canonical hashes (`graph_hash`, `code_hash`, `analysis_hash`) should be stored
  alongside checkpoints to guarantee provenance tracking across phases.
