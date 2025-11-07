use asm_code::{CSSCode, StateHandle};
use asm_core::{RunProvenance, SchemaVersion};
use criterion::{criterion_group, criterion_main, Criterion};

fn provenance() -> RunProvenance {
    RunProvenance {
        input_hash: "input".into(),
        graph_hash: "graph".into(),
        code_hash: String::new(),
        seed: 101,
        created_at: "2024-01-01T00:00:00Z".into(),
        tool_versions: Default::default(),
    }
}

fn build_code() -> CSSCode {
    let mut x_checks = Vec::new();
    let mut z_checks = Vec::new();
    for i in 0..50 {
        x_checks.push(vec![i, (i + 1) % 50]);
        z_checks.push(vec![i, (i + 2) % 50]);
    }
    CSSCode::new(
        50,
        x_checks,
        z_checks,
        SchemaVersion::new(1, 0, 0),
        provenance(),
    )
    .unwrap()
}

fn bench_syndrome(c: &mut Criterion) {
    let code = build_code();
    let states: Vec<_> = (0..32)
        .map(|seed| {
            let bits: Vec<u8> = (0..code.num_variables())
                .map(|idx| ((idx as u64 + seed) % 2) as u8)
                .collect();
            StateHandle::from_bits(bits).unwrap()
        })
        .collect();
    let state_refs: Vec<&dyn asm_core::ConstraintState> = states
        .iter()
        .map(|s| s as &dyn asm_core::ConstraintState)
        .collect();

    c.bench_function("syndrome_scan", |b| {
        b.iter(|| {
            let _ = code.violations_for_states(&state_refs).unwrap();
        })
    });
}

criterion_group!(benches, bench_syndrome);
criterion_main!(benches);
