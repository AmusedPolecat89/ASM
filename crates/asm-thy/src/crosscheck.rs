use asm_core::errors::{AsmError, ErrorInfo};
use serde::{Deserialize, Serialize};

use crate::policies::Policy;
use crate::symbolic::{NumMat, SymExpr};

fn crosscheck_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, message.into()))
}

/// Result describing a single symbolic ↔ numeric comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrosscheckResult {
    /// Whether the comparison satisfied the configured policy.
    pub pass: bool,
    /// Rounded metric describing the deviation between matrices.
    pub metric: f64,
    /// Threshold used during the decision.
    pub threshold: f64,
}

/// Cross-checks a numeric matrix against a symbolic expression with the provided policy.
pub fn crosscheck_numeric(
    symbolic: &SymExpr,
    numeric: &NumMat,
    policy: &Policy,
) -> Result<CrosscheckResult, AsmError> {
    if symbolic.dim == 0 || numeric.dim == 0 {
        return Err(crosscheck_error(
            "empty-matrix",
            "symbolic and numeric matrices must be non-empty",
        ));
    }
    if symbolic.dim != numeric.dim {
        return Err(crosscheck_error(
            "dimension-mismatch",
            format!(
                "symbolic dim {} != numeric dim {}",
                symbolic.dim, numeric.dim
            ),
        ));
    }
    if symbolic.entries.len() != numeric.entries.len() {
        return Err(crosscheck_error(
            "entry-mismatch",
            "symbolic and numeric matrices must provide identical entry counts",
        ));
    }

    let mut diff_norm = 0.0;
    for (sym, num) in symbolic.entries.iter().zip(numeric.entries.iter()) {
        let delta = sym - num;
        diff_norm += delta * delta;
    }
    let metric = policy.round(diff_norm.sqrt());
    Ok(CrosscheckResult {
        pass: metric <= policy.abs_tol,
        metric,
        threshold: policy.abs_tol,
    })
}
