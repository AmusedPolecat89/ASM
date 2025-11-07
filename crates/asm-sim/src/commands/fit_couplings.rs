use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_gauge::from_json_slice as gauge_from_slice;
use asm_gauge::GaugeReport;
use asm_int::{
    interact_full, serde::to_canonical_json_bytes, FitOpts, KernelOpts, MeasureOpts, PrepSpec,
};
use asm_spec::from_json_slice as spec_from_slice;
use asm_spec::SpectrumReport;
use clap::Args;

#[derive(Args, Debug)]
pub struct FitCouplingsArgs {
    /// Spectrum report emitted during Phase 11.
    #[arg(long)]
    pub spectrum: PathBuf,
    /// Gauge report emitted during Phase 12.
    #[arg(long)]
    pub gauge: PathBuf,
    /// YAML configuration describing the preparation.
    #[arg(long)]
    pub prep: PathBuf,
    /// YAML configuration describing the kernel.
    #[arg(long)]
    pub kernel: PathBuf,
    /// YAML configuration describing the measurement procedure.
    #[arg(long)]
    pub measure: PathBuf,
    /// YAML configuration describing the coupling fit.
    #[arg(long)]
    pub fit: PathBuf,
    /// Deterministic seed controlling the preparation.
    #[arg(long)]
    pub seed: u64,
    /// Output directory for the couplings fit artefacts.
    #[arg(long)]
    pub out: PathBuf,
}

fn load_yaml<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(serde_yaml::from_slice(&bytes)?)
}

fn load_reports(path: &PathBuf) -> Result<SpectrumReport, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(spec_from_slice(&bytes)?)
}

fn load_gauge(path: &PathBuf) -> Result<GaugeReport, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(gauge_from_slice(&bytes)?)
}

/// Fits couplings for a single state without persisting the trajectory payload.
pub fn run(args: &FitCouplingsArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let spectrum = load_reports(&args.spectrum)?;
    let gauge = load_gauge(&args.gauge)?;
    let prep_spec: PrepSpec = load_yaml(&args.prep)?;
    let kernel: KernelOpts = load_yaml(&args.kernel)?;
    let measure: MeasureOpts = load_yaml(&args.measure)?;
    let fit: FitOpts = load_yaml(&args.fit)?;

    let (prepared, _trajectory, obs, couplings, report) = interact_full(
        &spectrum, &gauge, &prep_spec, &kernel, &measure, &fit, args.seed,
    )?;

    fs::write(
        args.out.join("prepared_state.json"),
        to_canonical_json_bytes(&prepared)?,
    )?;
    fs::write(
        args.out.join("observables.json"),
        to_canonical_json_bytes(&obs)?,
    )?;
    fs::write(
        args.out.join("couplings_fit.json"),
        to_canonical_json_bytes(&couplings)?,
    )?;
    fs::write(
        args.out.join("interaction_report.json"),
        to_canonical_json_bytes(&report)?,
    )?;

    Ok(())
}
