# Contributing to ASM

Thank you for your interest in contributing to the ASM experiment workspace.
This document summarises the expectations for pull requests that target the
v1.0 release candidate stream.

## Getting started

1. Clone the repository and install the pinned Rust toolchain from
   `rust-toolchain.toml` (or run `rustup show` inside the Docker image).
2. Run the replication smoke test:
   ```bash
   ./replication/run.sh
   ```
3. Generate dashboards and figures to make sure your local environment can
   reproduce the release artefacts:
   ```bash
   python3 scripts/render_dashboards.py --fixtures fixtures/phase10 --out dashboards
   python3 scripts/make_figures.py --fixtures fixtures/phase10 --figures paper/figures
   ```

## Coding standards

* Keep all Rust changes formatted with `cargo fmt` and free from warnings under
  `cargo clippy --all-targets`.
* Avoid introducing new public API surface without updating `CHANGELOG.md`.
* Do not add large binary artefacts (>5 MB) to the repository.
* Prefer deterministic algorithms; if randomness is unavoidable, make the seed
  explicit and document it.

## Pull request checklist

* [ ] Tests updated or added (`cargo test`, doc tests, and relevant Python unit
      tests).
* [ ] Replication pack passes locally if artefacts are touched.
* [ ] Dashboards regenerated when registry schema or ablation summaries change.
* [ ] Paper figures and PDF rebuilt if the narrative or data changes.
* [ ] `CHANGELOG.md` updated when touching public APIs or CLI contracts.

For larger changes, open an RFC that outlines the motivation, design and impact
before sending a pull request. The governance group described in
`docs/GOVERNANCE.md` triages RFCs on a weekly basis.
