use asm_core::errors::{AsmError, ErrorInfo};
use serde::{Deserialize, Serialize};

use crate::rep::RepMatrices;

fn round(value: f64) -> f64 {
    (value * 1e9).round() / 1e9
}

fn gauge_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, message))
}

fn default_tolerance() -> f64 {
    1e-6
}

/// Options controlling closure checks and structure tensor extraction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClosureOpts {
    /// Maximum allowed commutator residual for the algebra to be considered closed.
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
}

impl Default for ClosureOpts {
    fn default() -> Self {
        Self {
            tolerance: default_tolerance(),
        }
    }
}

/// Structure tensor entry describing how commutators expand over the generator basis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructureTensorEntry {
    /// Left generator index i.
    pub i: usize,
    /// Right generator index j.
    pub j: usize,
    /// Basis index k contributing to the expansion.
    pub k: usize,
    /// Coefficient f^k_{ij} recorded after rounding.
    pub value: f64,
}

/// Summary of the closure check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClosureReport {
    /// Whether the algebra closed within the provided tolerance.
    pub closed: bool,
    /// Maximum deviation recorded across all commutators.
    pub max_dev: f64,
    /// Structure tensor entries describing reconstructed commutators.
    pub structure_tensors: Vec<StructureTensorEntry>,
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn matmul(a: &[f64], b: &[f64], dim: usize) -> Vec<f64> {
    let mut out = vec![0.0; dim * dim];
    for row in 0..dim {
        for col in 0..dim {
            let mut acc = 0.0;
            for k in 0..dim {
                acc += a[row * dim + k] * b[k * dim + col];
            }
            out[row * dim + col] = acc;
        }
    }
    out
}

fn subtract(a: &[f64], b: &[f64]) -> Vec<f64> {
    a.iter().zip(b).map(|(x, y)| x - y).collect()
}

fn norm(matrix: &[f64]) -> f64 {
    matrix.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// Computes commutators of the provided representation and estimates structure tensors.
pub fn check_closure(rep: &RepMatrices, opts: &ClosureOpts) -> Result<ClosureReport, AsmError> {
    if rep.dim == 0 {
        return Err(gauge_error(
            "empty-representation",
            "representation dimension must be positive",
        ));
    }
    if rep.gens.is_empty() {
        return Err(gauge_error(
            "missing-generators",
            "closure check requires at least one generator",
        ));
    }

    let dim = rep.dim;
    let mut max_dev: f64 = 0.0;
    let mut tensors = Vec::new();
    for (i, gi) in rep.gens.iter().enumerate() {
        for (j, gj) in rep.gens.iter().enumerate() {
            if j <= i {
                continue;
            }
            let gi_gj = matmul(&gi.matrix, &gj.matrix, dim);
            let gj_gi = matmul(&gj.matrix, &gi.matrix, dim);
            let comm = subtract(&gi_gj, &gj_gi);
            let comm_norm = norm(&comm);
            if comm_norm == 0.0 {
                continue;
            }
            let mut reconstruction = vec![0.0; dim * dim];
            for (k, gk) in rep.gens.iter().enumerate() {
                let denom = dot(&gk.matrix, &gk.matrix).max(1e-12);
                let coeff = dot(&comm, &gk.matrix) / denom;
                let coeff = round(coeff);
                tensors.push(StructureTensorEntry {
                    i,
                    j,
                    k,
                    value: coeff,
                });
                for (idx, value) in gk.matrix.iter().enumerate() {
                    reconstruction[idx] += coeff * value;
                }
            }
            let residual = subtract(&comm, &reconstruction);
            let residual_norm = norm(&residual);
            max_dev = max_dev.max(residual_norm);
        }
    }

    Ok(ClosureReport {
        closed: max_dev <= opts.tolerance,
        max_dev: round(max_dev),
        structure_tensors: tensors,
    })
}
