use std::error::Error;
use std::process::Command;

use asm_exp::to_canonical_json_bytes;
use clap::Args;
use serde::Serialize;

#[derive(Args, Debug)]
pub struct VersionArgs {
    /// Emit extended metadata including git and toolchain information.
    #[arg(long)]
    pub long: bool,
}

#[derive(Debug, Serialize)]
struct VersionInfo {
    version: String,
    git_commit: String,
    rustc: String,
    features: Vec<String>,
}

pub fn run(args: &VersionArgs) -> Result<(), Box<dyn Error>> {
    if !args.long {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let info = gather_info()?;
    let json = to_canonical_json_bytes(&info).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    println!("{}", String::from_utf8(json)?);
    Ok(())
}

fn gather_info() -> Result<VersionInfo, Box<dyn Error>> {
    let git_commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".into());
    let rustc = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "rustc unavailable".into());
    let mut features = Vec::new();
    if option_env!("CARGO_FEATURE_SIMD").is_some() {
        features.push("simd".into());
    }
    if features.is_empty() {
        features.push("default".into());
    }
    Ok(VersionInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        git_commit,
        rustc,
        features,
    })
}
