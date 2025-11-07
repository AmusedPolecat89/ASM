use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_thy::bundle::{build_manuscript_bundle, BundlePlan};
use clap::Args;

#[derive(Args, Debug)]
pub struct PaperPackArgs {
    /// Root directories to scan for artefacts (comma separated).
    #[arg(long, value_delimiter = ',', num_args = 1..)]
    pub roots: Vec<PathBuf>,
    /// YAML plan describing what to include in the bundle.
    #[arg(long)]
    pub plan: PathBuf,
    /// Output directory for the manuscript bundle.
    #[arg(long)]
    pub out: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser, Debug)]
    struct Wrapper {
        #[command(flatten)]
        args: PaperPackArgs,
    }

    #[test]
    fn parses_multiple_roots_from_single_flag() {
        let wrapper = Wrapper::parse_from([
            "test-bin",
            "--roots",
            "first",
            "second",
            "--plan",
            "plan.yaml",
            "--out",
            "out-dir",
        ]);

        assert_eq!(
            wrapper.args.roots,
            vec![PathBuf::from("first"), PathBuf::from("second")]
        );
    }
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
