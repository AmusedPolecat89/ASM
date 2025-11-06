use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::provenance::SchemaVersion;
use asm_core::{Hypergraph, NodeId};
use serde::{Deserialize, Serialize};

use crate::flags::{HypergraphConfig, KUniformity};
use crate::hypergraph::HypergraphImpl;

/// Serializes the graph to a compact binary representation using `bincode`.
pub fn graph_to_bytes(graph: &HypergraphImpl) -> Result<Vec<u8>, AsmError> {
    let serializable = SerializableGraph::from_graph(graph);
    bincode::serialize(&serializable)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("serialize-bytes", err.to_string())))
}

/// Restores a graph from its binary representation.
pub fn graph_from_bytes(bytes: &[u8]) -> Result<HypergraphImpl, AsmError> {
    let serializable: SerializableGraph = bincode::deserialize(bytes)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("deserialize-bytes", err.to_string())))?;
    serializable.into_graph()
}

/// Serializes the graph to a JSON string.
pub fn graph_to_json(graph: &HypergraphImpl) -> Result<String, AsmError> {
    let serializable = SerializableGraph::from_graph(graph);
    serde_json::to_string_pretty(&serializable)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("serialize-json", err.to_string())))
}

/// Restores a graph from a JSON string.
pub fn graph_from_json(json: &str) -> Result<HypergraphImpl, AsmError> {
    let serializable: SerializableGraph = serde_json::from_str(json)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("deserialize-json", err.to_string())))?;
    serializable.into_graph()
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializableGraph {
    config: SerializableConfig,
    nodes: Vec<bool>,
    edges: Vec<SerializableEdge>,
}

impl SerializableGraph {
    fn from_graph(graph: &HypergraphImpl) -> Self {
        let config = SerializableConfig::from_config(graph.config());
        let nodes = graph.node_states();
        let edges = graph
            .edge_payloads()
            .into_iter()
            .map(|(alive, sources, destinations)| SerializableEdge {
                alive,
                sources: sources.iter().map(|id| id.as_raw()).collect(),
                destinations: destinations.iter().map(|id| id.as_raw()).collect(),
            })
            .collect();
        Self {
            config,
            nodes,
            edges,
        }
    }

    fn into_graph(self) -> Result<HypergraphImpl, AsmError> {
        let config = self.config.into_config();
        let mut graph = HypergraphImpl::new(config);
        for alive in self.nodes {
            let node_id = graph.add_node()?;
            if !alive {
                graph.remove_node(node_id)?;
            }
        }
        for edge in self.edges {
            let sources: Vec<NodeId> = edge.sources.into_iter().map(NodeId::from_raw).collect();
            let destinations: Vec<NodeId> = edge
                .destinations
                .into_iter()
                .map(NodeId::from_raw)
                .collect();
            if edge.alive {
                graph.add_hyperedge(&sources, &destinations)?;
            } else {
                graph.push_dead_edge(sources, destinations);
            }
        }
        Ok(graph)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializableConfig {
    causal_mode: bool,
    max_in_degree: Option<usize>,
    max_out_degree: Option<usize>,
    k_uniform: Option<SerializableUniformity>,
    schema_version: SchemaVersion,
}

impl SerializableConfig {
    fn from_config(config: &HypergraphConfig) -> Self {
        Self {
            causal_mode: config.causal_mode,
            max_in_degree: config.max_in_degree,
            max_out_degree: config.max_out_degree,
            k_uniform: config.k_uniform.map(SerializableUniformity::from),
            schema_version: config.schema_version,
        }
    }

    fn into_config(self) -> HypergraphConfig {
        HypergraphConfig {
            causal_mode: self.causal_mode,
            max_in_degree: self.max_in_degree,
            max_out_degree: self.max_out_degree,
            k_uniform: self.k_uniform.map(|k| k.into()),
            schema_version: self.schema_version,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializableEdge {
    alive: bool,
    sources: Vec<u64>,
    destinations: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum SerializableUniformity {
    Total { total: usize, min_sources: usize },
    Balanced { sources: usize, destinations: usize },
}

impl From<KUniformity> for SerializableUniformity {
    fn from(value: KUniformity) -> Self {
        match value {
            KUniformity::Total { total, min_sources } => {
                SerializableUniformity::Total { total, min_sources }
            }
            KUniformity::Balanced {
                sources,
                destinations,
            } => SerializableUniformity::Balanced {
                sources,
                destinations,
            },
        }
    }
}

impl From<SerializableUniformity> for KUniformity {
    fn from(value: SerializableUniformity) -> Self {
        match value {
            SerializableUniformity::Total { total, min_sources } => {
                KUniformity::Total { total, min_sources }
            }
            SerializableUniformity::Balanced {
                sources,
                destinations,
            } => KUniformity::Balanced {
                sources,
                destinations,
            },
        }
    }
}
