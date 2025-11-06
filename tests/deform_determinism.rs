use std::fs;
use std::path::PathBuf;

use asm_code::{serde as code_serde, CSSCode};
use asm_exp::{deform, to_canonical_json_bytes, DeformSpec};
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
fn deform_reports_are_deterministic() {
    let (code, graph) = load_fixture();
    let state = StateRef {
        graph: &graph,
        code: &code,
    };
    let spec = DeformSpec::degree_tweak(1);
    let report_a = deform(&state, &spec, 7101).expect("deformation");
    let report_b = deform(&state, &spec, 7101).expect("deformation");
    assert_eq!(report_a, report_b);
    let bytes_a = to_canonical_json_bytes(&report_a).expect("json");
    let bytes_b = to_canonical_json_bytes(&report_b).expect("json");
    assert_eq!(bytes_a, bytes_b);
}
