# Phase 15 — Theory Assertions & Manuscript Bundles

Phase 15 introduces the `asm-thy` crate and accompanying CLI plumbing for expressing
publication-grade theory checks against numeric artefacts. The goal is to produce
deterministic assertion reports and reproducible manuscript bundles from Phase 11–14
outputs.

## Public API

The crate exposes three primary entry points:

- `run_assertions(inputs: &AssertionInputs, policy: &Policy) -> Result<AssertionReport, AsmError>`
  executes the registered checks (Ward, closure, dispersion linearity, correlation–gap
  relation, coupling residuals, running sanity, and landscape filter rates) under a
  caller-provided policy.
- `crosscheck_numeric(symbolic: &SymExpr, numeric: &NumMat, policy: &Policy)` provides a
  reusable helper for comparing symbolic matrices against numeric evaluations using
  Frobenius norms and policy-controlled tolerances.
- `build_manuscript_bundle(src_roots: &[PathBuf], out: &Path, plan: &BundlePlan)` collects
  JSON/CSV artefacts and optional figures into `paper/inputs/` with canonical hashes.

`Policy` captures rounding, absolute/relative tolerances, closure and Ward requirements,
fit residual bounds, and acceptable anthropic pass-rate ranges. Policies are serializable
via YAML (`configs/phase15/policy_default.yaml`).

`AssertionReport` documents the outcome of each check along with provenance (policy and
input hashes) and a stable `analysis_hash`. `ManuscriptBundle` records copied inputs,
source mappings, and a `bundle_hash` suitable for manuscript automation.

## CLI extensions (`asm-sim`)

New top-level commands support theory validation workflows:

- `asm-sim assert` runs the assertion suite for a single vacuum:

  ```bash
  asm-sim assert \
    --spectrum fixtures/phase11/t1_seed0/spectrum_report.json \
    --gauge    fixtures/phase12/t1_seed0/gauge_report.json \
    --interact analysis/t1_seed0/interaction/interaction_report.json \
    --policy   configs/phase15/policy_default.yaml \
    --out      analysis/t1_seed0/assertions/
  ```

  Optional `--running` and `--summary` flags add Phase 13 running reports and Phase 14
  summaries to the assertion bundle.

- `asm-sim assert-batch` scans a landscape run, replays assertions per job, and emits
  `index.json` alongside per-job `assert_report.json` artefacts:

  ```bash
  asm-sim assert-batch \
    --root runs/landscape/smoke/ \
    --policy configs/phase15/policy_default.yaml \
    --out runs/landscape/smoke/assertions/
  ```

- `asm-sim paper-pack` builds deterministic manuscript inputs according to a bundle plan:

  ```bash
  asm-sim paper-pack \
    --roots runs/landscape/smoke/,fixtures/phase11/,fixtures/phase12/,analysis/ \
    --plan  configs/phase15/bundle.yaml \
    --out   paper/inputs/
  ```

`paper-pack` copies matching artefacts, optionally flattens paths, and writes
`paper/inputs/manifest.json` with canonical hashes for reproducibility.

## Determinism & Tolerances

- Assertions round metrics at `policy.rounding` (default `1e-9`) and compare against
  thresholds from the policy or artefact provenance.
- Failure notes capture short diagnostics without affecting the deterministic ordering
  of fields.
- Canonical JSON (stable key ordering) underpins all emitted reports and manifests;
  hashes derive from SHA256 digests of canonical serialisations.

Strict mode (`policy.strict = true`) enforces the presence of every input (spectrum,
interaction, running, and summary) and tightens failure semantics.

## Manuscript Bundle Plans

Bundle plans (`configs/phase15/bundle.yaml`) declare glob inclusions, whether to copy
figures, and whether to flatten output paths. The builder preserves deterministic
ordering and emits a manifest summarising source→destination mappings.

## Reproducibility Notes

- Regression tests cover determinism, policy strictness, symbolic cross-checks, and JSON
  round-trips.
- The Criterion benchmark (`cargo bench -p asm-thy --bench thy_assertions_throughput`)
  measures assertion throughput and refreshes `repro/phase15/bench_assert.json`.
- `paper/inputs/.gitkeep` keeps the target directory versioned while remaining empty
  until `paper-pack` executes.

Refer to Phase 13 and Phase 14 documentation for the upstream artefacts consumed by the
assertion suite.
