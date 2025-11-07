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
pub struct InteractArgs {
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
    /// YAML configuration describing measurements.
    #[arg(long)]
    pub measure: PathBuf,
    /// YAML configuration describing coupling fits.
    #[arg(long)]
    pub fit: PathBuf,
    /// Deterministic seed controlling participant selection.
    #[arg(long)]
    pub seed: u64,
    /// Output directory where artefacts will be stored.
    #[arg(long)]
    pub out: PathBuf,
}

fn load_yaml<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let value = serde_yaml::from_slice(&bytes)?;
    Ok(value)
}

fn load_reports(args: &InteractArgs) -> Result<(SpectrumReport, GaugeReport), Box<dyn Error>> {
    let spectrum_bytes = fs::read(&args.spectrum)?;
    let gauge_bytes = fs::read(&args.gauge)?;
    let spectrum = spec_from_slice(&spectrum_bytes)?;
    let gauge = gauge_from_slice(&gauge_bytes)?;
    Ok((spectrum, gauge))
}

/// Executes a single deterministic interaction experiment and persists canonical JSON artefacts.
pub fn run(args: &InteractArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let (spectrum, gauge) = load_reports(args)?;
    let prep_spec: PrepSpec = load_yaml(&args.prep)?;
    let kernel: KernelOpts = load_yaml(&args.kernel)?;
    let measure: MeasureOpts = load_yaml(&args.measure)?;
    let fit: FitOpts = load_yaml(&args.fit)?;

    let (prepared, trajectory, obs, couplings, report) = interact_full(
        &spectrum, &gauge, &prep_spec, &kernel, &measure, &fit, args.seed,
    )?;

    fs::write(
        args.out.join("prepared_state.json"),
        to_canonical_json_bytes(&prepared)?,
    )?;

    if kernel.save_trajectory {
        fs::write(
            args.out.join("trajectory.json"),
            to_canonical_json_bytes(&trajectory)?,
        )?;
    }

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
