use std::fs;
use std::path::PathBuf;

use asm_code::serde as code_serde;
use asm_int::{fit_running, RunningOpts};
use asm_rg::StateRef;
use asm_graph::graph_from_json;

fn load_state(dir: &str) -> (asm_graph::HypergraphImpl, asm_code::CSSCode) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let graph_json = fs::read_to_string(base.join(dir).join("graph.json")).expect("graph");
    let code_json = fs::read_to_string(base.join(dir).join("code.json")).expect("code");
    let graph = graph_from_json(&graph_json).expect("graph decode");
    let code = code_serde::from_json(&code_json).expect("code decode");
    (graph, code)
}

#[test]
fn running_report_has_finite_beta() {
    let (graph_a, code_a) = load_state("fixtures/validation_vacua/t1_seed0/end_state");
    let (graph_b, code_b) = load_state("fixtures/validation_vacua/t1_seed1/end_state");
    let graphs = vec![graph_a, graph_b];
    let codes = vec![code_a, code_b];
    let states: Vec<_> = graphs
        .iter()
        .zip(codes.iter())
        .map(|(graph, code)| StateRef { graph, code })
        .collect();
    let opts = RunningOpts::default();
    let report = fit_running(&states, &opts).expect("running");
    assert_eq!(report.steps.len(), 2);
    assert!(report.pass);
    for value in report.beta_summary.dg_dlog_mu.iter() {
        assert!(value.is_finite());
        assert!(value.abs() <= report.thresholds.beta_tolerance + 1e-6);
    }
    assert!(report.beta_summary.dlambda_dlog_mu.is_finite());
}
