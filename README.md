# ASM

Core workspace for the ASM engine prototypes.

* [Phase 1 API](docs/phase1-api.md)
* [Phase 2 Graph API](crates/asm-graph/docs/phase2-graph-api.md)
* [Phase 3 Code API](crates/asm-code/docs/phase3-code-api.md)
* [Phase 4 Ensemble API](crates/asm-mcmc/docs/phase4-ensemble-api.md)

## CLI utilities

The `asm-sim` binary drives sampler executions and post-run analysis:

```bash
# Run an ensemble sweep from a YAML config and state manifest
asm-sim mcmc --config CONFIG.yaml --in state.json --out runs/example/

# Analyse a completed run directory and emit dispersion reports
asm-sim analyze --input runs/example/ --out runs/example/analysis/
```
