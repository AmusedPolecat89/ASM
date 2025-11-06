use std::fs;
use std::path::PathBuf;

use asm_code::{serde as code_serde, CSSCode};
use asm_graph::{graph_from_json, HypergraphImpl};
use asm_spec::{build_operators, correlation_scan, CorrelSpec, OpOpts};

fn load_fixture() -> (CSSCode, HypergraphImpl) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let code_path = base.join("fixtures/validation_vacua/t1_seed0/end_state/code.json");
    let graph_path = base.join("fixtures/validation_vacua/t1_seed0/end_state/graph.json");
    let code_json = fs::read_to_string(code_path).expect("code fixture");
    let graph_json = fs::read_to_string(graph_path).expect("graph fixture");
    let code = code_serde::from_json(&code_json).expect("decode code");
    let graph = graph_from_json(&graph_json).expect("decode graph");
    (code, graph)
}

#[test]
fn correlation_estimates_stable() {
    let (code, graph) = load_fixture();
    let operators = build_operators(&graph, &code, &OpOpts::default()).expect("operators");
    let spec = CorrelSpec::default();
    let first = correlation_scan(&operators, &spec, 9001).expect("correlation");
    let second = correlation_scan(&operators, &spec, 9001).expect("correlation");
    assert_eq!(first.method, second.method);
    assert_eq!(first.ci, second.ci);
    assert_eq!(first.residuals, second.residuals);
    let rel = if first.xi.abs() > f64::EPSILON {
        ((first.xi - second.xi) / first.xi).abs()
    } else {
        0.0
    };
    assert!(rel <= 0.05, "correlation length drifted: {rel}");
}
