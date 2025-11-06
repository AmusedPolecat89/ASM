use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::rng::RngHandle;
use asm_core::{EdgeId, Hypergraph, NodeId};
use rand::seq::SliceRandom;

use crate::hash::canonical_hash;
use crate::hypergraph::HypergraphImpl;

/// Result of performing a rewiring move.
#[derive(Debug)]
pub struct RewireOutcome {
    /// Indicates whether the graph structure changed.
    pub changed: bool,
    /// Canonical structural hash of the graph after the move.
    pub hash: String,
}

/// Outcome of running a validator without mutating the graph.
#[derive(Debug)]
pub enum RewireDryRun {
    /// The move would succeed.
    Valid {
        /// Optional canonical hash preview produced during validation when the move changes the structure.
        hash_preview: Option<String>,
    },
    /// The move would fail with the provided error.
    Invalid(AsmError),
}

/// Swaps the destination sets of two hyperedges.
pub fn rewire_swap_targets(
    graph: &mut HypergraphImpl,
    edge_a: EdgeId,
    edge_b: EdgeId,
) -> Result<RewireOutcome, AsmError> {
    let changed = swap_targets_impl(graph, edge_a, edge_b)?;
    let hash = canonical_hash(graph)?;
    Ok(RewireOutcome { changed, hash })
}

/// Dry-run validator for [`rewire_swap_targets`].
pub fn rewire_swap_targets_dry_run(
    graph: &HypergraphImpl,
    edge_a: EdgeId,
    edge_b: EdgeId,
) -> RewireDryRun {
    let mut trial = graph.clone();
    match swap_targets_impl(&mut trial, edge_a, edge_b) {
        Ok(changed) => {
            let hash = canonical_hash(&trial).ok();
            RewireDryRun::Valid {
                hash_preview: hash.filter(|_| changed),
            }
        }
        Err(err) => RewireDryRun::Invalid(err),
    }
}

fn swap_targets_impl(
    graph: &mut HypergraphImpl,
    edge_a: EdgeId,
    edge_b: EdgeId,
) -> Result<bool, AsmError> {
    if edge_a == edge_b {
        return Ok(false);
    }
    let sources_a = graph.src_of(edge_a)?.to_vec();
    let sources_b = graph.src_of(edge_b)?.to_vec();
    let targets_a = graph.dst_of(edge_a)?.to_vec();
    let targets_b = graph.dst_of(edge_b)?.to_vec();
    if targets_a == targets_b {
        return Ok(false);
    }
    graph.overwrite_edge(edge_a, &sources_a, &targets_b)?;
    match graph.overwrite_edge(edge_b, &sources_b, &targets_a) {
        Ok(_) => Ok(true),
        Err(err) => {
            let _ = graph.overwrite_edge(edge_a, &sources_a, &targets_a);
            Err(err)
        }
    }
}

/// Retargets a subset of destinations for a given edge.
pub fn rewire_retarget(
    graph: &mut HypergraphImpl,
    edge: EdgeId,
    removed: &[NodeId],
    added: &[NodeId],
) -> Result<RewireOutcome, AsmError> {
    let changed = retarget_impl(graph, edge, removed, added)?;
    let hash = canonical_hash(graph)?;
    Ok(RewireOutcome { changed, hash })
}

/// Validator for [`rewire_retarget`].
pub fn rewire_retarget_dry_run(
    graph: &HypergraphImpl,
    edge: EdgeId,
    removed: &[NodeId],
    added: &[NodeId],
) -> RewireDryRun {
    let mut trial = graph.clone();
    match retarget_impl(&mut trial, edge, removed, added) {
        Ok(changed) => {
            let hash = canonical_hash(&trial).ok();
            RewireDryRun::Valid {
                hash_preview: hash.filter(|_| changed),
            }
        }
        Err(err) => RewireDryRun::Invalid(err),
    }
}

fn retarget_impl(
    graph: &mut HypergraphImpl,
    edge: EdgeId,
    removed: &[NodeId],
    added: &[NodeId],
) -> Result<bool, AsmError> {
    let sources = graph.src_of(edge)?.to_vec();
    let mut destinations = graph.dst_of(edge)?.to_vec();
    if removed.is_empty() && added.is_empty() {
        return Ok(false);
    }
    for node in removed {
        if !destinations.contains(node) {
            return Err(AsmError::Graph(
                ErrorInfo::new("missing-destination", "destination is not part of the edge")
                    .with_context("edge", edge.as_raw().to_string())
                    .with_context("node", node.as_raw().to_string()),
            ));
        }
    }
    for node in added {
        graph.node(*node)?;
    }
    let original = destinations.clone();
    destinations.retain(|node| !removed.contains(node));
    destinations.extend_from_slice(added);
    destinations.sort_by_key(|id| id.as_raw());
    destinations.dedup();
    if destinations.is_empty() {
        return Err(AsmError::Graph(ErrorInfo::new(
            "empty-destinations",
            "retargeting would remove all destinations",
        )));
    }
    if destinations == original {
        return Ok(false);
    }
    graph.overwrite_edge(edge, &sources, &destinations)?;
    Ok(true)
}

/// Performs a degree-aware local rewiring to balance inbound load.
pub fn rewire_resource_balanced(
    graph: &mut HypergraphImpl,
    node: NodeId,
    rng: &mut RngHandle,
) -> Result<RewireOutcome, AsmError> {
    let changed = resource_balanced_impl(graph, node, rng)?;
    let hash = canonical_hash(graph)?;
    Ok(RewireOutcome { changed, hash })
}

/// Validator for [`rewire_resource_balanced`].
pub fn rewire_resource_balanced_dry_run(
    graph: &HypergraphImpl,
    node: NodeId,
    rng: &mut RngHandle,
) -> RewireDryRun {
    let mut trial = graph.clone();
    let mut rng_clone = rng.clone();
    match resource_balanced_impl(&mut trial, node, &mut rng_clone) {
        Ok(changed) => {
            let hash = canonical_hash(&trial).ok();
            RewireDryRun::Valid {
                hash_preview: hash.filter(|_| changed),
            }
        }
        Err(err) => RewireDryRun::Invalid(err),
    }
}

fn resource_balanced_impl(
    graph: &mut HypergraphImpl,
    node: NodeId,
    rng: &mut RngHandle,
) -> Result<bool, AsmError> {
    graph.node(node)?;
    let outgoing = graph.outgoing_edges(node)?;
    if outgoing.is_empty() {
        return Ok(false);
    }
    let mut in_degrees = Vec::new();
    for candidate in graph.nodes() {
        in_degrees.push((candidate, graph.in_degree(candidate)?));
    }
    in_degrees.sort_by(|(node_a, deg_a), (node_b, deg_b)| {
        deg_a
            .cmp(deg_b)
            .then_with(|| node_a.as_raw().cmp(&node_b.as_raw()))
    });
    let Some(&(target_node, _)) = in_degrees.first() else {
        return Ok(false);
    };
    let Some(&edge_id) = outgoing.choose(rng) else {
        return Ok(false);
    };
    let mut destinations = graph.dst_of(edge_id)?.to_vec();
    if destinations.contains(&target_node) {
        return Ok(false);
    }
    let mut worst = destinations[0];
    let mut worst_degree = 0usize;
    for candidate in &destinations {
        let degree = in_degrees
            .iter()
            .find(|(node_id, _)| node_id == candidate)
            .map(|(_, degree)| *degree)
            .unwrap_or(0);
        if degree > worst_degree {
            worst_degree = degree;
            worst = *candidate;
        }
    }
    if worst == target_node {
        return Ok(false);
    }
    destinations.retain(|node_id| *node_id != worst);
    destinations.push(target_node);
    let sources = graph.src_of(edge_id)?.to_vec();
    graph.overwrite_edge(edge_id, &sources, &destinations)?;
    Ok(true)
}
