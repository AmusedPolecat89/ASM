# Phase 7 – Experiment Orchestration API

The `asm-exp` crate exposes deterministic helpers for scripted experiment suites:

- `deform` applies named, parameterised deformations to an ASM state and returns a
  `DeformationReport` containing canonical hashes and invariant flags.
- `sweep` expands a sweep plan (grid or Latin hypercube) into a set of reproducible
  jobs and records them inside a `SweepReport`.
- `estimate_gaps` provides dispersion and spectral gap surrogates with tightly
  controlled rounding and deterministic random seeds.
- `build_runbook` assembles a reproducibility manifest with hashed identifiers.

## Schemas

All reports serialise to canonical JSON using sorted object keys. This guarantees
byte-identical artefacts for identical inputs, enabling strict determinism in CI.

### `DeformationReport`

```jsonc
{
  "input_hash": "...",          // canonical state hash
  "deform_hash": "...",         // hash of spec + seed
  "params": { /* user spec */ },
  "n_ops": 0,
  "invariants_ok": true,
  "end_state_hashes": ["..."],
  "notes": "mode=graph ops=0"
}
```

### `SweepReport`

```jsonc
{
  "plan_hash": "...",
  "jobs": [
    {
      "params": {"degree_cap": 2},
      "seed": 123,
      "status": "completed",
      "out_dir": "job_0000",
      "end_hashes": ["..."]
    }
  ],
  "metrics": {"jobs": 4, "parallelism": 1}
}
```

### `GapReport`

```jsonc
{
  "method": "dispersion",
  "gap_value": 0.123,
  "ci": [0.11, 0.13],
  "residuals": [0.0, ...],
  "passes": true,
  "thresholds": {"max": 0.2}
}
```

### `RunBook`

```jsonc
{
  "id": "...",
  "created_at": "2024-01-01T00:00:00Z",
  "commit": "deadbeef",
  "seeds": [1, 2, 3],
  "inputs": ["runs/demo"],
  "artifacts": ["analysis/a.json"],
  "summary": {"jobs": 4}
}
```

## CLI extensions

`asm-sim` now exposes four subcommands:

- `asm-sim deform --input STATE_DIR --spec spec.yaml --seed 7101 --out analysis/deform/`
- `asm-sim sweep --plan sweeps.yaml --seed 8001 --out sweeps/run/`
- `asm-sim gaps --input STATE_DIR --method dispersion --out analysis/gaps.json`
- `asm-sim report --inputs sweeps/run/job_* --out summary/`

Each command emits canonical JSON artefacts aligned with the schemas above.

## Reproducibility

All deterministic helpers rely on canonical hashing from the core graph/code crates and
use fixed rounding to nine decimal places for floating point values. Seeds feed directly
into `StdRng`, ensuring identical sequences across platforms. Benchmarks write reference
reports to `repro/phase7/bench_sweep.json` to support regression tracking.
