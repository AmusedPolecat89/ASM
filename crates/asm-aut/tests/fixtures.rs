use std::fs;
use std::path::PathBuf;

use asm_code::serde as code_serde;
use asm_code::CSSCode;
use asm_core::{AsmError, ErrorInfo};
use asm_graph::{graph_from_json, HypergraphImpl};
use serde::Deserialize;

use asm_aut::invariants::ProvenanceInfo;

/// Fixture manifest metadata extracted from JSON snapshots.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FixtureManifest {
    pub master_seed: Option<u64>,
    pub run_directory: Option<String>,
    pub commit: Option<String>,
}

/// In-memory representation of a vacuum fixture.
#[derive(Debug)]
pub struct FixtureState {
    pub code: CSSCode,
    pub graph: HypergraphImpl,
    pub manifest: FixtureManifest,
}

#[derive(Debug, Deserialize)]
struct ManifestWrapper {
    #[serde(default)]
    master_seed: Option<u64>,
    #[serde(default)]
    seed_label: Option<String>,
    #[serde(default)]
    code_hash: Option<String>,
    #[serde(default)]
    graph_hash: Option<String>,
    #[serde(default)]
    commit: Option<String>,
    #[serde(default)]
    config: Option<ManifestConfig>,
}

#[derive(Debug, Deserialize, Default)]
struct ManifestConfig {
    #[serde(default)]
    output: ManifestOutput,
}

#[derive(Debug, Deserialize, Default)]
struct ManifestOutput {
    #[serde(default)]
    run_directory: Option<String>,
}

/// Loads a named fixture from `fixtures/validation_vacua`.
pub fn load_fixture(name: &str) -> Result<FixtureState, AsmError> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/validation_vacua")
        .join(name);
    let code_path = root.join("end_state/code.json");
    let graph_path = root.join("end_state/graph.json");
    let manifest_path = root.join("manifest.json");
    let code_json = fs::read_to_string(&code_path).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("fixture-read", err.to_string())
                .with_context("path", code_path.display().to_string()),
        )
    })?;
    let graph_json = fs::read_to_string(&graph_path).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("fixture-read", err.to_string())
                .with_context("path", graph_path.display().to_string()),
        )
    })?;
    let manifest_json = fs::read_to_string(&manifest_path).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("fixture-read", err.to_string())
                .with_context("path", manifest_path.display().to_string()),
        )
    })?;

    let code = code_serde::from_json(&code_json)?;
    let graph = graph_from_json(&graph_json)?;
    let manifest: ManifestWrapper = serde_json::from_str(&manifest_json)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("manifest-parse", err.to_string())))?;

    let fixture_manifest = FixtureManifest {
        master_seed: manifest.master_seed,
        run_directory: manifest.config.and_then(|cfg| cfg.output.run_directory),
        commit: manifest.commit,
    };

    Ok(FixtureState {
        code,
        graph,
        manifest: fixture_manifest,
    })
}

/// Builds provenance information from a fixture manifest.
pub fn provenance_from_manifest(manifest: &FixtureManifest) -> ProvenanceInfo {
    ProvenanceInfo {
        seed: manifest.master_seed,
        run_id: manifest.run_directory.clone(),
        checkpoint_id: None,
        commit: manifest.commit.clone(),
    }
}
