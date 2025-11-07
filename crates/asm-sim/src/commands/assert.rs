use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_gauge::{from_json_slice as gauge_from_slice, GaugeReport};
use asm_int::report::InteractionReport;
use asm_int::running::RunningReport;
use asm_int::serde::from_json_slice as int_from_slice;
use asm_land::report::SummaryReport;
use asm_land::serde::from_json_slice as land_from_slice;
use asm_spec::{from_json_slice as spec_from_slice, SpectrumReport};
use asm_thy::{run_assertions, serde::to_canonical_json_bytes, AssertionInputs, Policy};
use clap::Args;

#[derive(Args, Debug)]
pub struct AssertArgs {
    /// Spectrum report emitted during Phase 11.
    #[arg(long)]
    pub spectrum: PathBuf,
    /// Gauge report emitted during Phase 12.
    #[arg(long)]
    pub gauge: PathBuf,
    /// Interaction report emitted during Phase 13.
    #[arg(long = "interact")]
    pub interaction: PathBuf,
    /// Optional running report emitted during Phase 13.
    #[arg(long)]
    pub running: Option<PathBuf>,
    /// Optional summary report emitted during Phase 14.
    #[arg(long)]
    pub summary: Option<PathBuf>,
    /// Policy YAML describing tolerances.
    #[arg(long)]
    pub policy: PathBuf,
    /// Output directory where assertion artefacts will be written.
    #[arg(long)]
    pub out: PathBuf,
}

fn load_policy(path: &PathBuf) -> Result<Policy, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let policy: Policy = serde_yaml::from_slice(&bytes)?;
    Ok(policy)
}

fn load_summary(path: &Option<PathBuf>) -> Result<Option<SummaryReport>, Box<dyn Error>> {
    if let Some(path) = path {
        let bytes = fs::read(path)?;
        let summary = land_from_slice(&bytes)?;
        Ok(Some(summary))
    } else {
        Ok(None)
    }
}

/// Executes a single assertion run on a vacuum and persists the report.
pub fn run(args: &AssertArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let spectrum_bytes = fs::read(&args.spectrum)?;
    let gauge_bytes = fs::read(&args.gauge)?;
    let interaction_bytes = fs::read(&args.interaction)?;
    let spectrum: SpectrumReport = spec_from_slice(&spectrum_bytes)?;
    let gauge: GaugeReport = gauge_from_slice(&gauge_bytes)?;
    let interaction: InteractionReport = int_from_slice(&interaction_bytes)?;
    let running: Option<RunningReport> = if let Some(path) = &args.running {
        let bytes = fs::read(path)?;
        Some(int_from_slice(&bytes)?)
    } else {
        None
    };
    let summary = load_summary(&args.summary)?;
    let policy = load_policy(&args.policy)?;

    let mut inputs = AssertionInputs::default();
    inputs.spectrum = Some(spectrum);
    inputs.gauge = Some(gauge);
    inputs.interaction = Some(interaction);
    inputs.running = running;
    inputs.summary = summary;

    let report = run_assertions(&inputs, &policy)?;
    fs::write(
        args.out.join("assert_report.json"),
        to_canonical_json_bytes(&report)?,
    )?;
    Ok(())
}
