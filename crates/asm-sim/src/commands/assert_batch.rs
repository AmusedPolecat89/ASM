use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use asm_gauge::{from_json_slice as gauge_from_slice, GaugeReport};
use asm_int::report::InteractionReport;
use asm_int::running::RunningReport;
use asm_int::serde::from_json_slice as int_from_slice;
use asm_land::metrics::JobKpi;
use asm_land::report::SummaryReport;
use asm_land::serde::from_json_slice as land_from_slice;
use asm_spec::{from_json_slice as spec_from_slice, SpectrumReport};
use asm_thy::{run_assertions, serde::to_canonical_json_bytes, AssertionInputs, Policy};
use clap::Args;
use serde::Serialize;

#[derive(Args, Debug)]
pub struct AssertBatchArgs {
    /// Root directory containing landscape run outputs.
    #[arg(long)]
    pub root: PathBuf,
    /// Policy YAML describing tolerances.
    #[arg(long)]
    pub policy: PathBuf,
    /// Output directory for assertion reports.
    #[arg(long)]
    pub out: PathBuf,
}

fn load_policy(path: &PathBuf) -> Result<Policy, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(serde_yaml::from_slice(&bytes)?)
}

fn load_summary(root: &Path) -> Option<SummaryReport> {
    let candidate = root.join("summary").join("SummaryReport.json");
    if candidate.exists() {
        fs::read(&candidate)
            .ok()
            .and_then(|bytes| land_from_slice(&bytes).ok())
    } else {
        None
    }
}

fn load_kpi(path: &Path) -> Option<JobKpi> {
    fs::read(path)
        .ok()
        .and_then(|bytes| land_from_slice::<JobKpi>(&bytes).ok())
}

#[derive(Serialize)]
struct BatchIndexEntry {
    job: String,
    report: String,
}

/// Executes assertions for every job in a landscape run.
pub fn run(args: &AssertBatchArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let policy = load_policy(&args.policy)?;
    let summary = load_summary(&args.root);
    let mut entries = Vec::new();

    let mut job_dirs: Vec<PathBuf> = fs::read_dir(&args.root)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .map(|entry| entry.path())
        .filter(|path| path.join("spectrum/spectrum_report.json").exists())
        .collect();
    job_dirs.sort();

    for job_dir in job_dirs {
        let job_name = job_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("job")
            .to_string();
        let spectrum_bytes = fs::read(job_dir.join("spectrum/spectrum_report.json"))?;
        let gauge_bytes = fs::read(job_dir.join("gauge/gauge_report.json"))?;
        let interact_bytes = fs::read(job_dir.join("interact/interaction_report.json"))?;
        let spectrum: SpectrumReport = spec_from_slice(&spectrum_bytes)?;
        let gauge: GaugeReport = gauge_from_slice(&gauge_bytes)?;
        let interaction: InteractionReport = int_from_slice(&interact_bytes)?;
        let running: Option<RunningReport> = {
            let path = job_dir.join("running/running_report.json");
            if path.exists() {
                let bytes = fs::read(path)?;
                Some(int_from_slice(&bytes)?)
            } else {
                None
            }
        };
        let kpi = load_kpi(&job_dir.join("kpi.json"));

        let mut inputs = AssertionInputs::default();
        inputs.spectrum = Some(spectrum);
        inputs.gauge = Some(gauge);
        inputs.interaction = Some(interaction);
        inputs.running = running;
        if let Some(summary) = &summary {
            inputs.summary = Some(summary.clone());
        }
        if let Some(kpi) = kpi {
            inputs.add_kpi(kpi);
        }

        let report = run_assertions(&inputs, &policy)?;
        let job_out = args.out.join(&job_name);
        fs::create_dir_all(&job_out)?;
        let report_path = job_out.join("assert_report.json");
        fs::write(&report_path, to_canonical_json_bytes(&report)?)?;
        entries.push(BatchIndexEntry {
            job: job_name,
            report: report_path
                .strip_prefix(&args.out)
                .unwrap()
                .to_string_lossy()
                .to_string(),
        });
    }

    let index_bytes = to_canonical_json_bytes(&entries)?;
    fs::write(args.out.join("index.json"), index_bytes)?;
    Ok(())
}
