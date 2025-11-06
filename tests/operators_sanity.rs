use std::fs;
use std::path::PathBuf;

use asm_code::{serde as code_serde, CSSCode};
use asm_graph::{graph_from_json, HypergraphImpl};
use asm_spec::{build_operators, OpOpts};

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
fn operators_are_deterministic() {
    let (code, graph) = load_fixture();
    let opts = OpOpts::default();
    let first = build_operators(&graph, &code, &opts).expect("operators");
    let second = build_operators(&graph, &code, &opts).expect("operators");
    assert_eq!(first.info, second.info);
    assert_eq!(first.entries, second.entries);
}
