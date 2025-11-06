use asm_code::css;
use asm_code::css::CSSCode;
use asm_core::AsmError;
use asm_graph::{forman_curvature_edges, forman_curvature_nodes, HypergraphImpl};
use serde::{Deserialize, Serialize};

use crate::config::ScoringWeights;

/// Breakdown of the scoring proxies used to construct the total energy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnergyBreakdown {
    /// Compressed description length proxy.
    pub cmdl: f64,
    /// Spectrum regularity proxy.
    pub spec: f64,
    /// Curvature variance proxy.
    pub curv: f64,
    /// Weighted total energy.
    pub total: f64,
}

impl EnergyBreakdown {
    /// Creates a zeroed breakdown for convenience.
    pub fn zero() -> Self {
        Self {
            cmdl: 0.0,
            spec: 0.0,
            curv: 0.0,
            total: 0.0,
        }
    }
}

/// Computes the weighted energy for the provided code/graph pair.
pub fn score(
    code: &CSSCode,
    graph: &HypergraphImpl,
    weights: &ScoringWeights,
) -> Result<EnergyBreakdown, AsmError> {
    let cmdl = cmdl_proxy(code);
    let spec = spec_proxy(code);
    let curv = curv_proxy(graph)?;

    let total = weights.cmdl * cmdl + weights.spec * spec + weights.curv * curv;

    Ok(EnergyBreakdown {
        cmdl,
        spec,
        curv,
        total,
    })
}

fn cmdl_proxy(code: &CSSCode) -> f64 {
    let (vars, x_checks, z_checks, _, _, _, _) = css::into_parts(code);
    let generator_count = (x_checks.len() + z_checks.len()) as f64;
    if generator_count == 0.0 {
        return 0.0;
    }
    let total_support: usize = x_checks
        .iter()
        .chain(z_checks.iter())
        .map(|constraint| constraint.variables().len())
        .sum();
    let avg_support = total_support as f64 / generator_count.max(1.0);

    // Lightweight Lempel-Ziv style complexity proxy: cumulative log of gaps between variables.
    let mut lz_proxy = 0.0;
    for constraint in x_checks.iter().chain(z_checks.iter()) {
        let mut prev = 0usize;
        for &var in constraint.variables() {
            let gap = var.abs_diff(prev);
            lz_proxy += (gap as f64 + 1.0).ln();
            prev = var;
        }
        // Account for trailing gap up to variable count.
        if vars > 0 {
            let tail_gap = vars - prev.min(vars - 1);
            lz_proxy += (tail_gap as f64 + 1.0).ln();
        }
    }

    generator_count + avg_support + lz_proxy / generator_count.max(1.0)
}

fn spec_proxy(code: &CSSCode) -> f64 {
    let rank_x = code.rank_x() as f64;
    let rank_z = code.rank_z() as f64;
    let nx = code.num_constraints_x() as f64;
    let nz = code.num_constraints_z() as f64;
    let rank_deficit = (nx + nz) - (rank_x + rank_z);

    // Support variance encourages uniform stabiliser weights.
    let (vars, x_checks, z_checks, _, _, _, _) = css::into_parts(code);
    let supports: Vec<f64> = x_checks
        .iter()
        .chain(z_checks.iter())
        .map(|constraint| constraint.variables().len() as f64)
        .collect();
    let support_var = variance(&supports).unwrap_or(0.0);

    // Penalise codes with extremely small supports relative to variable count.
    let sparse_penalty = if vars > 0 {
        supports
            .iter()
            .map(|&support| ((support + 1.0) / vars as f64).powf(0.75))
            .sum::<f64>()
            / supports.len().max(1) as f64
    } else {
        0.0
    };

    rank_deficit.powi(2) + support_var + sparse_penalty
}

fn curv_proxy(graph: &HypergraphImpl) -> Result<f64, AsmError> {
    let node_curvatures = forman_curvature_nodes(graph)?;
    let edge_curvatures = forman_curvature_edges(graph)?;
    let node_vals: Vec<f64> = node_curvatures
        .iter()
        .map(|(_, value)| *value as f64)
        .collect();
    let edge_vals: Vec<f64> = edge_curvatures
        .iter()
        .map(|(_, value)| *value as f64)
        .collect();
    let node_var = variance(&node_vals).unwrap_or(0.0);
    let edge_var = variance(&edge_vals).unwrap_or(0.0);
    Ok((node_var + edge_var) / 2.0)
}

fn variance(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mean = values.iter().copied().sum::<f64>() / values.len() as f64;
    let sq_mean = values.iter().copied().map(|v| v * v).sum::<f64>() / values.len() as f64;
    Some((sq_mean - mean * mean).max(0.0))
}
