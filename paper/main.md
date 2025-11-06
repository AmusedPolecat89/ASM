---
title: "ASM v1.0-rc Preprint"
author:
  - name: Avery Doe
  - name: Jordan Smith
keywords:
  - quantum error correction
  - renormalisation group
  - reproducibility
abstract: |
  We present the ASM v1.0 release candidate, a deterministic replication and
  experimentation stack for surface code vacua. The accompanying data products
  and dashboards are generated directly from the repository, enabling
  transparent evaluation of spectral gaps, RG flows, and ablation studies.
---

# Introduction

The ASM pipeline couples deterministic experiment orchestration with
reproducible analytics. Every figure in this preprint is generated from data
checked into the repository or produced by the replication pack.

# Methods

We rely on the `asm-exp` crate for deterministic deformation, sweep, gap, and
ablation experiments. Registry records are stored in SQLite and aggregated into
Markdown dashboards for review. Figures are regenerated via
`scripts/make_figures.py`, ensuring consistent styling and provenance.

# Results

## Vacuum stability

![Energy traces for release seeds](figures/energy_vs_sweep_seed0.pdf)
![Energy traces for release seeds](figures/energy_vs_sweep_seed1.pdf)

## Gap estimates

![Dispersion fit](figures/dispersion_seed0.pdf)

## RG covariance summary

![Covariance overview](figures/rg_cov.pdf)

## Ablation landscape

![Ablation summary](figures/ablations_overview.pdf)

# Discussion

The deterministic automation ensures that nightly builds and release candidates
share identical artefacts. Provenance for each figure is embedded via metadata
captured during the build.

# Reproducibility

* Replication pack: `./replication/run.sh`
* Dashboards: `python3 scripts/render_dashboards.py`
* Figures: `python3 scripts/make_figures.py`
* Paper: `bash scripts/build_paper.sh`

# References

Full references are managed in `refs.bib` and processed via Pandoc.

# Provenance

Commit: *$commit$*  
Build date: *$build_date$*
