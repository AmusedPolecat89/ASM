use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::rng::derive_substream_seed;
use asm_rg::StateRef;
use serde::{Deserialize, Serialize};

use crate::fit::{couplings_from_seed, CouplingsFit, FitOpts};
use crate::hash::canonical_state_hash;
use crate::hash::{round_f64, seed_from_hash, stable_hash_string};

fn running_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::RG(ErrorInfo::new(code, message.into()))
}

fn default_beta_window() -> usize {
    3
}

fn default_beta_tolerance() -> f64 {
    0.05
}

/// Short β-function style summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BetaSummary {
    /// Averaged β estimate for each gauge coupling.
    pub dg_dlog_mu: [f64; 3],
    /// β estimate for the quartic coupling.
    pub dlambda_dlog_mu: f64,
}

/// Thresholds applied when validating running consistency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunningThresholds {
    /// Maximum tolerated β-norm.
    pub beta_tolerance: f64,
    /// Sliding window used when computing finite differences.
    pub beta_window: usize,
}

/// Per-step running entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunningStep {
    /// Reference energy scale.
    pub scale: f64,
    /// Coupling fit associated with the step.
    pub fit: CouplingsFit,
}

/// Configuration for the running fit procedure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunningOpts {
    /// Optional explicit scales for the RG steps.
    #[serde(default)]
    pub explicit_scales: Vec<f64>,
    /// Sliding window size when estimating finite differences.
    #[serde(default = "default_beta_window")]
    pub beta_window: usize,
    /// Maximum tolerated β-norm.
    #[serde(default = "default_beta_tolerance")]
    pub beta_tolerance: f64,
    /// Coupling fit options reused across steps.
    #[serde(default)]
    pub fit: FitOpts,
}

impl Default for RunningOpts {
    fn default() -> Self {
        Self {
            explicit_scales: Vec::new(),
            beta_window: default_beta_window(),
            beta_tolerance: default_beta_tolerance(),
            fit: FitOpts::default(),
        }
    }
}

impl RunningOpts {
    /// Constructs running options using an existing fit configuration.
    pub fn from_fit(fit: FitOpts) -> Self {
        Self {
            fit,
            ..Self::default()
        }
    }
}

/// Aggregate running report summarising couplings across RG steps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunningReport {
    /// Per-step running data.
    pub steps: Vec<RunningStep>,
    /// Aggregated β-function style summary.
    pub beta_summary: BetaSummary,
    /// Whether all validation checks passed.
    pub pass: bool,
    /// Thresholds used during validation.
    pub thresholds: RunningThresholds,
    /// Stable hash of the running bundle.
    pub running_hash: String,
}

fn compute_scales(opts: &RunningOpts, len: usize) -> Vec<f64> {
    if !opts.explicit_scales.is_empty() {
        let mut scales = opts.explicit_scales.clone();
        scales.resize(len, *opts.explicit_scales.last().unwrap_or(&1.0));
        return scales
            .into_iter()
            .map(|scale| round_f64(scale.max(1e-6)))
            .collect();
    }
    (0..len)
        .map(|idx| round_f64(1.0 + idx as f64 * 0.25))
        .collect()
}

fn estimate_beta(entries: &[RunningStep]) -> BetaSummary {
    if entries.len() < 2 {
        return BetaSummary {
            dg_dlog_mu: [0.0; 3],
            dlambda_dlog_mu: 0.0,
        };
    }
    let mut dg = [0.0; 3];
    let mut dlambda = 0.0;
    let mut count = 0.0;
    for pair in entries.windows(2) {
        let first = &pair[0];
        let second = &pair[1];
        let log_ratio = (second.scale / first.scale).ln().max(1e-6);
        for (idx, value) in dg.iter_mut().enumerate() {
            *value += (second.fit.g[idx] - first.fit.g[idx]) / log_ratio;
        }
        dlambda += (second.fit.lambda_h - first.fit.lambda_h) / log_ratio;
        count += 1.0;
    }
    if count > 0.0 {
        for value in dg.iter_mut() {
            *value = round_f64(*value / count);
        }
        dlambda = round_f64(dlambda / count);
    }
    BetaSummary {
        dg_dlog_mu: dg,
        dlambda_dlog_mu: dlambda,
    }
}

fn validate_beta(summary: &BetaSummary, opts: &RunningOpts) -> bool {
    summary
        .dg_dlog_mu
        .iter()
        .all(|value| value.abs() <= opts.beta_tolerance)
        && summary.dlambda_dlog_mu.abs() <= opts.beta_tolerance
}

/// Fits couplings for each RG step and aggregates a deterministic running report.
pub fn fit_running(
    rg_chain: &[StateRef<'_>],
    opts: &RunningOpts,
) -> Result<RunningReport, AsmError> {
    if rg_chain.is_empty() {
        return Err(running_error(
            "empty-chain",
            "running requires at least one RG state",
        ));
    }

    let mut steps = Vec::new();
    let scales = compute_scales(opts, rg_chain.len());
    for (idx, state) in rg_chain.iter().enumerate() {
        let hash = canonical_state_hash(state)?;
        let seed = seed_from_hash(&hash) ^ derive_substream_seed(idx as u64 + 1, 11);
        let mut fit = couplings_from_seed(seed, &opts.fit);
        fit.scale = round_f64(scales[idx]);
        fit.fit_hash = stable_hash_string(&(
            fit.scale,
            &fit.g,
            fit.lambda_h,
            &fit.yukawa,
            &fit.ci.g,
            fit.ci.lambda_h,
            fit.ci.yukawa,
            fit.fit_resid,
        ))?;
        steps.push(RunningStep {
            scale: fit.scale,
            fit,
        });
    }

    let beta_summary = estimate_beta(&steps);
    let thresholds = RunningThresholds {
        beta_tolerance: opts.beta_tolerance,
        beta_window: opts.beta_window,
    };
    let pass = validate_beta(&beta_summary, opts);
    let running_hash = stable_hash_string(&(
        &steps,
        &beta_summary.dg_dlog_mu,
        beta_summary.dlambda_dlog_mu,
        thresholds.beta_tolerance,
        thresholds.beta_window,
        pass,
    ))?;

    Ok(RunningReport {
        steps,
        beta_summary,
        pass,
        thresholds,
        running_hash,
    })
}
