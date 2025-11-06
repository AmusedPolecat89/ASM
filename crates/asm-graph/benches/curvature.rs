use asm_core::rng::RngHandle;
use asm_graph::{forman_curvature_edges, gen_bounded_degree};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn curvature_bench(c: &mut Criterion) {
    let mut rng = RngHandle::from_seed(123);
    let graph = gen_bounded_degree(2_500, 4, 4, &mut rng).unwrap();
    c.bench_function("forman_curvature_edges", |b| {
        b.iter(|| {
            let values = forman_curvature_edges(&graph).unwrap();
            black_box(values);
        });
    });
}

criterion_group!(benches, curvature_bench);
criterion_main!(benches);
