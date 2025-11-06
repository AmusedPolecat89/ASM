use sha2::{Digest, Sha256};

use asm_core::{AsmError, ErrorInfo};
use serde::{Deserialize, Serialize};

use crate::canonical::CanonicalStructures;
use crate::code_aut::CodeAutReport;
use crate::graph_aut::GraphAutReport;
use crate::invariants::{combine_for_hash, ProvenanceInfo};
use crate::logical::LogicalReport;
use crate::spectral::SpectralReport;

/// Canonical hashes embedded within analysis reports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HashReport {
    /// Content addressed hash of the entire analysis report.
    pub analysis_hash: String,
    /// Canonical structural hash of the hypergraph.
    pub graph_hash: String,
    /// Canonical structural hash of the CSS code.
    pub code_hash: String,
}

/// Computes deterministic hashes for an analysis report.
pub fn compute_hashes(
    canonical: &CanonicalStructures,
    graph: &GraphAutReport,
    code: &CodeAutReport,
    logical: &LogicalReport,
    spectral: &SpectralReport,
    provenance: &ProvenanceInfo,
) -> Result<HashReport, AsmError> {
    let payload = combine_for_hash(graph, code, logical, spectral, provenance)?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.graph_hash.as_bytes());
    hasher.update(canonical.code_hash.as_bytes());
    let payload_bytes = serde_json::to_vec(&payload)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("analysis-hash", err.to_string())))?;
    hasher.update(&payload_bytes);
    let digest = hasher.finalize();
    Ok(HashReport {
        analysis_hash: hex::encode(digest),
        graph_hash: canonical.graph_hash.clone(),
        code_hash: canonical.code_hash.clone(),
    })
}
