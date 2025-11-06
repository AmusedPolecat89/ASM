use std::collections::BTreeSet;

use asm_core::{EdgeId, NodeId};

/// Converts a [`NodeId`] into its underlying index within adjacency arrays.
pub(crate) fn node_index(id: NodeId) -> usize {
    id.as_raw() as usize
}

/// Converts an [`EdgeId`] into its underlying index within adjacency arrays.
pub(crate) fn edge_index(id: EdgeId) -> usize {
    id.as_raw() as usize
}

/// Creates a [`NodeId`] from an index.
pub(crate) fn make_node(index: usize) -> NodeId {
    NodeId::from_raw(index as u64)
}

/// Creates an [`EdgeId`] from an index.
pub(crate) fn make_edge(index: usize) -> EdgeId {
    EdgeId::from_raw(index as u64)
}

/// Ensures that the list of node identifiers is sorted and contains no duplicates.
pub(crate) fn canonicalize_nodes(nodes: &[NodeId]) -> Vec<NodeId> {
    let mut set = BTreeSet::new();
    for node in nodes {
        set.insert(*node);
    }
    set.into_iter().collect()
}
