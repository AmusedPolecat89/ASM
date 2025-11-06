use asm_code::dispersion::{estimate_dispersion, DispersionOptions};
use asm_code::serde::{from_json, to_json};
use asm_code::{CSSCode, StateHandle};
use asm_core::{Hypergraph, RunProvenance, SchemaVersion};
use asm_graph::{HypergraphConfig, HypergraphImpl};

fn provenance(seed: u64) -> RunProvenance {
    RunProvenance {
        input_hash: "input".into(),
        graph_hash: "graph".into(),
        code_hash: String::new(),
        seed,
        created_at: "2024-01-01T00:00:00Z".into(),
        tool_versions: Default::default(),
    }
}

fn build_code(seed: u64) -> CSSCode {
    CSSCode::new(
        6,
        vec![vec![0, 1], vec![2, 3], vec![4, 5]],
        vec![vec![0, 1], vec![2, 3], vec![4, 5]],
        SchemaVersion::new(1, 0, 0),
        provenance(seed),
    )
    .unwrap()
}

fn build_graph() -> HypergraphImpl {
    let mut config = HypergraphConfig::default();
    config.k_uniform = None;
    let mut graph = HypergraphImpl::new(config);
    let ids: Vec<_> = (0..6).map(|_| graph.add_node().unwrap()).collect();
    graph.add_hyperedge(&[ids[0], ids[1]], &[ids[2]]).unwrap();
    graph.add_hyperedge(&[ids[2]], &[ids[3], ids[4]]).unwrap();
    graph.add_hyperedge(&[ids[4]], &[ids[5]]).unwrap();
    graph
}

#[test]
fn repeated_runs_match() {
    let code_a = build_code(23);
    let code_b = build_code(23);
    assert_eq!(code_a.canonical_hash(), code_b.canonical_hash());

    let state = StateHandle::from_bits(vec![1, 0, 1, 0, 1, 0]).unwrap();
    let defects = code_a.find_defects(&code_a.violations_for_state(&state).unwrap());
    let species: Vec<_> = defects.iter().map(|d| d.species).collect();

    let opts = DispersionOptions::default();
    let graph = build_graph();
    let report_a = estimate_dispersion(&code_a, &graph, &species, &opts).unwrap();
    let report_b = estimate_dispersion(&code_b, &graph, &species, &opts).unwrap();

    assert_eq!(report_a.common_c, report_b.common_c);
    assert_eq!(report_a.residuals, report_b.residuals);
    assert_eq!(report_a.diagnostics, report_b.diagnostics);

    let json = to_json(&code_a).unwrap();
    let restored = from_json(&json).unwrap();
    assert_eq!(code_a.canonical_hash(), restored.canonical_hash());
}
