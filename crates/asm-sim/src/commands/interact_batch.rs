use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use asm_core::rng::derive_substream_seed;
use asm_gauge::from_json_slice as gauge_from_slice;
use asm_gauge::GaugeReport;
use asm_int::{
    interact_full, serde::to_canonical_json_bytes, FitOpts, KernelOpts, MeasureOpts, PrepSpec,
};
use asm_spec::from_json_slice as spec_from_slice;
use asm_spec::SpectrumReport;
use clap::Args;
use glob::glob;

#[derive(Args, Debug)]
pub struct InteractBatchArgs {
    /// Glob selecting all spectrum reports.
    #[arg(long = "spectra")]
    pub spectra_glob: String,
    /// Glob selecting gauge reports.
    #[arg(long = "gauge-glob")]
    pub gauge_glob: String,
    /// YAML configuration describing the preparation template.
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
    /// Master seed used to derive per-job substreams.
    #[arg(long)]
    pub seed: u64,
    /// Output directory for batch artefacts.
    #[arg(long)]
    pub out: PathBuf,
}

#[derive(serde::Serialize)]
struct BatchIndexEntry {
    id: String,
    spectrum: PathBuf,
    gauge: PathBuf,
    report: PathBuf,
    prep_hash: String,
    analysis_hash: String,
}

fn load_yaml<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let value = serde_yaml::from_slice(&bytes)?;
    Ok(value)
}

fn discover(glob_pattern: &str) -> Result<BTreeMap<String, PathBuf>, Box<dyn Error>> {
    let mut map = BTreeMap::new();
    for entry in glob(glob_pattern)? {
        let path = entry?;
        let id = key_for(&path);
        map.insert(id, path);
    }
    Ok(map)
}

fn key_for(path: &Path) -> String {
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .map(|value| value.to_string())
        .or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(|value| value.to_string())
        })
        .unwrap_or_else(|| "job".to_string())
}

fn load_reports(path: &Path) -> Result<SpectrumReport, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(spec_from_slice(&bytes)?)
}

fn load_gauge(path: &Path) -> Result<GaugeReport, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(gauge_from_slice(&bytes)?)
}

/// Executes a batch of deterministic interaction experiments.
pub fn run(args: &InteractBatchArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let spectra = discover(&args.spectra_glob)?;
    let gauges = discover(&args.gauge_glob)?;
    let prep_spec: PrepSpec = load_yaml(&args.prep)?;
    let kernel: KernelOpts = load_yaml(&args.kernel)?;
    let measure: MeasureOpts = load_yaml(&args.measure)?;
    let fit: FitOpts = load_yaml(&args.fit)?;

    let mut index = Vec::new();
    for (idx, (job_id, spectrum_path)) in spectra.iter().enumerate() {
        let Some(gauge_path) = gauges.get(job_id) else {
            return Err(format!(
                "missing gauge report matching spectrum '{}': {}",
                job_id,
                spectrum_path.display()
            )
            .into());
        };
        let spectrum = load_reports(spectrum_path)?;
        let gauge = load_gauge(gauge_path)?;
        let job_seed = derive_substream_seed(args.seed, idx as u64 + 1);
        let (prepared, trajectory, obs, couplings, report) = interact_full(
            &spectrum, &gauge, &prep_spec, &kernel, &measure, &fit, job_seed,
        )?;
        let job_dir = args.out.join(job_id);
        fs::create_dir_all(&job_dir)?;
        fs::write(
            job_dir.join("prepared_state.json"),
            to_canonical_json_bytes(&prepared)?,
        )?;
        if kernel.save_trajectory {
            fs::write(
                job_dir.join("trajectory.json"),
                to_canonical_json_bytes(&trajectory)?,
            )?;
        }
        fs::write(
            job_dir.join("observables.json"),
            to_canonical_json_bytes(&obs)?,
        )?;
        fs::write(
            job_dir.join("couplings_fit.json"),
            to_canonical_json_bytes(&couplings)?,
        )?;
        fs::write(
            job_dir.join("interaction_report.json"),
            to_canonical_json_bytes(&report)?,
        )?;
        index.push(BatchIndexEntry {
            id: job_id.clone(),
            spectrum: spectrum_path.clone(),
            gauge: gauge_path.clone(),
            report: job_dir.join("interaction_report.json"),
            prep_hash: prepared.prep_hash,
            analysis_hash: report.analysis_hash,
        });
    }

    fs::write(
        args.out.join("index.json"),
        to_canonical_json_bytes(&index)?,
    )?;

    Ok(())
}
