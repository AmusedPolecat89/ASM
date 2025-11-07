use sha2::{Digest, Sha256};

use crate::manifest::PluginManifest;
use crate::serde::to_canonical_json_bytes;
use asm_core::errors::AsmError;

pub fn compute_manifest_hash(manifest: &PluginManifest) -> Result<String, AsmError> {
    let bytes = to_canonical_json_bytes(manifest)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

pub fn compute_plugin_hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

// Provide minimal hex helper without adding new dep by re-exporting from hex crate via sha2? Need hex crate.
