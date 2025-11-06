use std::fs;
use std::path::PathBuf;

use asm_code::{serde as code_serde, CSSCode};
use asm_graph::{graph_from_json, HypergraphImpl};
use asm_spec::{
    analyze_spectrum, from_json_slice, to_canonical_json_bytes, CorrelSpec, DispersionSpec,
    ExcitationSpec, OpOpts, PropOpts, SpecOpts,
};

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
fn spectrum_report_roundtrips() {
    let (code, graph) = load_fixture();
    let mut dispersion = DispersionSpec::default();
    dispersion.k_points = 16;
    dispersion.modes = 2;
    let spec_opts = SpecOpts {
        ops: OpOpts::default(),
        excitation: ExcitationSpec::default(),
        propagation: PropOpts {
            iterations: 16,
            tolerance: 1e-6,
            seed: 7777,
        },
        dispersion,
        correlation: CorrelSpec::default(),
        master_seed: 9999,
        fit_tolerance: 1e-6,
    };
    let report = analyze_spectrum(&graph, &code, &spec_opts).expect("spectrum");
    let bytes = to_canonical_json_bytes(&report).expect("serialize");
    let restored = from_json_slice::<asm_spec::SpectrumReport>(&bytes).expect("deserialize");
    assert_eq!(report, restored);
}
