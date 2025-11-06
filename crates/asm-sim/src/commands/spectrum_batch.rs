use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use asm_core::rng::derive_substream_seed;
use asm_spec::{
    analyze_spectrum, to_canonical_json_bytes, CorrelSpec, DispersionSpec, ExcitationSpec, OpOpts,
    OpsVariant, PropOpts, SpecOpts,
};
use clap::Args;
use glob::glob;
use serde::Serialize;

use super::deform::load_state;

fn parse_ops_variant(value: &str) -> Result<OpsVariant, Box<dyn Error>> {
    match value {
        "default" => Ok(OpsVariant::Default),
        "alt" => Ok(OpsVariant::Alt),
        other => Err(format!("unknown ops variant '{other}'").into()),
    }
}

#[derive(Args, Debug)]
pub struct SpectrumBatchArgs {
    /// Input directories or glob patterns pointing to end states.
    #[arg(long = "inputs", value_name = "PATH", num_args = 1..)]
    pub inputs: Vec<PathBuf>,
    /// Output directory for batched spectrum artefacts.
    #[arg(long)]
    pub out: PathBuf,
    /// Number of k-points to use during the dispersion scan.
    #[arg(long, default_value_t = 64)]
    pub k_points: usize,
    /// Number of modes to retain per spectrum report.
    #[arg(long, default_value_t = 3)]
    pub modes: usize,
    /// Master deterministic seed shared across the batch.
    #[arg(long)]
    pub seed: u64,
    /// Operator variant: "default" or "alt".
    #[arg(long, default_value = "default")]
    pub ops_variant: String,
    /// Fit tolerance recorded in provenance payloads.
    #[arg(long, default_value_t = 1e-6)]
    pub fit_tol: f64,
    /// Excitation support size.
    #[arg(long, default_value_t = 3)]
    pub support: usize,
    /// Propagation iterations.
    #[arg(long, default_value_t = 16)]
    pub iterations: usize,
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

fn resolve_inputs(paths: &[PathBuf]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut resolved = Vec::new();
    for path in paths {
        let text = path.to_string_lossy();
        if text.contains('*') || text.contains('?') || text.contains('[') {
            for entry in glob(&text)? {
                resolved.push(entry?);
            }
        } else {
            resolved.push(path.clone());
        }
    }
    resolved.sort();
    resolved.dedup();
    if resolved.is_empty() {
        return Err("no inputs resolved for spectrum batch".into());
    }
    Ok(resolved)
}

fn label_for(path: &Path, idx: usize) -> String {
    let mut label = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|value| value.to_string())
        .or_else(|| {
            path.parent().and_then(|parent| {
                parent
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|s| s.to_string())
            })
        })
        .unwrap_or_else(|| format!("state_{idx:02}"));
    if label == "end_state" {
        if let Some(parent_label) = path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
        {
            label = parent_label.to_string();
        }
    }
    label
}

fn write_single(
    out_dir: &Path,
    label: &str,
    report: &asm_spec::SpectrumReport,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(out_dir)?;
    fs::write(
        out_dir.join("operators.json"),
        to_canonical_json_bytes(&report.operators)?,
    )?;
    fs::write(
        out_dir.join("dispersion.json"),
        to_canonical_json_bytes(&report.dispersion)?,
    )?;
    fs::write(
        out_dir.join("correlation.json"),
        to_canonical_json_bytes(&report.correlation)?,
    )?;
    fs::write(
        out_dir.join("spectrum_report.json"),
        to_canonical_json_bytes(report)?,
    )?;
    // Include a lightweight metadata stub for convenience.
    fs::write(
        out_dir.join("metadata.json"),
        to_canonical_json_bytes(&serde_json::json!({
            "label": label,
            "analysis_hash": report.analysis_hash,
        }))?,
    )?;
    Ok(())
}

pub fn run(args: &SpectrumBatchArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let inputs = resolve_inputs(&args.inputs)?;
    let variant = parse_ops_variant(&args.ops_variant)?;

    let mut dispersion = DispersionSpec::default();
    dispersion.k_points = args.k_points.max(1);
    dispersion.modes = args.modes.max(1);
    let correlation = CorrelSpec::default();

    let mut index_entries = Vec::new();
    for (idx, input) in inputs.iter().enumerate() {
        let loaded = load_state(input)?;
        let label = label_for(input, idx);
        let sub_seed = derive_substream_seed(args.seed, idx as u64 + 10);
        let prop_opts = PropOpts {
            iterations: args.iterations.max(1),
            tolerance: args.fit_tol,
            seed: derive_substream_seed(args.seed, idx as u64),
        };
        let mut excitation = ExcitationSpec::default();
        excitation.support = args.support.max(1);
        let spec_opts = SpecOpts {
            ops: OpOpts { variant },
            excitation,
            propagation: prop_opts,
            dispersion: dispersion.clone(),
            correlation: correlation.clone(),
            master_seed: sub_seed,
            fit_tolerance: args.fit_tol,
        };
        let report = analyze_spectrum(&loaded.graph, &loaded.code, &spec_opts)?;
        let dir_name = format!("{:02}_{}", idx, label);
        let run_dir = args.out.join(&dir_name);
        write_single(&run_dir, &label, &report)?;
        index_entries.push(BatchEntry {
            label,
            report: format!("{}/spectrum_report.json", dir_name),
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
