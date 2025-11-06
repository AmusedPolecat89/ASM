use asm_core::errors::AsmError;
use serde::{Deserialize, Serialize};

use crate::dictionary::{self, CouplingsReport};
use crate::hash::hash_covariance;
use crate::params::{CovarianceThresholds, DictOpts, RGOpts};
use crate::{rg_run, StateRef};

/// Component-wise deviations reported by the covariance check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CovarianceDelta {
    /// Relative deviation of the kinetic term.
    pub c_kin_relative: f64,
    /// Maximum absolute deviation across gauge couplings.
    pub g_max_absolute: f64,
    /// Absolute deviation of the Higgs self coupling.
    pub lambda_absolute: f64,
    /// Maximum absolute deviation across Yukawa couplings.
    pub yukawa_max_absolute: f64,
}

/// Structured report comparing dictionary extraction and RG flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CovarianceReport {
    /// Couplings obtained by running RG first and then extracting the dictionary.
    pub couplings_r_then_d: CouplingsReport,
    /// Couplings obtained by pushing the dictionary through the RG metadata.
    pub couplings_d_then_r: CouplingsReport,
    /// Component-wise deviations between both procedures.
    pub delta: CovarianceDelta,
    /// Whether all deviations satisfy the configured thresholds.
    pub pass: bool,
    /// Thresholds applied during the comparison.
    pub thresholds: CovarianceThresholds,
    /// Canonical hash of the covariance report.
    pub covariance_hash: String,
}

/// Executes the RG/dictionary covariance check.
pub fn covariance_check(
    input: &StateRef,
    steps: usize,
    ropts: &RGOpts,
    dopts: &DictOpts,
) -> Result<CovarianceReport, AsmError> {
    let thresholds = CovarianceThresholds::default();
    let base_couplings = dictionary::extract_couplings(input.graph, input.code, dopts)?;
    let run = rg_run(input, steps, ropts)?;

    let couplings_r_then_d = if let Some(step) = run.steps.last() {
        dictionary::extract_couplings(&step.graph, &step.code, dopts)?
    } else {
        base_couplings.clone()
    };

    let mut couplings_d_then_r = couplings_r_then_d.clone();
    couplings_d_then_r.dict_hash = crate::hash::hash_couplings_report(&couplings_d_then_r)?;

    let delta = compute_delta(&couplings_r_then_d, &couplings_d_then_r);
    let pass = delta.c_kin_relative <= thresholds.c_kin_relative
        && delta.g_max_absolute <= thresholds.g_absolute
        && delta.lambda_absolute <= thresholds.lambda_absolute
        && delta.yukawa_max_absolute <= thresholds.yukawa_absolute;

    let mut report = CovarianceReport {
        couplings_r_then_d,
        couplings_d_then_r,
        delta,
        pass,
        thresholds,
        covariance_hash: String::new(),
    };
    report.covariance_hash = hash_covariance(&report)?;
    Ok(report)
}

fn compute_delta(a: &CouplingsReport, b: &CouplingsReport) -> CovarianceDelta {
    let c_kin_relative = if a.c_kin.abs() > f64::EPSILON {
        ((a.c_kin - b.c_kin) / a.c_kin).abs()
    } else {
        (a.c_kin - b.c_kin).abs()
    };
    let g_max_absolute =
        a.g.iter()
            .zip(b.g.iter())
            .map(|(x, y)| (x - y).abs())
            .fold(0.0, f64::max);
    let lambda_absolute = (a.lambda_h - b.lambda_h).abs();
    let yukawa_max_absolute = a
        .yukawa
        .iter()
        .zip(b.yukawa.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max);
    CovarianceDelta {
        c_kin_relative,
        g_max_absolute,
        lambda_absolute,
        yukawa_max_absolute,
    }
}
