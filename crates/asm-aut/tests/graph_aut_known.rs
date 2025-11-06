use asm_aut::{analyze_state, ScanOpts};
use asm_core::{AsmError, Hypergraph, RunProvenance, SchemaVersion};
use asm_graph::{HypergraphConfig, HypergraphImpl, KUniformity};

fn build_cycle_graph() -> Result<HypergraphImpl, AsmError> {
    let config = HypergraphConfig {
        causal_mode: false,
        max_in_degree: None,
        max_out_degree: None,
        k_uniform: Some(KUniformity::Total {
            total: 2,
            min_sources: 1,
        }),
        schema_version: SchemaVersion::new(2, 0, 0),
    };
    let mut graph = HypergraphImpl::new(config);
    let a = graph.add_node()?;
    let b = graph.add_node()?;
    let c = graph.add_node()?;
    graph.add_hyperedge(&[a], &[b])?;
    graph.add_hyperedge(&[b], &[c])?;
    graph.add_hyperedge(&[c], &[a])?;
    Ok(graph)
}

fn trivial_code() -> Result<asm_code::CSSCode, AsmError> {
    asm_code::CSSCode::new(
        3,
        vec![vec![0, 1], vec![1, 2], vec![0, 2]],
        vec![vec![0, 1, 2]],
        SchemaVersion::new(1, 0, 0),
        RunProvenance::default(),
    )
}

#[test]
fn triangle_graph_has_cyclic_automorphisms() -> Result<(), AsmError> {
    let graph = build_cycle_graph()?;
    let code = trivial_code()?;
    let opts = ScanOpts::default();
    let report = analyze_state(&graph, &code, &opts)?;
    assert_eq!(report.graph_aut.order, 3);
    assert_eq!(report.graph_aut.orbit_hist, vec![3]);
    Ok(())
}
