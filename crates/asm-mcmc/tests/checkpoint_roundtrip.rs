use std::path::Path;

use asm_code::css::CSSCode;
use asm_core::provenance::{RunProvenance, SchemaVersion};
use asm_core::Hypergraph;
use asm_graph::{HypergraphConfig, HypergraphImpl, KUniformity};
use tempfile::tempdir;

use asm_mcmc::{resume, run, MoveCounts, RunConfig};

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

fn checkpoint_config(root: &Path) -> RunConfig {
    let mut config = RunConfig::default();
    config.sweeps = 3;
    config.move_counts = MoveCounts {
        generator_flips: 1,
        row_ops: 1,
        graph_rewires: 1,
        worm_moves: 1,
    };
    config.output.run_directory = Some(root.join("run"));
    config.checkpoint.interval = 1;
    config
}

#[test]
fn resume_from_checkpoint_preserves_hashes() {
    let code = sample_code();
    let graph = sample_graph();
    let dir = tempdir().unwrap();
    let config = checkpoint_config(dir.path());

    let summary = run(&config, 888, &code, &graph).unwrap();
    assert!(!summary.checkpoints.is_empty());
    let checkpoint_path = summary.checkpoints.last().unwrap().clone();

    let resumed = resume(&checkpoint_path).unwrap();
    assert_eq!(summary.final_code_hash, resumed.final_code_hash);
    assert_eq!(summary.final_graph_hash, resumed.final_graph_hash);
}
