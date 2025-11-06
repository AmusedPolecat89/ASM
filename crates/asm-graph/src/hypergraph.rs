use std::collections::{BTreeMap, BTreeSet};

use asm_core::{
    errors::{AsmError, ErrorInfo},
    DegreeBounds, EdgeId, HyperedgeEndpoints, Hypergraph, NodeId,
};

use crate::flags::HypergraphConfig;
use crate::ids::{canonicalize_nodes, edge_index, make_edge, make_node, node_index};

/// Tracks the maximum degree configuration exposed by the graph.
#[derive(Debug, Clone, Copy)]
pub struct DegreeLimits {
    /// Maximum inbound degree permitted for any node.
    pub max_in: Option<usize>,
    /// Maximum outbound degree permitted for any node.
    pub max_out: Option<usize>,
}

impl DegreeLimits {
    /// Returns a descriptor with no limits.
    pub const fn unlimited() -> Self {
        Self {
            max_in: None,
            max_out: None,
        }
    }
}

/// Canonical signature used to deduplicate hyperedges.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EdgeSignature {
    sources: Vec<u64>,
    destinations: Vec<u64>,
}

impl EdgeSignature {
    pub(crate) fn new(sources: &[NodeId], destinations: &[NodeId]) -> Self {
        Self {
            sources: sources.iter().map(|id| id.as_raw()).collect(),
            destinations: destinations.iter().map(|id| id.as_raw()).collect(),
        }
    }

    /// Returns the raw source identifiers used by the signature.
    pub fn raw_sources(&self) -> &[u64] {
        &self.sources
    }

    /// Returns the raw destination identifiers used by the signature.
    pub fn raw_destinations(&self) -> &[u64] {
        &self.destinations
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NodeRecord {
    alive: bool,
    in_edges: BTreeSet<EdgeId>,
    out_edges: BTreeSet<EdgeId>,
}

impl NodeRecord {
    fn new() -> Self {
        Self {
            alive: true,
            in_edges: BTreeSet::new(),
            out_edges: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EdgeRecord {
    alive: bool,
    sources: Vec<NodeId>,
    destinations: Vec<NodeId>,
    signature: EdgeSignature,
}

impl EdgeRecord {
    fn new(sources: Vec<NodeId>, destinations: Vec<NodeId>) -> Self {
        let signature = EdgeSignature::new(&sources, &destinations);
        Self {
            alive: true,
            sources,
            destinations,
            signature,
        }
    }

    fn dead(sources: Vec<NodeId>, destinations: Vec<NodeId>) -> Self {
        let signature = EdgeSignature::new(&sources, &destinations);
        Self {
            alive: false,
            sources,
            destinations,
            signature,
        }
    }
}

/// Deterministic directed hypergraph implementation.
#[derive(Debug, Clone)]
pub struct HypergraphImpl {
    config: HypergraphConfig,
    nodes: Vec<NodeRecord>,
    edges: Vec<EdgeRecord>,
    signatures: BTreeSet<EdgeSignature>,
}

impl HypergraphImpl {
    /// Creates an empty hypergraph with the provided configuration.
    pub fn new(config: HypergraphConfig) -> Self {
        Self {
            config,
            nodes: Vec::new(),
            edges: Vec::new(),
            signatures: BTreeSet::new(),
        }
    }

    /// Returns the configuration used by this graph.
    pub fn config(&self) -> &HypergraphConfig {
        &self.config
    }

    /// Returns whether the graph enforces causal mode.
    pub fn is_causal_mode(&self) -> bool {
        self.config.causal_mode
    }

    /// Returns the configured degree limits.
    pub fn degree_limits(&self) -> DegreeLimits {
        DegreeLimits {
            max_in: self.config.max_in_degree,
            max_out: self.config.max_out_degree,
        }
    }

    /// Returns the inbound degree for the provided node.
    pub fn in_degree(&self, node: NodeId) -> Result<usize, AsmError> {
        let record = self.node(node)?;
        Ok(record.in_edges.len())
    }

    /// Returns the outbound degree for the provided node.
    pub fn out_degree(&self, node: NodeId) -> Result<usize, AsmError> {
        let record = self.node(node)?;
        Ok(record.out_edges.len())
    }

    /// Returns all edges touching the provided node.
    pub fn edges_touching(&self, node: NodeId) -> Result<Vec<EdgeId>, AsmError> {
        let record = self.node(node)?;
        let mut edges: BTreeSet<EdgeId> = record.in_edges.iter().copied().collect();
        edges.extend(record.out_edges.iter().copied());
        Ok(edges.into_iter().collect())
    }

    /// Returns the outbound edges for the provided node.
    pub(crate) fn outgoing_edges(&self, node: NodeId) -> Result<Vec<EdgeId>, AsmError> {
        let record = self.node(node)?;
        Ok(record.out_edges.iter().copied().collect())
    }

    /// Returns the inbound edges for the provided node.
    pub(crate) fn incoming_edges(&self, node: NodeId) -> Result<Vec<EdgeId>, AsmError> {
        let record = self.node(node)?;
        Ok(record.in_edges.iter().copied().collect())
    }

    /// Returns the raw alive flags for each stored node.
    pub(crate) fn node_states(&self) -> Vec<bool> {
        self.nodes.iter().map(|node| node.alive).collect()
    }

    /// Returns the stored edge payloads for serialization.
    pub(crate) fn edge_payloads(&self) -> Vec<(bool, Vec<NodeId>, Vec<NodeId>)> {
        self.edges
            .iter()
            .map(|edge| (edge.alive, edge.sources.clone(), edge.destinations.clone()))
            .collect()
    }

    /// Returns the source nodes of a hyperedge.
    pub fn src_of(&self, edge: EdgeId) -> Result<&[NodeId], AsmError> {
        Ok(&self.edge(edge)?.sources)
    }

    /// Returns the destination nodes of a hyperedge.
    pub fn dst_of(&self, edge: EdgeId) -> Result<&[NodeId], AsmError> {
        Ok(&self.edge(edge)?.destinations)
    }

    /// Replaces an existing hyperedge with new endpoints while validating invariants.
    pub(crate) fn overwrite_edge(
        &mut self,
        edge: EdgeId,
        new_sources: &[NodeId],
        new_destinations: &[NodeId],
    ) -> Result<(), AsmError> {
        let previous = self.detach_edge(edge)?;
        let sources = canonicalize_nodes(new_sources);
        let destinations = canonicalize_nodes(new_destinations);
        match self.attach_to_slot(edge, sources, destinations) {
            Ok(()) => Ok(()),
            Err(err) => {
                let _ = self.restore_edge(edge, previous);
                Err(err)
            }
        }
    }

    pub(crate) fn restore_edge(
        &mut self,
        edge: EdgeId,
        mut record: EdgeRecord,
    ) -> Result<(), AsmError> {
        record.alive = true;
        let sources = record.sources.clone();
        let destinations = record.destinations.clone();
        if let Some(slot) = self.edges.get_mut(edge_index(edge)) {
            *slot = record.clone();
        }
        for source in sources {
            self.node_mut(source)?.out_edges.insert(edge);
        }
        for destination in destinations {
            self.node_mut(destination)?.in_edges.insert(edge);
        }
        self.signatures.insert(record.signature.clone());
        Ok(())
    }

    /// Returns an iterator over all alive node identifiers.
    pub(crate) fn node_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.alive)
            .map(|(idx, _)| make_node(idx))
            .collect()
    }

    /// Returns an iterator over all alive edge identifiers.
    pub(crate) fn edge_ids(&self) -> Vec<EdgeId> {
        self.edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.alive)
            .map(|(idx, _)| make_edge(idx))
            .collect()
    }

    pub(crate) fn node(&self, id: NodeId) -> Result<&NodeRecord, AsmError> {
        self.nodes
            .get(node_index(id))
            .filter(|record| record.alive)
            .ok_or_else(|| {
                graph_error("unknown-node", "node does not exist").with_context("node", id.as_raw())
            })
    }

    pub(crate) fn node_mut(&mut self, id: NodeId) -> Result<&mut NodeRecord, AsmError> {
        self.nodes
            .get_mut(node_index(id))
            .filter(|record| record.alive)
            .ok_or_else(|| {
                graph_error("unknown-node", "node does not exist").with_context("node", id.as_raw())
            })
    }

    pub(crate) fn edge(&self, id: EdgeId) -> Result<&EdgeRecord, AsmError> {
        self.edges
            .get(edge_index(id))
            .filter(|record| record.alive)
            .ok_or_else(|| {
                graph_error("unknown-edge", "edge does not exist").with_context("edge", id.as_raw())
            })
    }

    fn ensure_uniformity(
        &self,
        sources: &[NodeId],
        destinations: &[NodeId],
    ) -> Result<(), AsmError> {
        if let Some(rule) = &self.config.k_uniform {
            if !rule.validate(sources.len(), destinations.len()) {
                return Err(graph_error(
                    "invalid-arity",
                    "hyperedge violates k-uniform configuration",
                )
                .with_context("sources", sources.len())
                .with_context("destinations", destinations.len()));
            }
        }
        Ok(())
    }

    fn ensure_degrees(&self, sources: &[NodeId], destinations: &[NodeId]) -> Result<(), AsmError> {
        if let Some(max_out) = self.config.max_out_degree {
            for node in sources {
                let record = self.node(*node)?;
                if record.out_edges.len() + 1 > max_out {
                    return Err(graph_error(
                        "out-degree-cap",
                        "outbound degree cap would be exceeded",
                    )
                    .with_context("node", node.as_raw())
                    .with_context("cap", max_out.to_string())
                    .with_context("attempted", (record.out_edges.len() + 1).to_string()));
                }
            }
        }
        if let Some(max_in) = self.config.max_in_degree {
            for node in destinations {
                let record = self.node(*node)?;
                if record.in_edges.len() + 1 > max_in {
                    return Err(graph_error(
                        "in-degree-cap",
                        "inbound degree cap would be exceeded",
                    )
                    .with_context("node", node.as_raw())
                    .with_context("cap", max_in.to_string())
                    .with_context("attempted", (record.in_edges.len() + 1).to_string()));
                }
            }
        }
        Ok(())
    }

    fn ensure_unique(&self, signature: &EdgeSignature) -> Result<(), AsmError> {
        if self.signatures.contains(signature) {
            return Err(graph_error("duplicate-edge", "hyperedge already exists"));
        }
        Ok(())
    }

    fn validate_cycle_free(
        &self,
        sources: &[NodeId],
        destinations: &[NodeId],
    ) -> Result<(), AsmError> {
        if !self.config.causal_mode {
            return Ok(());
        }
        if self.would_create_cycle(sources, destinations)? {
            return Err(graph_error(
                "would-create-cycle",
                "operation would introduce a directed cycle",
            ));
        }
        Ok(())
    }

    fn would_create_cycle(
        &self,
        sources: &[NodeId],
        destinations: &[NodeId],
    ) -> Result<bool, AsmError> {
        let mut adjacency: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();
        for edge in self.edges.iter() {
            if !edge.alive {
                continue;
            }
            for source in &edge.sources {
                let entry = adjacency.entry(*source).or_default();
                entry.extend(edge.destinations.iter().copied());
            }
        }
        for source in sources {
            let entry = adjacency.entry(*source).or_default();
            entry.extend(destinations.iter().copied());
        }
        let mut states: BTreeMap<NodeId, VisitState> = BTreeMap::new();
        for node in self.node_ids() {
            states.insert(node, VisitState::NotVisited);
        }
        for source in adjacency.keys() {
            states.entry(*source).or_insert(VisitState::NotVisited);
        }
        for node in states.keys().copied().collect::<Vec<_>>() {
            if dfs(node, &adjacency, &mut states) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) fn detach_edge(&mut self, id: EdgeId) -> Result<EdgeRecord, AsmError> {
        let idx = edge_index(id);
        let record = self.edges.get_mut(idx).ok_or_else(|| {
            graph_error("unknown-edge", "edge does not exist").with_context("edge", id.as_raw())
        })?;
        if !record.alive {
            return Err(graph_error("unknown-edge", "edge does not exist")
                .with_context("edge", id.as_raw()));
        }
        record.alive = false;
        self.signatures.remove(&record.signature);
        for source in &record.sources {
            if let Some(node) = self.nodes.get_mut(node_index(*source)) {
                node.out_edges.remove(&id);
            }
        }
        for destination in &record.destinations {
            if let Some(node) = self.nodes.get_mut(node_index(*destination)) {
                node.in_edges.remove(&id);
            }
        }
        Ok(record.clone())
    }

    fn attach_to_slot(
        &mut self,
        edge: EdgeId,
        sources: Vec<NodeId>,
        destinations: Vec<NodeId>,
    ) -> Result<(), AsmError> {
        self.ensure_uniformity(&sources, &destinations)?;
        self.ensure_degrees(&sources, &destinations)?;
        self.validate_cycle_free(&sources, &destinations)?;
        let record = EdgeRecord::new(sources.clone(), destinations.clone());
        self.ensure_unique(&record.signature)?;
        if let Some(slot) = self.edges.get_mut(edge_index(edge)) {
            *slot = record.clone();
        } else {
            self.edges.push(record.clone());
        }
        for source in sources {
            self.node_mut(source)?.out_edges.insert(edge);
        }
        for destination in destinations {
            self.node_mut(destination)?.in_edges.insert(edge);
        }
        self.signatures.insert(record.signature.clone());
        Ok(())
    }

    pub(crate) fn push_dead_edge(&mut self, sources: Vec<NodeId>, destinations: Vec<NodeId>) {
        let record = EdgeRecord::dead(sources, destinations);
        self.edges.push(record);
    }
}

impl Default for HypergraphImpl {
    fn default() -> Self {
        Self::new(HypergraphConfig::default())
    }
}

impl Hypergraph for HypergraphImpl {
    fn nodes(&self) -> Box<dyn std::iter::ExactSizeIterator<Item = NodeId> + '_> {
        Box::new(self.node_ids().into_iter())
    }

    fn edges(&self) -> Box<dyn std::iter::ExactSizeIterator<Item = EdgeId> + '_> {
        Box::new(self.edge_ids().into_iter())
    }

    fn hyperedge(&self, edge: EdgeId) -> Result<HyperedgeEndpoints, AsmError> {
        let record = self.edge(edge)?;
        Ok(HyperedgeEndpoints {
            sources: record.sources.clone().into_boxed_slice(),
            destinations: record.destinations.clone().into_boxed_slice(),
        })
    }

    fn degree_bounds(&self) -> Result<DegreeBounds, AsmError> {
        let mut min_in: Option<usize> = None;
        let mut max_in: Option<usize> = None;
        let mut min_out: Option<usize> = None;
        let mut max_out: Option<usize> = None;
        for node in self.nodes.iter().filter(|node| node.alive) {
            let in_deg = node.in_edges.len();
            let out_deg = node.out_edges.len();
            min_in = Some(min_in.map(|v| v.min(in_deg)).unwrap_or(in_deg));
            max_in = Some(max_in.map(|v| v.max(in_deg)).unwrap_or(in_deg));
            min_out = Some(min_out.map(|v| v.min(out_deg)).unwrap_or(out_deg));
            max_out = Some(max_out.map(|v| v.max(out_deg)).unwrap_or(out_deg));
        }
        Ok(DegreeBounds {
            min_in_degree: min_in,
            max_in_degree: max_in,
            min_out_degree: min_out,
            max_out_degree: max_out,
        })
    }

    fn add_node(&mut self) -> Result<NodeId, AsmError> {
        let id = make_node(self.nodes.len());
        self.nodes.push(NodeRecord::new());
        Ok(id)
    }

    fn add_hyperedge(
        &mut self,
        sources: &[NodeId],
        destinations: &[NodeId],
    ) -> Result<EdgeId, AsmError> {
        if sources.is_empty() || destinations.is_empty() {
            return Err(graph_error(
                "empty-endpoints",
                "hyperedges require non-empty source and destination sets",
            ));
        }
        let sources = canonicalize_nodes(sources);
        let destinations = canonicalize_nodes(destinations);
        self.ensure_uniformity(&sources, &destinations)?;
        self.ensure_degrees(&sources, &destinations)?;
        self.validate_cycle_free(&sources, &destinations)?;
        let edge = EdgeRecord::new(sources.clone(), destinations.clone());
        self.ensure_unique(&edge.signature)?;
        let id = make_edge(self.edges.len());
        for source in &sources {
            self.node_mut(*source)?.out_edges.insert(id);
        }
        for destination in &destinations {
            self.node_mut(*destination)?.in_edges.insert(id);
        }
        self.signatures.insert(edge.signature.clone());
        self.edges.push(edge);
        Ok(id)
    }

    fn remove_node(&mut self, node: NodeId) -> Result<(), AsmError> {
        let idx = node_index(node);
        let record = self.nodes.get(idx).ok_or_else(|| {
            graph_error("unknown-node", "node does not exist").with_context("node", node.as_raw())
        })?;
        if !record.alive {
            return Err(graph_error("unknown-node", "node does not exist")
                .with_context("node", node.as_raw()));
        }
        if !record.in_edges.is_empty() || !record.out_edges.is_empty() {
            return Err(graph_error(
                "node-not-isolated",
                "cannot remove node with incident edges",
            )
            .with_context("node", node.as_raw())
            .with_context("in_edges", record.in_edges.len())
            .with_context("out_edges", record.out_edges.len()));
        }
        let record = self.nodes.get_mut(idx).unwrap();
        record.alive = false;
        Ok(())
    }

    fn remove_hyperedge(&mut self, edge: EdgeId) -> Result<(), AsmError> {
        self.detach_edge(edge)?;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
    NotVisited,
    Visiting,
    Visited,
}

fn dfs(
    node: NodeId,
    adjacency: &BTreeMap<NodeId, BTreeSet<NodeId>>,
    states: &mut BTreeMap<NodeId, VisitState>,
) -> bool {
    match states.get(&node).copied().unwrap_or(VisitState::NotVisited) {
        VisitState::Visiting => true,
        VisitState::Visited => false,
        VisitState::NotVisited => {
            states.insert(node, VisitState::Visiting);
            if let Some(neighbours) = adjacency.get(&node) {
                for neighbour in neighbours {
                    if dfs(*neighbour, adjacency, states) {
                        return true;
                    }
                }
            }
            states.insert(node, VisitState::Visited);
            false
        }
    }
}

fn graph_error(code: impl Into<String>, message: impl Into<String>) -> AsmError {
    AsmError::Graph(ErrorInfo::new(code, message))
}

trait ContextExt {
    fn with_context(self, key: impl Into<String>, value: impl ToString) -> AsmError;
}

impl ContextExt for AsmError {
    fn with_context(self, key: impl Into<String>, value: impl ToString) -> AsmError {
        match self {
            AsmError::Graph(info) => AsmError::Graph(info.with_context(key, value.to_string())),
            other => other,
        }
    }
}
