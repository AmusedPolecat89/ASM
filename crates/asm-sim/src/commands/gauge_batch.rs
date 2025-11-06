use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use asm_aut::AnalysisReport;
use asm_core::rng::derive_substream_seed;
use asm_gauge::ClosureOpts;
use asm_gauge::{analyze_gauge, build_rep, to_canonical_json_bytes, GaugeOpts, RepOpts, WardOpts};
use asm_spec::{from_json_slice as spectrum_from_slice, SpectrumReport};
use clap::Args;
use glob::glob;
use serde::Serialize;

#[derive(Args, Debug)]
pub struct GaugeBatchArgs {
    /// Spectrum report paths or glob patterns.
    #[arg(long = "spectra", value_name = "PATH", num_args = 1..)]
    pub spectra: Vec<PathBuf>,
    /// Automorphism report paths or glob patterns.
    #[arg(long = "aut-glob", value_name = "PATH", num_args = 1..)]
    pub aut: Vec<PathBuf>,
    /// Output directory for gauge artefacts.
    #[arg(long)]
    pub out: PathBuf,
    /// Maximum number of generators per representation.
    #[arg(long, default_value_t = 3)]
    pub max_generators: usize,
    /// Closure tolerance recorded in reports.
    #[arg(long, default_value_t = 1e-6)]
    pub closure_tol: f64,
    /// Ward tolerance recorded in reports.
    #[arg(long, default_value_t = 1e-5)]
    pub ward_tol: f64,
    /// Master deterministic seed used to derive per-run substreams.
    #[arg(long, default_value_t = 0)]
    pub seed: u64,
}

#[derive(Debug, Serialize)]
struct BatchEntry {
    label: String,
    report: String,
    analysis_hash: String,
}

#[derive(Debug, Serialize)]
struct BatchIndex {
    runs: Vec<BatchEntry>,
}

fn resolve_paths(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut resolved = Vec::new();
    for input in inputs {
        let text = input.to_string_lossy();
        if text.contains('*') || text.contains('?') || text.contains('[') {
            for entry in glob(&text)? {
                resolved.push(entry?);
            }
        } else {
            resolved.push(input.clone());
        }
    }
    resolved.sort();
    resolved.dedup();
    if resolved.is_empty() {
        return Err("no input paths resolved".into());
    }
    Ok(resolved)
}

fn load_spectrum(path: &Path) -> Result<SpectrumReport, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(spectrum_from_slice(&bytes)?)
}

fn load_analysis(path: &Path) -> Result<AnalysisReport, Box<dyn Error>> {
    let json = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}

fn label_for(path: &Path, idx: usize) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .map(|value| value.to_string())
        .or_else(|| {
            path.parent().and_then(|parent| {
                parent
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|value| value.to_string())
            })
        })
        .unwrap_or_else(|| format!("spectrum_{idx:02}"))
}

fn match_analysis<'a>(
    spectrum: &SpectrumReport,
    analyses: &'a BTreeMap<(String, String), AnalysisReport>,
) -> Result<&'a AnalysisReport, Box<dyn Error>> {
    let key = (spectrum.graph_hash.clone(), spectrum.code_hash.clone());
    let Some(report) = analyses.get(&key) else {
        return Err(format!(
            "no analysis report matching graph/code hash {} / {}",
            spectrum.graph_hash, spectrum.code_hash
        )
        .into());
    };
    Ok(report)
}

pub fn run(args: &GaugeBatchArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let spectra = resolve_paths(&args.spectra)?;
    let aut_paths = resolve_paths(&args.aut)?;

    let mut analysis_map: BTreeMap<(String, String), AnalysisReport> = BTreeMap::new();
    for path in aut_paths {
        let report = load_analysis(&path)?;
        analysis_map.insert(
            (
                report.hashes.graph_hash.clone(),
                report.hashes.code_hash.clone(),
            ),
            report,
        );
    }

    if analysis_map.is_empty() {
        return Err("no analysis reports loaded".into());
    }

    let mut index_entries = Vec::new();
    for (idx, spectrum_path) in spectra.iter().enumerate() {
        let spectrum = load_spectrum(spectrum_path)?;
        let analysis = match_analysis(&spectrum, &analysis_map)?;
        let label = label_for(spectrum_path, idx);
        let sub_seed = if args.seed == 0 {
            0
        } else {
            derive_substream_seed(args.seed, idx as u64)
        };

        let mut rep_opts = RepOpts::default();
        rep_opts.max_generators = args.max_generators.max(1);
        if sub_seed != 0 {
            rep_opts.seed = Some(sub_seed);
        }
        let closure_opts = ClosureOpts {
            tolerance: args.closure_tol,
        };
        let ward_opts = WardOpts {
            relative_tol: args.ward_tol,
        };
        let gauge_opts = GaugeOpts {
            rep: rep_opts.clone(),
            closure: closure_opts.clone(),
            ward: ward_opts.clone(),
            seed: sub_seed,
            ..GaugeOpts::default()
        };

        let rep = build_rep(&spectrum, analysis, &rep_opts)?;
        let report = analyze_gauge(&spectrum, analysis, &spectrum.operators.info, &gauge_opts)?;

        let dir_name = format!("{:02}_{}", idx, label);
        let run_dir = args.out.join(&dir_name);
        fs::create_dir_all(&run_dir)?;
        fs::write(run_dir.join("rep.json"), to_canonical_json_bytes(&rep)?)?;
        fs::write(
            run_dir.join("closure.json"),
            to_canonical_json_bytes(&report.closure)?,
        )?;
        fs::write(
            run_dir.join("decomp.json"),
            to_canonical_json_bytes(&report.decomp)?,
        )?;
        fs::write(
            run_dir.join("ward.json"),
            to_canonical_json_bytes(&report.ward)?,
        )?;
        fs::write(
            run_dir.join("gauge_report.json"),
            to_canonical_json_bytes(&report)?,
        )?;

        index_entries.push(BatchEntry {
            label,
            report: format!("{}/gauge_report.json", dir_name),
            analysis_hash: report.analysis_hash.clone(),
        });
    }

    let index = BatchIndex {
        runs: index_entries,
    };
    fs::write(
        args.out.join("index.json"),
        to_canonical_json_bytes(&index)?,
    )?;

    Ok(())
}
