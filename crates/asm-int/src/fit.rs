use asm_core::errors::{AsmError, ErrorInfo};
use serde::{Deserialize, Serialize};

use crate::hash::{round_f64, stable_hash_string};
use crate::measure::ObsReport;

fn fit_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Code(ErrorInfo::new(code, message.into()))
}

fn default_tolerance() -> f64 {
    1e-6
}

fn default_max_iters() -> usize {
    64
}

/// Optional bounds applied to the fitted couplings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FitBounds {
    /// Lower bound applied uniformly to the couplings.
    pub min: f64,
    /// Upper bound applied uniformly to the couplings.
    pub max: f64,
}

impl FitBounds {
    fn clamp(&self, value: f64) -> f64 {
        value.clamp(self.min, self.max)
    }
}

/// Deterministic coupling fit configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FitOpts {
    /// Named model variant recorded in the provenance.
    #[serde(default = "default_model_variant")]
    pub model_variant: String,
    /// Optional scalar bounds applied to the couplings.
    pub bounds: Option<FitBounds>,
    /// Optional Gaussian prior strength applied during regularisation.
    pub prior_strength: Option<f64>,
    /// Maximum solver iterations.
    #[serde(default = "default_max_iters")]
    pub max_iters: usize,
    /// Solver tolerance.
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
}

fn default_model_variant() -> String {
    "default".to_string()
}

impl Default for FitOpts {
    fn default() -> Self {
        Self {
            model_variant: default_model_variant(),
            bounds: None,
            prior_strength: None,
            max_iters: default_max_iters(),
            tolerance: default_tolerance(),
        }
    }
}

/// Confidence interval payload for coupling fits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FitConfidenceIntervals {
    /// One-sigma interval for gauge couplings.
    pub g: [f64; 3],
    /// One-sigma interval for the quartic coupling.
    pub lambda_h: f64,
    /// One-sigma interval for the Yukawa sector (shared for brevity).
    pub yukawa: f64,
}

impl FitConfidenceIntervals {
    fn scaled(scale: f64) -> Self {
        Self {
            g: [scale; 3],
            lambda_h: scale * 0.5,
            yukawa: scale * 0.25,
        }
    }
}

/// Deterministic coupling fit report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CouplingsFit {
    /// Reference scale at which the fit was performed.
    pub scale: f64,
    /// Gauge couplings.
    pub g: [f64; 3],
    /// Scalar quartic coupling.
    pub lambda_h: f64,
    /// Yukawa couplings.
    pub yukawa: Vec<f64>,
    /// Confidence intervals associated with the couplings.
    pub ci: FitConfidenceIntervals,
    /// Deterministic residual norm.
    pub fit_resid: f64,
    /// Stable hash of the fit payload.
    pub fit_hash: String,
    /// Optional note when the system is underdetermined.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underdetermined: Option<String>,
}

fn apply_bounds(bounds: Option<&FitBounds>, value: f64) -> f64 {
    bounds.map(|b| b.clamp(value)).unwrap_or(value)
}

fn stabilise(value: f64, opts: &FitOpts) -> f64 {
    let prior = opts.prior_strength.unwrap_or(0.0);
    round_f64(value / (1.0 + prior))
}

fn estimate_scale(obs: &ObsReport) -> f64 {
    let avg = obs
        .xsecs
        .iter()
        .chain(obs.amplitudes.iter())
        .copied()
        .sum::<f64>();
    round_f64((avg / (obs.xsecs.len() + obs.amplitudes.len()).max(1) as f64).max(1e-6))
}

fn estimate_core_couplings(obs: &ObsReport, opts: &FitOpts) -> [f64; 3] {
    let scale = estimate_scale(obs);
    let mut g = [scale, scale * 0.8, scale * 1.2];
    if let Some(bounds) = &opts.bounds {
        for value in g.iter_mut() {
            *value = apply_bounds(Some(bounds), *value);
        }
    }
    g.map(|val| stabilise(val, opts))
}

fn estimate_lambda(obs: &ObsReport, opts: &FitOpts) -> f64 {
    let base = obs
        .phases
        .iter()
        .copied()
        .map(|phase| phase.abs())
        .sum::<f64>()
        / obs.phases.len().max(1) as f64;
    let scaled = apply_bounds(opts.bounds.as_ref(), base * 0.3);
    stabilise(scaled, opts)
}

fn estimate_yukawa(obs: &ObsReport, opts: &FitOpts) -> Vec<f64> {
    if obs.amplitudes.is_empty() {
        return vec![0.0];
    }
    obs.amplitudes
        .iter()
        .map(|amp| stabilise(apply_bounds(opts.bounds.as_ref(), amp * 0.75), opts))
        .collect()
}

fn compute_residual(obs: &ObsReport, fit: &[f64; 3]) -> f64 {
    let target = obs.xsecs.iter().copied().sum::<f64>();
    let model = fit.iter().copied().sum::<f64>();
    round_f64((target - model).abs())
}

/// Fits effective couplings at a reference scale from the measured observables.
pub fn fit_couplings(obs: &ObsReport, fopts: &FitOpts) -> Result<CouplingsFit, AsmError> {
    if obs.xsecs.is_empty() && obs.amplitudes.is_empty() {
        return Err(fit_error(
            "insufficient-observables",
            "at least one observable is required to fit couplings",
        ));
    }

    let scale = estimate_scale(obs);
    let mut g = estimate_core_couplings(obs, fopts);
    let lambda_h = estimate_lambda(obs, fopts);
    let mut yukawa = estimate_yukawa(obs, fopts);
    if yukawa.len() > 8 {
        yukawa.truncate(8);
    }

    let fit_resid = compute_residual(obs, &g);
    let ci = FitConfidenceIntervals::scaled(round_f64(fopts.tolerance.sqrt()));
    let fit_hash = stable_hash_string(&(
        scale,
        &g,
        lambda_h,
        &yukawa,
        &ci.g,
        ci.lambda_h,
        ci.yukawa,
        fit_resid,
        &fopts.model_variant,
    ))?;

    let underdetermined = if obs.xsecs.len() < g.len() {
        Some("fewer observables than couplings".to_string())
    } else {
        None
    };

    // Apply bounds after stabilisation to guarantee deterministic ordering.
    if let Some(bounds) = &fopts.bounds {
        for value in g.iter_mut() {
            *value = round_f64(bounds.clamp(*value));
        }
    }

    Ok(CouplingsFit {
        scale,
        g,
        lambda_h,
        yukawa,
        ci,
        fit_resid,
        fit_hash,
        underdetermined,
    })
}

pub(crate) fn couplings_from_seed(seed: u64, fopts: &FitOpts) -> CouplingsFit {
    use asm_core::rng::{derive_substream_seed, RngHandle};
    use rand::Rng;

    let base = (seed % 10_000) as f64 / 10_000.0 + 0.5;
    let mut rng = RngHandle::from_seed(derive_substream_seed(seed, 7));
    let mut g = [base, base * 1.1, base * 0.9];
    for value in g.iter_mut() {
        let noise = (rng.gen::<f64>() - 0.5) * fopts.tolerance;
        *value = round_f64(*value + noise);
    }
    let lambda_h = round_f64(base * 0.6);
    let mut yukawa = Vec::new();
    for idx in 0..3 {
        let noise = (rng.gen::<f64>() - 0.5) * fopts.tolerance * 0.5;
        yukawa.push(round_f64(base * (0.4 + idx as f64 * 0.1) + noise));
    }
    let ci = FitConfidenceIntervals::scaled(round_f64(fopts.tolerance.sqrt()));
    let fit_resid = round_f64(g.iter().copied().sum::<f64>() * 0.01);
    let fit_hash = stable_hash_string(&(
        seed,
        &g,
        lambda_h,
        &yukawa,
        &ci.g,
        ci.lambda_h,
        ci.yukawa,
        fit_resid,
    ))
    .expect("hash");

    CouplingsFit {
        scale: round_f64(1.0 + base * 0.5),
        g,
        lambda_h,
        yukawa,
        ci,
        fit_resid,
        fit_hash,
        underdetermined: None,
    }
}
