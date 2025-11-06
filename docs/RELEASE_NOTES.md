# ASM v0.9 “Preprint” Release Notes

This preprint release packages a reproducible subset of the ASM workflow focused on deterministic sampler runs, lightweight analysis, and reporting.

## Included

- New `asm-exp` integration enabling deterministic deformations, sweeps, gap estimation, and runbook assembly
- `asm-sim` CLI extensions (deform, sweep, gaps, report) used by the replication script
- Replication pack (`replication/`) with curated configs, seed vacua, golden fingerprints, and one-button runner
- Docker image definition (`docker/Dockerfile`) mirroring the CI environment
- Release CI (`.github/workflows/release.yml`) that gates formatting, linting, tests, replication, and container builds
- Documentation on reproducibility guarantees (`docs/REPRODUCIBILITY.md`)

## Excluded

- Large-scale ensembles or long sweeps (configs are intentionally tiny)
- High-volume raw run directories—only minimal seeds and aggregated reports are versioned
- Automated publication of container images (CI builds the image but does not push to a registry)

## Extending the Pack

1. Duplicate the configs in `replication/configs/` and adjust parameters cautiously (keep sweeps ≤150, replicas ≤2).
2. Regenerate goldens by running `./replication/run.sh`, reviewing `replication/out`, and copying the updated digests into `replication/expected/`.
3. Update `docs/REPRODUCIBILITY.md` and `docs/RELEASE_NOTES.md` to reflect any new observables or tolerances.
4. Re-tag and trigger the release workflow to publish new artefacts.

