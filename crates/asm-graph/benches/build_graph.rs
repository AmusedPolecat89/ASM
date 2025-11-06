use asm_core::rng::RngHandle;
use asm_graph::gen_bounded_degree;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn build_graph_bench(c: &mut Criterion) {
    c.bench_function("build_graph_5k", |b| {
        b.iter(|| {
            let mut rng = RngHandle::from_seed(42);
            let graph = gen_bounded_degree(5_000, 4, 4, &mut rng).unwrap();
            black_box(graph);
        });
    });
}

criterion_group!(benches, build_graph_bench);
criterion_main!(benches);
