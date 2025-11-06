use std::fs;
use std::path::PathBuf;
use std::sync::Once;

use asm_exp::{
    run_ablation, to_canonical_json_bytes, AblationMode, AblationPlan, ToleranceSpec,
};
use criterion::{criterion_group, criterion_main, Criterion};
use serde_json::json;

fn plan() -> AblationPlan {
    AblationPlan {
        name: "bench_ablations".into(),
        mode: AblationMode::Grid,
        samples: None,
        factors: [
            ("graph.degree_cap".into(), vec![json!(3), json!(4)]),
            ("moves.worm_weight".into(), vec![json!(0.1), json!(0.2), json!(0.3)]),
        ]
        .into_iter()
        .collect(),
        fixed: [("sampler.sweeps".into(), json!(24))].into_iter().collect(),
        tolerances: [(
            "exchange_acceptance".into(),
            ToleranceSpec {
                min: Some(0.2),
                max: Some(0.5),
                abs: Some(1e-6),
                rel: Some(1e-3),
            },
        )]
        .into_iter()
        .collect(),
    }
}

fn ensure_baseline(plan: &AblationPlan) {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let report = run_ablation(plan, 4242).expect("ablation");
        let bytes = to_canonical_json_bytes(&report).expect("json");
        let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("repro/phase9");
        fs::create_dir_all(&out_dir).expect("create baseline dir");
        fs::write(out_dir.join("bench_ablations.json"), bytes).expect("write baseline");
    });
}

fn bench_ablation(c: &mut Criterion) {
    let plan = plan();
    ensure_baseline(&plan);
    c.bench_function("ablations_throughput", |b| {
        b.iter(|| {
            let _ = run_ablation(&plan, 999).expect("ablation");
        });
    });
}

criterion_group!(benches, bench_ablation);
criterion_main!(benches);
