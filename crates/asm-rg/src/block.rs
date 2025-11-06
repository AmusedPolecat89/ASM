use std::collections::BTreeMap;

use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::{Hypergraph, NodeId};
use asm_graph::HypergraphImpl;

use crate::params::RGOpts;

/// Deterministic partition of fine nodes into coarse blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockPartition {
    blocks: Vec<Vec<NodeId>>,
    lookup: BTreeMap<NodeId, usize>,
}

impl BlockPartition {
    /// Returns the ordered list of blocks.
    pub fn blocks(&self) -> &[Vec<NodeId>] {
        &self.blocks
    }

    /// Returns the block index that contains the provided node.
    pub fn block_index(&self, node: NodeId) -> Option<usize> {
        self.lookup.get(&node).copied()
    }
}

/// Partitions the nodes of `graph` into deterministic blocks based on `opts`.
pub fn partition_nodes(graph: &HypergraphImpl, opts: &RGOpts) -> Result<BlockPartition, AsmError> {
    let opts = opts.sanitised();
    let mut nodes: Vec<NodeId> = graph.nodes().collect();
    if nodes.is_empty() {
        let info = ErrorInfo::new(
            "empty-graph",
            "RG requires at least one node to build a partition",
        );
        return Err(AsmError::RG(info));
    }

    nodes.sort_by_key(|node| mix(node.as_raw(), opts.seed));

    let mut blocks: Vec<Vec<NodeId>> = Vec::new();
    let mut lookup = BTreeMap::new();
    let mut current = Vec::new();
    for node in nodes {
        if current.len() >= opts.max_block_size {
            if current.is_empty() {
                current.push(node);
            } else {
                blocks.push(std::mem::take(&mut current));
                current.push(node);
            }
        } else {
            current.push(node);
        }
    }
    if !current.is_empty() {
        blocks.push(current);
    }

    for (idx, block) in blocks.iter().enumerate() {
        for node in block {
            lookup.insert(*node, idx);
        }
    }

    Ok(BlockPartition { blocks, lookup })
}

fn mix(value: u64, seed: u64) -> u64 {
    let mut x = value ^ seed;
    x = x.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 33;
    x = x.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x ^= x >> 29;
    x
}
