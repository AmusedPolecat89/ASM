use asm_aut::AnalysisReport;
use asm_gauge::{build_rep, RepOpts};
use asm_spec::{from_json_slice as spectrum_from_slice, SpectrumReport};

fn load_spectrum() -> SpectrumReport {
    let bytes = include_bytes!("../fixtures/phase11/t1_seed0/spectrum_report.json");
    spectrum_from_slice(bytes).expect("decode spectrum")
}

fn load_analysis() -> AnalysisReport {
    let json = include_str!("../fixtures/phase12/analysis/t1_seed0/analysis_report.json");
    serde_json::from_str(json).expect("decode analysis")
}

#[test]
fn rep_determinism() {
    let spectrum = load_spectrum();
    let analysis = load_analysis();
    let rep_a = build_rep(&spectrum, &analysis, &RepOpts::default()).expect("rep");
    let rep_b = build_rep(&spectrum, &analysis, &RepOpts::default()).expect("rep");
    assert_eq!(rep_a, rep_b);
}
