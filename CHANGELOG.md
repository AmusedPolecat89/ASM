# Changelog

## [Unreleased]
### Added
- Phase 9 ablation and registry APIs in `asm-exp`, including deterministic `run_ablation` helpers, registry append/query utilities, and associated CLI plumbing in `asm-sim ablation`.
- Nightly and PR workflows, registry dashboards, and reproducibility scripts for automated ablation comparison and reporting.
- Phase 10 release tooling including the preprint build pipeline, Markdown dashboards, `asm-sim doctor/demo/version` commands, and publication metadata (CITATION, CONTRIBUTING, CODE_OF_CONDUCT, SECURITY).
- Phase 11 spectrum analysis crate (`asm-spec`) with deterministic operators, excitation, dispersion, and correlation helpers plus new `asm-sim spectrum` and `asm-sim spectrum-batch` subcommands.

### Changed
- Documented stability freeze expectations and added dashboards plus CHANGELOG gate for public API updates.
