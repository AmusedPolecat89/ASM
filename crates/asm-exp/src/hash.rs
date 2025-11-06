use asm_code::hash::canonical_code_hash;
use asm_core::errors::AsmError;
use asm_graph::canonical_hash as graph_hash;
use asm_rg::StateRef;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::serde::to_canonical_json_bytes;

/// Computes a stable hexadecimal hash for the provided serializable payload.
pub fn stable_hash_string<T: Serialize>(value: &T) -> Result<String, AsmError> {
    let bytes = to_canonical_json_bytes(value)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("{:x}", digest))
}

/// Computes a canonical hash for the provided state reference by combining
/// graph and code canonical hashes under a single digest.
pub fn canonical_state_hash(state: &StateRef<'_>) -> Result<String, AsmError> {
    let graph = graph_hash(state.graph).map_err(|err| match err {
        AsmError::Graph(info) => AsmError::Graph(info),
        other => other,
    })?;
    let code = canonical_code_hash(state.code);
    stable_hash_string(&(graph, code))
}
