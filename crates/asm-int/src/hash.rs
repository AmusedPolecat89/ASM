use asm_code::hash::canonical_code_hash;
use asm_core::errors::AsmError;
use asm_graph::canonical_hash as graph_hash;
use asm_rg::StateRef;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::serde::to_canonical_json_bytes;

/// Computes a stable hexadecimal hash for the provided serialisable payload.
pub fn stable_hash_string<T: Serialize>(value: &T) -> Result<String, AsmError> {
    let bytes = to_canonical_json_bytes(value)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("{:x}", digest))
}

/// Computes a canonical hash for the provided RG state reference.
pub fn canonical_state_hash(state: &StateRef<'_>) -> Result<String, AsmError> {
    let graph = graph_hash(state.graph).map_err(|err| match err {
        AsmError::Graph(info) => AsmError::Graph(info),
        other => other,
    })?;
    let code = canonical_code_hash(state.code);
    stable_hash_string(&(graph, code))
}

/// Converts a hexadecimal hash string into a deterministic seed.
pub fn seed_from_hash(hash: &str) -> u64 {
    let trimmed = hash.as_bytes();
    let mut acc: u64 = 0;
    for chunk in trimmed.chunks(8) {
        let mut value: u64 = 0;
        for &byte in chunk {
            let digit = match byte {
                b'0'..=b'9' => (byte - b'0') as u64,
                b'a'..=b'f' => (byte - b'a' + 10) as u64,
                b'A'..=b'F' => (byte - b'A' + 10) as u64,
                _ => 0,
            };
            value = (value << 4) | digit;
        }
        acc ^= value;
    }
    acc
}

/// Rounds a floating point value to the canonical precision used by Phase 13.
pub fn round_f64(value: f64) -> f64 {
    let scaled = (value * 1e9).round();
    scaled / 1e9
}
