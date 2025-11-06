use std::fs;
use std::path::{Path, PathBuf};

use asm_core::errors::ErrorInfo;
use asm_core::AsmError;
use serde::{Deserialize, Serialize};

use crate::config::RunConfig;

/// Structured manifest describing a completed or running ensemble sweep.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
    /// Configuration used for the run.
    pub config: RunConfig,
    /// Master seed used to derive replica substreams.
    pub master_seed: u64,
    /// Optional seed label captured from the configuration.
    pub seed_label: Option<String>,
    /// Canonical hash of the terminal code state.
    pub code_hash: String,
    /// Canonical hash of the terminal graph state.
    pub graph_hash: String,
    /// Metrics file produced during the run (relative to run directory).
    pub metrics_file: Option<PathBuf>,
    /// Checkpoint files generated during the run (relative order preserved).
    pub checkpoints: Vec<PathBuf>,
}

impl RunManifest {
    /// Writes the manifest to a JSON file.
    pub fn write(&self, path: &Path) -> Result<(), AsmError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                AsmError::Serde(
                    ErrorInfo::new("manifest-mkdir", err.to_string())
                        .with_context("path", parent.display().to_string()),
                )
            })?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("manifest-serialize", err.to_string())
                    .with_context("path", path.display().to_string()),
            )
        })?;
        fs::write(path, json).map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("manifest-write", err.to_string())
                    .with_context("path", path.display().to_string()),
            )
        })
    }

    /// Loads a manifest from disk.
    pub fn load(path: &Path) -> Result<Self, AsmError> {
        let contents = fs::read_to_string(path).map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("manifest-read", err.to_string())
                    .with_context("path", path.display().to_string()),
            )
        })?;
        serde_json::from_str(&contents).map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("manifest-parse", err.to_string())
                    .with_context("path", path.display().to_string()),
            )
        })
    }
}
