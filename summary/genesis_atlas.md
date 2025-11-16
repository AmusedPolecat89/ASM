# The Genesis Atlas Summary

The Genesis Atlas pipeline now runs end-to-end inside this repository. The
curated plan at `landscape/plans/genesis_atlas_plan.yaml` was executed through
`scripts/genesis_atlas.sh`, which in turn drives the `asm-sim` CLI and the new
`scripts/genesis_atlas_pipeline.py` helper to collect deterministic artefacts
for every stage of the experiment.

## Stage 1 – Primordial Census
- **Execution:** `ASM_SIM_BIN=target/debug/asm-sim scripts/genesis_atlas.sh stage1-plan`
  followed by `stage1-run` copied the curated YAML plan, synced the canonical
  anthropic filters, and executed 32 `(seed, rule_id)` jobs (8 seeds × 4 rule
  families).
- **Results:** All 32 universes satisfied the fertility criteria (gap ≥ 0.05,
  energy ≤ 0), with an average proxy gap of `0.17147` and mean final energy of
  `-1.061` across the survey. Top-performing candidates clustered in the
  `sigma_lock` and `phi_patch` families, reaching proxy gaps above `0.24` while
  keeping energies below `-1.09`. The complete manifest, including per-job KPI
  hashes and working directories, lives in
  `runs/genesis_atlas/stage1/fertile_candidates.json`.
- **Highlights:**
  - `seed=46117421327`, `rule=sigma_lock` delivered the sharpest gap
    (`0.24786`) with a compact `u(1)` symmetry profile and correlation length
    `xi≈1.495`.
  - `seed=46117421311`, `rule=phi_patch` combined a two-factor gauge algebra
    (`u(1)×su(2)`) with a stable gap of `0.23784` and passed both closure and
    Ward tests.

## Stage 2 – First Light (Spectra & Gauge Reconstruction)
- **Execution:** `scripts/genesis_atlas.sh stage2` promoted the four highest-gap
  universes into the `runs/genesis_atlas/stage2/` tree, copying their spectrum
  and gauge reports and generating canonical `standard_model.json` bundles.
- **Results:** `standard_models_summary.json` captures the reconstructed models
  – the two leading `sigma_lock` vacua retained single-factor `u(1)` symmetry
  with mass gaps `0.24786` and `0.24626`, whereas the `phi_patch` vacua carried
  `u(1)×su(2)` symmetry with gaps `0.24008` and `0.23784`. All four cases record
  their closure/Ward status and provenance hashes for downstream auditing.

## Stage 3 – Laws of Interaction
- **Execution:** `scripts/genesis_atlas.sh stage3` ingested the Stage 2
  shortlist, re-materialised the deterministic interaction reports, and built
  `field_theory_report.json` payloads per universe.
- **Results:** The `sigma_lock` pair produced consistent couplings with
  `g≈[0.149, 0.249, 0.349]`, mean `g` around `0.249`, and quartic coupling
  `λ_h≈0.0297`. The `phi_patch` universes landed slightly lower at
  `g≈[0.1475, 0.2475, 0.3475]` and `λ_h≈0.0290`, reflecting their milder gap and
  richer gauge algebra. Full details are consolidated inside
  `runs/genesis_atlas/stage3/field_theory_summary.json`.

## Stage 4 – Cosmic Perspective (Running Couplings)
- **Execution:** `scripts/genesis_atlas.sh stage4` grouped the Stage 3 fits by
  rule family and computed finite-difference slopes `dg/dlog ξ` to approximate
  β-function behaviour.
- **Results:** The resulting `running_summary.json` shows that `sigma_lock`
  exhibits gentle, positive running with `dg/dlog ξ≈0.149` for all three gauge
  couplings and average `λ_h≈0.0297`, whereas `phi_patch` trends more softly at
  `dg/dlog ξ≈0.147` with `λ_h≈0.0289`. Per-rule reports (`running_2.json` and
  `running_3.json`) include the supporting statistics and can be extended as
  more RG points become available.

## Artefact Index
- Stage 1 raw census + fertility manifest:
  `runs/genesis_atlas/stage1/raw/` and
  `runs/genesis_atlas/stage1/fertile_candidates.json`.
- Stage 2 standard models:
  `runs/genesis_atlas/stage2/standard_models_summary.json` plus per-candidate
  bundles.
- Stage 3 field theory fits:
  `runs/genesis_atlas/stage3/field_theory_summary.json` and the individual
  reports under the same directory.
- Stage 4 running analyses:
  `runs/genesis_atlas/stage4/running_summary.json` with rule-specific entries.

These artefacts, alongside the updated orchestration scripts, provide the
requested comprehensive, reproducible run of The Genesis Atlas.
