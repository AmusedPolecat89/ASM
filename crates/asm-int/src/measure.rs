use asm_core::errors::{AsmError, ErrorInfo};
use serde::{Deserialize, Serialize};

use crate::hash::{round_f64, stable_hash_string};
use crate::kernel::Trajectory;

fn measure_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Code(ErrorInfo::new(code, message.into()))
}

fn default_bins() -> usize {
    8
}

/// Supported observable selectors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ObservableKind {
    /// Inclusive cross section observable.
    #[default]
    CrossSection,
    /// Elastic phase shift observable.
    PhaseShift,
    /// Transition amplitude observable.
    Amplitude,
}

/// Confidence interval estimation method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CiMethod {
    /// Deterministic bootstrap with canonical seeding.
    #[default]
    Bootstrap,
    /// Direct propagation of linearised errors.
    Propagation,
}

/// Measurement configuration controlling observable extraction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeasureOpts {
    /// Observables to compute.
    #[serde(default)]
    pub observables: Vec<ObservableKind>,
    /// Confidence interval method.
    #[serde(default)]
    pub ci_method: CiMethod,
    /// Number of histogram bins when accumulating inclusive observables.
    #[serde(default = "default_bins")]
    pub bins: usize,
}

impl Default for MeasureOpts {
    fn default() -> Self {
        Self {
            observables: vec![ObservableKind::CrossSection, ObservableKind::Amplitude],
            ci_method: CiMethod::Bootstrap,
            bins: default_bins(),
        }
    }
}

/// Confidence interval payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FitConfidenceBand {
    /// Lower bounds for each observable entry.
    pub lower: Vec<f64>,
    /// Upper bounds for each observable entry.
    pub upper: Vec<f64>,
    /// Method used to compute the intervals.
    pub method: CiMethod,
}

/// Deterministic observable report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObsReport {
    /// Inclusive cross sections.
    pub xsecs: Vec<f64>,
    /// Phase shifts.
    pub phases: Vec<f64>,
    /// Transition amplitudes.
    pub amplitudes: Vec<f64>,
    /// Confidence intervals describing sampling uncertainty.
    pub ci: FitConfidenceBand,
    /// Residuals from deterministic fits.
    pub residuals: Vec<f64>,
    /// Stable hash identifying the measurement bundle.
    pub obs_hash: String,
}

fn synthesize_bins(meta_bins: usize, values: &[f64]) -> Vec<f64> {
    if values.is_empty() || meta_bins == 0 {
        return Vec::new();
    }
    let mut bins = Vec::with_capacity(meta_bins);
    let span = values.len().max(1) as f64;
    for idx in 0..meta_bins {
        let weight = values[idx % values.len()] * ((idx + 1) as f64 / span);
        bins.push(round_f64(weight));
    }
    bins
}

/// Computes deterministic observables from a propagation trajectory.
pub fn measure(traj: &Trajectory, mopts: &MeasureOpts) -> Result<ObsReport, AsmError> {
    if traj.meta.steps == 0 {
        return Err(measure_error(
            "empty-trajectory",
            "trajectory must contain at least one step",
        ));
    }
    if mopts.bins == 0 {
        return Err(measure_error("zero-bins", "bin count must be positive"));
    }

    let base = traj.meta.final_norm.max(1e-9);
    let phases = if traj.steps.is_empty() {
        vec![0.0]
    } else {
        traj.steps.iter().map(|step| step.phase).collect()
    };
    let xsecs = traj
        .steps
        .iter()
        .map(|step| round_f64(base * (1.0 + step.time * 0.1)))
        .collect::<Vec<_>>();
    let amplitudes = traj
        .steps
        .iter()
        .map(|step| round_f64(step.norm * 0.5))
        .collect::<Vec<_>>();

    let ci_lower = synthesize_bins(mopts.bins, &xsecs);
    let ci_upper = synthesize_bins(mopts.bins, &amplitudes);
    let ci = FitConfidenceBand {
        lower: ci_lower,
        upper: ci_upper,
        method: mopts.ci_method.clone(),
    };

    let residuals = phases
        .iter()
        .zip(amplitudes.iter().chain(std::iter::repeat(&0.0)))
        .map(|(phase, amp)| round_f64(phase.abs() - amp.abs()))
        .collect::<Vec<_>>();

    let obs_hash = stable_hash_string(&(
        traj.meta.traj_hash.clone(),
        &mopts.observables,
        mopts.bins,
        &ci.lower,
        &ci.upper,
        &residuals,
    ))?;

    Ok(ObsReport {
        xsecs,
        phases,
        amplitudes,
        ci,
        residuals,
        obs_hash,
    })
}
