use asm_code::serde::{from_bytes, from_json, to_bytes, to_json};
use asm_code::CSSCode;
use asm_core::{RunProvenance, SchemaVersion};

fn provenance() -> RunProvenance {
    RunProvenance {
        input_hash: "input".into(),
        graph_hash: "graph".into(),
        code_hash: String::new(),
        seed: 29,
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

#[test]
fn json_round_trip() {
    let code = build_code();
    let json = to_json(&code).unwrap();
    let restored = from_json(&json).unwrap();
    assert_eq!(code.canonical_hash(), restored.canonical_hash());
}

#[test]
fn binary_round_trip() {
    let code = build_code();
    let bytes = to_bytes(&code).unwrap();
    let restored = from_bytes(&bytes).unwrap();
    assert_eq!(code.canonical_hash(), restored.canonical_hash());
}
