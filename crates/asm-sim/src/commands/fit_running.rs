use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_code::serde as code_serde;
use asm_gauge::from_json_slice as gauge_from_slice;
use asm_gauge::GaugeReport;
use asm_graph::{graph_from_json, HypergraphImpl};
use asm_int::{
    fit_running as fit_running_inner, serde::to_canonical_json_bytes, FitOpts, RunningOpts,
};
use asm_rg::StateRef;
use asm_spec::from_json_slice as spec_from_slice;
use asm_spec::SpectrumReport;
use clap::Args;

#[derive(Args, Debug)]
pub struct FitRunningArgs {
    /// Root directory containing RG steps with graph/code snapshots.
    #[arg(long = "rg-root")]
    pub rg_root: PathBuf,
    /// Spectrum report emitted during Phase 11.
    #[arg(long)]
    pub spectrum: PathBuf,
    /// Gauge report emitted during Phase 12.
    #[arg(long)]
    pub gauge: PathBuf,
    /// YAML configuration describing the per-step coupling fit.
    #[arg(long)]
    pub fit: PathBuf,
    /// Optional YAML configuration overriding running thresholds.
    #[arg(long)]
    pub running: Option<PathBuf>,
    /// Output directory for the running report.
    #[arg(long)]
    pub out: PathBuf,
}

fn load_yaml<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(serde_yaml::from_slice(&bytes)?)
}

fn load_spectrum(path: &PathBuf) -> Result<SpectrumReport, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(spec_from_slice(&bytes)?)
}

fn load_gauge(path: &PathBuf) -> Result<GaugeReport, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(gauge_from_slice(&bytes)?)
}

fn load_rg_states(
    root: &PathBuf,
) -> Result<(Vec<HypergraphImpl>, Vec<asm_code::CSSCode>), Box<dyn Error>> {
    let mut graphs = Vec::new();
    let mut codes = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let step_dir = entry.path();
        let graph_path = step_dir.join("graph.json");
        let code_path = step_dir.join("code.json");
        if !graph_path.exists() || !code_path.exists() {
            continue;
        }
        let graph_json = fs::read_to_string(graph_path)?;
        let code_json = fs::read_to_string(code_path)?;
        graphs.push(graph_from_json(&graph_json)?);
        codes.push(code_serde::from_json(&code_json)?);
    }
    Ok((graphs, codes))
}

fn validate_hashes(spec: &SpectrumReport, gauge: &GaugeReport) -> Result<(), Box<dyn Error>> {
    if spec.graph_hash != gauge.graph_hash || spec.code_hash != gauge.code_hash {
        return Err("spectrum and gauge reports refer to different states".into());
    }
    Ok(())
}

/// Fits running couplings across a deterministic RG chain.
pub fn run(args: &FitRunningArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let spectrum = load_spectrum(&args.spectrum)?;
    let gauge = load_gauge(&args.gauge)?;
    validate_hashes(&spectrum, &gauge)?;

    let fit_opts: FitOpts = load_yaml(&args.fit)?;
    let mut running_opts = if let Some(path) = &args.running {
        load_yaml::<RunningOpts>(path)?
    } else {
        RunningOpts::from_fit(fit_opts.clone())
    };
    running_opts.fit = fit_opts.clone();

    let (graphs, codes) = load_rg_states(&args.rg_root)?;
    if graphs.is_empty() || codes.is_empty() {
        return Err("rg-root does not contain any step directories".into());
    }
    let mut states = Vec::new();
    for (graph, code) in graphs.iter().zip(codes.iter()) {
        states.push(StateRef { graph, code });
    }

    let report = fit_running_inner(&states, &running_opts)?;
    fs::write(
        args.out.join("running_report.json"),
        to_canonical_json_bytes(&report)?,
    )?;

    Ok(())
}
