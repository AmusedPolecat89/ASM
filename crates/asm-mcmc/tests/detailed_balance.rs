use asm_code::css::CSSCode;
use asm_core::provenance::{RunProvenance, SchemaVersion};
use asm_core::rng::RngHandle;
use asm_core::Hypergraph;
use asm_graph::{HypergraphConfig, HypergraphImpl, KUniformity};

use asm_mcmc::moves_code;
use asm_mcmc::moves_graph;

fn sample_code() -> CSSCode {
    let schema = SchemaVersion::new(1, 0, 0);
    let mut provenance = RunProvenance::default();
    provenance.seed = 1;
    CSSCode::new(
        4,
        vec![vec![0, 1], vec![2, 3]],
        Vec::new(),
        schema,
        provenance,
    )
    .expect("valid css code")
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
    let n0 = graph.add_node().unwrap();
    let n1 = graph.add_node().unwrap();
    let n2 = graph.add_node().unwrap();
    let n3 = graph.add_node().unwrap();
    graph.add_hyperedge(&[n0], &[n1]).unwrap();
    graph.add_hyperedge(&[n2], &[n3]).unwrap();
    graph
}

#[test]
fn proposal_probabilities_are_symmetric() {
    let code = sample_code();
    let mut rng = RngHandle::from_seed(7);
    let proposal = moves_code::propose_generator_flip(&code, &mut rng).unwrap();
    assert!((proposal.forward_prob - proposal.reverse_prob).abs() < 1e-12);

    let mut rng = RngHandle::from_seed(9);
    let proposal = moves_code::propose_row_operation(&code, &mut rng).unwrap();
    assert!((proposal.forward_prob - proposal.reverse_prob).abs() < 1e-12);

    let graph = sample_graph();
    let mut rng = RngHandle::from_seed(11);
    let proposal = moves_graph::propose_swap_targets(&graph, &mut rng).unwrap();
    assert!((proposal.forward_prob - proposal.reverse_prob).abs() < 1e-12);

    let mut rng = RngHandle::from_seed(13);
    let proposal = moves_graph::propose_retarget(&graph, &mut rng).unwrap();
    assert!((proposal.forward_prob - proposal.reverse_prob).abs() < 1e-12);
}
