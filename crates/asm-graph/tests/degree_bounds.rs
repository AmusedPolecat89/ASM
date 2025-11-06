use asm_core::errors::AsmError;
use asm_core::Hypergraph;
use asm_graph::{HypergraphConfig, HypergraphImpl, KUniformity};

#[test]
fn outbound_cap_enforced() {
    let mut config = HypergraphConfig::default();
    config.max_out_degree = Some(1);
    config.max_in_degree = Some(2);
    config.k_uniform = Some(KUniformity::Total {
        total: 2,
        min_sources: 1,
    });
    let mut graph = HypergraphImpl::new(config);

    let a = graph.add_node().unwrap();
    let b = graph.add_node().unwrap();
    let c = graph.add_node().unwrap();

    graph.add_hyperedge(&[a], &[b]).unwrap();
    let err = graph.add_hyperedge(&[a], &[c]).unwrap_err();
    match err {
        AsmError::Graph(info) => {
            assert_eq!(info.code, "out-degree-cap");
            assert_eq!(info.context.get("node"), Some(&a.as_raw().to_string()));
            assert_eq!(info.context.get("cap"), Some(&"1".to_string()));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn inbound_cap_enforced() {
    let mut config = HypergraphConfig::default();
    config.max_out_degree = Some(2);
    config.max_in_degree = Some(1);
    config.k_uniform = Some(KUniformity::Total {
        total: 2,
        min_sources: 1,
    });
    let mut graph = HypergraphImpl::new(config);

    let a = graph.add_node().unwrap();
    let b = graph.add_node().unwrap();
    let c = graph.add_node().unwrap();

    graph.add_hyperedge(&[a], &[c]).unwrap();
    let err = graph.add_hyperedge(&[b], &[c]).unwrap_err();
    match err {
        AsmError::Graph(info) => {
            assert_eq!(info.code, "in-degree-cap");
            assert_eq!(info.context.get("node"), Some(&c.as_raw().to_string()));
            assert_eq!(info.context.get("cap"), Some(&"1".to_string()));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
