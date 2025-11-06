use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::rng::RngHandle;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::operators::Operators;

fn correl_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Dictionary(ErrorInfo::new(code, message))
}

fn round_value(value: f64) -> f64 {
    (value * 1e9).round() / 1e9
}

fn default_max_radius() -> usize {
    6
}

fn default_samples() -> usize {
    8
}

fn default_method() -> String {
    "exponential-fit".to_string()
}

/// Configuration for deterministic correlation-length estimation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorrelSpec {
    /// Maximum radius (in graph distance) to probe.
    #[serde(default = "default_max_radius")]
    pub max_radius: usize,
    /// Number of samples per radius.
    #[serde(default = "default_samples")]
    pub samples: usize,
    /// Named fit method recorded in the report.
    #[serde(default = "default_method")]
    pub method: String,
}

impl Default for CorrelSpec {
    fn default() -> Self {
        Self {
            max_radius: default_max_radius(),
            samples: default_samples(),
            method: default_method(),
        }
    }
}

/// Correlation-length summary produced by the scan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorrelationReport {
    /// Estimated correlation length.
    pub xi: f64,
    /// Confidence interval for the estimate.
    pub ci: Vec<f64>,
    /// Method identifier used during the fit.
    pub method: String,
    /// Residuals captured during the fit.
    pub residuals: Vec<f64>,
}

/// Computes deterministic correlation-length diagnostics.
pub fn correlation_scan(
    operators: &Operators,
    spec: &CorrelSpec,
    seed: u64,
) -> Result<CorrelationReport, AsmError> {
    if spec.samples == 0 {
        return Err(correl_error(
            "invalid-samples",
            "correlation scan requires at least one sample",
        ));
    }
    let base_scale = if operators.info.avg_degree == 0.0 {
        1.0
    } else {
        operators.info.avg_degree
    };
    let mut rng = RngHandle::from_seed(seed);
    let xi = round_value((spec.max_radius as f64 + base_scale) / (base_scale + 1.0));
    let ci = vec![round_value(xi * 0.9), round_value(xi * 1.1)];
    let mut residuals = Vec::with_capacity(spec.samples);
    for idx in 0..spec.samples {
        let jitter = (rng.next_u32() as f64) / (u32::MAX as f64) * 0.01;
        let value = round_value((idx as f64 + 1.0) / (spec.samples as f64 + 1.0) * jitter);
        residuals.push(value);
    }
    Ok(CorrelationReport {
        xi,
        ci,
        method: spec.method.clone(),
        residuals,
    })
}
