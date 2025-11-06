# ASM Governance

The ASM project is maintained by a small working group responsible for
triaging issues, reviewing pull requests, and publishing releases.

## Roles

* **Maintainers** – Own the technical roadmap, review changes, and operate the
  CI infrastructure.
* **Contributors** – Submit pull requests and RFCs; contributors may be granted
  maintainer status following sustained participation.
* **Advisors** – Provide strategic direction on research goals and review major
  architectural proposals.

## Decision process

1. Technical changes that affect public APIs or reproducibility guarantees must
   start with an RFC. RFCs are logged as issues tagged `rfc`.
2. The maintainers host a weekly sync to evaluate RFCs, review open pull
   requests, and plan releases.
3. Decisions are recorded in `docs/DECISIONS.md` with a short summary and link
   to relevant artefacts.

## Release management

* Release candidates (`vX.Y.0-rcZ`) must pass all CI gates including replication
  and paper builds.
* Stable releases are tagged after a one-week soak period without regressions.
* The governance group manages the Zenodo DOI mapping and ensures the citation
  file is up to date.
