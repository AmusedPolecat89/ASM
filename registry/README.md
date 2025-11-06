# Ablation Registry

The registry stores deterministic records for every ablation job executed via `asm-sim ablation` or CI workflows.

- CSV registry: `registry/asm.csv`
- SQLite registry: `registry/asm.sqlite`

Each row captures the provenance date, commit, plan identifiers, canonical parameter JSON, and metrics JSON. Use `scripts/summarize_registry.py --registry registry/asm.sqlite --out dashboards` to generate KPI dashboards.

Schema: see [`schema.sql`](schema.sql).
