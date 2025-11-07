use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use asm_dsr::query::RegistryQuery;
use asm_dsr::QueryParams;
use clap::Args;
use rusqlite::Connection;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

#[derive(Args, Debug)]
pub struct VerifyArgs {
    /// Bundle to verify
    #[arg(long)]
    pub bundle: PathBuf,
    /// Optional registry path used for cross checking hashes
    #[arg(long)]
    pub registry: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct SubmissionManifest {
    artifacts: Vec<ManifestArtifact>,
}

#[derive(Debug, Deserialize)]
struct ManifestArtifact {
    path: String,
    sha256: String,
}

pub fn run(args: &VerifyArgs) -> Result<(), Box<dyn Error>> {
    let file = File::open(&args.bundle)?;
    let mut archive = ZipArchive::new(file)?;
    let manifest_bytes = read_entry(&mut archive, "manifest.json")?;
    let manifest: SubmissionManifest = serde_json::from_slice(&manifest_bytes)?;
    for artifact in &manifest.artifacts {
        let bytes = read_entry(&mut archive, &artifact.path)?;
        let hash = hex::encode(Sha256::digest(&bytes));
        if hash != artifact.sha256 {
            return Err(format!(
                "artifact {} hash mismatch: expected {} got {}",
                artifact.path, artifact.sha256, hash
            )
            .into());
        }
    }
    if let Some(registry) = &args.registry {
        let conn = Connection::open(registry)?;
        let query = RegistryQuery::execute(&conn, &QueryParams::default())?;
        for artifact in &manifest.artifacts {
            let exists = query
                .artifacts
                .iter()
                .any(|record| record.sha256 == artifact.sha256);
            if !exists {
                return Err(format!("artifact {} missing from registry", artifact.path).into());
            }
        }
    }
    println!("bundle verified successfully");
    Ok(())
}

fn read_entry<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut file = archive.by_name(name)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}
