use std::fs;
use std::path::PathBuf;

use asm_code::{serde as code_serde, CSSCode};
use asm_exp::{
    build_runbook, deform, estimate_gaps, from_json_slice, run_ablation, sweep,
    to_canonical_json_bytes, AblationMode, AblationPlan, DeformSpec, GapMethod, GapOpts,
    GridParameter, RunMeta, SweepPlan, SweepStrategy, ToleranceSpec,
};
use asm_graph::{graph_from_json, HypergraphImpl};
use asm_rg::StateRef;
use serde_json::json;

fn load_fixture() -> (CSSCode, HypergraphImpl) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let code_path = base
        .join("fixtures/validation_vacua/t1_seed0/end_state/code.json");
    let graph_path = base
        .join("fixtures/validation_vacua/t1_seed0/end_state/graph.json");
    let code_json = fs::read_to_string(code_path).expect("code fixture");
    let graph_json = fs::read_to_string(graph_path).expect("graph fixture");
    let code = code_serde::from_json(&code_json).expect("decode code");
    let graph = graph_from_json(&graph_json).expect("decode graph");
    (code, graph)
}

#[test]
fn schema_roundtrips_are_identical() {
    let (code, graph) = load_fixture();
    let state = StateRef {
        graph: &graph,
        code: &code,
    };
    let spec = DeformSpec::degree_tweak(2);
    let deform_report = deform(&state, &spec, 42).expect("deform");
    let deform_bytes = to_canonical_json_bytes(&deform_report).expect("json");
    let parsed_deform: asm_exp::DeformationReport =
        from_json_slice(&deform_bytes).expect("parse");
    assert_eq!(deform_report, parsed_deform);

    let plan = SweepPlan {
        strategy: SweepStrategy::Grid {
            parameters: vec![GridParameter {
                name: "scale".to_string(),
                values: vec![json!(2), json!(3)],
            }],
        },
        scheduler: Default::default(),
    };
    let sweep_report = sweep(&plan, 7).expect("sweep");
    let sweep_bytes = to_canonical_json_bytes(&sweep_report).expect("json");
    let parsed_sweep: asm_exp::SweepReport = from_json_slice(&sweep_bytes).expect("parse");
    assert_eq!(sweep_report, parsed_sweep);

    let gap_opts = GapOpts {
        method: GapMethod::Dispersion,
        thresholds: json!({"max": 0.2}),
        tolerance: 0.05,
    };
    let gap_report = estimate_gaps(&state, &gap_opts).expect("gaps");
    let gap_bytes = to_canonical_json_bytes(&gap_report).expect("json");
    let parsed_gap: asm_exp::GapReport = from_json_slice(&gap_bytes).expect("parse");
    assert_eq!(gap_report, parsed_gap);

    let ablation_plan = AblationPlan {
        name: "roundtrip".into(),
        mode: AblationMode::Grid,
        samples: None,
        factors: [("sampler.steps".into(), vec![json!(1), json!(2)])]
            .into_iter()
            .collect(),
        fixed: [("rg.scale".into(), json!(2))].into_iter().collect(),
        tolerances: [(
            "kpi".into(),
            ToleranceSpec {
                min: Some(0.0),
                max: Some(1.0),
                abs: Some(1e-6),
                rel: Some(1e-3),
            },
        )]
        .into_iter()
        .collect(),
    };
    let ablation_report = run_ablation(&ablation_plan, 9001).expect("ablation");
    let ablation_bytes = to_canonical_json_bytes(&ablation_report).expect("json");
    let parsed_ablation: asm_exp::AblationReport =
        from_json_slice(&ablation_bytes).expect("parse");
    assert_eq!(ablation_report, parsed_ablation);

    let inputs = [PathBuf::from("runs/demo"), PathBuf::from("sweeps/demo/job_0000")];
    let meta = RunMeta {
        created_at: "2024-01-01T00:00:00Z".into(),
        commit: "deadbeef".into(),
        seeds: vec![1, 2, 3],
        artifacts: vec!["analysis/a.json".into()],
        summary: json!({"jobs": 1}),
    };
    let runbook = build_runbook(&inputs, &meta).expect("runbook");
    let runbook_bytes = to_canonical_json_bytes(&runbook).expect("json");
    let parsed_runbook: asm_exp::RunBook = from_json_slice(&runbook_bytes).expect("parse");
    assert_eq!(runbook, parsed_runbook);
}

fn load_gauge_inputs() -> (asm_spec::SpectrumReport, asm_aut::AnalysisReport) {
    let spectrum_bytes = include_bytes!("../fixtures/phase11/t1_seed0/spectrum_report.json");
    let spectrum = asm_spec::from_json_slice(spectrum_bytes).expect("spectrum");
    let analysis_json = include_str!("../fixtures/phase12/analysis/t1_seed0/analysis_report.json");
    let analysis: asm_aut::AnalysisReport = serde_json::from_str(analysis_json).expect("analysis");
    (spectrum, analysis)
}

#[test]
fn gauge_representation_roundtrip() {
    let (spectrum, analysis) = load_gauge_inputs();
    let rep = asm_gauge::build_rep(&spectrum, &analysis, &asm_gauge::RepOpts::default())
        .expect("rep");
    let bytes = asm_gauge::to_canonical_json_bytes(&rep).expect("json");
    let decoded = asm_gauge::from_json_slice::<asm_gauge::RepMatrices>(&bytes).expect("decode");
    assert_eq!(rep, decoded);
}

#[test]
fn gauge_report_roundtrip_is_stable() {
    let (spectrum, analysis) = load_gauge_inputs();
    let opts = asm_gauge::GaugeOpts { seed: 6060, ..asm_gauge::GaugeOpts::default() };
    let report = asm_gauge::analyze_gauge(&spectrum, &analysis, &spectrum.operators.info, &opts)
        .expect("gauge report");
    let bytes = asm_gauge::to_canonical_json_bytes(&report).expect("json");
    let decoded = asm_gauge::from_json_slice::<asm_gauge::GaugeReport>(&bytes).expect("decode");
    assert_eq!(report, decoded);
}
