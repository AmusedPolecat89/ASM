use asm_core::provenance::SchemaVersion;

/// Configuration options that control the behaviour of [`HypergraphImpl`](crate::HypergraphImpl).
#[derive(Debug, Clone)]
pub struct HypergraphConfig {
    /// Whether causal mode is enabled (cycle-introducing operations are rejected).
    pub causal_mode: bool,
    /// Maximum inbound degree permitted for any node.
    pub max_in_degree: Option<usize>,
    /// Maximum outbound degree permitted for any node.
    pub max_out_degree: Option<usize>,
    /// Optional arity constraint enforced on every hyperedge.
    pub k_uniform: Option<KUniformity>,
    /// Schema version stored alongside serialized payloads.
    pub schema_version: SchemaVersion,
}

impl Default for HypergraphConfig {
    fn default() -> Self {
        Self {
            causal_mode: true,
            max_in_degree: Some(8),
            max_out_degree: Some(8),
            k_uniform: Some(KUniformity::Balanced {
                sources: 2,
                destinations: 2,
            }),
            schema_version: SchemaVersion::new(2, 0, 0),
        }
    }
}

/// Describes the uniformity constraints applied to newly created hyperedges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KUniformity {
    /// The total number of incident nodes (sources + destinations) must match this constant.
    Total {
        /// Combined number of endpoints that must appear in every hyperedge.
        total: usize,
        /// Minimum number of sources required within the total count.
        min_sources: usize,
    },
    /// Sources and destinations must individually match the configured counts.
    Balanced {
        /// Number of source nodes per hyperedge.
        sources: usize,
        /// Number of destination nodes per hyperedge.
        destinations: usize,
    },
}

impl KUniformity {
    /// Validates the provided endpoints against this uniformity rule.
    pub fn validate(&self, sources: usize, destinations: usize) -> bool {
        match self {
            KUniformity::Total { total, min_sources } => {
                sources + destinations == *total && sources >= *min_sources && destinations >= 1
            }
            KUniformity::Balanced {
                sources: s,
                destinations: d,
            } => sources == *s && destinations == *d,
        }
    }
}
