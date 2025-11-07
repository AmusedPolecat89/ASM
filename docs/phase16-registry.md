# Phase 16 Dataset Registry

The `asm-dsr` crate provides a deterministic SQLite-backed registry for
community submissions. Bundles produced with `asm-sim publish bundle` or the
`scripts/pack_dataset.py` helper contain a canonical `manifest.json` plus all
referenced artefacts.

## Schema

`asm_dsr::schema::init_schema` creates the `submissions`, `artifacts`, and
`metrics` tables. Each artifact row stores the canonical SHA256 alongside an
optional analysis hash. Registry helpers ensure canonical ordering when
exporting JSON (`asm_dsr::export::export_json`) or CSV summaries.

## CLI workflow

```
asm-sim publish bundle --root runs/landscape/smoke/ --out bundles/smoke.zip \
  --submitter alice --toolchain "asm 0.16"
asm-sim submit --bundle bundles/smoke.zip --registry registry/asm.sqlite
asm-sim verify bundle --bundle bundles/smoke.zip --registry registry/asm.sqlite
```

Submissions are materialised under `<registry>.artifacts/` and can be queried
via `asm_dsr::query::RegistryQuery` or the new web dashboard generator.
