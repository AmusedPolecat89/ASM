use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use asm_gauge::from_json_slice as gauge_from_slice;
use asm_gauge::GaugeReport;
use asm_int::fit::{FitConfidenceIntervals, FitOpts};
use asm_int::kernel::{KernelOpts, Trajectory, TrajectoryMeta};
use asm_int::measure::MeasureOpts;
use asm_int::report::{InteractionProvenance, InteractionReport};
use asm_int::running::{BetaSummary, RunningReport, RunningStep, RunningThresholds};
use asm_int::CouplingsFit;
use asm_land::report::{PassRates, SummaryReport, SummaryTotals};
use asm_spec::from_json_slice as spec_from_slice;
use asm_spec::SpectrumReport;
use asm_thy::{AssertionInputs, Policy};

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

fn load_fixture_reports() -> (SpectrumReport, GaugeReport) {
    let root = workspace_root();
    let spectrum_bytes =
        fs::read(root.join("fixtures/phase11/t1_seed0/spectrum_report.json")).unwrap();
    let gauge_bytes = fs::read(root.join("fixtures/phase12/t1_seed0/gauge_report.json")).unwrap();
    let spectrum = spec_from_slice(&spectrum_bytes).unwrap();
    let gauge = gauge_from_slice(&gauge_bytes).unwrap();
    (spectrum, gauge)
}

fn sample_couplings() -> CouplingsFit {
    CouplingsFit {
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
    }
}

fn sample_interaction(spectrum: &SpectrumReport, couplings: CouplingsFit) -> InteractionReport {
    InteractionReport {
        analysis_hash: "interaction-sample".to_string(),
        graph_hash: spectrum.graph_hash.clone(),
        code_hash: spectrum.code_hash.clone(),
        prep_hash: "prep-sample".to_string(),
        obs_hash: "obs-sample".to_string(),
        fit: couplings,
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
    }
}

fn sample_running(couplings: &CouplingsFit) -> RunningReport {
    let mut first = couplings.clone();
    first.scale = 1.0;
    first.fit_hash = "running-fit-1".to_string();
    let mut second = couplings.clone();
    second.scale = 1.5;
    second.g[0] += 0.01;
    second.fit_hash = "running-fit-2".to_string();
    RunningReport {
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
    }
}

fn sample_summary() -> SummaryReport {
    SummaryReport {
        totals: SummaryTotals {
            jobs: 2,
            passing: 1,
        },
        pass_rates: PassRates { anthropic: 0.5 },
        distributions: BTreeMap::new(),
        quantiles: BTreeMap::new(),
        correlations: BTreeMap::new(),
        notes: vec![],
    }
}

pub fn sample_inputs() -> (AssertionInputs, Policy) {
    let (spectrum, gauge) = load_fixture_reports();
    let couplings = sample_couplings();
    let interaction = sample_interaction(&spectrum, couplings.clone());
    let running = sample_running(&couplings);
    let summary = sample_summary();

    let mut inputs = AssertionInputs::default();
    inputs.spectrum = Some(spectrum);
    inputs.gauge = Some(gauge);
    inputs.interaction = Some(interaction);
    inputs.running = Some(running);
    inputs.summary = Some(summary);
    (inputs, Policy::default())
}
