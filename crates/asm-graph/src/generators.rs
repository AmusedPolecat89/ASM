use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::rng::RngHandle;
use asm_core::{Hypergraph, NodeId};
use rand::seq::SliceRandom;

use crate::flags::{HypergraphConfig, KUniformity};
use crate::hypergraph::HypergraphImpl;

/// Generates a bounded-degree hypergraph with deterministic randomness.
pub fn gen_bounded_degree(
    n_nodes: usize,
    degree_max: usize,
    k_uniform: usize,
    rng: &mut RngHandle,
) -> Result<HypergraphImpl, AsmError> {
    if n_nodes == 0 {
        return Err(AsmError::Graph(ErrorInfo::new(
            "empty-graph",
            "bounded degree generator requires at least one node",
        )));
    }
    let (sources_per_edge, destinations_per_edge) = balanced_partition(k_uniform);
    let config = HypergraphConfig {
        max_in_degree: Some(degree_max.max(1)),
        max_out_degree: Some(degree_max.max(1)),
        k_uniform: Some(KUniformity::Balanced {
            sources: sources_per_edge,
            destinations: destinations_per_edge,
        }),
        ..HypergraphConfig::default()
    };

    let mut graph = HypergraphImpl::new(config);
    let nodes: Vec<NodeId> = (0..n_nodes)
        .map(|_| graph.add_node())
        .collect::<Result<_, _>>()?;

    let max_attempts = n_nodes.saturating_mul(degree_max.max(1) * 16);
    let mut stagnation = 0usize;
    for _ in 0..max_attempts {
        let sources = sample_subset(&nodes, sources_per_edge, rng);
        let destinations = sample_subset(&nodes, destinations_per_edge, rng);
        if overlaps(&sources, &destinations) {
            continue;
        }
        match graph.add_hyperedge(&sources, &destinations) {
            Ok(_) => stagnation = 0,
            Err(err) if is_soft_error(&err) => {
                stagnation += 1;
                if stagnation > n_nodes * 4 {
                    break;
                }
            }
            Err(err) => return Err(err),
        }
    }

    Ok(graph)
}

/// Generates a quasi-regular bounded-degree hypergraph.
pub fn gen_quasi_regular(
    n_nodes: usize,
    degree_target: usize,
    k_uniform: usize,
    rng: &mut RngHandle,
) -> Result<HypergraphImpl, AsmError> {
    let mut graph = gen_bounded_degree(n_nodes, degree_target.max(1), k_uniform, rng)?;
    balance_in_degrees(&mut graph, degree_target, rng)?;
    Ok(graph)
}

fn balance_in_degrees(
    graph: &mut HypergraphImpl,
    degree_target: usize,
    rng: &mut RngHandle,
) -> Result<(), AsmError> {
    if degree_target == 0 {
        return Ok(());
    }
    let iterations = graph.edge_payloads().len() * 2;
    for _ in 0..iterations {
        let (max_node, max_in) = max_in_degree(graph)?;
        let (min_node, min_in) = min_in_degree(graph)?;
        if max_in <= min_in + 1 || max_in <= degree_target {
            break;
        }
        if max_node == min_node {
            break;
        }
        let incoming = graph.incoming_edges(max_node)?;
        let Some(&edge_id) = incoming.choose(rng) else {
            break;
        };
        let sources = graph.src_of(edge_id)?.to_vec();
        let mut destinations = graph.dst_of(edge_id)?.to_vec();
        if !destinations.contains(&max_node) || destinations.contains(&min_node) {
            continue;
        }
        destinations.retain(|node| *node != max_node);
        destinations.push(min_node);
        match graph.overwrite_edge(edge_id, &sources, &destinations) {
            Ok(_) => {}
            Err(err) if is_soft_error(&err) => {
                // attempt to revert happens in overwrite_edge; continue search
                continue;
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

fn sample_subset(nodes: &[NodeId], count: usize, rng: &mut RngHandle) -> Vec<NodeId> {
    let mut buffer: Vec<NodeId> = nodes.to_vec();
    buffer.shuffle(rng);
    buffer.truncate(count.min(buffer.len()));
    buffer.sort_by_key(|id| id.as_raw());
    buffer
}

fn overlaps(a: &[NodeId], b: &[NodeId]) -> bool {
    let mut idx_a = 0;
    let mut idx_b = 0;
    while idx_a < a.len() && idx_b < b.len() {
        match a[idx_a].as_raw().cmp(&b[idx_b].as_raw()) {
            std::cmp::Ordering::Less => idx_a += 1,
            std::cmp::Ordering::Greater => idx_b += 1,
            std::cmp::Ordering::Equal => return true,
        }
    }
    false
}

fn balanced_partition(k_uniform: usize) -> (usize, usize) {
    let total = k_uniform.max(2);
    let sources = total / 2;
    let destinations = total - sources;
    (sources.max(1), destinations.max(1))
}

fn is_soft_error(error: &AsmError) -> bool {
    matches!(
        error,
        AsmError::Graph(info)
            if matches!(
                info.code.as_str(),
                "duplicate-edge" | "would-create-cycle" | "out-degree-cap" | "in-degree-cap"
            )
    )
}

fn max_in_degree(graph: &HypergraphImpl) -> Result<(NodeId, usize), AsmError> {
    let mut best = None;
    for node in graph.nodes() {
        let degree = graph.in_degree(node)?;
        match &mut best {
            None => best = Some((node, degree)),
            Some((_, current)) if degree > *current => best = Some((node, degree)),
            _ => {}
        }
    }
    best.ok_or_else(|| AsmError::Graph(ErrorInfo::new("no-nodes", "graph has no nodes")))
}

fn min_in_degree(graph: &HypergraphImpl) -> Result<(NodeId, usize), AsmError> {
    let mut best = None;
    for node in graph.nodes() {
        let degree = graph.in_degree(node)?;
        match &mut best {
            None => best = Some((node, degree)),
            Some((_, current)) if degree < *current => best = Some((node, degree)),
            _ => {}
        }
    }
    best.ok_or_else(|| AsmError::Graph(ErrorInfo::new("no-nodes", "graph has no nodes")))
}
