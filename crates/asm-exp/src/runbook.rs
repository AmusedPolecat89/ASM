use std::path::PathBuf;

use asm_core::errors::AsmError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::hash::stable_hash_string;

/// Metadata describing the run context for a reproducible report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunMeta {
    pub created_at: String,
    pub commit: String,
    #[serde(default)]
    pub seeds: Vec<u64>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub summary: Value,
}

/// Deterministic runbook containing provenance and artefact references.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunBook {
    pub id: String,
    pub created_at: String,
    pub commit: String,
    pub seeds: Vec<u64>,
    pub inputs: Vec<String>,
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub summary: Value,
}

/// Builds a deterministic runbook covering the provided inputs and metadata.
pub fn build_runbook(inputs: &[PathBuf], meta: &RunMeta) -> Result<RunBook, AsmError> {
    let resolved_inputs: Vec<String> = inputs
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect();
    let id = stable_hash_string(&(resolved_inputs.clone(), meta))?;
    Ok(RunBook {
        id,
        created_at: meta.created_at.clone(),
        commit: meta.commit.clone(),
        seeds: meta.seeds.clone(),
        inputs: resolved_inputs,
        artifacts: meta.artifacts.clone(),
        summary: meta.summary.clone(),
    })
}
