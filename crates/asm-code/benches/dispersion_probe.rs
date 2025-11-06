use asm_code::dispersion::{estimate_dispersion, DispersionOptions};
use asm_code::{CSSCode, StateHandle};
use asm_core::{Hypergraph, RunProvenance, SchemaVersion};
use asm_graph::{HypergraphConfig, HypergraphImpl};
use criterion::{criterion_group, criterion_main, Criterion};

fn provenance() -> RunProvenance {
    RunProvenance {
        input_hash: "input".into(),
        graph_hash: "graph".into(),
        code_hash: String::new(),
        seed: 211,
        created_at: "2024-01-01T00:00:00Z".into(),
        tool_versions: Default::default(),
    }
}

fn build_code() -> CSSCode {
    let mut x_checks = Vec::new();
    let mut z_checks = Vec::new();
    for i in 0..40 {
        x_checks.push(vec![i, (i + 1) % 40]);
        z_checks.push(vec![i, (i + 3) % 40]);
    }
    CSSCode::new(
        40,
        x_checks,
        z_checks,
        SchemaVersion::new(1, 0, 0),
        provenance(),
    )
    .unwrap()
}

fn build_graph() -> HypergraphImpl {
    let mut graph = HypergraphImpl::new(HypergraphConfig::default());
    let nodes: Vec<_> = (0..40).map(|_| graph.add_node().unwrap()).collect();
    for i in 0..40 {
        let src = nodes[i];
        let dst = nodes[(i + 1) % 40];
        let aux = nodes[(i + 2) % 40];
        let _ = graph.add_hyperedge(&[src], &[dst, aux]);
    }
    graph
}

fn bench_dispersion(c: &mut Criterion) {
    let code = build_code();
    let graph = build_graph();
    let state = StateHandle::from_bits(vec![1; code.num_variables()]).unwrap();
    let defects = code.find_defects(&code.violations_for_state(&state).unwrap());
    let species: Vec<_> = defects.iter().map(|d| d.species).collect();
    let opts = DispersionOptions::default();

    c.bench_function("dispersion_probe", |b| {
        b.iter(|| {
            let _ = estimate_dispersion(&code, &graph, &species, &opts).unwrap();
        })
    });
}

criterion_group!(benches, bench_dispersion);
criterion_main!(benches);
