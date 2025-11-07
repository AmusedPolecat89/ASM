use std::collections::BTreeMap;
use std::iter::FromIterator;

use ::serde::{Deserialize, Serialize};
use asm_core::errors::{AsmError, ErrorInfo};
use serde_json::{Map, Value};

fn serde_error(code: &str, err: impl ToString) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, err.to_string()))
}

fn canonicalize(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut ordered = BTreeMap::new();
            for (key, val) in map {
                ordered.insert(key, canonicalize(val));
            }
            Value::Object(Map::from_iter(ordered))
        }
        Value::Array(values) => {
            let canonical_values = values.into_iter().map(canonicalize).collect();
            Value::Array(canonical_values)
        }
        other => other,
    }
}

/// Serializes a value into canonical JSON bytes with deterministic ordering.
pub fn to_canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, AsmError> {
    let value = serde_json::to_value(value).map_err(|err| serde_error("json-encode", err))?;
    let canonical = canonicalize(value);
    let mut bytes = Vec::new();
    serde_json::to_writer(&mut bytes, &canonical).map_err(|err| serde_error("json-write", err))?;
    Ok(bytes)
}

/// Restores a value from canonical JSON bytes.
pub fn from_json_slice<T: for<'de> Deserialize<'de>>(data: &[u8]) -> Result<T, AsmError> {
    serde_json::from_slice(data).map_err(|err| serde_error("json-read", err))
}
