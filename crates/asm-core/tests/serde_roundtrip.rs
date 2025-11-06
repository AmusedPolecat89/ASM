use asm_core::provenance::{RunProvenance, SchemaVersion};
use asm_core::Couplings;

#[test]
fn couplings_round_trip_json() {
    let provenance = RunProvenance {
        input_hash: "input".into(),
        graph_hash: "graph".into(),
        code_hash: "code".into(),
        seed: 99,
        created_at: "2023-10-31T00:00:00Z".into(),
        tool_versions: [("asm-core".into(), "0.1.0".into())].into_iter().collect(),
    };
    let couplings = Couplings {
        schema_version: SchemaVersion::new(1, 0, 0),
        provenance: provenance.clone(),
        c_kin: 1.0,
        gauge: [1.0, 2.0, 3.0],
        yukawa: vec![0.1, 0.2, 0.3],
        lambda_h: 0.5,
        notes: Some("roundtrip".into()),
    };

    let json = serde_json::to_string_pretty(&couplings).expect("serialize");
    let decoded: Couplings = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(decoded, couplings);
    assert_eq!(decoded.provenance, provenance);
}
