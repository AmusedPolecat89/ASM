use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_core::rng::derive_substream_seed;
use asm_spec::{
    analyze_spectrum, to_canonical_json_bytes, CorrelSpec, DispersionSpec, ExcitationSpec, OpOpts,
    OpsVariant, PropOpts, SpecOpts,
};
use clap::Args;

use super::deform::load_state;

fn parse_ops_variant(value: &str) -> Result<OpsVariant, Box<dyn Error>> {
    match value {
        "default" => Ok(OpsVariant::Default),
        "alt" => Ok(OpsVariant::Alt),
        other => Err(format!("unknown ops variant '{other}'").into()),
    }
}

#[derive(Args, Debug)]
pub struct SpectrumArgs {
    /// Input directory containing graph/code JSON or a run manifest.
    #[arg(long)]
    pub input: PathBuf,
    /// Output directory for spectrum artefacts.
    #[arg(long)]
    pub out: PathBuf,
    /// Number of k-points to use during the dispersion scan.
    #[arg(long, default_value_t = 64)]
    pub k_points: usize,
    /// Number of modes to retain in the dispersion report.
    #[arg(long, default_value_t = 3)]
    pub modes: usize,
    /// Master deterministic seed.
    #[arg(long)]
    pub seed: u64,
    /// Operator variant: "default" or "alt".
    #[arg(long, default_value = "default")]
    pub ops_variant: String,
    /// Fit tolerance recorded in the provenance payload.
    #[arg(long, default_value_t = 1e-6)]
    pub fit_tol: f64,
    /// Support size for the seeded excitation.
    #[arg(long, default_value_t = 3)]
    pub support: usize,
    /// Number of propagation iterations to perform.
    #[arg(long, default_value_t = 16)]
    pub iterations: usize,
}

pub fn run(args: &SpectrumArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let loaded = load_state(&args.input)?;

    let variant = parse_ops_variant(&args.ops_variant)?;
    let mut excitation = ExcitationSpec::default();
    excitation.support = args.support.max(1);

    let mut dispersion = DispersionSpec::default();
    dispersion.k_points = args.k_points.max(1);
    dispersion.modes = args.modes.max(1);

    let correlation = CorrelSpec::default();

    let prop_opts = PropOpts {
        iterations: args.iterations.max(1),
        tolerance: args.fit_tol,
        seed: derive_substream_seed(args.seed, 0),
    };

    let spec_opts = SpecOpts {
        ops: OpOpts { variant },
        excitation,
        propagation: prop_opts,
        dispersion,
        correlation,
        master_seed: args.seed,
        fit_tolerance: args.fit_tol,
    };

    let report = analyze_spectrum(&loaded.graph, &loaded.code, &spec_opts)?;

    fs::write(
        args.out.join("operators.json"),
        to_canonical_json_bytes(&report.operators)?,
    )?;
    fs::write(
        args.out.join("dispersion.json"),
        to_canonical_json_bytes(&report.dispersion)?,
    )?;
    fs::write(
        args.out.join("correlation.json"),
        to_canonical_json_bytes(&report.correlation)?,
    )?;
    fs::write(
        args.out.join("spectrum_report.json"),
        to_canonical_json_bytes(&report)?,
    )?;

    Ok(())
}
