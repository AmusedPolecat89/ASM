use std::collections::BTreeMap;

use asm_core::{AsmError, ErrorInfo};
use serde::{Deserialize, Serialize};

use crate::code_aut::CodeAutReport;
use crate::graph_aut::GraphAutReport;
use crate::logical::LogicalReport;
use crate::spectral::SpectralReport;
use crate::{AnalysisReport, SimilarityScore};

/// Provenance metadata preserved in analysis reports.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceInfo {
    /// Master seed associated with the run.
    pub seed: Option<u64>,
    /// Identifier for the run or vacuum directory.
    pub run_id: Option<String>,
    /// Checkpoint identifier when analysing time series data.
    pub checkpoint_id: Option<String>,
    /// Source commit describing the simulation inputs.
    pub commit: Option<String>,
}

/// Computes canonical hashes for an analysis report.
pub fn combine_for_hash(
    graph: &GraphAutReport,
    code: &CodeAutReport,
    logical: &LogicalReport,
    spectral: &SpectralReport,
    provenance: &ProvenanceInfo,
) -> Result<serde_json::Value, AsmError> {
    serde_json::to_value(serde_json::json!({
        "graph_aut": graph,
        "code_aut": code,
        "logical": logical,
        "spectral": spectral,
        "provenance": provenance,
    }))
    .map_err(|err| AsmError::Serde(ErrorInfo::new("analysis-hash", err.to_string())))
}

/// Computes a deterministic similarity score between two reports.
pub fn compare_reports(a: &AnalysisReport, b: &AnalysisReport) -> SimilarityScore {
    let mut components = BTreeMap::new();

    let graph_delta = combine_graph_delta(&a.graph_aut, &b.graph_aut);
    components.insert("graph".to_string(), graph_delta);

    let code_delta = combine_code_delta(&a.code_aut, &b.code_aut);
    components.insert("code".to_string(), code_delta);

    let logical_delta = combine_logical_delta(&a.logical, &b.logical);
    components.insert("logical".to_string(), logical_delta);

    let spectral_delta = combine_spectral_delta(&a.spectral, &b.spectral);
    components.insert("spectral".to_string(), spectral_delta);

    let distance = if components.is_empty() {
        0.0
    } else {
        components.values().sum::<f64>() / components.len() as f64
    };

    SimilarityScore {
        distance,
        components,
    }
}

fn combine_graph_delta(a: &GraphAutReport, b: &GraphAutReport) -> f64 {
    let order_delta = log_ratio_delta(a.order, b.order);
    let orbit_delta = histogram_delta(&a.orbit_hist, &b.orbit_hist);
    (order_delta + orbit_delta) / 2.0
}

fn combine_code_delta(a: &CodeAutReport, b: &CodeAutReport) -> f64 {
    let order_delta = log_ratio_delta(a.order, b.order);
    let css_delta = if a.css_preserving == b.css_preserving {
        0.0
    } else {
        1.0
    };
    (order_delta + css_delta) / 2.0
}

fn combine_logical_delta(a: &LogicalReport, b: &LogicalReport) -> f64 {
    let rx = normalised_difference(a.rank_x as f64, b.rank_x as f64);
    let rz = normalised_difference(a.rank_z as f64, b.rank_z as f64);
    let sig = if a.comm_signature == b.comm_signature {
        0.0
    } else {
        1.0
    };
    (rx + rz + sig) / 3.0
}

fn combine_spectral_delta(a: &SpectralReport, b: &SpectralReport) -> f64 {
    let laplacian = vector_delta(&a.laplacian_topk, &b.laplacian_topk);
    let stabilizer = vector_delta(&a.stabilizer_topk, &b.stabilizer_topk);
    (laplacian + stabilizer) / 2.0
}

fn log_ratio_delta(a: u64, b: u64) -> f64 {
    let ax = (a as f64 + 1.0).ln();
    let bx = (b as f64 + 1.0).ln();
    normalised_difference(ax, bx)
}

fn normalised_difference(a: f64, b: f64) -> f64 {
    if a == b {
        return 0.0;
    }
    let denom = a.abs() + b.abs();
    if denom == 0.0 {
        0.0
    } else {
        ((a - b).abs() / denom).min(1.0)
    }
}

fn histogram_delta(a: &[u32], b: &[u32]) -> f64 {
    let sum_a: f64 = a.iter().map(|&x| x as f64).sum();
    let sum_b: f64 = b.iter().map(|&x| x as f64).sum();
    if sum_a == 0.0 && sum_b == 0.0 {
        return 0.0;
    }
    let max_len = a.len().max(b.len());
    let mut delta = 0.0;
    for idx in 0..max_len {
        let va = if idx < a.len() {
            a[idx] as f64 / sum_a.max(1.0)
        } else {
            0.0
        };
        let vb = if idx < b.len() {
            b[idx] as f64 / sum_b.max(1.0)
        } else {
            0.0
        };
        delta += (va - vb).abs();
    }
    (delta * 0.5).min(1.0)
}

fn vector_delta(a: &[f64], b: &[f64]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let max_len = a.len().max(b.len());
    let mut sum_sq = 0.0;
    for idx in 0..max_len {
        let va = if idx < a.len() { a[idx] } else { 0.0 };
        let vb = if idx < b.len() { b[idx] } else { 0.0 };
        sum_sq += (va - vb).powi(2);
    }
    let dist = sum_sq.sqrt();
    dist / (dist + 1.0)
}
