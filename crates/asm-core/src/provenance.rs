//! Provenance and schema descriptors shared across ASM artifacts.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Semantic version describing the schema of serialized payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// Major version incremented for breaking changes.
    pub major: u32,
    /// Minor version incremented for additive changes.
    pub minor: u32,
    /// Patch version incremented for bug fixes and documentation updates.
    pub patch: u32,
}

impl SchemaVersion {
    /// Creates a new schema version descriptor.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl Default for SchemaVersion {
    fn default() -> Self {
        Self::new(1, 0, 0)
    }
}

/// Provenance information attached to every serialized artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RunProvenance {
    /// Hash of the input configuration used to produce the data.
    pub input_hash: String,
    /// Canonical hash of the hypergraph on which the run operated.
    pub graph_hash: String,
    /// Canonical hash of the constraint code.
    pub code_hash: String,
    /// Master deterministic seed used for all randomness.
    pub seed: u64,
    /// ISO-8601 timestamp recording when the artifact was generated.
    pub created_at: String,
    /// Version map for all tools involved in the run.
    pub tool_versions: BTreeMap<String, String>,
}
