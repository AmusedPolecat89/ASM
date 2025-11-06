use asm_core::{Hypergraph, RunProvenance, SchemaVersion};
use asm_graph::{HypergraphConfig, HypergraphImpl, KUniformity};
use asm_rg::{dictionary::extract_couplings, DictOpts};
use criterion::{criterion_group, criterion_main, Criterion};

fn build_graph() -> HypergraphImpl {
    let config = HypergraphConfig {
        causal_mode: false,
        max_in_degree: None,
        max_out_degree: None,
        k_uniform: Some(KUniformity::Total {
            total: 2,
            min_sources: 1,
        }),
        schema_version: SchemaVersion::new(2, 0, 0),
    };
    let mut graph = HypergraphImpl::new(config);
    let a = graph.add_node().unwrap();
    let b = graph.add_node().unwrap();
    graph.add_hyperedge(&[a], &[b]).unwrap();
    graph
}

fn build_code() -> asm_code::CSSCode {
    asm_code::CSSCode::new(
        2,
        vec![vec![0, 1]],
        vec![vec![0, 1]],
        SchemaVersion::new(1, 0, 0),
        RunProvenance::default(),
    )
    .unwrap()
}

fn bench_dictionary(c: &mut Criterion) {
    let graph = build_graph();
    let code = build_code();
    let opts = DictOpts::default();
    c.bench_function("extract_couplings_small", |b| {
        b.iter(|| {
            let _ = extract_couplings(&graph, &code, &opts).unwrap();
        });
    });
}

criterion_group!(benches, bench_dictionary);
criterion_main!(benches);
