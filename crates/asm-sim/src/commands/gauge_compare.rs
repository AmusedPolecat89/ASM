use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_gauge::{from_json_slice as gauge_from_slice, to_canonical_json_bytes, GaugeReport};
use clap::Args;
use serde::Serialize;

#[derive(Args, Debug)]
pub struct GaugeCompareArgs {
    /// First gauge report to compare.
    #[arg(long = "a")]
    pub report_a: PathBuf,
    /// Second gauge report to compare.
    #[arg(long = "b")]
    pub report_b: PathBuf,
    /// Output directory for the diff artefact.
    #[arg(long)]
    pub out: PathBuf,
}

#[derive(Debug, Serialize)]
struct SectionDelta {
    same: bool,
    value_a: f64,
    value_b: f64,
    delta: f64,
}

#[derive(Debug, Serialize)]
struct GaugeDiff {
    rep_hash_equal: bool,
    closure: SectionDelta,
    decomp: SectionDelta,
    ward: SectionDelta,
}

fn load_report(path: &PathBuf) -> Result<GaugeReport, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(gauge_from_slice(&bytes)?)
}

fn section(value_a: f64, value_b: f64) -> SectionDelta {
    let delta = (value_a - value_b).abs();
    SectionDelta {
        same: delta <= 1e-9,
        value_a,
        value_b,
        delta,
    }
}

pub fn run(args: &GaugeCompareArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let report_a = load_report(&args.report_a)?;
    let report_b = load_report(&args.report_b)?;

    let diff = GaugeDiff {
        rep_hash_equal: report_a.rep_hash == report_b.rep_hash,
        closure: section(report_a.closure.max_dev, report_b.closure.max_dev),
        decomp: section(report_a.decomp.residual_norm, report_b.decomp.residual_norm),
        ward: section(report_a.ward.max_comm_norm, report_b.ward.max_comm_norm),
    };

    fs::write(args.out.join("diff.json"), to_canonical_json_bytes(&diff)?)?;

    Ok(())
}
