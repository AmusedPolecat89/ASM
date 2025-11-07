# Phase 16 Plugins

Phase 16 introduces the sandboxed plugin host exposed by the `asm-host` crate
and the `asm-sim plugin` CLI. Plugins describe their capabilities through a
`plugin.toml` manifest and expose a versioned ABI (`ASM_ABI_VERSION = 1`).

## ABI overview

The host expects every plugin to export an `AsmPluginInfo` structure and an
`AsmPluginVTable`. The info block advertises the ABI version, the plugin name,
and the bitflag capability mask. The vtable contains optional entry points for
each stage (`graph_generate`, `code_generate`, `spectrum`, `gauge`,
`interact`). Entry points receive canonical JSON payloads and must return an
`AsmStatus`.

## Sandboxing

`configs/phase16/limits.yaml` controls the wall-clock, CPU, and memory caps.
`asm_host::SandboxGuard` enforces these limits and surfaces deterministic error
classes if a plugin exceeds them. File system access is restricted to the
runtime scratch directory supplied by the host.

## Registry workflow

`asm-sim plugin install --registry registry/plugins/ path/to/plugin.toml`
validates the manifest, records canonical hashes, and stores the optional
plugin binary. `asm-sim plugin verify` re-hashes the stored binary and manifest
to guarantee deterministic installs. The examples in
`plugins/examples/` provide stubs for graph, code, and spectrum providers.
