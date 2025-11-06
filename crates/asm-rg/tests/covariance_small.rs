use asm_core::{Hypergraph, RunProvenance, SchemaVersion};
use asm_graph::{HypergraphConfig, HypergraphImpl, KUniformity};
use asm_rg::{covariance::covariance_check, DictOpts, RGOpts, StateRef};

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
fn covariance_report_passes_for_small_instance() {
    let graph = build_graph();
    let code = build_code();
    let rg_opts = RGOpts::default();
    let dict_opts = DictOpts::default();
    let state = StateRef {
        graph: &graph,
        code: &code,
    };

    let report = covariance_check(&state, 2, &rg_opts, &dict_opts).unwrap();
    assert!(report.pass, "covariance check should pass by default");
    assert!(report.delta.c_kin_relative <= report.thresholds.c_kin_relative + 1e-12);
    assert!(report.delta.g_max_absolute <= report.thresholds.g_absolute + 1e-12);
    assert!(report.delta.lambda_absolute <= report.thresholds.lambda_absolute + 1e-12);
    assert!(report.delta.yukawa_max_absolute <= report.thresholds.yukawa_absolute + 1e-12);
}
