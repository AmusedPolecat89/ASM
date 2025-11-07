use serde::{Deserialize, Serialize};

/// Simple symbolic matrix expression stored in row-major order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SymExpr {
    /// Matrix dimension.
    pub dim: usize,
    /// Row-major entries representing the symbolic matrix.
    pub entries: Vec<f64>,
}

impl SymExpr {
    /// Constructs a diagonal matrix expression with the provided diagonal entries.
    pub fn from_diagonal(diagonal: &[f64]) -> Self {
        let dim = diagonal.len();
        let mut entries = vec![0.0; dim * dim];
        for (idx, value) in diagonal.iter().enumerate() {
            entries[idx * dim + idx] = *value;
        }
        Self { dim, entries }
    }

    /// Returns the trace of the symbolic matrix.
    pub fn trace(&self) -> f64 {
        (0..self.dim)
            .map(|idx| self.entries[idx * self.dim + idx])
            .sum()
    }
}

/// Numeric matrix in row-major order used for cross-checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NumMat {
    /// Matrix dimension.
    pub dim: usize,
    /// Row-major numeric entries.
    pub entries: Vec<f64>,
}

impl NumMat {
    /// Constructs a numeric matrix from row-major entries.
    pub fn new(dim: usize, entries: Vec<f64>) -> Self {
        Self { dim, entries }
    }

    /// Computes the Frobenius norm of the matrix.
    pub fn frobenius_norm(&self) -> f64 {
        self.entries
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt()
    }
}

/// Computes the commutator [A, B] = AB - BA for the provided matrices.
pub fn commutator(a: &SymExpr, b: &SymExpr) -> SymExpr {
    let dim = a.dim.min(b.dim);
    if dim == 0 {
        return SymExpr {
            dim: 0,
            entries: Vec::new(),
        };
    }
    let stride_a = a.dim.max(1);
    let stride_b = b.dim.max(1);
    let mut ab = vec![0.0; dim * dim];
    let mut ba = vec![0.0; dim * dim];
    for row in 0..dim {
        for col in 0..dim {
            let mut sum_ab = 0.0;
            let mut sum_ba = 0.0;
            for k in 0..dim {
                sum_ab += a.entries[row * stride_a + k] * b.entries[k * stride_b + col];
                sum_ba += b.entries[row * stride_b + k] * a.entries[k * stride_a + col];
            }
            ab[row * dim + col] = sum_ab;
            ba[row * dim + col] = sum_ba;
        }
    }
    let entries = ab.into_iter().zip(ba).map(|(lhs, rhs)| lhs - rhs).collect();
    SymExpr { dim, entries }
}

/// Returns the Hermitian adjoint of the provided matrix (which is the transpose here).
pub fn adjoint(expr: &SymExpr) -> SymExpr {
    let mut entries = vec![0.0; expr.entries.len()];
    for row in 0..expr.dim {
        for col in 0..expr.dim {
            entries[col * expr.dim + row] = expr.entries[row * expr.dim + col];
        }
    }
    SymExpr {
        dim: expr.dim,
        entries,
    }
}
