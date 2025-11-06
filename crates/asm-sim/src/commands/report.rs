use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use asm_exp::{build_runbook, to_canonical_json_bytes, RunMeta};
use clap::Args;
use serde_json::json;
use serde_yaml::from_str;

#[derive(Args, Debug)]
pub struct ReportArgs {
    #[arg(long = "inputs", value_name = "PATH")]
    pub inputs: Vec<PathBuf>,
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long)]
    pub meta: Option<PathBuf>,
}

pub fn run(args: &ReportArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let meta = load_meta(args.meta.as_ref())?;
    let runbook =
        build_runbook(&args.inputs, &meta).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    let json = to_canonical_json_bytes(&runbook).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    fs::write(args.out.join("runbook.json"), json)?;
    write_summary(&args.out, &runbook)?;
    write_markdown(&args.out, &runbook)?;
    Ok(())
}

fn load_meta(path: Option<&PathBuf>) -> Result<RunMeta, Box<dyn Error>> {
    if let Some(path) = path {
        let raw = fs::read_to_string(path)?;
        let meta: RunMeta = from_str(&raw)?;
        Ok(meta)
    } else {
        Ok(RunMeta {
            created_at: "1970-01-01T00:00:00Z".to_string(),
            commit: "unknown".to_string(),
            seeds: Vec::new(),
            artifacts: Vec::new(),
            summary: json!({"notes": "auto-generated"}),
        })
    }
}

fn write_summary(out: &PathBuf, runbook: &asm_exp::RunBook) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out.join("summary.csv"))?;
    writeln!(file, "input")?;
    for input in &runbook.inputs {
        writeln!(file, "{input}")?;
    }
    Ok(())
}

fn write_markdown(out: &PathBuf, runbook: &asm_exp::RunBook) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out.join("report.md"))?;
    writeln!(file, "# Experiment RunBook")?;
    writeln!(file, "\n- ID: {}", runbook.id)?;
    writeln!(file, "- Commit: {}", runbook.commit)?;
    writeln!(file, "- Created: {}", runbook.created_at)?;
    writeln!(file, "- Inputs: {}", runbook.inputs.len())?;
    Ok(())
}
