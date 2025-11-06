use asm_core::errors::AsmError;
use asm_core::Hypergraph;
use asm_graph::{HypergraphConfig, HypergraphImpl, KUniformity};

#[test]
fn causal_mode_blocks_cycles() {
    let mut config = HypergraphConfig::default();
    config.k_uniform = Some(KUniformity::Total {
        total: 2,
        min_sources: 1,
    });
    let mut graph = HypergraphImpl::new(config);

    let a = graph.add_node().unwrap();
    let b = graph.add_node().unwrap();
    let c = graph.add_node().unwrap();

    graph.add_hyperedge(&[a], &[b]).unwrap();
    graph.add_hyperedge(&[b], &[c]).unwrap();
    let err = graph.add_hyperedge(&[c], &[a]).unwrap_err();
    assert!(matches!(err, AsmError::Graph(info) if info.code == "would-create-cycle"));
}

#[test]
fn non_causal_mode_allows_cycles() {
    let mut config = HypergraphConfig::default();
    config.causal_mode = false;
    config.k_uniform = Some(KUniformity::Total {
        total: 2,
        min_sources: 1,
    });
    let mut graph = HypergraphImpl::new(config);

    let a = graph.add_node().unwrap();
    let b = graph.add_node().unwrap();
    let c = graph.add_node().unwrap();

    graph.add_hyperedge(&[a], &[b]).unwrap();
    graph.add_hyperedge(&[b], &[c]).unwrap();
    graph.add_hyperedge(&[c], &[a]).unwrap();
    assert_eq!(graph.edges().len(), 3);
}
