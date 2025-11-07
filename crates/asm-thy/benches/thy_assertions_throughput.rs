use std::fs;
use std::path::PathBuf;

use asm_gauge::from_json_slice as gauge_from_slice;
use asm_int::fit::{FitConfidenceIntervals, FitOpts};
use asm_int::kernel::{KernelOpts, Trajectory, TrajectoryMeta};
use asm_int::measure::MeasureOpts;
use asm_int::report::{InteractionProvenance, InteractionReport};
use asm_int::running::{BetaSummary, RunningReport, RunningStep, RunningThresholds};
use asm_int::CouplingsFit;
use asm_spec::from_json_slice as spec_from_slice;
use asm_spec::SpectrumReport;
use asm_thy::serde::to_canonical_json_bytes;
use asm_thy::{run_assertions, AssertionInputs, Policy};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

fn load_fixture_reports() -> (SpectrumReport, asm_gauge::GaugeReport) {
    let root = workspace_root();
    let spectrum_bytes =
        fs::read(root.join("fixtures/phase11/t1_seed0/spectrum_report.json")).expect("spectrum");
    let gauge_bytes =
        fs::read(root.join("fixtures/phase12/t1_seed0/gauge_report.json")).expect("gauge");
    let spectrum = spec_from_slice(&spectrum_bytes).expect("decode spectrum");
    let gauge = gauge_from_slice(&gauge_bytes).expect("decode gauge");
    (spectrum, gauge)
}

fn sample_inputs() -> AssertionInputs {
    let (spectrum, gauge) = load_fixture_reports();
    let couplings = CouplingsFit {
        scale: 1.0,
        g: [0.9, 0.8, 1.1],
        lambda_h: 0.2,
        yukawa: vec![0.1, 0.2],
        ci: FitConfidenceIntervals {
            g: [0.05; 3],
            lambda_h: 0.02,
            yukawa: 0.01,
        },
        fit_resid: 1.0,
        fit_hash: "sample-fit".to_string(),
        underdetermined: None,
    };
    let interaction = InteractionReport {
        analysis_hash: "interaction-sample".to_string(),
        graph_hash: spectrum.graph_hash.clone(),
        code_hash: spectrum.code_hash.clone(),
        prep_hash: "prep-sample".to_string(),
        obs_hash: "obs-sample".to_string(),
        fit: couplings.clone(),
        trajectory: Trajectory {
            meta: TrajectoryMeta {
                steps: 4,
                total_time: 0.64,
                final_norm: 1.0,
                traj_hash: "traj-sample".to_string(),
            },
            steps: Vec::new(),
        },
        provenance: InteractionProvenance {
            seed: 42,
            kernel: KernelOpts::default(),
            measure: MeasureOpts::default(),
            fit: FitOpts::default(),
        },
    };
    let mut first = couplings.clone();
    first.scale = 1.0;
    first.fit_hash = "running-fit-1".to_string();
    let mut second = couplings.clone();
    second.scale = 1.5;
    second.g[0] += 0.01;
    second.fit_hash = "running-fit-2".to_string();
    let running = RunningReport {
        steps: vec![
            RunningStep {
                scale: first.scale,
                fit: first,
            },
            RunningStep {
                scale: second.scale,
                fit: second,
            },
        ],
        beta_summary: BetaSummary {
            dg_dlog_mu: [0.01, 0.0, 0.0],
            dlambda_dlog_mu: 0.0,
        },
        pass: true,
        thresholds: RunningThresholds {
            beta_tolerance: 0.05,
            beta_window: 3,
        },
        running_hash: "running-sample".to_string(),
    };
    let mut inputs = AssertionInputs::default();
    inputs.spectrum = Some(spectrum);
    inputs.gauge = Some(gauge);
    inputs.interaction = Some(interaction);
    inputs.running = Some(running);
    inputs
}

fn assertions_benchmark(c: &mut Criterion) {
    let mut inputs = sample_inputs();
    let policy = Policy::default();
    c.bench_function("assertions/light", |b| {
        b.iter(|| {
            let _ = run_assertions(black_box(&inputs), black_box(&policy)).expect("assertions");
        });
    });

    let report = run_assertions(&inputs, &policy).expect("assertions");
    let bench_dir = workspace_root().join("repro/phase15");
    fs::create_dir_all(&bench_dir).expect("bench dir");
    fs::write(
        bench_dir.join("bench_assert.json"),
        to_canonical_json_bytes(&report).expect("serialize"),
    )
    .expect("write bench report");
}

criterion_group!(benches, assertions_benchmark);
criterion_main!(benches);
