# Contributing

We welcome deterministic contributions to the ASM ecosystem. Please ensure that
all new artefacts round-trip through the canonical JSON helpers and that CLI
interfaces follow the reproducibility contracts documented in the phase docs.

## Plugin checklist

Before submitting a plugin for inclusion in `plugins/examples/` or the
allowlist, ensure that:

1. `plugin.toml` declares `abi_version = 1` and the supported capabilities.
2. All entry points respect seeded randomness and return deterministic JSON.
3. The plugin ships with a LICENSE file and a short README describing usage.
4. You have run `asm-sim plugin install --registry registry/plugins/ <path>`
   followed by `asm-sim plugin verify --registry registry/plugins/ <name>`.

## Dataset submissions

Community bundles should be built with `asm-sim publish bundle` (or the
`scripts/pack_dataset.py` helper) and verified with
`asm-sim verify bundle`. Registries committed to the repository must only
include smoke-scale fixtures; larger runs should be published as release
artifacts.
