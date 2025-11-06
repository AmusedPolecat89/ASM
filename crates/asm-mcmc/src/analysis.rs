use std::fs;
use std::path::{Path, PathBuf};

use asm_code::dispersion::{estimate_dispersion, DispersionOptions, DispersionReport};
use asm_code::{CSSCode, SpeciesId};
use asm_core::errors::ErrorInfo;
use asm_core::AsmError;
use asm_graph::{graph_from_json, HypergraphImpl};

use crate::checkpoint::{self, CheckpointPayload};

/// Loads the cold replica end state (code and graph) from a run directory.
pub fn load_end_state(run_dir: &Path) -> Result<(CSSCode, HypergraphImpl), AsmError> {
    let code_path = run_dir.join("end_state").join("code.json");
    let graph_path = run_dir.join("end_state").join("graph.json");

    let code_json = fs::read_to_string(&code_path).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("end-state-read", err.to_string())
                .with_context("path", code_path.display().to_string()),
        )
    })?;
    let graph_json = fs::read_to_string(&graph_path).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("end-state-read", err.to_string())
                .with_context("path", graph_path.display().to_string()),
        )
    })?;

    let code = asm_code::serde::from_json(&code_json)?;
    let graph = graph_from_json(&graph_json)?;
    Ok((code, graph))
}

/// Computes dispersion information for a given state.
pub fn dispersion_for_state(
    code: &CSSCode,
    graph: &HypergraphImpl,
    species: &[SpeciesId],
    options: &DispersionOptions,
) -> Result<DispersionReport, AsmError> {
    let species_list = if species.is_empty() {
        code.species_catalog()
    } else {
        species.to_vec()
    };
    estimate_dispersion(code, graph, &species_list, options)
}

/// Computes dispersion data for the cold replica stored inside a checkpoint file.
pub fn dispersion_for_checkpoint(
    checkpoint_path: &Path,
    species: &[SpeciesId],
    options: &DispersionOptions,
) -> Result<DispersionReport, AsmError> {
    let payload = CheckpointPayload::load(checkpoint_path)?;
    let states = checkpoint::restore_payload(&payload)?;
    let Some((_, code, graph, _)) = states.into_iter().next() else {
        return Err(AsmError::Serde(
            ErrorInfo::new("checkpoint-empty", "checkpoint contained no replicas")
                .with_context("path", checkpoint_path.display().to_string()),
        ));
    };
    dispersion_for_state(&code, &graph, species, options)
}

/// Resolves checkpoint paths from a manifest-relative listing.
pub fn resolve_checkpoint_paths(run_dir: &Path, manifest_paths: &[PathBuf]) -> Vec<PathBuf> {
    manifest_paths
        .iter()
        .map(|relative| run_dir.join(relative))
        .collect()
}
