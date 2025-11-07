use std::fs;
use std::time::Instant;

use asm_land::{build_atlas, plan::load_plan, report::AtlasOpts, run_plan, summarize, RunOpts};
use criterion::{criterion_group, criterion_main, Criterion};
use serde_json::json;
use tempfile::tempdir;

fn write_baseline(plan: &asm_land::plan::Plan, duration: f64) {
    let jobs = plan.seeds.len() * plan.rules().len();
    let payload = json!({
        "plan_hash": plan.plan_hash().unwrap_or_default(),
        "jobs": jobs,
        "seconds": duration,
    });
    fs::create_dir_all("repro/phase14").expect("baseline dir");
    fs::write(
        "repro/phase14/bench_landscape.json",
        serde_json::to_vec_pretty(&payload).expect("serialize baseline"),
    )
    .expect("write baseline");
}

fn bench_landscape(c: &mut Criterion) {
    let plan = load_plan("landscape/plans/smoke.yaml").expect("load plan");
    let warm_dir = tempdir().expect("warm dir");
    let start = Instant::now();
    let _report = run_plan(&plan, warm_dir.path(), &RunOpts::default()).expect("baseline run");
    let duration = start.elapsed().as_secs_f64();
    let filters = asm_land::filters::load_filters(&plan.filters_path()).expect("filters");
    let _ = summarize(warm_dir.path(), &filters).expect("summary");
    let _ = build_atlas(warm_dir.path(), &AtlasOpts::default()).expect("atlas");
    write_baseline(&plan, duration);

    c.bench_function("landscape_throughput", |b| {
        b.iter(|| {
            let dir = tempdir().expect("bench dir");
            run_plan(&plan, dir.path(), &RunOpts::default()).expect("bench run");
        });
    });
}

criterion_group!(benches, bench_landscape);
criterion_main!(benches);
