use asm_core::errors::AsmError;
use asm_core::Hypergraph;
use asm_graph::{
    canonical_hash, graph_from_json, graph_to_json, HypergraphConfig, HypergraphImpl, KUniformity,
};

#[test]
fn add_remove_and_serialize() {
    let mut config = HypergraphConfig::default();
    config.max_in_degree = Some(8);
    config.max_out_degree = Some(8);
    config.k_uniform = Some(KUniformity::Total {
        total: 4,
        min_sources: 1,
    });
    config.causal_mode = false;

    let mut graph = HypergraphImpl::new(config);
    let n0 = graph.add_node().unwrap();
    let n1 = graph.add_node().unwrap();
    let n2 = graph.add_node().unwrap();
    let n3 = graph.add_node().unwrap();

    let e0 = graph.add_hyperedge(&[n0, n1], &[n2, n3]).unwrap();
    let e1 = graph.add_hyperedge(&[n1, n2], &[n0, n3]).unwrap();

    assert_eq!(graph.nodes().len(), 4);
    assert_eq!(graph.edges().len(), 2);

    let bounds = graph.degree_bounds().unwrap();
    assert_eq!(bounds.max_in_degree, Some(2));
    assert_eq!(bounds.max_out_degree, Some(2));

    let hash_before = canonical_hash(&graph).unwrap();
    let json = graph_to_json(&graph).unwrap();
    let restored = graph_from_json(&json).unwrap();
    let hash_after = canonical_hash(&restored).unwrap();
    assert_eq!(hash_before, hash_after);

    assert!(matches!(
        graph.remove_node(n0),
        Err(AsmError::Graph(info)) if info.code == "node-not-isolated"
    ));

    graph.remove_hyperedge(e0).unwrap();
    graph.remove_hyperedge(e1).unwrap();

    let e2 = graph.add_hyperedge(&[n0, n1], &[n2, n3]).unwrap();
    assert_eq!(e2.as_raw(), 2);
    graph.remove_hyperedge(e2).unwrap();

    graph.remove_node(n0).unwrap();
}
