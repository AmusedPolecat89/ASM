use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_code::serde as code_serde;
use asm_mcmc::analysis;
use asm_mcmc::manifest::RunManifest;
use asm_rg::{rg_run, serde_io, RGOpts, StateRef};
use clap::Args;

use crate::write_json;

#[derive(Args, Debug)]
pub struct RgArgs {
    /// Input vacuum or run directory containing a manifest.
    #[arg(long)]
    pub input: PathBuf,
    /// Number of RG steps to execute.
    #[arg(long, default_value_t = 1)]
    pub steps: usize,
    /// Output directory where RG artefacts will be written.
    #[arg(long)]
    pub out: PathBuf,
    /// Deterministic scale factor applied at each step.
    #[arg(long, default_value_t = 2)]
    pub scale: usize,
    /// Seed controlling deterministic block ordering.
    #[arg(long, default_value_t = 0)]
    pub seed: u64,
}

pub fn run(args: &RgArgs) -> Result<(), Box<dyn Error>> {
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
    let state = StateRef {
        graph: &graph,
        code: &code,
    };
    let run =
        rg_run(&state, args.steps, &rg_opts).map_err(|err| Box::new(err) as Box<dyn Error>)?;

    let run_json =
        serde_io::run_to_json(&run.report).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    fs::write(args.out.join("rg_run.json"), run_json)?;

    for (index, step) in run.steps.iter().enumerate() {
        let step_dir = args.out.join(format!("step_{index:03}"));
        fs::create_dir_all(&step_dir)?;
        let step_json =
            serde_io::step_to_json(&step.report).map_err(|err| Box::new(err) as Box<dyn Error>)?;
        fs::write(step_dir.join("report.json"), step_json)?;
        let graph_json =
            asm_graph::graph_to_json(&step.graph).map_err(|err| Box::new(err) as Box<dyn Error>)?;
        fs::write(step_dir.join("graph.json"), graph_json)?;
        let code_json =
            code_serde::to_json(&step.code).map_err(|err| Box::new(err) as Box<dyn Error>)?;
        fs::write(step_dir.join("code.json"), code_json)?;
    }

    let summary = serde_json::json!({
        "input": args.input.display().to_string(),
        "steps": args.steps,
        "scale_factor": rg_opts.scale_factor,
        "seed": rg_opts.seed,
        "run_hash": run.report.run_hash,
    });
    write_json(args.out.join("summary.json"), &summary)?;

    Ok(())
}
