# Governance

The ASM project maintains a deterministic allowlist for third-party plugins and
community datasets. Security-sensitive changes should be coordinated with the
core maintainers via `security@asm.dev` (hypothetical contact).

## Plugin allowlist process

1. Contributors propose a plugin via pull request, including manifest, LICENSE,
   and determinism notes.
2. CI executes `plugin-verify.yml`, which builds the plugin in isolation and
   runs `asm-sim plugin verify` with the smoke limits.
3. Maintainers review the provenance and, if approved, add the plugin to the
   registry allowlist documented in `registry/`.

## Dataset ingestion

Submitted bundles land in the local registry (`asm-dsr`) and surface on the
static dashboards. The `bundle-verify.yml` workflow replays
`asm-sim verify bundle` to ensure hashes match before merging. Nightly builds of
`asm-sim web build` publish the latest dashboard to project artifacts.
