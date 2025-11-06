use std::collections::BTreeMap;

use asm_core::errors::AsmError;
use serde::{Deserialize, Serialize};

use crate::invariants::GeneratorInvariants;
use crate::rep::RepMatrices;

fn round(value: f64) -> f64 {
    (value * 1e9).round() / 1e9
}

fn default_trace_tol() -> f64 {
    1e-6
}

/// Options controlling algebra factor decomposition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecompOpts {
    /// Absolute trace tolerance used to classify u(1) vs su(2)-like factors.
    #[serde(default = "default_trace_tol")]
    pub trace_tol: f64,
}

impl Default for DecompOpts {
    fn default() -> Self {
        Self {
            trace_tol: default_trace_tol(),
        }
    }
}

/// Descriptor for a single algebra factor detected during decomposition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FactorInfo {
    /// Factor classification: "u1", "su2", or "other".
    pub r#type: String,
    /// Dimensionality of the retained basis.
    pub dim: usize,
    /// Estimated rank of the factor.
    pub rank: usize,
    /// Deterministic invariants associated with the factor.
    pub invariants: BTreeMap<String, f64>,
}

/// Report describing the decomposition of the gauge algebra.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecompReport {
    /// Individual factors discovered during decomposition.
    pub factors: Vec<FactorInfo>,
    /// Residual norm capturing how well the factors explain the generators.
    pub residual_norm: f64,
}

fn classify(trace: f64, tol: f64, symmetry: f64) -> &'static str {
    if trace.abs() <= tol {
        if symmetry <= tol {
            "su2"
        } else {
            "other"
        }
    } else {
        "u1"
    }
}

/// Deterministically decomposes the representation into labelled factors.
pub fn decompose(rep: &RepMatrices, opts: &DecompOpts) -> Result<DecompReport, AsmError> {
    if rep.gens.is_empty() {
        return Ok(DecompReport {
            factors: Vec::new(),
            residual_norm: 0.0,
        });
    }
    let mut factors = Vec::with_capacity(rep.gens.len());
    let mut residual = 0.0;
    for gen in &rep.gens {
        let invariants = GeneratorInvariants::from_matrix(&gen.matrix, rep.dim);
        residual += (invariants.trace.abs() - opts.trace_tol).max(0.0);
        let mut map = BTreeMap::new();
        map.insert("trace".to_string(), invariants.trace);
        map.insert("frobenius".to_string(), invariants.frobenius);
        map.insert("symmetry".to_string(), invariants.symmetry);
        let factor_type = classify(invariants.trace, opts.trace_tol, invariants.symmetry);
        factors.push(FactorInfo {
            r#type: factor_type.to_string(),
            dim: rep.dim,
            rank: rep.dim,
            invariants: map,
        });
    }

    Ok(DecompReport {
        factors,
        residual_norm: round(residual),
    })
}
