# Phase 9 — Ablations, Nightlies & Stability Freeze

Phase 9 introduces deterministic ablation experiments, registry tooling, and CI automation to guarantee reproducible comparisons across nightly and pull request builds.

## Public API Additions

The `asm-exp` crate now exposes:

- `run_ablation(plan: &AblationPlan, seed: u64) -> AblationReport`
- `registry_append(db: &Registry, report: &AblationReport)`
- `registry_query(db: &Registry, q: &Query) -> Table`

### AblationPlan Schema

```yaml
name: string                      # unique identifier for the plan
mode: grid | lhs                  # execution mode
samples: integer?                 # required when mode: lhs
factors:                          # ordered (BTreeMap) factors to sweep
  param_name: [values...]         # values (grid) or [min, max] for lhs
fixed:                            # optional fixed parameters
  key: value
tolerances:                       # KPI thresholds used for comparisons
  kpi_name:
    min: float?
    max: float?
    abs: float (default 1e-9)
    rel: float (default 1e-3)
```

For LHS plans each factor must provide at least two numeric entries `[min, max]`. Grid plans cartesian expand the provided value lists.

### AblationReport Schema

```json
{
  "plan_name": "topo_vs_worm",
  "plan_hash": "…",
  "jobs": [
    {
      "params": {"graph.degree_cap": 3, "moves.worm_weight": 0.0, …},
      "seed": 9002,
      "metrics": {
        "status": "completed",
        "kpis": {
          "exchange_acceptance": {"value": 0.31, "pass": true},
          "common_c_residual": {"value": 0.021, "pass": true}
        }
      }
    },
    …
  ],
  "summary": {
    "jobs": 4,
    "plan": "topo_vs_worm",
    "kpis": {
      "exchange_acceptance": {"mean": 0.29, "pass_rate": 1.0, "all_pass": true},
      "common_c_residual": {"mean": 0.018, "pass_rate": 1.0, "all_pass": true}
    },
    "provenance": {
      "created_at": "1970-01-01T00:00:00Z",
      "seed": 9001,
      "commit": "unknown"
    }
  },
  "artifacts": []
}
```

The `summary.provenance` fields are deliberately stable to preserve determinism across runs.

### Registry Schema

Registries can be stored either as CSV (`registry/asm.csv`) or SQLite (`registry/asm.sqlite`). Rows follow:

| Column    | Description |
|-----------|-------------|
| date      | ISO timestamp (from report provenance) |
| commit    | Git commit (from report provenance) |
| plan_name | Ablation plan identifier |
| plan_hash | Stable hash of plan+seed |
| job_id    | Zero-based job index |
| params    | Canonical JSON for parameters |
| metrics   | Canonical JSON for metrics payload |

Dashboards produced by `scripts/summarize_registry.py` emit:

- `dashboards/kpi_trends.csv`
- `dashboards/kpi_trends.md`

These files summarise KPI means, standard deviations, and pass rates grouped by plan.

## CLI Additions

`asm-sim` now exposes an `ablation` subcommand:

```bash
asm-sim ablation --plan ablation/plans/topo_vs_worm.yaml \
                 --seed 9001 \
                 --out ablation/out/topo_vs_worm \
                 --registry registry/asm.sqlite
```

The command writes `ablation_report.json`, `summary.json`, and per-job folders under the output directory. When `--registry` is supplied the report is appended to the registry in a deterministic fashion.

## Scripts & Automation

- `scripts/run_ablation.sh` orchestrates plan execution, registry appends, and golden comparisons via `scripts/compare_to_golden.py`.
- `scripts/compare_to_golden.py` validates reports against `ablation/goldens/*.gold.json` using tolerances from the plan.
- `scripts/extract_goldens.py` writes canonical goldens for a given report.
- `scripts/summarize_registry.py` converts registry entries into CSV/Markdown dashboards for long-term trend tracking.

## CI Workflows

Two GitHub Actions workflows guard Phase 9 functionality:

1. `pr-validation.yml` (pull requests)
   - runs formatting, clippy, tests, and doc tests
   - executes the short `topo_vs_worm` ablation and compares against goldens
   - fails on size or CHANGELOG gate violations
   - posts KPI deltas as a PR comment

2. `nightly.yml` (scheduled)
   - executes the full ablation suite (all three plans)
   - appends results to the registry and publishes dashboards and reports as artifacts

## Stability Freeze

Phase 9 introduces a stability gate: public API changes in `crates/*/src/lib.rs` or CLI command modules under `crates/asm-sim/src/commands/` require a matching `CHANGELOG.md` update. The CI workflows enforce this requirement and ensure deterministic artifacts (no timestamps outside provenance).

