use std::fs;
use std::path::PathBuf;

use asm_gauge::from_json_slice as gauge_from_slice;
use asm_int::{
    interact_full,
    FitOpts,
    KernelMode,
    KernelOpts,
    MeasureOpts,
    PrepSpec,
};
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
fn coupling_fit_is_stable() {
    let (spectrum, gauge) = load_reports();
    let prep = PrepSpec::default();
    let mut kernel = KernelOpts::default();
    kernel.mode = KernelMode::Light;
    kernel.save_trajectory = false;
    let measure = MeasureOpts::default();
    let fit = FitOpts::default();

    let (_, _, _, fit_a, _) =
        interact_full(&spectrum, &gauge, &prep, &kernel, &measure, &fit, 77).expect("first run");
    let (_, _, _, fit_b, _) =
        interact_full(&spectrum, &gauge, &prep, &kernel, &measure, &fit, 77).expect("second run");

    assert_eq!(fit_a, fit_b);
    assert!(fit_a.fit_resid.abs() <= 1.0);
}
