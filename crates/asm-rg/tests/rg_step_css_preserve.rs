use asm_core::{Hypergraph, RunProvenance, SchemaVersion};
use asm_graph::{HypergraphConfig, HypergraphImpl, KUniformity};
use asm_rg::{rg_step, RGOpts};

fn build_graph() -> HypergraphImpl {
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
    let a = graph.add_node().unwrap();
    let b = graph.add_node().unwrap();
    graph.add_hyperedge(&[a], &[b]).unwrap();
    graph
}

fn build_code() -> asm_code::CSSCode {
    asm_code::CSSCode::new(
        2,
        vec![vec![0, 1]],
        vec![vec![0, 1]],
        SchemaVersion::new(1, 0, 0),
        RunProvenance::default(),
    )
    .unwrap()
}

#[test]
fn rg_step_preserves_css_structure() {
    let graph = build_graph();
    let code = build_code();
    let opts = RGOpts::default();

    let step = rg_step(&graph, &code, &opts).expect("rg_step should succeed");
    assert!(step.report.css_preserved);
    assert!(step.report.kept_fraction >= 1.0 - 1e-9);
    assert_eq!(step.report.lost_constraints, 0);
    assert_eq!(step.code.num_constraints_x(), code.num_constraints_x());
    assert_eq!(step.code.num_constraints_z(), code.num_constraints_z());
}
