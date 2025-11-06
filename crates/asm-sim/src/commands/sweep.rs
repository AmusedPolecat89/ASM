use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_exp::{sweep, to_canonical_json_bytes, SweepPlan, SweepReport};
use clap::Args;
use serde_yaml::from_str;

#[derive(Args, Debug)]
pub struct SweepArgs {
    #[arg(long)]
    pub plan: PathBuf,
    #[arg(long)]
    pub seed: u64,
    #[arg(long)]
    pub out: PathBuf,
}

pub fn run(args: &SweepArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let plan_text = fs::read_to_string(&args.plan)?;
    let plan: SweepPlan = from_str(&plan_text)?;
    let report = sweep(&plan, args.seed).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    persist_report(&args.out, &report)?;
    Ok(())
}

fn persist_report(out: &PathBuf, report: &SweepReport) -> Result<(), Box<dyn Error>> {
    let bytes = to_canonical_json_bytes(report).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    fs::write(out.join("sweep_report.json"), bytes)?;
    for job in &report.jobs {
        let job_dir = out.join(&job.out_dir);
        fs::create_dir_all(&job_dir)?;
        let params_bytes =
            to_canonical_json_bytes(&job.params).map_err(|err| Box::new(err) as Box<dyn Error>)?;
        fs::write(job_dir.join("params.json"), params_bytes)?;
        let status = format!("{}\nseed={}", job.status, job.seed);
        fs::write(job_dir.join("STATUS"), status)?;
    }
    Ok(())
}
