use std::sync::Arc;

use asm_core::rng::RngHandle;
use asm_graph::gen_bounded_degree;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn queries_bench(c: &mut Criterion) {
    let mut rng = RngHandle::from_seed(7);
    let graph = gen_bounded_degree(2_000, 4, 4, &mut rng).unwrap();
    let graph = Arc::new(graph);
    let edges: Vec<_> = graph.edges().collect();
    let nodes: Vec<_> = graph.nodes().collect();

    c.bench_function("hyperedge_lookup", |b| {
        b.iter(|| {
            for edge in &edges {
                black_box(graph.hyperedge(*edge).unwrap());
            }
        });
    });

    c.bench_function("degree_queries", |b| {
        b.iter(|| {
            for node in &nodes {
                black_box(graph.in_degree(*node).unwrap());
                black_box(graph.out_degree(*node).unwrap());
            }
        });
    });
}

criterion_group!(benches, queries_bench);
criterion_main!(benches);
