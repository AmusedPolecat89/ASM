use asm_core::Hypergraph;
use asm_graph::{
    forman_curvature_edges, forman_curvature_nodes, ollivier_lite_nodes, HypergraphConfig,
    HypergraphImpl, KUniformity,
};

fn star_graph() -> HypergraphImpl {
    let mut config = HypergraphConfig::default();
    config.k_uniform = Some(KUniformity::Total {
        total: 2,
        min_sources: 1,
    });
    let mut graph = HypergraphImpl::new(config);

    let center = graph.add_node().unwrap();
    let leaves: Vec<_> = (0..4).map(|_| graph.add_node().unwrap()).collect();
    for leaf in &leaves {
        graph.add_hyperedge(&[center], &[*leaf]).unwrap();
    }
    graph
}

fn chain_graph(len: usize) -> HypergraphImpl {
    let mut config = HypergraphConfig::default();
    config.k_uniform = Some(KUniformity::Total {
        total: 2,
        min_sources: 1,
    });
    let mut graph = HypergraphImpl::new(config);
    let nodes: Vec<_> = (0..len).map(|_| graph.add_node().unwrap()).collect();
    for pair in nodes.windows(2) {
        graph.add_hyperedge(&[pair[0]], &[pair[1]]).unwrap();
    }
    graph
}

#[test]
fn forman_curvature_orders_star() {
    let graph = star_graph();
    let nodes = forman_curvature_nodes(&graph).unwrap();
    let center = nodes
        .iter()
        .find(|(node, _)| node.as_raw() == 0)
        .expect("center node");
    let leaf = nodes
        .iter()
        .find(|(node, _)| node.as_raw() == 1)
        .expect("leaf node");
    assert!(center.1 >= leaf.1);
}

#[test]
fn forman_curvature_orders_chain() {
    let graph = chain_graph(5);
    let nodes = forman_curvature_nodes(&graph).unwrap();
    let end = nodes.first().unwrap().1;
    let middle = nodes[2].1;
    assert!(middle >= end);

    let edges = forman_curvature_edges(&graph).unwrap();
    assert_eq!(edges.len(), 4);
    assert!(edges.iter().all(|(_, value)| value.is_finite()));
}

#[test]
fn ollivier_values_are_deterministic() {
    let graph = chain_graph(6);
    let values = ollivier_lite_nodes(&graph, 4).unwrap();
    let values_again = ollivier_lite_nodes(&graph, 4).unwrap();
    assert_eq!(values, values_again);
    assert!(values.iter().all(|(_, value)| value.is_finite()));
}
