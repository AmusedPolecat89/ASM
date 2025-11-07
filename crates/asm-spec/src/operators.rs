use std::collections::BTreeMap;

use asm_code::CSSCode;
use asm_core::{
    errors::{AsmError, ErrorInfo},
    HyperedgeEndpoints, Hypergraph,
};
use asm_graph::HypergraphImpl;
use serde::{Deserialize, Serialize};

use crate::hash::stable_hash_string;

fn graph_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Graph(ErrorInfo::new(code, message))
}

fn round_weight(value: f64) -> f64 {
    (value * 1e9).round() / 1e9
}

/// Determines how effective operators are assembled from the graph.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OpsVariant {
    /// Canonical Laplacian-style construction.
    Default,
    /// Alternate weighting emphasising destination degrees.
    Alt,
}

#[allow(clippy::derivable_impls)]
impl Default for OpsVariant {
    fn default() -> Self {
        OpsVariant::Default
    }
}

/// Options controlling operator construction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpOpts {
    /// Variant to use when building the operator matrices.
    #[serde(default)]
    pub variant: OpsVariant,
}

impl Default for OpOpts {
    fn default() -> Self {
        Self {
            variant: OpsVariant::Default,
        }
    }
}

/// Sparse operator entry represented in coordinate form.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorEntry {
    /// Row index of the operator entry.
    pub row: usize,
    /// Column index of the operator entry.
    pub col: usize,
    /// Deterministic weight assigned to the entry.
    pub weight: f64,
}

/// Node-level summaries captured while constructing operators.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeSummary {
    /// Canonical node identifier.
    pub node: u64,
    /// Total degree recorded for the node.
    pub degree: usize,
}

/// Metadata describing the constructed operators.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorsInfo {
    /// Number of nodes in the originating graph.
    pub num_nodes: usize,
    /// Number of hyperedges present in the graph.
    pub num_edges: usize,
    /// Number of non-zero entries in the assembled operator.
    pub nnz: usize,
    /// Average degree across all nodes.
    pub avg_degree: f64,
    /// Maximum observed node degree.
    pub max_degree: usize,
    /// Number of variables in the CSS code.
    pub code_variables: usize,
    /// Rank of the X stabiliser subsystem.
    pub code_rank_x: usize,
    /// Rank of the Z stabiliser subsystem.
    pub code_rank_z: usize,
    /// Canonical hash of the operator structure.
    pub hash: String,
}

/// Effective operator bundle with sparse entries and metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Operators {
    /// Metadata describing the construction.
    pub info: OperatorsInfo,
    /// Sparse entries for the assembled operator.
    pub entries: Vec<OperatorEntry>,
    /// Per-node summaries useful for downstream diagnostics.
    pub node_degrees: Vec<NodeSummary>,
}

fn collect_endpoints(
    endpoints: &HyperedgeEndpoints,
    node_map: &BTreeMap<u64, usize>,
    degrees: &mut [usize],
) -> Result<Vec<(usize, usize)>, AsmError> {
    let mut pairs = Vec::new();
    for source in endpoints.sources.iter() {
        let Some(&row) = node_map.get(&source.as_raw()) else {
            return Err(graph_error(
                "unknown-node",
                "source node missing from operator map",
            ));
        };
        degrees[row] += 1;
        for destination in endpoints.destinations.iter() {
            let Some(&col) = node_map.get(&destination.as_raw()) else {
                return Err(graph_error(
                    "unknown-node",
                    "destination node missing from operator map",
                ));
            };
            degrees[col] += 1;
            pairs.push((row, col));
        }
    }
    Ok(pairs)
}

fn entry_weight(variant: OpsVariant, endpoints: &HyperedgeEndpoints) -> f64 {
    let sources = endpoints.sources.len().max(1) as f64;
    let destinations = endpoints.destinations.len().max(1) as f64;
    match variant {
        OpsVariant::Default => round_weight(1.0 / (sources * destinations)),
        OpsVariant::Alt => {
            let factor = (sources + destinations) / (sources * destinations);
            round_weight(factor * 0.5)
        }
    }
}

/// Builds deterministic sparse operators from the provided state description.
pub fn build_operators(
    graph: &HypergraphImpl,
    code: &CSSCode,
    opts: &OpOpts,
) -> Result<Operators, AsmError> {
    let nodes: Vec<_> = graph.nodes().collect();
    if nodes.is_empty() {
        return Err(graph_error(
            "empty-graph",
            "cannot build operators for empty graph",
        ));
    }
    let mut node_map = BTreeMap::new();
    for (idx, node) in nodes.iter().enumerate() {
        node_map.insert(node.as_raw(), idx);
    }
    let mut degrees = vec![0usize; nodes.len()];
    let mut entries = Vec::new();
    let mut edge_count = 0usize;

    for edge in graph.edges() {
        let endpoints = graph.hyperedge(edge)?;
        let pairs = collect_endpoints(&endpoints, &node_map, &mut degrees)?;
        let weight = entry_weight(opts.variant, &endpoints);
        for (row, col) in pairs {
            entries.push(OperatorEntry { row, col, weight });
        }
        edge_count += 1;
    }

    entries.sort_by(|a, b| {
        a.row
            .cmp(&b.row)
            .then_with(|| a.col.cmp(&b.col))
            .then_with(|| {
                a.weight
                    .partial_cmp(&b.weight)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let mut coalesced: Vec<OperatorEntry> = Vec::with_capacity(entries.len());
    for entry in entries {
        if let Some(prev) = coalesced.last_mut() {
            if prev.row == entry.row && prev.col == entry.col {
                prev.weight = round_weight(prev.weight + entry.weight);
                continue;
            }
        }
        coalesced.push(entry);
    }

    let nnz = coalesced.len();
    let total_degree: usize = degrees.iter().copied().sum();
    let avg_degree = if degrees.is_empty() {
        0.0
    } else {
        round_weight(total_degree as f64 / degrees.len() as f64)
    };
    let max_degree = degrees.iter().copied().max().unwrap_or(0);

    let node_degrees = nodes
        .iter()
        .map(|node| {
            let idx = node_map.get(&node.as_raw()).copied().unwrap_or_default();
            NodeSummary {
                node: node.as_raw(),
                degree: degrees[idx],
            }
        })
        .collect();

    let hash = stable_hash_string(&coalesced)?;

    let info = OperatorsInfo {
        num_nodes: nodes.len(),
        num_edges: edge_count,
        nnz,
        avg_degree,
        max_degree,
        code_variables: code.num_variables(),
        code_rank_x: code.rank_x(),
        code_rank_z: code.rank_z(),
        hash,
    };

    Ok(Operators {
        info,
        entries: coalesced,
        node_degrees,
    })
}
