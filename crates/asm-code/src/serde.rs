use asm_core::{AsmError, ErrorInfo, RunProvenance, SchemaVersion};
use serde::{Deserialize, Serialize};

use crate::css::{CSSCode, Constraint};
use crate::hash;

#[derive(Debug, Serialize, Deserialize)]
struct SerializableCSSCode {
    schema_version: SchemaVersion,
    provenance: RunProvenance,
    num_variables: usize,
    x_checks: Vec<Vec<usize>>,
    z_checks: Vec<Vec<usize>>,
    rank_x: usize,
    rank_z: usize,
}

fn serialize_constraints(constraints: &[Constraint]) -> Vec<Vec<usize>> {
    constraints
        .iter()
        .map(|constraint| constraint.variables().to_vec())
        .collect()
}

fn deserialize_constraints(data: &[Vec<usize>]) -> Vec<Constraint> {
    data.iter()
        .map(|vars| Constraint::new(vars.clone()))
        .collect()
}

/// Serializes a CSS code to a JSON string.
pub fn to_json(code: &CSSCode) -> Result<String, AsmError> {
    let (num_variables, x_checks, z_checks, schema_version, mut provenance, rank_x, rank_z) =
        hash::decompose(code);
    if provenance.code_hash.is_empty() {
        provenance.code_hash = code.canonical_hash();
    }
    let payload = SerializableCSSCode {
        schema_version,
        provenance,
        num_variables,
        x_checks: serialize_constraints(&x_checks),
        z_checks: serialize_constraints(&z_checks),
        rank_x,
        rank_z,
    };
    serde_json::to_string_pretty(&payload)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("json-serialize", err.to_string())))
}

/// Restores a CSS code from a JSON string.
pub fn from_json(data: &str) -> Result<CSSCode, AsmError> {
    let payload: SerializableCSSCode = serde_json::from_str(data)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("json-deserialize", err.to_string())))?;
    let x_checks = deserialize_constraints(&payload.x_checks);
    let z_checks = deserialize_constraints(&payload.z_checks);
    Ok(hash::reconstruct(
        payload.num_variables,
        x_checks,
        z_checks,
        payload.schema_version,
        payload.provenance,
        payload.rank_x,
        payload.rank_z,
    ))
}

/// Serializes a CSS code into a binary blob.
pub fn to_bytes(code: &CSSCode) -> Result<Vec<u8>, AsmError> {
    let json = to_json(code)?;
    bincode::serialize(&json)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("bincode-serialize", err.to_string())))
}

/// Rehydrates a CSS code from a binary blob.
pub fn from_bytes(bytes: &[u8]) -> Result<CSSCode, AsmError> {
    let json: String = bincode::deserialize(bytes)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("bincode-deserialize", err.to_string())))?;
    from_json(&json)
}
