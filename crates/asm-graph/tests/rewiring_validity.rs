use std::collections::{BTreeMap, BTreeSet};

use asm_core::rng::RngHandle;
use asm_core::{Hypergraph, NodeId};
use asm_graph::{
    canonical_hash, rewire_resource_balanced, rewire_resource_balanced_dry_run, rewire_retarget,
    rewire_retarget_dry_run, rewire_swap_targets, rewire_swap_targets_dry_run, HypergraphConfig,
    HypergraphImpl, KUniformity, RewireDryRun,
};

fn assert_invariants(graph: &HypergraphImpl) {
    let mut signatures = BTreeSet::new();
    for edge_id in graph.edges() {
        let endpoints = graph.hyperedge(edge_id).unwrap();
        let mut src = endpoints.sources.to_vec();
        src.sort_by_key(|id| id.as_raw());
        src.dedup();
        let mut dst = endpoints.destinations.to_vec();
        dst.sort_by_key(|id| id.as_raw());
        dst.dedup();
        assert_eq!(src.len(), endpoints.sources.len());
        assert_eq!(dst.len(), endpoints.destinations.len());
        signatures.insert((src.clone(), dst.clone()));
    }

    if let Some(cap) = graph.config().max_out_degree {
        for node in graph.nodes() {
            assert!(graph.out_degree(node).unwrap() <= cap);
        }
    }
    if let Some(cap) = graph.config().max_in_degree {
        for node in graph.nodes() {
            assert!(graph.in_degree(node).unwrap() <= cap);
        }
    }

    if graph.config().causal_mode {
        assert!(!has_cycle(graph));
    }
}

fn has_cycle(graph: &HypergraphImpl) -> bool {
    let mut adjacency: BTreeMap<NodeId, Vec<NodeId>> = BTreeMap::new();
    for edge_id in graph.edges() {
        let endpoints = graph.hyperedge(edge_id).unwrap();
        for source in endpoints.sources.iter() {
            let entry = adjacency.entry(*source).or_default();
            entry.extend(endpoints.destinations.iter().copied());
        }
    }
    let mut states: BTreeMap<NodeId, VisitState> = BTreeMap::new();
    for node in graph.nodes() {
        states.insert(node, VisitState::NotVisited);
    }
    for node in adjacency.keys().copied().collect::<Vec<_>>() {
        if dfs(node, &adjacency, &mut states) {
            return true;
        }
    }
    false
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
    NotVisited,
    Visiting,
    Visited,
}

fn dfs(
    node: NodeId,
    adjacency: &BTreeMap<NodeId, Vec<NodeId>>,
    states: &mut BTreeMap<NodeId, VisitState>,
) -> bool {
    match states.get(&node).copied().unwrap_or(VisitState::NotVisited) {
        VisitState::Visiting => true,
        VisitState::Visited => false,
        VisitState::NotVisited => {
            states.insert(node, VisitState::Visiting);
            if let Some(neighbours) = adjacency.get(&node) {
                for neighbour in neighbours {
                    if dfs(*neighbour, adjacency, states) {
                        return true;
                    }
                }
            }
            states.insert(node, VisitState::Visited);
            false
        }
    }
}

#[test]
fn rewiring_moves_preserve_structure() {
    let mut config = HypergraphConfig::default();
    config.max_in_degree = Some(4);
    config.max_out_degree = Some(4);
    config.k_uniform = Some(KUniformity::Balanced {
        sources: 2,
        destinations: 2,
    });
    config.causal_mode = false;
    let mut graph = HypergraphImpl::new(config);

    let nodes: Vec<_> = (0..6).map(|_| graph.add_node().unwrap()).collect();
    let e0 = graph
        .add_hyperedge(&[nodes[0], nodes[1]], &[nodes[2], nodes[3]])
        .unwrap();
    let e1 = graph
        .add_hyperedge(&[nodes[1], nodes[2]], &[nodes[3], nodes[4]])
        .unwrap();
    let e2 = graph
        .add_hyperedge(&[nodes[2], nodes[3]], &[nodes[4], nodes[5]])
        .unwrap();

    assert_invariants(&graph);
    let initial_hash = canonical_hash(&graph).unwrap();

    let outcome = rewire_swap_targets(&mut graph, e0, e1).unwrap();
    assert!(outcome.changed);
    assert_invariants(&graph);
    match rewire_swap_targets_dry_run(&graph, e1, e2) {
        RewireDryRun::Valid { .. } => {}
        other => panic!("unexpected validator result: {other:?}"),
    }

    let retarget = rewire_retarget(&mut graph, e2, &[nodes[4]], &[nodes[0]]).unwrap();
    assert!(retarget.changed);
    assert_invariants(&graph);
    match rewire_retarget_dry_run(&graph, e2, &[nodes[0]], &[nodes[5]]) {
        RewireDryRun::Invalid(err) => {
            assert_eq!(err.info().code, "invalid-arity");
        }
        other => panic!("unexpected validator result: {other:?}"),
    }

    let mut rng = RngHandle::from_seed(7);
    let resource = rewire_resource_balanced(&mut graph, nodes[1], &mut rng).unwrap();
    if resource.changed {
        assert_invariants(&graph);
    }
    let dry = rewire_resource_balanced_dry_run(&graph, nodes[1], &mut rng);
    match dry {
        RewireDryRun::Valid { .. } | RewireDryRun::Invalid(_) => {}
    }

    let final_hash = canonical_hash(&graph).unwrap();
    assert_ne!(initial_hash, final_hash);
}
