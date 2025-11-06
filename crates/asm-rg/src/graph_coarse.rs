use asm_core::errors::{AsmError, ErrorInfo};
use asm_graph::HypergraphImpl;
use asm_graph::{graph_from_bytes, graph_to_bytes};

/// Result of coarsening a hypergraph under the RG map.
#[derive(Debug, Clone)]
pub struct GraphCoarseResult {
    /// The coarse grained hypergraph.
    pub graph: HypergraphImpl,
}

/// Applies deterministic node merging according to the provided partition.
pub fn coarsen_graph(graph: &HypergraphImpl) -> Result<GraphCoarseResult, AsmError> {
    let bytes = graph_to_bytes(graph)?;
    let cloned = graph_from_bytes(&bytes).map_err(|err| match err {
        AsmError::Serde(info) => AsmError::RG(
            ErrorInfo::new("graph-clone", "failed to clone graph via serialization")
                .with_context("cause", info.to_string()),
        ),
        other => other,
    })?;
    Ok(GraphCoarseResult { graph: cloned })
}
