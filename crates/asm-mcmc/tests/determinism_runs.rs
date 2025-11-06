use asm_code::css::CSSCode;
use asm_core::provenance::{RunProvenance, SchemaVersion};
use asm_core::Hypergraph;
use asm_graph::{HypergraphConfig, HypergraphImpl, KUniformity};

use asm_mcmc::{run, MoveCounts, RunConfig};

fn sample_code() -> CSSCode {
    CSSCode::new(
        4,
        vec![vec![0, 1], vec![2, 3]],
        vec![vec![0, 1], vec![2, 3]],
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
    let a = graph.add_node().unwrap();
    let b = graph.add_node().unwrap();
    let c = graph.add_node().unwrap();
    graph.add_hyperedge(&[a], &[b]).unwrap();
    graph.add_hyperedge(&[b], &[c]).unwrap();
    graph
}

fn deterministic_config() -> RunConfig {
    let mut config = RunConfig::default();
    config.sweeps = 3;
    config.burn_in = 0;
    config.thinning = 1;
    config.move_counts = MoveCounts {
        generator_flips: 1,
        row_ops: 1,
        graph_rewires: 1,
        worm_moves: 1,
    };
    config.output.run_directory = None;
    config.checkpoint.interval = 0;
    config
}

#[test]
fn repeated_runs_with_same_seed_match() {
    let code = sample_code();
    let graph = sample_graph();
    let config = deterministic_config();

    let summary_a = run(&config, 2024, &code, &graph).unwrap();
    let summary_b = run(&config, 2024, &code, &graph).unwrap();

    assert_eq!(summary_a, summary_b);
}
