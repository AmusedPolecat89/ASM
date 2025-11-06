use asm_code::css::CSSCode;
use asm_core::provenance::{RunProvenance, SchemaVersion};
use asm_graph::flags::{HypergraphConfig, KUniformity};
use asm_graph::HypergraphImpl;
use criterion::{criterion_group, criterion_main, Criterion};

use asm_mcmc::{run, MoveCounts, RunConfig};

fn sample_code() -> CSSCode {
    CSSCode::new(
        6,
        vec![vec![0, 1, 2], vec![3, 4, 5]],
        vec![vec![0, 1, 2], vec![3, 4, 5]],
        SchemaVersion::new(1, 0, 0),
        RunProvenance::default(),
    )
    .unwrap()
}

fn sample_graph() -> HypergraphImpl {
    let config = HypergraphConfig {
        causal_mode: false,
        max_in_degree: None,
        max_out_degree: None,
        k_uniform: Some(KUniformity::Balanced {
            sources: 1,
            destinations: 1,
        }),
        schema_version: SchemaVersion::new(2, 0, 0),
    };
    let mut graph = HypergraphImpl::new(config);
    let nodes: Vec<_> = (0..6).map(|_| graph.add_node().unwrap()).collect();
    for window in nodes.windows(2) {
        graph.add_hyperedge(&[window[0]], &[window[1]]).unwrap();
    }
    graph
}

fn bench_sweep(c: &mut Criterion) {
    let code = sample_code();
    let graph = sample_graph();
    let mut config = RunConfig::default();
    config.sweeps = 5;
    config.move_counts = MoveCounts {
        generator_flips: 2,
        row_ops: 2,
        graph_rewires: 2,
        worm_moves: 2,
    };
    config.output.run_directory = None;
    config.checkpoint.interval = 0;

    c.bench_function("mcmc_sweep", |b| {
        b.iter(|| {
            let _ = run(&config, 42, &code, &graph).unwrap();
        })
    });
}

criterion_group!(benches, bench_sweep);
criterion_main!(benches);
