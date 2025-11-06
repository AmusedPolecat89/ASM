use asm_code::css::CSSCode;
use asm_core::provenance::{RunProvenance, SchemaVersion};
use asm_core::Hypergraph;
use asm_graph::{HypergraphConfig, HypergraphImpl, KUniformity};

use asm_mcmc::{run, MoveCounts, RunConfig};

fn sample_code() -> CSSCode {
    let schema = SchemaVersion::new(1, 0, 0);
    CSSCode::new(
        4,
        vec![vec![0, 1], vec![2, 3]],
        vec![vec![0, 1], vec![2, 3]],
        schema,
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

fn base_config() -> RunConfig {
    let mut config = RunConfig::default();
    config.sweeps = 4;
    config.burn_in = 0;
    config.thinning = 1;
    config.move_counts = MoveCounts {
        generator_flips: 1,
        row_ops: 1,
        graph_rewires: 1,
        worm_moves: 0,
    };
    config.checkpoint.interval = 0;
    config.output.run_directory = None;
    config
}

#[test]
fn worm_moves_increase_coverage_samples() {
    let code = sample_code();
    let graph = sample_graph();

    let mut config_no_worm = base_config();
    config_no_worm.move_counts.worm_moves = 0;
    let summary_no_worm = run(&config_no_worm, 123, &code, &graph).unwrap();

    let mut config_worm = base_config();
    config_worm.move_counts.worm_moves = 2;
    let summary_worm = run(&config_worm, 123, &code, &graph).unwrap();

    assert!(
        summary_worm.coverage.worm_samples > summary_no_worm.coverage.worm_samples,
        "worm moves should record additional samples"
    );
    assert!(
        summary_worm.coverage.unique_state_hashes >= summary_no_worm.coverage.unique_state_hashes
    );
}
