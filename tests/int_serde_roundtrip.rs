use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_code::serde as code_serde;
use asm_gauge::from_json_slice as gauge_from_slice;
use asm_graph::graph_from_json;
use asm_int::{
    fit_running,
    interact_full,
    serde::{from_json_slice, to_canonical_json_bytes},
    FitOpts,
    KernelMode,
    KernelOpts,
    MeasureOpts,
    PrepSpec,
    RunningOpts,
};
use asm_rg::StateRef;
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

fn load_running_states() -> (Vec<asm_graph::HypergraphImpl>, Vec<asm_code::CSSCode>) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dirs = [
        "fixtures/validation_vacua/t1_seed0/end_state",
        "fixtures/validation_vacua/t1_seed1/end_state",
    ];
    let mut graphs = Vec::new();
    let mut codes = Vec::new();
    for dir in dirs {
        let graph_json = fs::read_to_string(base.join(dir).join("graph.json")).expect("graph");
        let code_json = fs::read_to_string(base.join(dir).join("code.json")).expect("code");
        graphs.push(graph_from_json(&graph_json).expect("graph decode"));
        codes.push(code_serde::from_json(&code_json).expect("code decode"));
    }
    (graphs, codes)
}

#[test]
fn interaction_artifacts_roundtrip() -> Result<(), Box<dyn Error>> {
    let (spectrum, gauge) = load_reports();
    let prep = PrepSpec::default();
    let mut kernel = KernelOpts::default();
    kernel.mode = KernelMode::Light;
    kernel.save_trajectory = true;
    let measure = MeasureOpts::default();
    let fit = FitOpts::default();
    let running_opts = RunningOpts::default();

    let (prepared, trajectory, obs, couplings, interaction) =
        interact_full(&spectrum, &gauge, &prep, &kernel, &measure, &fit, 902)?;

    let prepared_bytes = to_canonical_json_bytes(&prepared)?;
    assert_eq!(prepared, from_json_slice(&prepared_bytes)?);

    let traj_bytes = to_canonical_json_bytes(&trajectory)?;
    assert_eq!(trajectory, from_json_slice(&traj_bytes)?);

    let obs_bytes = to_canonical_json_bytes(&obs)?;
    assert_eq!(obs, from_json_slice(&obs_bytes)?);

    let fit_bytes = to_canonical_json_bytes(&couplings)?;
    assert_eq!(couplings, from_json_slice(&fit_bytes)?);

    let report_bytes = to_canonical_json_bytes(&interaction)?;
    assert_eq!(interaction, from_json_slice(&report_bytes)?);

    let (graphs, codes) = load_running_states();
    let states: Vec<_> = graphs
        .iter()
        .zip(codes.iter())
        .map(|(graph, code)| StateRef { graph, code })
        .collect();
    let running = fit_running(&states, &running_opts)?;
    let running_bytes = to_canonical_json_bytes(&running)?;
    assert_eq!(running, from_json_slice(&running_bytes)?);

    Ok(())
}
