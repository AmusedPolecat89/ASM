use std::fs;
use std::path::PathBuf;

use asm_code::{serde as code_serde, CSSCode};
use asm_exp::{estimate_gaps, GapMethod, GapOpts};
use asm_graph::{graph_from_json, HypergraphImpl};
use asm_rg::StateRef;

fn load_fixture() -> (CSSCode, HypergraphImpl) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let code_path = base
        .join("fixtures/validation_vacua/t1_seed0/end_state/code.json");
    let graph_path = base
        .join("fixtures/validation_vacua/t1_seed0/end_state/graph.json");
    let code_json = fs::read_to_string(code_path).expect("code fixture");
    let graph_json = fs::read_to_string(graph_path).expect("graph fixture");
    let code = code_serde::from_json(&code_json).expect("decode code");
    let graph = graph_from_json(&graph_json).expect("decode graph");
    (code, graph)
}

#[test]
fn gap_estimates_repeat_within_tolerance() {
    let (code, graph) = load_fixture();
    let state = StateRef {
        graph: &graph,
        code: &code,
    };
    let opts = GapOpts {
        method: GapMethod::Spectral,
        thresholds: serde_json::json!({"min": 0.05}),
        tolerance: 0.01,
    };
    let report_a = estimate_gaps(&state, &opts).expect("gaps");
    let report_b = estimate_gaps(&state, &opts).expect("gaps");
    let diff = (report_a.gap_value - report_b.gap_value).abs();
    let denom = report_a.gap_value.abs().max(1e-9);
    assert!(diff / denom <= 0.05, "gap diff {diff}");
    assert_eq!(report_a.method, "spectral");
    assert_eq!(report_a, report_b);
}
