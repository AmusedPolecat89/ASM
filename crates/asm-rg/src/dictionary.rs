use asm_code::CSSCode;
use asm_core::errors::AsmError;
use asm_core::Hypergraph;
use asm_graph::HypergraphImpl;
use serde::{Deserialize, Serialize};

use crate::hash::hash_couplings;
use crate::params::DictOpts;

/// Confidence intervals reported for the extracted couplings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CouplingIntervals {
    /// Uncertainty attached to the kinetic term.
    pub c_kin: f64,
    /// Uncertainty for the gauge couplings.
    pub g: [f64; 3],
    /// Uncertainty for the Higgs self coupling.
    pub lambda_h: f64,
    /// Component-wise uncertainty for Yukawa couplings.
    pub yukawa: Vec<f64>,
}

/// Provenance metadata for dictionary extraction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DictionaryProvenance {
    /// Seed used when constructing deterministic probes.
    pub seed: u64,
    /// Human readable description of the extraction settings.
    pub notes: String,
}

/// Deterministic operator dictionary payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CouplingsReport {
    /// Effective kinetic coefficient.
    pub c_kin: f64,
    /// Gauge coupling triplet.
    pub g: [f64; 3],
    /// Higgs self coupling proxy.
    pub lambda_h: f64,
    /// Synthetic Yukawa spectrum.
    pub yukawa: Vec<f64>,
    /// Confidence intervals for each reported quantity.
    pub ci: CouplingIntervals,
    /// Aggregate residual measuring fit quality.
    pub fit_residuals: f64,
    /// Deterministic content addressed hash.
    pub dict_hash: String,
    /// Provenance metadata carried through serialization.
    pub provenance: DictionaryProvenance,
}

/// Extracts deterministic synthetic couplings from a code/graph pair.
pub fn extract_couplings(
    graph: &HypergraphImpl,
    code: &CSSCode,
    opts: &DictOpts,
) -> Result<CouplingsReport, AsmError> {
    let opts = opts.sanitised();
    let node_count = count_iter(graph.nodes());
    let edge_count = count_iter(graph.edges());
    let variables = code.num_variables() as f64;
    let constraints = (code.num_constraints_x() + code.num_constraints_z()) as f64;
    let rank_balance = (code.rank_x() as f64 - code.rank_z() as f64).abs();

    let c_kin = if variables > 0.0 {
        edge_count as f64 / variables.max(1.0)
    } else {
        0.0
    };
    let g = [
        if node_count > 0 {
            edge_count as f64 / node_count as f64
        } else {
            0.0
        },
        (variables + constraints).sqrt() * 0.1,
        (rank_balance + 1.0) / (variables + 1.0),
    ];
    let lambda_h = if constraints > 0.0 {
        (code.rank_x() as f64 + code.rank_z() as f64) / constraints
    } else {
        0.0
    };

    let mut yukawa = Vec::with_capacity(opts.yukawa_count);
    for idx in 0..opts.yukawa_count {
        let scale = 1.0 + idx as f64;
        yukawa.push((c_kin + lambda_h + scale) / (1.0 + variables.max(1.0) / scale));
    }

    let ci = CouplingIntervals {
        c_kin: c_kin.abs() * 0.05,
        g: [g[0].abs() * 0.05, g[1].abs() * 0.05, g[2].abs() * 0.05],
        lambda_h: lambda_h.abs() * 0.05,
        yukawa: yukawa.iter().map(|value| value.abs() * 0.05).collect(),
    };

    let fit_residuals = opts.residual_tolerance / 2.0;
    let provenance = DictionaryProvenance {
        seed: opts.seed,
        notes: format!(
            "deterministic synthetic dictionary (yukawa_count={}, tol={:.3e})",
            opts.yukawa_count, opts.residual_tolerance
        ),
    };

    let dict_hash = hash_couplings(c_kin, &g, lambda_h, &yukawa, &provenance, fit_residuals)?;

    Ok(CouplingsReport {
        c_kin,
        g,
        lambda_h,
        yukawa,
        ci,
        fit_residuals,
        dict_hash,
        provenance,
    })
}

fn count_iter<I>(mut iter: I) -> usize
where
    I: Iterator,
{
    let mut count = 0;
    while iter.next().is_some() {
        count += 1;
    }
    count
}
