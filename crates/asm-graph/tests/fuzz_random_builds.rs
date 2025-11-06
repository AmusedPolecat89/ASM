use std::collections::BTreeSet;

use asm_core::rng::RngHandle;
use asm_core::Hypergraph;
use asm_graph::{
    canonical_hash, gen_bounded_degree, graph_from_bytes, graph_to_bytes, rewire_resource_balanced,
    HypergraphImpl,
};
use proptest::prelude::*;

fn check_invariants(graph: &HypergraphImpl, degree_cap: usize) {
    let edge_ids: Vec<_> = graph.edges().collect();
    let mut signatures = BTreeSet::new();
    for edge_id in edge_ids.iter().copied() {
        let endpoints = graph.hyperedge(edge_id).unwrap();
        let mut sig = (endpoints.sources.to_vec(), endpoints.destinations.to_vec());
        sig.0.sort_by_key(|id| id.as_raw());
        sig.1.sort_by_key(|id| id.as_raw());
        signatures.insert(sig);
    }
    assert_eq!(signatures.len(), edge_ids.len());
    for node in graph.nodes() {
        assert!(graph.in_degree(node).unwrap() <= degree_cap);
        assert!(graph.out_degree(node).unwrap() <= degree_cap);
    }
}

proptest! {
    #[test]
    fn random_generators_respect_invariants(seed in any::<u64>(), nodes in 3usize..10, degree in 1usize..4) {
        let mut rng = RngHandle::from_seed(seed);
        let mut graph = gen_bounded_degree(nodes, degree, 4, &mut rng).unwrap();
        check_invariants(&graph, degree.max(1));

        let bytes = graph_to_bytes(&graph).unwrap();
        let restored = graph_from_bytes(&bytes).unwrap();
        prop_assert_eq!(canonical_hash(&graph).unwrap(), canonical_hash(&restored).unwrap());

        let node_ids: Vec<_> = graph.nodes().collect();
        for node in node_ids.into_iter().take(3) {
            let _ = rewire_resource_balanced(&mut graph, node, &mut rng);
            check_invariants(&graph, degree.max(1));
        }
    }
}
