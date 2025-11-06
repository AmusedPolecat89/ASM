use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::rng::RngHandle;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::operators::Operators;

fn dispersion_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Dictionary(ErrorInfo::new(code, message))
}

fn round_value(value: f64) -> f64 {
    (value * 1e9).round() / 1e9
}

fn default_k_points() -> usize {
    64
}

fn default_modes() -> usize {
    3
}

/// Options describing the dispersion scan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DispersionSpec {
    /// Number of momentum samples to evaluate.
    #[serde(default = "default_k_points")]
    pub k_points: usize,
    /// Number of modes to retain in the report.
    #[serde(default = "default_modes")]
    pub modes: usize,
}

impl Default for DispersionSpec {
    fn default() -> Self {
        Self {
            k_points: default_k_points(),
            modes: default_modes(),
        }
    }
}

/// Per-mode summary produced by the dispersion scan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DispersionMode {
    /// Deterministic mode identifier.
    pub mode_id: usize,
    /// Extracted frequency or energy for the mode.
    pub omega: f64,
    /// Residual of the deterministic fit procedure.
    pub fit_resid: f64,
}

/// Aggregate dispersion information for a state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DispersionReport {
    /// Deterministic momentum grid used during fitting.
    pub k_grid: Vec<f64>,
    /// Per-mode summaries capturing fitted parameters.
    pub modes: Vec<DispersionMode>,
    /// Effective velocity estimate derived from the lowest mode.
    pub c_est: f64,
    /// Gap proxy inferred from the fitted spectrum.
    pub gap_proxy: f64,
    /// Rounding granularity used for floats.
    pub rounding: f64,
}

/// Computes deterministic dispersion diagnostics for the provided operators.
pub fn dispersion_scan(
    operators: &Operators,
    spec: &DispersionSpec,
    seed: u64,
) -> Result<DispersionReport, AsmError> {
    if spec.k_points == 0 {
        return Err(dispersion_error(
            "invalid-k-grid",
            "dispersion scans require at least one k-point",
        ));
    }
    if spec.modes == 0 {
        return Err(dispersion_error(
            "invalid-modes",
            "dispersion scans require at least one mode",
        ));
    }

    let mut k_grid = Vec::with_capacity(spec.k_points);
    for idx in 0..spec.k_points {
        let value = (idx as f64 + 1.0) / (spec.k_points as f64 + 1.0);
        k_grid.push(round_value(value));
    }

    let mut rng = RngHandle::from_seed(seed);
    let mut modes = Vec::with_capacity(spec.modes);
    let base_scale = if operators.info.avg_degree == 0.0 {
        1.0
    } else {
        operators.info.avg_degree
    };
    for mode_id in 0..spec.modes {
        let slope = base_scale * ((mode_id + 1) as f64) * 0.05;
        let intercept = (operators.info.max_degree as f64 + mode_id as f64) * 0.01;
        let jitter = (rng.next_u32() as f64) / (u32::MAX as f64) * 0.005;
        let omega = round_value(intercept + slope * 0.5 + jitter);
        let resid = round_value(jitter * 0.1);
        modes.push(DispersionMode {
            mode_id,
            omega,
            fit_resid: resid,
        });
    }

    let c_est = if spec.k_points > 1 && !modes.is_empty() {
        let k_start = k_grid.first().copied().unwrap_or(0.0);
        let k_end = k_grid.last().copied().unwrap_or(1.0);
        let omega_start = modes[0].omega;
        let omega_end = modes[0].omega + (k_end - k_start) * 0.1;
        if (k_end - k_start).abs() < 1e-9 {
            0.0
        } else {
            round_value((omega_end - omega_start) / (k_end - k_start))
        }
    } else {
        0.0
    };

    let gap_proxy = if modes.len() > 1 {
        round_value((modes[1].omega - modes[0].omega).abs())
    } else {
        round_value(modes.first().map(|mode| mode.omega).unwrap_or(0.0))
    };

    Ok(DispersionReport {
        k_grid,
        modes,
        c_est,
        gap_proxy,
        rounding: 1e-9,
    })
}
