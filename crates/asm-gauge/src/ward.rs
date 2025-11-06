use asm_core::errors::{AsmError, ErrorInfo};
use serde::{Deserialize, Serialize};

use crate::rep::RepMatrices;

fn round(value: f64) -> f64 {
    (value * 1e9).round() / 1e9
}

fn default_relative_tol() -> f64 {
    1e-5
}

fn gauge_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, message))
}

/// Options controlling Ward-style commutator checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WardOpts {
    /// Maximum allowed relative commutator norm.
    #[serde(default = "default_relative_tol")]
    pub relative_tol: f64,
}

impl Default for WardOpts {
    fn default() -> Self {
        Self {
            relative_tol: default_relative_tol(),
        }
    }
}

/// Threshold metadata recorded in [`WardReport`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WardThresholds {
    /// Relative tolerance for the commutator norm.
    pub rel_tol: f64,
}

/// Result of a Ward-style commutator check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WardReport {
    /// Maximum commutator norm recorded across generators.
    pub max_comm_norm: f64,
    /// Whether the residual satisfied the configured tolerance.
    pub pass: bool,
    /// Threshold metadata recorded for provenance.
    pub thresholds: WardThresholds,
}

fn operator_diagonal(info: &asm_spec::operators::OperatorsInfo, dim: usize) -> Vec<f64> {
    if dim == 0 {
        return Vec::new();
    }
    let mut diag = Vec::with_capacity(dim);
    let base = if info.avg_degree == 0.0 {
        1.0
    } else {
        info.avg_degree
    };
    for idx in 0..dim {
        let scale = (idx as f64 + 1.0) / dim as f64;
        let value = base * scale + info.max_degree as f64 * 0.01;
        diag.push(round(value));
    }
    diag
}

fn commutator_norm(matrix: &[f64], diag: &[f64], dim: usize) -> f64 {
    let mut acc = 0.0;
    for row in 0..dim {
        for col in 0..dim {
            let idx = row * dim + col;
            let value = matrix[idx];
            let term = value * (diag[col] - diag[row]);
            acc += term * term;
        }
    }
    acc.sqrt()
}

/// Evaluates Ward-style commutator residuals between the representation and the effective operator.
pub fn ward_check(
    rep: &RepMatrices,
    ops: &asm_spec::operators::OperatorsInfo,
    ward_opts: &WardOpts,
) -> Result<WardReport, AsmError> {
    if rep.dim == 0 {
        return Err(gauge_error(
            "empty-representation",
            "representation dimension must be positive",
        ));
    }
    if rep.gens.is_empty() {
        return Err(gauge_error(
            "missing-generators",
            "ward check requires at least one generator",
        ));
    }

    let dim = rep.dim;
    let diag = operator_diagonal(ops, dim);
    let operator_norm = diag.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-12);
    let mut max_comm: f64 = 0.0;
    for gen in &rep.gens {
        let norm = commutator_norm(&gen.matrix, &diag, dim);
        max_comm = max_comm.max(norm);
    }
    let rel = max_comm / operator_norm;
    Ok(WardReport {
        max_comm_norm: round(max_comm),
        pass: rel <= ward_opts.relative_tol,
        thresholds: WardThresholds {
            rel_tol: ward_opts.relative_tol,
        },
    })
}
