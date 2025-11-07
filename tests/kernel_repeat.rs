use std::fs;
use std::path::PathBuf;

use asm_gauge::from_json_slice as gauge_from_slice;
use asm_int::{evolve, prepare_state, KernelMode, KernelOpts, PrepSpec};
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
fn kernel_replay_is_identical() {
    let (spectrum, gauge) = load_reports();
    let prep_spec = PrepSpec::default();
    let prepared = prepare_state(&spectrum, &gauge, &prep_spec, 2401).expect("prepared");
    let mut kernel = KernelOpts::default();
    kernel.mode = KernelMode::Light;
    kernel.save_trajectory = true;

    let traj_a = evolve(&prepared, &kernel).expect("traj a");
    let traj_b = evolve(&prepared, &kernel).expect("traj b");
    assert_eq!(traj_a, traj_b);
    assert!(!traj_a.meta.traj_hash.is_empty());
}
