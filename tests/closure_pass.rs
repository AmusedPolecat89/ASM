use asm_aut::AnalysisReport;
use asm_gauge::{build_rep, check_closure, ClosureOpts, RepOpts};
use asm_spec::{from_json_slice as spectrum_from_slice, SpectrumReport};

fn load_inputs() -> (SpectrumReport, AnalysisReport) {
    let spectrum_bytes = include_bytes!("../fixtures/phase11/t1_seed0/spectrum_report.json");
    let spectrum = spectrum_from_slice(spectrum_bytes).expect("spectrum");
    let analysis_json = include_str!("../fixtures/phase12/analysis/t1_seed0/analysis_report.json");
    let analysis = serde_json::from_str(analysis_json).expect("analysis");
    (spectrum, analysis)
}

#[test]
fn closure_residual_within_tol() {
    let (spectrum, analysis) = load_inputs();
    let rep = build_rep(&spectrum, &analysis, &RepOpts::default()).expect("rep");
    let report = check_closure(&rep, &ClosureOpts::default()).expect("closure");
    assert!(report.closed, "closure report should pass: {:?}", report);
    assert!(report.max_dev <= ClosureOpts::default().tolerance + 1e-9);
}
