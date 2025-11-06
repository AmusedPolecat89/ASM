use asm_core::errors::ErrorInfo;
use asm_core::{AsmError, EdgeId, Hypergraph, NodeId, RngHandle};
use asm_graph::{
    canonical_hash, rewire_resource_balanced, rewire_retarget, rewire_swap_targets, HypergraphImpl,
};
use rand::RngCore;

/// Result of a graph rewiring proposal.
#[derive(Debug, Clone)]
pub struct GraphMoveProposal {
    /// Candidate graph returned by the move.
    pub candidate: HypergraphImpl,
    /// Forward proposal probability for MH acceptance.
    pub forward_prob: f64,
    /// Reverse proposal probability for MH acceptance.
    pub reverse_prob: f64,
    /// Identifiers of edges touched by the move (if applicable).
    pub touched_edges: Vec<EdgeId>,
    /// Optional node touched by the move.
    pub touched_node: Option<NodeId>,
    /// Canonical hash of the candidate graph after the proposal.
    pub candidate_hash: String,
    /// Human readable description of the move.
    pub description: String,
}

/// Swaps the target sets of two hyperedges.
pub fn propose_swap_targets(
    graph: &HypergraphImpl,
    rng: &mut RngHandle,
) -> Result<GraphMoveProposal, AsmError> {
    let edge_ids: Vec<EdgeId> = graph.edges().collect();
    if edge_ids.len() < 2 {
        return Err(AsmError::Graph(ErrorInfo::new(
            "insufficient-edges",
            "need at least two edges for swap",
        )));
    }
    let idx_a = (rng.next_u64() as usize) % edge_ids.len();
    let mut idx_b = (rng.next_u64() as usize) % edge_ids.len();
    if idx_b == idx_a {
        idx_b = (idx_b + 1) % edge_ids.len();
    }
    let edge_a = edge_ids[idx_a];
    let edge_b = edge_ids[idx_b];

    let mut candidate = graph.clone();
    let outcome = rewire_swap_targets(&mut candidate, edge_a, edge_b)?;

    Ok(GraphMoveProposal {
        candidate_hash: outcome.hash,
        candidate,
        forward_prob: 2.0 / (edge_ids.len() * (edge_ids.len() - 1)) as f64,
        reverse_prob: 2.0 / (edge_ids.len() * (edge_ids.len() - 1)) as f64,
        touched_edges: vec![edge_a, edge_b],
        touched_node: None,
        description: format!("swap-targets:e{}-e{}", edge_a.as_raw(), edge_b.as_raw()),
    })
}

/// Retargets one destination from a hyperedge to another node.
pub fn propose_retarget(
    graph: &HypergraphImpl,
    rng: &mut RngHandle,
) -> Result<GraphMoveProposal, AsmError> {
    let edge_ids: Vec<EdgeId> = graph.edges().collect();
    if edge_ids.is_empty() {
        return Err(AsmError::Graph(ErrorInfo::new(
            "no-edges",
            "graph has no edges to retarget",
        )));
    }
    let nodes: Vec<NodeId> = graph.nodes().collect();
    if nodes.len() < 2 {
        return Err(AsmError::Graph(ErrorInfo::new(
            "insufficient-nodes",
            "need at least two nodes for retarget",
        )));
    }
    let edge_index = (rng.next_u64() as usize) % edge_ids.len();
    let edge = edge_ids[edge_index];
    let destinations = graph.dst_of(edge)?;
    if destinations.is_empty() {
        return Err(AsmError::Graph(
            ErrorInfo::new("empty-destinations", "edge has no destinations to retarget")
                .with_context("edge", edge.as_raw().to_string()),
        ));
    }
    let remove_idx = (rng.next_u64() as usize) % destinations.len();
    let removed = destinations[remove_idx];
    let mut added = nodes[(rng.next_u64() as usize) % nodes.len()];
    if added == removed {
        added = nodes[(nodes.len() + remove_idx + 1) % nodes.len()];
    }

    let mut candidate = graph.clone();
    let outcome = rewire_retarget(&mut candidate, edge, &[removed], &[added])?;

    Ok(GraphMoveProposal {
        candidate_hash: outcome.hash,
        candidate,
        forward_prob: 1.0 / (edge_ids.len() * nodes.len()) as f64,
        reverse_prob: 1.0 / (edge_ids.len() * nodes.len()) as f64,
        touched_edges: vec![edge],
        touched_node: Some(added),
        description: format!(
            "retarget:e{}:{}->{}",
            edge.as_raw(),
            removed.as_raw(),
            added.as_raw()
        ),
    })
}

/// Performs a resource balanced move around a randomly chosen node.
pub fn propose_resource_balanced(
    graph: &HypergraphImpl,
    rng: &mut RngHandle,
) -> Result<GraphMoveProposal, AsmError> {
    let nodes: Vec<NodeId> = graph.nodes().collect();
    if nodes.is_empty() {
        return Err(AsmError::Graph(ErrorInfo::new(
            "no-nodes",
            "graph has no nodes to rebalance",
        )));
    }
    let node = nodes[(rng.next_u64() as usize) % nodes.len()];
    let mut candidate = graph.clone();
    let mut move_rng = rng.clone();
    let outcome = rewire_resource_balanced(&mut candidate, node, &mut move_rng)?;
    let candidate_hash = if outcome.changed {
        outcome.hash
    } else {
        canonical_hash(&candidate)?
    };
    Ok(GraphMoveProposal {
        candidate,
        candidate_hash,
        forward_prob: 1.0 / nodes.len() as f64,
        reverse_prob: 1.0 / nodes.len() as f64,
        touched_edges: Vec::new(),
        touched_node: Some(node),
        description: format!("resource-balance:n{}", node.as_raw()),
    })
}
