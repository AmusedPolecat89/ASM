use asm_core::{Hypergraph, RunProvenance, SchemaVersion};
use asm_graph::{HypergraphConfig, HypergraphImpl, KUniformity};
use asm_rg::{dictionary::extract_couplings, DictOpts};

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
fn dictionary_hash_is_stable() {
    let graph = build_graph();
    let code = build_code();
    let opts = DictOpts::default();
    let report_a = extract_couplings(&graph, &code, &opts).unwrap();
    let report_b = extract_couplings(&graph, &code, &opts).unwrap();
    assert_eq!(report_a.dict_hash, report_b.dict_hash);

    let mut opts_shifted = opts.clone();
    opts_shifted.seed = opts.seed + 1;
    let report_c = extract_couplings(&graph, &code, &opts_shifted).unwrap();
    let diff = (report_a.c_kin - report_c.c_kin).abs();
    assert!(diff <= 1e-9);
}
