use std::collections::BTreeMap;

use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::{EdgeId, Hypergraph, NodeId};

use crate::hypergraph::HypergraphImpl;

/// Computes the Forman curvature for every edge in the graph.
pub fn forman_curvature_edges(graph: &HypergraphImpl) -> Result<Vec<(EdgeId, f32)>, AsmError> {
    let mut results = Vec::new();
    for edge_id in graph.edges() {
        let endpoints = graph.hyperedge(edge_id)?;
        let mut value = 2.0 - (endpoints.sources.len() + endpoints.destinations.len()) as f32;
        for source in endpoints.sources.iter() {
            let degree = graph.out_degree(*source)? as f32;
            value += 1.0 / (1.0 + degree);
        }
        for dest in endpoints.destinations.iter() {
            let degree = graph.in_degree(*dest)? as f32;
            value += 1.0 / (1.0 + degree);
        }
        results.push((edge_id, value));
    }
    results.sort_by_key(|(edge, _)| edge.as_raw());
    Ok(results)
}

/// Computes the Forman curvature aggregated at each node.
pub fn forman_curvature_nodes(graph: &HypergraphImpl) -> Result<Vec<(NodeId, f32)>, AsmError> {
    let mut accum: BTreeMap<NodeId, (f32, usize)> = BTreeMap::new();
    for (edge_id, curvature) in forman_curvature_edges(graph)? {
        let endpoints = graph.hyperedge(edge_id)?;
        for node in endpoints
            .sources
            .iter()
            .chain(endpoints.destinations.iter())
        {
            let entry = accum.entry(*node).or_insert((0.0, 0));
            entry.0 += curvature;
            entry.1 += 1;
        }
    }
    let mut results: Vec<_> = accum
        .into_iter()
        .map(|(node, (value, count))| (node, value / count as f32))
        .collect();
    results.sort_by_key(|(node, _)| node.as_raw());
    Ok(results)
}

/// Computes a fast Ollivier-style curvature proxy using iterative averaging.
pub fn ollivier_lite_nodes(
    graph: &HypergraphImpl,
    iterations: u32,
) -> Result<Vec<(NodeId, f32)>, AsmError> {
    if iterations == 0 {
        return Err(AsmError::Graph(ErrorInfo::new(
            "zero-iterations",
            "ollivier-lite proxy requires at least one iteration",
        )));
    }
    let mut adjacency: BTreeMap<NodeId, Vec<NodeId>> = BTreeMap::new();
    let mut values: BTreeMap<NodeId, f32> = BTreeMap::new();
    for node in graph.nodes() {
        let neighbours = graph.edges_touching(node)?;
        let mut local: Vec<NodeId> = Vec::new();
        for edge in neighbours {
            let endpoints = graph.hyperedge(edge)?;
            for candidate in endpoints
                .sources
                .iter()
                .chain(endpoints.destinations.iter())
            {
                if *candidate != node {
                    local.push(*candidate);
                }
            }
        }
        local.sort_by_key(|id| id.as_raw());
        local.dedup();
        adjacency.insert(node, local.clone());
        values.insert(node, 1.0 / (1.0 + adjacency[&node].len() as f32));
    }

    for _ in 0..iterations {
        let mut next = values.clone();
        for (node, neighbours) in adjacency.iter() {
            if neighbours.is_empty() {
                continue;
            }
            let avg = neighbours
                .iter()
                .map(|n| values.get(n).copied().unwrap_or(0.0))
                .sum::<f32>()
                / neighbours.len() as f32;
            let current = values[node];
            next.insert(*node, 0.5 * current + 0.5 * avg);
        }
        values = next;
    }

    let mut results: Vec<_> = values.into_iter().collect();
    results.sort_by_key(|(node, _)| node.as_raw());
    Ok(results)
}
