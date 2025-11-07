use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Args;
use serde::Serialize;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;
use zip::write::FileOptions;

#[derive(Args, Debug)]
pub struct PublishArgs {
    /// Root directory containing artefacts to bundle
    #[arg(long)]
    pub root: PathBuf,
    /// Output bundle path (.zip)
    #[arg(long)]
    pub out: PathBuf,
    /// Submitter identifier recorded in manifest
    #[arg(long)]
    pub submitter: String,
    /// Toolchain string recorded in manifest
    #[arg(long)]
    pub toolchain: String,
    /// Optional note stored in manifest
    #[arg(long)]
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
struct ManifestArtifact {
    kind: String,
    path: String,
    sha256: String,
}

#[derive(Debug, Serialize)]
struct SubmissionManifest {
    submitter: String,
    toolchain: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    artifacts: Vec<ManifestArtifact>,
    metrics: Vec<serde_json::Value>,
}

pub fn run(args: &PublishArgs) -> Result<(), Box<dyn Error>> {
    let mut artifacts = Vec::new();
    for entry in WalkDir::new(&args.root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let relative = path
            .strip_prefix(&args.root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = fs::read(path)?;
        let sha = hex::encode(Sha256::digest(&bytes));
        artifacts.push(ManifestArtifact {
            kind: detect_kind(path),
            path: relative,
            sha256: sha,
        });
    }
    artifacts.sort_by(|a, b| a.path.cmp(&b.path));
    let manifest = SubmissionManifest {
        submitter: args.submitter.clone(),
        toolchain: args.toolchain.clone(),
        notes: args.note.clone(),
        artifacts,
        metrics: Vec::new(),
    };
    write_bundle(&args.out, &manifest, &args.root)?;
    println!("bundle written to {}", args.out.display());
    Ok(())
}

fn detect_kind(path: &Path) -> String {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => "json".into(),
        Some("csv") => "csv".into(),
        Some("png") => "figure".into(),
        _ => "blob".into(),
    }
}

fn write_bundle(
    out: &Path,
    manifest: &SubmissionManifest,
    root: &Path,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(out)?;
    let mut writer = zip::ZipWriter::new(file);
    let options = FileOptions::default();
    writer.start_file("manifest.json", options)?;
    let manifest_bytes = serde_json::to_vec(manifest)?;
    writer.write_all(&manifest_bytes)?;
    for artifact in &manifest.artifacts {
        writer.start_file(&artifact.path, options)?;
        let data = fs::read(root.join(&artifact.path))?;
        writer.write_all(&data)?;
    }
    writer.finish()?;
    Ok(())
}
