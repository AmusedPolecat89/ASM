use std::fs;
use std::path::{Path, PathBuf};

use asm_code::css::CSSCode;
use asm_code::serde as code_serde;
use asm_core::errors::ErrorInfo;
use asm_core::AsmError;
use asm_graph::{graph_from_json, graph_to_json, HypergraphImpl};
use serde::{Deserialize, Serialize};

use crate::energy::EnergyBreakdown;

/// Serializable payload representing a checkpointed replica.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaCheckpoint {
    /// Temperature associated with the replica.
    pub temperature: f64,
    /// Serialized CSS code in JSON form.
    pub code_json: String,
    /// Serialized graph in JSON form.
    pub graph_json: String,
    /// Energy at the time of checkpointing.
    pub energy: EnergyBreakdown,
}

/// Aggregated checkpoint payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointPayload {
    /// Sweep number when the checkpoint was written.
    pub sweep: usize,
    /// Configuration snapshot associated with the run.
    pub config: crate::config::RunConfig,
    /// Master seed used to derive replica substreams.
    pub master_seed: u64,
    /// Replica states stored in the checkpoint.
    pub replicas: Vec<ReplicaCheckpoint>,
}

impl CheckpointPayload {
    /// Restores the payload from disk.
    pub fn load(path: &Path) -> Result<Self, AsmError> {
        let contents = fs::read_to_string(path).map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("checkpoint-read", err.to_string())
                    .with_context("path", path.display().to_string()),
            )
        })?;
        serde_json::from_str(&contents).map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("checkpoint-parse", err.to_string())
                    .with_context("path", path.display().to_string()),
            )
        })
    }

    /// Writes the payload to disk.
    pub fn store(&self, path: &Path) -> Result<(), AsmError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                AsmError::Serde(
                    ErrorInfo::new("checkpoint-mkdir", err.to_string())
                        .with_context("path", parent.display().to_string()),
                )
            })?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("checkpoint-serialize", err.to_string())
                    .with_context("path", path.display().to_string()),
            )
        })?;
        fs::write(path, json).map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("checkpoint-write", err.to_string())
                    .with_context("path", path.display().to_string()),
            )
        })
    }
}

/// Constructs a checkpoint payload from replica states.
pub fn build_payload(
    sweep: usize,
    config: &crate::config::RunConfig,
    master_seed: u64,
    replicas: &[(f64, &CSSCode, &HypergraphImpl, &EnergyBreakdown)],
) -> Result<CheckpointPayload, AsmError> {
    let mut payload = CheckpointPayload {
        sweep,
        config: config.clone(),
        master_seed,
        replicas: Vec::with_capacity(replicas.len()),
    };
    for (temperature, code, graph, energy) in replicas {
        let code_json = code_serde::to_json(code)?;
        let graph_json = graph_to_json(graph)?;
        payload.replicas.push(ReplicaCheckpoint {
            temperature: *temperature,
            code_json,
            graph_json,
            energy: (*energy).clone(),
        });
    }
    Ok(payload)
}

/// Restores concrete states from a checkpoint payload.
pub fn restore_payload(
    payload: &CheckpointPayload,
) -> Result<Vec<(f64, CSSCode, HypergraphImpl, EnergyBreakdown)>, AsmError> {
    let mut states = Vec::with_capacity(payload.replicas.len());
    for replica in &payload.replicas {
        let code = code_serde::from_json(&replica.code_json)?;
        let graph = graph_from_json(&replica.graph_json)?;
        states.push((replica.temperature, code, graph, replica.energy.clone()));
    }
    Ok(states)
}

/// Determines the next checkpoint file path using a deterministic numbering scheme.
pub fn checkpoint_path(root: &Path, sweep: usize) -> PathBuf {
    root.join(format!("ckpt_{sweep:05}.json"))
}
