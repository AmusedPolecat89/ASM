use std::collections::HashMap;

use asm_core::{AsmError, ErrorInfo};
use asm_graph::HypergraphImpl;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::canonical::{CanonicalEdge, CanonicalGraph, CanonicalStructures};

/// Automorphism statistics for a hypergraph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphAutReport {
    /// Estimated order of the automorphism group.
    pub order: u64,
    /// Whether enumeration was truncated (lower bound only).
    pub gens_truncated: bool,
    /// Histogram of orbit sizes after grouping canonical nodes.
    pub orbit_hist: Vec<u32>,
}

impl Default for GraphAutReport {
    fn default() -> Self {
        Self {
            order: 1,
            gens_truncated: false,
            orbit_hist: Vec::new(),
        }
    }
}

/// Computes graph automorphism information for the provided state.
pub fn analyse_graph(
    _graph: &HypergraphImpl,
    canonical: &CanonicalStructures,
) -> Result<GraphAutReport, AsmError> {
    let node_count = canonical.graph.len();
    if node_count == 0 {
        return Ok(GraphAutReport::default());
    }

    let exhaustive_limit = 7usize;
    if node_count > exhaustive_limit || canonical.graph.edges.len() > 8 {
        return Ok(GraphAutReport {
            order: 1,
            gens_truncated: true,
            orbit_hist: vec![1; node_count],
        });
    }

    let mut automorphisms = Vec::new();
    for perm in (0..node_count).permutations(node_count) {
        if is_automorphism(&canonical.graph, &perm) {
            automorphisms.push(perm);
        }
    }

    if automorphisms.is_empty() {
        let info = ErrorInfo::new(
            "graph-automorphism",
            "no automorphisms found including identity",
        )
        .with_context("nodes", node_count.to_string())
        .with_context("edges", canonical.graph.edges.len().to_string());
        return Err(AsmError::Graph(info));
    }

    let mut parent: Vec<usize> = (0..node_count).collect();
    for perm in &automorphisms {
        for (idx, &mapped) in perm.iter().enumerate() {
            union(&mut parent, idx, mapped);
        }
    }
    let mut orbit_sizes: HashMap<usize, u32> = HashMap::new();
    for idx in 0..node_count {
        let root = find(&mut parent, idx);
        *orbit_sizes.entry(root).or_insert(0) += 1;
    }
    let mut histogram: Vec<u32> = orbit_sizes.values().copied().collect();
    histogram.sort_unstable();

    Ok(GraphAutReport {
        order: automorphisms.len() as u64,
        gens_truncated: false,
        orbit_hist: histogram,
    })
}

fn is_automorphism(graph: &CanonicalGraph, perm: &[usize]) -> bool {
    let mut mapped_edges: Vec<CanonicalEdge> = graph
        .edges
        .iter()
        .map(|edge| apply_permutation(edge, perm))
        .collect();
    mapped_edges.sort();
    mapped_edges == graph.edges
}

fn apply_permutation(edge: &CanonicalEdge, perm: &[usize]) -> CanonicalEdge {
    let mut sources: Vec<usize> = edge.sources.iter().map(|&idx| perm[idx]).collect();
    let mut destinations: Vec<usize> = edge.destinations.iter().map(|&idx| perm[idx]).collect();
    sources.sort_unstable();
    destinations.sort_unstable();
    CanonicalEdge {
        sources,
        destinations,
    }
}

fn find(parent: &mut [usize], idx: usize) -> usize {
    if parent[idx] != idx {
        let root = find(parent, parent[idx]);
        parent[idx] = root;
    }
    parent[idx]
}

fn union(parent: &mut [usize], a: usize, b: usize) {
    let ra = find(parent, a);
    let rb = find(parent, b);
    if ra != rb {
        parent[rb] = ra;
    }
}
