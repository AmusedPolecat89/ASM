# Phase 16 Web Dashboards

`asm-web` turns the dataset registry into a deterministic static site. The
collector layer (`asm_web::collect_site_data`) gathers submissions, artifacts,
and metrics, while `asm_web::build_site` renders HTML pages and a manifest.

## Configuration

`configs/phase16/site.yaml` configures the site title, navigation items, and
featured runs. Additional styling lives in `site/assets/site.css`. All output
is written to `site/dist/` with canonical filenames.

## CLI

```
asm-sim web build --registry registry/asm.sqlite \
  --config configs/phase16/site.yaml \
  --out site/dist/
```

The resulting `manifest.json` records page counts and build time, enabling the
publication pipeline to track deterministic rebuilds.
