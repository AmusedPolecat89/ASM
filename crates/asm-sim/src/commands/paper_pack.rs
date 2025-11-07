use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_thy::bundle::{build_manuscript_bundle, BundlePlan};
use clap::Args;

#[derive(Args, Debug)]
pub struct PaperPackArgs {
    /// Root directories to scan for artefacts (comma separated).
    #[arg(long, value_delimiter = ',')]
    pub roots: Vec<PathBuf>,
    /// YAML plan describing what to include in the bundle.
    #[arg(long)]
    pub plan: PathBuf,
    /// Output directory for the manuscript bundle.
    #[arg(long)]
    pub out: PathBuf,
}

fn load_plan(path: &PathBuf) -> Result<BundlePlan, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(serde_yaml::from_slice(&bytes)?)
}

/// Builds the manuscript bundle under the requested output directory.
pub fn run(args: &PaperPackArgs) -> Result<(), Box<dyn Error>> {
    let plan = load_plan(&args.plan)?;
    build_manuscript_bundle(&args.roots, &args.out, &plan)?;
    Ok(())
}
