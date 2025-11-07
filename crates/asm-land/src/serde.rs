use std::collections::BTreeMap;
use std::iter::FromIterator;

use asm_core::errors::{AsmError, ErrorInfo};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{Map, Value};

fn serde_error(code: &str, err: impl ToString) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, err.to_string()))
}

fn canonicalize(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let ordered = map
                .into_iter()
                .map(|(key, value)| (key, canonicalize(value)))
                .collect::<BTreeMap<_, _>>();
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
    let value = serde_json::to_value(value).map_err(|err| serde_error("json_serialize", err))?;
    let canonical = canonicalize(value);
    let mut bytes = Vec::new();
    serde_json::to_writer(&mut bytes, &canonical).map_err(|err| serde_error("json_write", err))?;
    Ok(bytes)
}

/// Deserializes a value from JSON bytes using canonical schema validation.
pub fn from_json_slice<T: DeserializeOwned>(data: &[u8]) -> Result<T, AsmError> {
    serde_json::from_slice(data).map_err(|err| serde_error("json_deserialize", err))
}

/// Serializes a value into deterministic YAML.
pub fn to_yaml_string<T: Serialize>(value: &T) -> Result<String, AsmError> {
    serde_yaml::to_string(value).map_err(|err| serde_error("yaml_serialize", err))
}

/// Deserializes a YAML payload into the requested type.
pub fn from_yaml_slice<T: DeserializeOwned>(data: &[u8]) -> Result<T, AsmError> {
    serde_yaml::from_slice(data).map_err(|err| serde_error("yaml_deserialize", err))
}
