# ASM v0.9 Preprint Replication Pack

This directory bundles the minimal end-to-end workflow used for the v0.9 “preprint” release. Running the pack executes a deterministic sampler sweep on two vacuum seeds, analyses the end states, performs a short RG flow, extracts effective couplings, measures both dispersion and spectral gaps, and assembles a runbook report.

## Quickstart

```bash
# From the repository root
cd replication
make run
```

The `run` target invokes [`run.sh`](./run.sh), which orchestrates the following steps:

1. Build `asm-sim` and execute `mcmc` on two curated seeds using `configs/short.yaml`.
2. Run dispersion diagnostics and symmetry scans for each sampler output.
3. Execute a single RG step and extract dictionary couplings from the resulting state.
4. Estimate dispersion and spectral gaps.
5. Assemble a runbook bundle and copy the Markdown report to `replication/out/report.md`.
6. Compare the produced artefacts against the fingerprints in [`expected/`](./expected/).

A successful run exits with code `0`, prints the location of the report, and leaves the following artefacts inside `replication/out/`:

- `run_seed*/manifest.json`, `metrics.csv`, and analysis subdirectories
- `rg/rg_run.json` and `extract/couplings.{json,csv}`
- `gaps/{dispersion.json,spectral.json}`
- `report.md`, `report_bundle/runbook.json`, and `report_bundle/summary.csv`
- Deterministic digest files used for verification (e.g. `graph_hashes.txt`, `metrics_digest.json`)

## Expected Outputs

The golden hashes and metric fingerprints live in [`expected/`](./expected/). The replication script canonicalises JSON before diffing, so differences must stem from genuine determinism regressions.

## Cleaning Up

Use `make clean` (or remove the `out/` directory manually) to reset the replication workspace.

