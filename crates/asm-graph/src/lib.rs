#![deny(missing_docs)]
#![doc = include_str!("../docs/phase2-graph-api.md")]

//! Deterministic directed hypergraph engine implementing the `asm-core` contracts.

mod curvature;
mod flags;
mod generators;
mod hash;
mod hypergraph;
mod ids;
mod rewire;
mod serialization;

pub use flags::{HypergraphConfig, KUniformity};
pub use generators::{gen_bounded_degree, gen_quasi_regular};
pub use hash::canonical_hash;
pub use hypergraph::{DegreeLimits, EdgeSignature, HypergraphImpl};
pub use rewire::{
    rewire_resource_balanced, rewire_resource_balanced_dry_run, rewire_retarget,
    rewire_retarget_dry_run, rewire_swap_targets, rewire_swap_targets_dry_run, RewireDryRun,
    RewireOutcome,
};

/// Re-export curvature helpers for benchmarking convenience.
pub use curvature::{forman_curvature_edges, forman_curvature_nodes, ollivier_lite_nodes};

/// Re-export serialization helpers for downstream crates.
pub use serialization::{graph_from_bytes, graph_from_json, graph_to_bytes, graph_to_json};
