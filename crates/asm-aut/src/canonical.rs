use std::collections::HashMap;

use asm_code::{hash, CSSCode, Constraint};
use asm_core::{AsmError, EdgeId, HyperedgeEndpoints, Hypergraph, NodeId};
use asm_graph::{canonical_hash, HypergraphImpl};

/// Canonicalised hypergraph representation used by downstream invariants.
#[derive(Debug, Clone)]
pub struct CanonicalGraph {
    /// Stable ordering of node identifiers.
    pub node_order: Vec<NodeId>,
    /// Canonicalised hyperedges expressed in the canonical node order.
    pub edges: Vec<CanonicalEdge>,
}

impl CanonicalGraph {
    /// Returns the number of nodes contained within the canonical graph.
    pub fn len(&self) -> usize {
        self.node_order.len()
    }

    /// Returns whether the canonical graph contains no nodes.
    pub fn is_empty(&self) -> bool {
        self.node_order.is_empty()
    }
}

/// Canonical hyperedge description with sorted endpoints.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanonicalEdge {
    /// Source nodes expressed as canonical indices.
    pub sources: Vec<usize>,
    /// Destination nodes expressed as canonical indices.
    pub destinations: Vec<usize>,
}

/// Canonicalised CSS code structure preserving sorted stabiliser supports.
#[derive(Debug, Clone)]
pub struct CanonicalCode {
    /// Total number of physical variables in the code.
    pub num_variables: usize,
    /// Normalised X-type stabiliser supports.
    pub x_checks: Vec<Vec<usize>>,
    /// Normalised Z-type stabiliser supports.
    pub z_checks: Vec<Vec<usize>>,
}

impl CanonicalCode {
    /// Returns the number of X stabilisers.
    pub fn num_x(&self) -> usize {
        self.x_checks.len()
    }

    /// Returns the number of Z stabilisers.
    pub fn num_z(&self) -> usize {
        self.z_checks.len()
    }
}

/// Aggregated canonical structures for an ASM state.
#[derive(Debug, Clone)]
pub struct CanonicalStructures {
    /// Canonicalised hypergraph view.
    pub graph: CanonicalGraph,
    /// Canonicalised CSS code view.
    pub code: CanonicalCode,
    /// Canonical hash of the hypergraph.
    pub graph_hash: String,
    /// Canonical hash of the CSS code.
    pub code_hash: String,
}

impl CanonicalStructures {
    /// Computes canonical forms for the provided state.
    pub fn build(graph: &HypergraphImpl, code: &CSSCode) -> Result<Self, AsmError> {
        let graph_hash = canonical_hash(graph)?;
        let graph = canonicalise_graph(graph)?;
        let code_hash = hash::canonical_code_hash(code);
        let code = canonicalise_code(code);
        Ok(Self {
            graph,
            code,
            graph_hash,
            code_hash,
        })
    }
}

fn canonicalise_graph(graph: &HypergraphImpl) -> Result<CanonicalGraph, AsmError> {
    let mut node_order: Vec<NodeId> = graph.nodes().collect();
    node_order.sort_by_key(|id| id.as_raw());
    let mut index_map = HashMap::new();
    for (idx, node) in node_order.iter().enumerate() {
        index_map.insert(*node, idx);
    }

    let mut edges = Vec::new();
    for edge_id in graph.edges() {
        edges.push(canonicalise_edge(graph, edge_id, &index_map)?);
    }
    edges.sort();

    Ok(CanonicalGraph { node_order, edges })
}

fn canonicalise_edge(
    graph: &HypergraphImpl,
    edge_id: EdgeId,
    index_map: &HashMap<NodeId, usize>,
) -> Result<CanonicalEdge, AsmError> {
    let HyperedgeEndpoints {
        sources,
        destinations,
    } = graph.hyperedge(edge_id)?;
    let mut canonical_sources: Vec<usize> = sources.iter().map(|node| index_map[node]).collect();
    canonical_sources.sort_unstable();
    let mut canonical_destinations: Vec<usize> =
        destinations.iter().map(|node| index_map[node]).collect();
    canonical_destinations.sort_unstable();
    Ok(CanonicalEdge {
        sources: canonical_sources,
        destinations: canonical_destinations,
    })
}

fn canonicalise_code(code: &CSSCode) -> CanonicalCode {
    let (num_variables, x_checks, z_checks, ..) = hash::decompose(code);
    CanonicalCode {
        num_variables,
        x_checks: normalise_constraints(x_checks),
        z_checks: normalise_constraints(z_checks),
    }
}

fn normalise_constraints(constraints: Vec<Constraint>) -> Vec<Vec<usize>> {
    let mut normalised: Vec<Vec<usize>> = constraints
        .into_iter()
        .map(|constraint| constraint.variables().to_vec())
        .collect();
    for support in &mut normalised {
        support.sort_unstable();
    }
    normalised.sort();
    normalised
}

impl Default for CanonicalStructures {
    fn default() -> Self {
        Self {
            graph: CanonicalGraph {
                node_order: Vec::new(),
                edges: Vec::new(),
            },
            code: CanonicalCode {
                num_variables: 0,
                x_checks: Vec::new(),
                z_checks: Vec::new(),
            },
            graph_hash: String::new(),
            code_hash: String::new(),
        }
    }
}
