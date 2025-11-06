use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_exp::{estimate_gaps, to_canonical_json_bytes, GapMethod, GapOpts};
use asm_rg::StateRef;
use clap::Args;
use serde_json::Value;
use serde_yaml::from_str;

use super::deform::load_state;

#[derive(Args, Debug)]
pub struct GapsArgs {
    #[arg(long)]
    pub input: PathBuf,
    #[arg(long)]
    pub method: String,
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long, default_value_t = 1e-3)]
    pub tolerance: f64,
    #[arg(long)]
    pub thresholds: Option<PathBuf>,
}

pub fn run(args: &GapsArgs) -> Result<(), Box<dyn Error>> {
    if !args.out.exists() {
        if let Some(parent) = args.out.parent() {
            fs::create_dir_all(parent)?;
        }
    }
    let loaded = load_state(&args.input)?;
    let state_ref = StateRef {
        graph: &loaded.graph,
        code: &loaded.code,
    };
    let thresholds = if let Some(path) = &args.thresholds {
        let raw = fs::read_to_string(path)?;
        from_str::<Value>(&raw)?
    } else {
        Value::Null
    };
    let method = match args.method.as_str() {
        "dispersion" => GapMethod::Dispersion,
        "spectral" => GapMethod::Spectral,
        other => {
            return Err(format!("unsupported gap method: {other}").into());
        }
    };
    let opts = GapOpts {
        method,
        thresholds,
        tolerance: args.tolerance,
    };
    let report = estimate_gaps(&state_ref, &opts).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    let json = to_canonical_json_bytes(&report).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    fs::write(&args.out, json)?;
    Ok(())
}
