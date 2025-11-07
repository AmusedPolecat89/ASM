use std::fs::{self, File};
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use asm_core::errors::{AsmError, ErrorInfo};
use rusqlite::Connection;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::schema::{
    init_schema, insert_artifact, insert_metric, insert_submission, load_submissions,
    SubmissionRecord,
};

#[derive(Debug, Clone)]
pub struct IngestOptions {
    pub artifact_root: PathBuf,
    pub validate_hashes: bool,
}

impl IngestOptions {
    pub fn new(artifact_root: impl Into<PathBuf>) -> Self {
        Self {
            artifact_root: artifact_root.into(),
            validate_hashes: true,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ManifestArtifact {
    kind: String,
    path: String,
    sha256: String,
    #[serde(default)]
    analysis_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ManifestMetric {
    name: String,
    value: f64,
    #[serde(default)]
    unit: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubmissionManifest {
    submitter: String,
    toolchain: String,
    #[serde(default)]
    notes: Option<String>,
    artifacts: Vec<ManifestArtifact>,
    #[serde(default)]
    metrics: Vec<ManifestMetric>,
}

fn registry_error(code: &str, err: impl ToString) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, err.to_string()))
}

pub fn ingest_bundle(
    conn: &Connection,
    bundle_path: &Path,
    opts: &IngestOptions,
) -> Result<SubmissionRecord, AsmError> {
    init_schema(conn)?;
    let file = File::open(bundle_path).map_err(|err| {
        registry_error(
            "asm_dsr.bundle_open",
            format!("failed to open {}: {err}", bundle_path.display()),
        )
    })?;
    let mut archive = ZipArchive::new(file)
        .map_err(|err| registry_error("asm_dsr.bundle_parse", err.to_string()))?;
    let manifest_bytes = read_entry(&mut archive, "manifest.json")?;
    let manifest: SubmissionManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| registry_error("asm_dsr.bundle_manifest", err.to_string()))?;
    let submission_id = insert_submission(
        conn,
        &manifest.submitter,
        &manifest.toolchain,
        manifest.notes.as_deref(),
    )?;
    let submission_dir = opts
        .artifact_root
        .join(format!("submission_{submission_id}"));
    fs::create_dir_all(&submission_dir).map_err(|err| {
        registry_error(
            "asm_dsr.artifact_dir",
            format!("failed to create {}: {err}", submission_dir.display()),
        )
    })?;
    for artifact in &manifest.artifacts {
        let bytes = read_entry(&mut archive, &artifact.path)?;
        let hash = hex::encode(Sha256::digest(&bytes));
        if opts.validate_hashes && hash != artifact.sha256 {
            return Err(registry_error(
                "asm_dsr.hash_mismatch",
                format!(
                    "artifact {} expected {} got {}",
                    artifact.path, artifact.sha256, hash
                ),
            ));
        }
        let out_path = submission_dir.join(&artifact.path);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                registry_error(
                    "asm_dsr.artifact_dir",
                    format!("failed to create {}: {err}", parent.display()),
                )
            })?;
        }
        fs::write(&out_path, &bytes).map_err(|err| {
            registry_error(
                "asm_dsr.artifact_write",
                format!("failed to write {}: {err}", out_path.display()),
            )
        })?;
        insert_artifact(
            conn,
            submission_id,
            &artifact.kind,
            &artifact.path,
            &artifact.sha256,
            artifact.analysis_hash.as_deref(),
        )?;
    }
    for metric in &manifest.metrics {
        insert_metric(
            conn,
            submission_id,
            &metric.name,
            metric.value,
            metric.unit.as_deref(),
        )?;
    }
    let submissions = load_submissions(conn)?;
    submissions
        .into_iter()
        .find(|record| record.id == submission_id)
        .ok_or_else(|| registry_error("asm_dsr.lookup", "new submission missing"))
}

fn read_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<Vec<u8>, AsmError> {
    let mut file = archive
        .by_name(name)
        .map_err(|err| registry_error("asm_dsr.bundle_entry", format!("{name}: {err}")))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|err| registry_error("asm_dsr.bundle_read", err.to_string()))?;
    Ok(bytes)
}
