use asm_core::{Hypergraph, RunProvenance, SchemaVersion};
use asm_graph::{HypergraphConfig, HypergraphImpl, KUniformity};
use asm_rg::{
    covariance::covariance_check, dictionary::extract_couplings, rg_run, serde_io, DictOpts,
    RGOpts, StateRef,
};

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
fn serde_roundtrip_maintains_reports() {
    let graph = build_graph();
    let code = build_code();
    let state = StateRef {
        graph: &graph,
        code: &code,
    };
    let rg_opts = RGOpts::default();
    let dict_opts = DictOpts::default();

    let run = rg_run(&state, 1, &rg_opts).unwrap();
    let run_json = serde_io::run_to_json(&run.report).unwrap();
    let restored_run = serde_io::run_from_json(&run_json).unwrap();
    assert_eq!(run.report, restored_run);

    let couplings = extract_couplings(&graph, &code, &dict_opts).unwrap();
    let couplings_json = serde_io::couplings_to_json(&couplings).unwrap();
    let restored_couplings = serde_io::couplings_from_json(&couplings_json).unwrap();
    assert_eq!(couplings, restored_couplings);

    let covariance = covariance_check(&state, 1, &rg_opts, &dict_opts).unwrap();
    let covariance_json = serde_io::covariance_to_json(&covariance).unwrap();
    let restored_covariance = serde_io::covariance_from_json(&covariance_json).unwrap();
    assert_eq!(covariance, restored_covariance);
}
