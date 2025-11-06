use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_exp::{
    registry_append, run_ablation, to_canonical_json_bytes, AblationPlan, AblationReport, Registry,
};
use clap::Args;
use serde_yaml::from_str;

#[derive(Args, Debug)]
pub struct AblationArgs {
    /// Path to the ablation plan YAML.
    #[arg(long)]
    pub plan: PathBuf,
    /// Seed controlling deterministic sampling.
    #[arg(long, default_value_t = 9001)]
    pub seed: u64,
    /// Output directory for ablation artifacts.
    #[arg(long)]
    pub out: PathBuf,
    /// Optional registry path to append ablation rows.
    #[arg(long)]
    pub registry: Option<PathBuf>,
}

pub fn run(args: &AblationArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let plan_text = fs::read_to_string(&args.plan)?;
    let plan: AblationPlan = from_str(&plan_text)?;
    let report = run_ablation(&plan, args.seed).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    persist_report(&args.out, &report)?;
    if let Some(path) = &args.registry {
        let registry = Registry::from_path(path);
        registry_append(&registry, &report).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    }
    Ok(())
}

fn persist_report(out: &PathBuf, report: &AblationReport) -> Result<(), Box<dyn Error>> {
    let bytes = to_canonical_json_bytes(report).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    fs::write(out.join("ablation_report.json"), &bytes)?;
    let summary_bytes =
        to_canonical_json_bytes(&report.summary).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    fs::write(out.join("summary.json"), summary_bytes)?;
    for (idx, job) in report.jobs.iter().enumerate() {
        let job_dir = out.join(format!("job_{:04}", idx));
        fs::create_dir_all(&job_dir)?;
        let params_bytes =
            to_canonical_json_bytes(&job.params).map_err(|err| Box::new(err) as Box<dyn Error>)?;
        fs::write(job_dir.join("params.json"), params_bytes)?;
        let metrics_bytes =
            to_canonical_json_bytes(&job.metrics).map_err(|err| Box::new(err) as Box<dyn Error>)?;
        fs::write(job_dir.join("metrics.json"), metrics_bytes)?;
        fs::write(job_dir.join("SEED"), format!("{}\n", job.seed))?;
    }
    Ok(())
}
