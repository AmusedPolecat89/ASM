use serde::{Deserialize, Serialize};

fn round(value: f64) -> f64 {
    (value * 1e9).round() / 1e9
}

/// Deterministic invariants derived from a representation generator matrix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeneratorInvariants {
    /// Trace of the generator matrix.
    pub trace: f64,
    /// Frobenius norm of the generator matrix.
    pub frobenius: f64,
    /// Symmetry score comparing the matrix with its transpose.
    pub symmetry: f64,
}

impl GeneratorInvariants {
    /// Computes invariants for the provided dense matrix.
    pub fn from_matrix(matrix: &[f64], dim: usize) -> Self {
        let mut trace = 0.0;
        let mut frob_sq = 0.0;
        let mut asym_sq = 0.0;
        for row in 0..dim {
            for col in 0..dim {
                let idx = row * dim + col;
                let value = matrix[idx];
                if row == col {
                    trace += value;
                }
                frob_sq += value * value;
                if row < col {
                    let idx_t = col * dim + row;
                    let diff = value - matrix[idx_t];
                    asym_sq += diff * diff;
                }
            }
        }
        GeneratorInvariants {
            trace: round(trace),
            frobenius: round(frob_sq.sqrt()),
            symmetry: round(asym_sq.sqrt()),
        }
    }
}
