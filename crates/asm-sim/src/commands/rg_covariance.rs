use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_mcmc::analysis;
use asm_mcmc::manifest::RunManifest;
use asm_rg::{covariance::covariance_check, serde_io, DictOpts, RGOpts, StateRef};
use clap::Args;

use crate::write_json;

#[derive(Args, Debug)]
pub struct RgCovarianceArgs {
    /// Input vacuum or run directory containing a manifest.
    #[arg(long)]
    pub input: PathBuf,
    /// Number of RG steps to execute when comparing flows.
    #[arg(long, default_value_t = 1)]
    pub steps: usize,
    /// Output directory for covariance artefacts.
    #[arg(long)]
    pub out: PathBuf,
    /// Deterministic RG scale factor.
    #[arg(long, default_value_t = 2)]
    pub scale: usize,
    /// Deterministic RG seed.
    #[arg(long, default_value_t = 0)]
    pub seed: u64,
    /// Number of Yukawa couplings to propagate through the dictionary.
    #[arg(long, default_value_t = 4)]
    pub yukawa: usize,
    /// Residual tolerance used during dictionary extraction.
    #[arg(long, default_value_t = 1e-6)]
    pub residual_tolerance: f64,
}

pub fn run(args: &RgCovarianceArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let manifest_path = args.input.join("manifest.json");
    let _manifest =
        RunManifest::load(&manifest_path).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    let (code, graph) =
        analysis::load_end_state(&args.input).map_err(|err| Box::new(err) as Box<dyn Error>)?;

    let rg_opts = RGOpts {
        scale_factor: args.scale.max(1),
        max_block_size: args.scale.max(1),
        seed: args.seed,
    };
    let dict_opts = DictOpts {
        yukawa_count: args.yukawa.max(1),
        seed: args.seed,
        residual_tolerance: args.residual_tolerance.max(0.0),
    };
    let state = StateRef {
        graph: &graph,
        code: &code,
    };
    let report = covariance_check(&state, args.steps, &rg_opts, &dict_opts)
        .map_err(|err| Box::new(err) as Box<dyn Error>)?;

    let json =
        serde_io::covariance_to_json(&report).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    fs::write(args.out.join("covariance.json"), json)?;

    let summary = serde_json::json!({
        "input": args.input.display().to_string(),
        "steps": args.steps,
        "scale_factor": rg_opts.scale_factor,
        "seed": rg_opts.seed,
        "pass": report.pass,
        "covariance_hash": report.covariance_hash,
    });
    write_json(args.out.join("summary.json"), &summary)?;

    Ok(())
}
