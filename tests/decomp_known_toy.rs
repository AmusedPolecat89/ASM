use asm_aut::AnalysisReport;
use asm_gauge::{build_rep, decompose, DecompOpts, RepOpts};
use asm_spec::{from_json_slice as spectrum_from_slice, SpectrumReport};

fn load_inputs() -> (SpectrumReport, AnalysisReport) {
    let spectrum_bytes = include_bytes!("../fixtures/phase11/t1_seed0/spectrum_report.json");
    let spectrum = spectrum_from_slice(spectrum_bytes).expect("spectrum");
    let analysis_json = include_str!("../fixtures/phase12/analysis/t1_seed0/analysis_report.json");
    let analysis = serde_json::from_str(analysis_json).expect("analysis");
    (spectrum, analysis)
}

#[test]
fn decomposition_labels_expected_factors() {
    let (spectrum, analysis) = load_inputs();
    let rep = build_rep(&spectrum, &analysis, &RepOpts::default()).expect("rep");
    let report = decompose(&rep, &DecompOpts::default()).expect("decomp");
    assert!(!report.factors.is_empty());
    let types: Vec<_> = report.factors.iter().map(|factor| factor.r#type.as_str()).collect();
    assert!(types.contains(&"su2"));
    assert!(types.contains(&"u1"));
}
