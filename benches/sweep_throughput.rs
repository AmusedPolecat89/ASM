use std::fs;
use std::path::PathBuf;
use std::sync::Once;

use asm_exp::{sweep, to_canonical_json_bytes, GridParameter, SweepPlan, SweepStrategy};
use criterion::{criterion_group, criterion_main, Criterion};
use serde_json::json;

fn make_plan() -> SweepPlan {
    SweepPlan {
        strategy: SweepStrategy::Grid {
            parameters: vec![
                GridParameter {
                    name: "degree_cap".to_string(),
                    values: vec![json!(2), json!(3)],
                },
                GridParameter {
                    name: "worm_weight".to_string(),
                    values: vec![json!(0.1), json!(0.2), json!(0.3)],
                },
            ],
        },
        scheduler: Default::default(),
    }
}

fn ensure_baseline(plan: &SweepPlan) {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let report = sweep(plan, 4242).expect("sweep");
        let bytes = to_canonical_json_bytes(&report).expect("json");
        let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("repro/phase7");
        fs::create_dir_all(&out_dir).expect("create baseline dir");
        fs::write(out_dir.join("bench_sweep.json"), bytes).expect("write baseline");
    });
}

fn bench_sweep(c: &mut Criterion) {
    let plan = make_plan();
    ensure_baseline(&plan);
    c.bench_function("sweep_throughput", |b| {
        b.iter(|| {
            let _ = sweep(&plan, 1234).expect("sweep");
        });
    });
}

criterion_group!(benches, bench_sweep);
criterion_main!(benches);
