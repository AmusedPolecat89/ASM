use std::error::Error;
use std::path::PathBuf;

use asm_dsr::{ingest_bundle, init_schema, IngestOptions};
use clap::Args;
use rusqlite::Connection;

#[derive(Args, Debug)]
pub struct SubmitArgs {
    /// Path to the dataset bundle created with `asm-sim publish bundle`
    #[arg(long)]
    pub bundle: PathBuf,
    /// SQLite registry path
    #[arg(long)]
    pub registry: PathBuf,
}

pub fn run(args: &SubmitArgs) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = args.registry.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&args.registry)?;
    init_schema(&conn)?;
    let artifacts_dir = args.registry.with_extension("artifacts");
    std::fs::create_dir_all(&artifacts_dir)?;
    let opts = IngestOptions {
        artifact_root: artifacts_dir,
        validate_hashes: true,
    };
    let record = ingest_bundle(&conn, &args.bundle, &opts)?;
    println!("ingested submission {}", record.id);
    Ok(())
}
