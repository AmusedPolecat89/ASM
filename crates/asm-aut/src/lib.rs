#![deny(missing_docs)]
#![doc = "Automorphism and invariant analysis utilities for ASM states. See \
docs/phase5-aut-api.md for the detailed Phase 5 contract."]

/// Deterministic canonicalisation helpers for graphs and codes.
pub mod canonical;
/// Deterministic clustering and feature extraction utilities.
pub mod cluster;
/// CSS automorphism enumeration utilities.
pub mod code_aut;
/// Hypergraph automorphism enumeration utilities.
pub mod graph_aut;
/// Canonical hashing helpers combining invariants.
pub mod hash;
/// Aggregation of invariants and provenance metadata.
pub mod invariants;
/// Logical commutation profiling utilities.
pub mod logical;
/// JSON serialisation helpers for analysis and clustering results.
#[path = "serde.rs"]
pub mod serde_io;
/// Spectral invariant computations for graphs and codes.
pub mod spectral;

use std::collections::BTreeMap;

use asm_code::CSSCode;
use asm_core::AsmError;
use asm_graph::HypergraphImpl;
use canonical::CanonicalStructures;
use cluster::{cluster_reports, ClusterInfo};
use code_aut::CodeAutReport;
use graph_aut::GraphAutReport;
use hash::{compute_hashes, HashReport};
use invariants::ProvenanceInfo;
use logical::LogicalReport;
use serde::{Deserialize, Serialize};
use spectral::{SpectralOptions, SpectralReport};

/// Options controlling symmetry scans.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanOpts {
    /// Number of Laplacian eigenvalues to retain.
    pub laplacian_topk: usize,
    /// Number of stabilizer spectrum eigenvalues to retain.
    pub stabilizer_topk: usize,
    /// Optional provenance metadata to include in the report.
    #[serde(default)]
    pub provenance: Option<ProvenanceInfo>,
}

impl Default for ScanOpts {
    fn default() -> Self {
        Self {
            laplacian_topk: 16,
            stabilizer_topk: 16,
            provenance: None,
        }
    }
}

/// Options controlling deterministic clustering of analysis reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterOpts {
    /// Number of clusters to produce.
    pub k: usize,
    /// Maximum number of refinement iterations.
    pub max_iterations: usize,
    /// Deterministic tie-breaking seed used for centroid selection.
    pub seed: u64,
}

impl Default for ClusterOpts {
    fn default() -> Self {
        Self {
            k: 2,
            max_iterations: 16,
            seed: 0xA5A5_2024,
        }
    }
}

/// Summary of analysis invariants for a single state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisReport {
    /// Graph automorphism information.
    pub graph_aut: GraphAutReport,
    /// CSS code automorphism information.
    pub code_aut: CodeAutReport,
    /// Logical commutation profile extracted from the code.
    pub logical: LogicalReport,
    /// Spectral invariants for the state.
    pub spectral: SpectralReport,
    /// Canonical content addressed hashes for the analysis.
    pub hashes: HashReport,
    /// Provenance metadata describing the origin of the analysed state.
    pub provenance: ProvenanceInfo,
}

/// Pairwise similarity metric between two analysis reports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimilarityScore {
    /// Scalar distance in the range [0, 1].
    pub distance: f64,
    /// Per-component deltas normalised into [0, 1].
    pub components: BTreeMap<String, f64>,
}

/// Deterministic clustering summary for a collection of reports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterSummary {
    /// Identifier and statistics for each cluster discovered during analysis.
    pub clusters: Vec<ClusterInfo>,
}

/// Analyses a code/graph pair and produces the corresponding invariant report.
pub fn analyze_state(
    graph: &HypergraphImpl,
    code: &CSSCode,
    opts: &ScanOpts,
) -> Result<AnalysisReport, AsmError> {
    let canonical = CanonicalStructures::build(graph, code)?;
    let graph_aut = graph_aut::analyse_graph(graph, &canonical)?;
    let code_aut = code_aut::analyse_code(code)?;
    let logical = logical::analyse_logical(code)?;
    let spectral_opts = SpectralOptions {
        laplacian_topk: opts.laplacian_topk,
        stabilizer_topk: opts.stabilizer_topk,
    };
    let spectral = spectral::analyse_spectra(graph, code, &canonical, &spectral_opts)?;
    let provenance = opts.provenance.clone().unwrap_or_default();
    let hashes = compute_hashes(
        &canonical,
        &graph_aut,
        &code_aut,
        &logical,
        &spectral,
        &provenance,
    )?;

    Ok(AnalysisReport {
        graph_aut,
        code_aut,
        logical,
        spectral,
        hashes,
        provenance,
    })
}

/// Compares two analysis reports using a deterministic similarity metric.
pub fn compare(a: &AnalysisReport, b: &AnalysisReport) -> SimilarityScore {
    invariants::compare_reports(a, b)
}

/// Clusters a collection of analysis reports deterministically.
pub fn cluster(reports: &[AnalysisReport], opts: &ClusterOpts) -> ClusterSummary {
    ClusterSummary {
        clusters: cluster_reports(reports, opts),
    }
}
