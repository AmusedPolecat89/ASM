use asm_core::errors::AsmError;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::serde::to_canonical_json_bytes;

/// Computes a stable hexadecimal hash for the provided payload using canonical JSON ordering.
pub fn stable_hash_string<T: Serialize>(value: &T) -> Result<String, AsmError> {
    let bytes = to_canonical_json_bytes(value)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("{:x}", digest))
}
