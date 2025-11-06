# Reproducibility Guide (v0.9 Preprint)

This document records the environment, inputs, and determinism guarantees that underpin the v0.9 “preprint” replication pack.

## Environment & Tooling

- **Rust toolchain:** `1.75.0` (installed via `rustup`, minimal profile)
- **Host operating system:** Ubuntu 22.04 LTS (used in CI and Docker image)
- **Required system packages:** `build-essential`, `libssl-dev`, `pkg-config`, `curl`, `git`, `jq`, `python3`
- **Primary entry point:** [`replication/run.sh`](../replication/run.sh), which invokes `cargo run --bin asm-sim` for every subcommand
- **Docker reference:** [`docker/Dockerfile`](../docker/Dockerfile) builds the pinned environment and uses the same replication script as the container `CMD`

CI enforces the toolchain pin through the release workflow, and the Docker image offers an auditable snapshot of the dependencies.

## Seed Fixtures

The replication pack ships two compact vacua under [`replication/seeds/`](../replication/seeds/):

- `state_seed0.json` → `state_seed0/{code.json,graph.json}`
- `state_seed1.json` → `state_seed1/{code.json,graph.json}`

Both originate from the validated fixtures in `fixtures/validation_vacua` and contain only the final end-state serialisations (no large run directories).

## Deterministic Outputs

The replication script always executes the same ordered sequence:

1. `asm-sim mcmc` on each seed using `replication/configs/short.yaml`
2. `asm-sim analyze` (dispersion) and `asm-sim analyze --symmetry-scan`
3. `asm-sim rg --steps 1` followed by `asm-sim extract`
4. `asm-sim gaps --method dispersion` and `asm-sim gaps --method spectral`
5. `asm-sim report` to assemble the runbook

Deterministic fingerprints are written to `replication/out/` and compared against the goldens in [`replication/expected/`](../replication/expected/):

- `graph_hashes.txt` / `code_hashes.txt`: SHA-256 digests of the end-state graph and code JSON
- `common_c.json`: canonical JSON combining the dispersion summaries for both seeds
- `metrics_digest.json`: min/mean/max energy, last energy sample, and sample counts
- `gaps_{dispersion,spectral}.json`: canonicalised gap reports (sorted via `jq -S`)
- `rg_couplings.json`: canonical dictionary extraction report

All JSON artefacts are canonicalised (sorted keys, trailing newline) prior to comparison. Numerical tolerances mirror the CLI defaults:

- Dispersion residual tolerance: `0.03`
- Spectral gap rounding: intrinsic estimator precision, persisted at 1e-9 via canonical JSON

A mismatch in any digest causes the replication script to exit non-zero.

## Continuous Integration Guarantees

The release workflow (`.github/workflows/release.yml`) triggers on the `v0.9.0-preprint` tag and performs the following gates:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test`
- `cargo test --doc`
- `./replication/run.sh`
- `docker build -f docker/Dockerfile` and `docker run --rm asm-preprint:latest`

The workflow uploads the replication outputs, deterministic fingerprints, and benchmark baselines (`repro/**/*bench*.json`) as release artefacts.

## Floating-Point Stability

Gap estimates and dispersion fits are deterministic given the pinned toolchain. All floating-point payloads are written with full precision and diffed in canonical JSON form. No dynamic timestamps are embedded in the checked artefacts.

