use asm_code::{hash, CSSCode};
use asm_core::AsmError;
use asm_graph::HypergraphImpl;
use nalgebra::{DMatrix, SymmetricEigen};
use serde::{Deserialize, Serialize};

use crate::canonical::CanonicalStructures;

/// Options controlling spectral analyses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpectralOptions {
    /// Number of Laplacian eigenvalues to retain.
    pub laplacian_topk: usize,
    /// Number of stabiliser spectrum eigenvalues to retain.
    pub stabilizer_topk: usize,
}

/// Spectral invariants captured during analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SpectralReport {
    /// Top-k Laplacian eigenvalues derived from the canonical graph.
    pub laplacian_topk: Vec<f64>,
    /// Top-k eigenvalues of the stabiliser Gram matrix.
    pub stabilizer_topk: Vec<f64>,
}

/// Computes spectral invariants for the provided state.
pub fn analyse_spectra(
    _graph: &HypergraphImpl,
    code: &CSSCode,
    canonical: &CanonicalStructures,
    opts: &SpectralOptions,
) -> Result<SpectralReport, AsmError> {
    let laplacian = laplacian_spectrum(canonical, opts.laplacian_topk)?;
    let stabilizer = stabilizer_spectrum(code, opts.stabilizer_topk)?;
    Ok(SpectralReport {
        laplacian_topk: laplacian,
        stabilizer_topk: stabilizer,
    })
}

fn laplacian_spectrum(canonical: &CanonicalStructures, topk: usize) -> Result<Vec<f64>, AsmError> {
    let node_count = canonical.graph.len();
    if node_count == 0 || topk == 0 {
        return Ok(Vec::new());
    }
    let mut adjacency = DMatrix::<f64>::zeros(node_count, node_count);
    for edge in &canonical.graph.edges {
        let mut nodes = edge.sources.clone();
        nodes.extend(edge.destinations.iter().copied());
        nodes.sort_unstable();
        nodes.dedup();
        for &i in &nodes {
            for &j in &nodes {
                if i != j {
                    adjacency[(i, j)] += 1.0;
                }
            }
        }
    }
    let mut laplacian = DMatrix::<f64>::zeros(node_count, node_count);
    for i in 0..node_count {
        let degree: f64 = adjacency.row(i).iter().sum();
        laplacian[(i, i)] = degree;
    }
    laplacian -= &adjacency;
    let eigen = SymmetricEigen::new(laplacian);
    let mut eigenvalues: Vec<f64> = eigen.eigenvalues.iter().copied().collect();
    eigenvalues.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    eigenvalues.truncate(topk.min(eigenvalues.len()));
    Ok(eigenvalues.into_iter().map(round_eigenvalue).collect())
}

fn stabilizer_spectrum(code: &CSSCode, topk: usize) -> Result<Vec<f64>, AsmError> {
    if topk == 0 {
        return Ok(Vec::new());
    }
    let (num_variables, x_checks, z_checks, ..) = hash::decompose(code);
    let mut supports: Vec<Vec<usize>> = x_checks
        .into_iter()
        .chain(z_checks)
        .map(|constraint| constraint.variables().to_vec())
        .collect();
    supports.sort();
    if supports.is_empty() {
        return Ok(Vec::new());
    }
    let rows = supports.len();
    let mut matrix = DMatrix::<f64>::zeros(rows, num_variables);
    for (row, support) in supports.iter().enumerate() {
        for &var in support {
            matrix[(row, var)] = 1.0;
        }
    }
    let gram = &matrix * matrix.transpose();
    let sym_gram = 0.5 * (&gram + gram.transpose());
    let eigen = SymmetricEigen::new(sym_gram);
    let mut eigenvalues: Vec<f64> = eigen.eigenvalues.iter().copied().collect();
    eigenvalues.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    eigenvalues.truncate(topk.min(eigenvalues.len()));
    Ok(eigenvalues.into_iter().map(round_eigenvalue).collect())
}

fn round_eigenvalue(value: f64) -> f64 {
    let scaled = (value * 1e9).round();
    scaled / 1e9
}
