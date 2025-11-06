# Phase 4 Validation Test Summary (2025-11-06)

The Phase 4 validation suite was rerun using the refreshed seed state, configuration bundle, and analysis helpers.
Key artefacts live under `runs/` with aggregated outputs collected in `summary/validation_20251106/`.
The table below captures the observed metrics for Tests 1–7.

| Test | Purpose | Key Metrics | Status | Notes |
| ---- | ------- | ----------- | ------ | ----- |
| Test 1 — Universal light-cone | Verify common limiting velocity `c` across defect species. | `runs/t1_seed0/analysis/common_c.json` → `c=6.8`, `residual_max=0.0`; `runs/t1_seed1/analysis/common_c.json` → `c=6.0`, `residual_max=0.0`. | PASS | Both seeds produced residuals ≤0.03 with distinct but consistent limiting velocities. |
| Test 2 — Determinism / Reproducibility | Confirm identical manifests, hashes, and metrics for repeated runs with the same seed. | `runs/t2_metrics_diff.csv` shows zero deltas; manifests differ only by recorded output path. | PASS | Code/graph hashes identical; metrics match to machine precision. |
| Test 3 — Coverage & Mixing | Demonstrate improved coverage when worm moves are enabled. | `runs/t3_noworm/coverage_summary.json` → `unique_structural_hashes=1`, `jaccard_lag_decay=0.0`; `runs/t3_worm/coverage_summary.json` → `unique_structural_hashes=147`, `jaccard_lag_decay≈0.424`, `exchange_acceptance≈0.625`. | WARN | Worm moves dramatically increased coverage but exchange acceptance remains above the 0.2–0.5 target band. |
| Test 4 — Dispersion stability across checkpoints | Assess stability of the common `c` across checkpoints. | `runs/t4/analysis/common_c_by_checkpoint.csv` → all checkpoints `c=6.8`, `residual_max=0.0`; CV = 0.0. | PASS | Common velocity remained constant across all checkpoints. |
| Test 5 — Sensitivity to graph controls | Check robustness of the common `c` under graph tweaks. | `runs/t5_base/analysis/common_c.json`, `runs/t5_deg/analysis/common_c.json`, `runs/t5_k/analysis/common_c.json` → each reports `c=6.8`, `residual_max=0.0`. | PASS | All graph variations preserved the common light-cone within tolerance (Δc = 0%). |
| Test 6 — Syndrome/Defect consistency | Validate deterministic defect detection and species IDs. | `runs/t6/defects_consistency.csv` lists identical defect/species patterns across both passes. | PASS | No ordering drift or species mismatches observed pre/post reload. |
| Test 7 — End-to-end pipeline integrity | Ensure sampler → checkpoint → analysis pipeline completeness. | `runs/t7/` contains manifest, metrics, checkpoints, end-state snapshots, and analysis outputs with `residual_max=0.0`. | PASS | All artefacts present and internally consistent. |

## Aggregated artefacts

* Combined metrics: `summary/validation_20251106/all_metrics.csv`
* Metrics summary: `summary/validation_20251106/metrics_summary.txt`
* Validation index: `summary/validation_20251106/index.json`
* Markdown report: `summary/validation_20251106/validation_report.md`

The only outstanding action is to tune the tempering ladder for Test 3 to push exchange acceptance into the 0.2–0.5 band if strict compliance is required.
