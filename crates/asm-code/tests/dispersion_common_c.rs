use asm_code::dispersion::{estimate_dispersion, DispersionOptions};
use asm_code::{CSSCode, StateHandle};
use asm_core::{Hypergraph, RunProvenance, SchemaVersion};
use asm_graph::{HypergraphConfig, HypergraphImpl};

fn provenance() -> RunProvenance {
    RunProvenance {
        input_hash: "input".into(),
        graph_hash: "graph".into(),
        code_hash: String::new(),
        seed: 17,
        created_at: "2024-01-01T00:00:00Z".into(),
        tool_versions: Default::default(),
    }
}

fn build_code() -> CSSCode {
    CSSCode::new(
        4,
        vec![vec![0, 1], vec![2, 3]],
        vec![vec![0, 1], vec![2, 3]],
        SchemaVersion::new(1, 0, 0),
        provenance(),
    )
    .unwrap()
}

fn build_graph() -> HypergraphImpl {
    let mut config = HypergraphConfig::default();
    config.k_uniform = None;
    let mut graph = HypergraphImpl::new(config);
    let a = graph.add_node().unwrap();
    let b = graph.add_node().unwrap();
    let c = graph.add_node().unwrap();
    let d = graph.add_node().unwrap();
    graph.add_hyperedge(&[a, b], &[c]).expect("edge a,b -> c");
    graph.add_hyperedge(&[c], &[d]).expect("edge c -> d");
    graph
}

#[test]
fn dispersion_curves_share_common_velocity() {
    let code = build_code();
    let graph = build_graph();
    let state = StateHandle::from_bits(vec![1, 0, 1, 0]).unwrap();
    let violations = code.violations_for_state(&state).unwrap();
    let defects = code.find_defects(&violations);
    let species: Vec<_> = defects.iter().map(|d| d.species).collect();

    let opts = DispersionOptions {
        steps: vec![1, 3, 5],
        tolerance: 1e-3,
    };

    let report = estimate_dispersion(&code, &graph, &species, &opts).unwrap();
    assert_eq!(report.per_species.len(), species.len());
    assert_eq!(report.diagnostics.samples, species.len() * opts.steps.len());
    for residual in &report.residuals {
        assert!(residual.residual.abs() <= report.common_c);
    }
    assert!(report.common_c > 0.0);
}
