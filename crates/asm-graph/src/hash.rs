use asm_core::errors::AsmError;
use sha2::{Digest, Sha256};

use crate::flags::{HypergraphConfig, KUniformity};
use crate::hypergraph::{EdgeSignature, HypergraphImpl};
use asm_core::Hypergraph;

/// Computes the canonical structural hash for the provided graph.
pub fn canonical_hash(graph: &HypergraphImpl) -> Result<String, AsmError> {
    let mut hasher = Sha256::new();
    encode_config(graph.config(), &mut hasher);

    let nodes: Vec<_> = graph.nodes().collect();
    hasher.update((nodes.len() as u64).to_le_bytes());

    let mut signatures: Vec<EdgeSignature> = Vec::new();
    for edge_id in graph.edges() {
        let endpoints = graph.hyperedge(edge_id)?;
        signatures.push(EdgeSignature::new(
            endpoints.sources.as_ref(),
            endpoints.destinations.as_ref(),
        ));
    }
    signatures.sort();
    hasher.update((signatures.len() as u64).to_le_bytes());
    for signature in signatures {
        update_slice(signature.raw_sources(), &mut hasher);
        update_slice(signature.raw_destinations(), &mut hasher);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn encode_config(config: &HypergraphConfig, hasher: &mut Sha256) {
    if config.causal_mode {
        hasher.update(b"causal");
    } else {
        hasher.update(b"acyclic-off");
    }
    encode_option_usize("max-in", config.max_in_degree, hasher);
    encode_option_usize("max-out", config.max_out_degree, hasher);
    match config.k_uniform {
        None => hasher.update(b"kuniform:none"),
        Some(KUniformity::Balanced {
            sources,
            destinations,
        }) => {
            hasher.update(b"kuniform:balanced");
            hasher.update((sources as u64).to_le_bytes());
            hasher.update((destinations as u64).to_le_bytes());
        }
        Some(KUniformity::Total { total, min_sources }) => {
            hasher.update(b"kuniform:total");
            hasher.update((total as u64).to_le_bytes());
            hasher.update((min_sources as u64).to_le_bytes());
        }
    }
    hasher.update(config.schema_version.major.to_le_bytes());
    hasher.update(config.schema_version.minor.to_le_bytes());
    hasher.update(config.schema_version.patch.to_le_bytes());
}

fn encode_option_usize(label: &str, value: Option<usize>, hasher: &mut Sha256) {
    match value {
        Some(v) => {
            hasher.update(label.as_bytes());
            hasher.update(b":some");
            hasher.update((v as u64).to_le_bytes());
        }
        None => {
            hasher.update(label.as_bytes());
            hasher.update(b":none");
        }
    }
}

fn update_slice(values: &[u64], hasher: &mut Sha256) {
    hasher.update((values.len() as u64).to_le_bytes());
    for value in values {
        hasher.update(value.to_le_bytes());
    }
}
