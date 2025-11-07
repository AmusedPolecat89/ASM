use std::fs;
use std::path::PathBuf;

use asm_gauge::from_json_slice as gauge_from_slice;
use asm_int::{prepare_state, PrepSpec};
use asm_spec::from_json_slice as spec_from_slice;

fn load_reports() -> (asm_spec::SpectrumReport, asm_gauge::GaugeReport) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let spectrum_bytes = fs::read(base.join("fixtures/phase11/t1_seed0/spectrum_report.json"))
        .expect("spectrum fixture");
    let gauge_bytes = fs::read(base.join("fixtures/phase12/t1_seed0/gauge_report.json"))
        .expect("gauge fixture");
    let spectrum = spec_from_slice(&spectrum_bytes).expect("decode spectrum");
    let gauge = gauge_from_slice(&gauge_bytes).expect("decode gauge");
    (spectrum, gauge)
}

#[test]
fn preparation_is_deterministic() {
    let (spectrum, gauge) = load_reports();
    let prep_spec = PrepSpec::default();
    let state_a = prepare_state(&spectrum, &gauge, &prep_spec, 42).expect("prep a");
    let state_b = prepare_state(&spectrum, &gauge, &prep_spec, 42).expect("prep b");
    assert_eq!(state_a, state_b);
    assert!(!state_a.prep_hash.is_empty());
}
