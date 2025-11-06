use asm_aut::{analyze_state, ScanOpts};
use criterion::{criterion_group, criterion_main, Criterion};

#[path = "../tests/fixtures.rs"]
mod fixtures;

fn bench_scan(c: &mut Criterion) {
    let fixture = fixtures::load_fixture("t1_seed0").expect("fixture");
    let mut group = c.benchmark_group("scan_vacuum");
    group.bench_function("t1_seed0", |b| {
        b.iter(|| {
            let _ = analyze_state(&fixture.graph, &fixture.code, &ScanOpts::default()).unwrap();
        })
    });
    group.finish();
}

criterion_group!(benches, bench_scan);
criterion_main!(benches);
