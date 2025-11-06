use std::collections::BTreeSet;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use asm_aut::invariants::ProvenanceInfo;
use asm_aut::{
    analyze_state as aut_analyze_state, cluster as aut_cluster, serde_io as aut_serde,
    AnalysisReport, ClusterOpts as AutClusterOpts, ScanOpts as AutScanOpts,
};
use asm_code::dispersion::{DispersionOptions, DispersionReport};
use asm_code::{serde as code_serde, CSSCode, SpeciesId};
use asm_graph::{graph_from_json, HypergraphImpl};
use asm_mcmc::analysis;
use asm_mcmc::config::RunConfig;
use asm_mcmc::manifest::RunManifest;
use asm_mcmc::{run, RunSummary};
use clap::{Args as ClapArgs, Parser, Subcommand};
use commands::{
    extract::{self, ExtractArgs},
    rg::{self, RgArgs},
    rg_covariance::{self, RgCovarianceArgs},
};
use serde::Deserialize;
use serde_json::json;

mod commands;

#[derive(Parser, Debug)]
#[command(name = "asm-sim", about = "ASM ensemble sampler CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Execute a sampler run from a configuration and initial state snapshot.
    Mcmc(McmcArgs),
    /// Analyse an existing run directory and emit dispersion diagnostics.
    Analyze(AnalyzeArgs),
    /// Run the deterministic RG flow on a vacuum directory.
    Rg(RgArgs),
    /// Extract effective couplings for a single state.
    Extract(ExtractArgs),
    /// Compare dictionary extraction before/after RG.
    RgCovariance(RgCovarianceArgs),
}

#[derive(ClapArgs, Debug)]
struct McmcArgs {
    /// YAML configuration describing the sampler run.
    #[arg(long)]
    config: PathBuf,
    /// JSON manifest pointing to serialized code and graph inputs.
    #[arg(long = "in")]
    input: PathBuf,
    /// Output directory for run artefacts.
    #[arg(long)]
    out: PathBuf,
}

#[derive(ClapArgs, Debug)]
struct AnalyzeArgs {
    /// Input run directory produced by `asm-sim mcmc`.
    #[arg(long, required_unless_present = "cluster")]
    input: Option<PathBuf>,
    /// Additional analysis directories when clustering.
    #[arg(long = "inputs", value_name = "PATH")]
    inputs: Vec<PathBuf>,
    /// Output directory for analysis artefacts.
    #[arg(long)]
    out: PathBuf,
    /// Optional dispersion configuration overriding defaults.
    #[arg(long)]
    dispersion_config: Option<PathBuf>,
    /// Perform a symmetry scan using `asm-aut`.
    #[arg(long)]
    symmetry_scan: bool,
    /// Cluster existing analysis reports instead of running dispersion.
    #[arg(long)]
    cluster: bool,
    /// Number of Laplacian eigenvalues to retain during symmetry scans.
    #[arg(long, default_value_t = 16)]
    laplacian_topk: usize,
    /// Number of stabiliser eigenvalues to retain during symmetry scans.
    #[arg(long, default_value_t = 16)]
    stabilizer_topk: usize,
    /// Number of clusters to form when `--cluster` is set.
    #[arg(long = "clusters", default_value_t = 2)]
    cluster_count: usize,
    /// Maximum k-means refinement passes when clustering.
    #[arg(long = "cluster-iterations", default_value_t = 16)]
    cluster_iterations: usize,
    /// Emit top-N representative hashes per cluster.
    #[arg(long = "emit-representatives")]
    emit_representatives: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct StatePaths {
    code: PathBuf,
    graph: PathBuf,
}

#[derive(Debug, Deserialize)]
struct DispersionConfigFile {
    #[serde(default)]
    species: Vec<DispersionSpeciesValue>,
    #[serde(default)]
    steps: Vec<u32>,
    tolerance: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DispersionSpeciesValue {
    Numeric(u64),
    String(String),
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Mcmc(args) => run_mcmc(args),
        Command::Analyze(args) => run_analysis(args),
        Command::Rg(args) => rg::run(&args),
        Command::Extract(args) => extract::run(&args),
        Command::RgCovariance(args) => rg_covariance::run(&args),
    }
}

fn run_mcmc(args: McmcArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let config = load_config(&args.config, &args.out)?;
    let state_paths: StatePaths = serde_json::from_str(&fs::read_to_string(&args.input)?)?;
    let (code, graph) = load_state(&state_paths)?;

    let summary = run(&config, config.seed_policy.master_seed, &code, &graph)?;

    write_json(args.out.join("summary.json"), &summary)?;
    write_coverage_summary(&args.out, &summary)?;

    // Persist run configuration and input manifest for reproducibility.
    fs::copy(&args.config, args.out.join("config.yaml")).ok();
    fs::copy(&args.input, args.out.join("state.json")).ok();

    Ok(())
}

fn run_analysis(args: AnalyzeArgs) -> Result<(), Box<dyn Error>> {
    if args.cluster {
        run_cluster_mode(&args)
    } else if args.symmetry_scan {
        run_symmetry_scan(&args)
    } else {
        run_dispersion_mode(&args)
    }
}

fn run_dispersion_mode(args: &AnalyzeArgs) -> Result<(), Box<dyn Error>> {
    let Some(input_dir) = args.input.as_ref() else {
        return Err("--input is required unless --cluster is set".into());
    };
    fs::create_dir_all(&args.out)?;
    let manifest = RunManifest::load(&input_dir.join("manifest.json"))?;
    let (code, graph) = analysis::load_end_state(input_dir)?;

    let (mut species, mut options) =
        load_dispersion_job(args.dispersion_config.as_deref(), input_dir)?;
    if species.is_empty() {
        species = code.species_catalog();
    }
    if options.tolerance <= 0.0 {
        options.tolerance = 0.03;
    }

    let report = analysis::dispersion_for_state(&code, &graph, &species, &options)?;
    let dispersion_dir = args.out.join("dispersion");
    fs::create_dir_all(&dispersion_dir)?;
    write_species_reports(&dispersion_dir, &report)?;
    write_common_c(&args.out, &report, options.tolerance)?;

    let checkpoint_paths = if !manifest.checkpoints.is_empty() {
        analysis::resolve_checkpoint_paths(input_dir, &manifest.checkpoints)
    } else {
        collect_default_checkpoints(input_dir)?
    };
    let mut checkpoint_reports = Vec::new();
    for path in checkpoint_paths {
        if !path.exists() {
            continue;
        }
        let report = analysis::dispersion_for_checkpoint(&path, &species, &options)?;
        let label = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("checkpoint")
            .to_string();
        checkpoint_reports.push((label, report));
    }
    write_checkpoint_summary(&args.out, &checkpoint_reports, &report, options.tolerance)?;

    Ok(())
}

fn run_symmetry_scan(args: &AnalyzeArgs) -> Result<(), Box<dyn Error>> {
    let Some(input_dir) = args.input.as_ref() else {
        return Err("--input is required for symmetry scans".into());
    };
    fs::create_dir_all(&args.out)?;
    let manifest = RunManifest::load(&input_dir.join("manifest.json"))?;
    let (code, graph) = analysis::load_end_state(input_dir)?;
    let provenance = build_provenance(&manifest, input_dir);

    let scan_opts = AutScanOpts {
        laplacian_topk: args.laplacian_topk,
        stabilizer_topk: args.stabilizer_topk,
        provenance: Some(provenance),
    };
    let report = aut_analyze_state(&graph, &code, &scan_opts)?;

    write_json(args.out.join("analysis_report.json"), &report)?;
    write_spectral_csv(&args.out.join("spectral.csv"), &report)?;
    let index = json!({
        "inputs": [
            {
                "path": input_dir.display().to_string(),
                "analysis_hash": report.hashes.analysis_hash,
            }
        ]
    });
    write_json(args.out.join("index.json"), &index)?;
    Ok(())
}

fn run_cluster_mode(args: &AnalyzeArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let mut sources: Vec<PathBuf> = args.inputs.clone();
    if let Some(single) = &args.input {
        if sources.is_empty() {
            sources.push(single.clone());
        }
    }
    if sources.is_empty() {
        return Err("provide at least one --inputs path when clustering".into());
    }

    let mut reports = Vec::new();
    let mut location_map: HashMap<String, String> = HashMap::new();
    for source in sources {
        if source.is_dir() {
            let candidate = source.join("analysis_report.json");
            if candidate.exists() {
                let report = load_analysis_report(&candidate)?;
                location_map.insert(
                    report.hashes.analysis_hash.clone(),
                    candidate.display().to_string(),
                );
                reports.push(report);
            }
        } else if source.file_name().is_some() {
            let report = load_analysis_report(&source)?;
            location_map.insert(
                report.hashes.analysis_hash.clone(),
                source.display().to_string(),
            );
            reports.push(report);
        }
    }

    if reports.is_empty() {
        return Err("no analysis reports found in the provided paths".into());
    }

    let default_opts = AutClusterOpts::default();
    let cluster_opts = AutClusterOpts {
        k: args.cluster_count.min(reports.len()).max(1),
        max_iterations: args.cluster_iterations.max(1),
        seed: default_opts.seed,
    };
    let summary = aut_cluster(&reports, &cluster_opts);
    write_json(args.out.join("cluster_summary.json"), &summary)?;

    let mut index_entries = Vec::new();
    for report in &reports {
        let path = location_map
            .get(&report.hashes.analysis_hash)
            .cloned()
            .unwrap_or_default();
        index_entries.push(json!({
            "analysis_hash": report.hashes.analysis_hash,
            "path": path,
        }));
    }
    write_json(
        args.out.join("index.json"),
        &json!({ "reports": index_entries }),
    )?;

    if let Some(limit) = args.emit_representatives {
        let mut clusters = Vec::new();
        for cluster in &summary.clusters {
            let mut members = Vec::new();
            for hash in cluster.members.iter().take(limit) {
                let path = location_map.get(hash).cloned();
                members.push(json!({ "hash": hash, "path": path }));
            }
            clusters.push(json!({
                "cluster_id": cluster.cluster_id,
                "members": members,
            }));
        }
        write_json(
            args.out.join("representatives.json"),
            &json!({ "clusters": clusters }),
        )?;
    }

    Ok(())
}

fn build_provenance(manifest: &RunManifest, input_dir: &Path) -> ProvenanceInfo {
    let run_id = manifest
        .config
        .output
        .run_directory
        .as_ref()
        .and_then(|path| path.to_str())
        .map(|s| s.to_string())
        .or_else(|| Some(input_dir.display().to_string()));
    ProvenanceInfo {
        seed: Some(manifest.master_seed),
        run_id,
        checkpoint_id: None,
        commit: None,
    }
}

fn write_spectral_csv(path: &Path, report: &AnalysisReport) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::File::create(path)?;
    writeln!(file, "spectrum,index,value")?;
    for (idx, value) in report.spectral.laplacian_topk.iter().enumerate() {
        writeln!(file, "laplacian,{},{:.9}", idx, value)?;
    }
    for (idx, value) in report.spectral.stabilizer_topk.iter().enumerate() {
        writeln!(file, "stabilizer,{},{:.9}", idx, value)?;
    }
    Ok(())
}

fn load_analysis_report(path: &Path) -> Result<AnalysisReport, Box<dyn Error>> {
    let json = aut_serde::read_json(path).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    let report =
        aut_serde::analysis_from_json(&json).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    Ok(report)
}

fn load_config(path: &Path, out_dir: &Path) -> Result<RunConfig, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let mut config: RunConfig = serde_yaml::from_str(&contents)?;
    config.output.run_directory = Some(out_dir.to_path_buf());
    if config.output.checkpoint_dir.as_os_str().is_empty() {
        config.output.checkpoint_dir = PathBuf::from("checkpoints");
    }
    Ok(config)
}

fn load_state(paths: &StatePaths) -> Result<(CSSCode, HypergraphImpl), Box<dyn Error>> {
    let code_json = fs::read_to_string(&paths.code)?;
    let graph_json = fs::read_to_string(&paths.graph)?;
    let code = code_serde::from_json(&code_json)?;
    let graph = graph_from_json(&graph_json)?;
    Ok((code, graph))
}

fn write_json<P: AsRef<Path>, T: serde::Serialize>(
    path: P,
    value: &T,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(value)?;
    fs::write(path, json)?;
    Ok(())
}

fn write_coverage_summary(out_dir: &Path, summary: &RunSummary) -> Result<(), Box<dyn Error>> {
    let coverage = &summary.coverage;
    let exchange_mean = if summary.exchange_acceptance.is_empty() {
        0.0
    } else {
        summary.exchange_acceptance.iter().copied().sum::<f64>()
            / summary.exchange_acceptance.len() as f64
    };
    let payload = serde_json::json!({
        "unique_structural_hashes": coverage.unique_state_hashes,
        "worm_samples": coverage.worm_samples,
        "average_jaccard": coverage.average_jaccard,
        "jaccard_lag_decay": (1.0 - coverage.average_jaccard).max(0.0),
        "exchange_acceptance": summary.exchange_acceptance,
        "exchange_acceptance_mean": exchange_mean,
        "effective_sample_size": summary.effective_sample_size,
    });
    write_json(out_dir.join("coverage_summary.json"), &payload)
}

fn load_dispersion_job(
    explicit: Option<&Path>,
    run_dir: &Path,
) -> Result<(Vec<SpeciesId>, DispersionOptions), Box<dyn Error>> {
    let mut candidates = Vec::<PathBuf>::new();
    if let Some(path) = explicit {
        candidates.push(path.to_path_buf());
    }
    candidates.push(run_dir.join("dispersion.yaml"));
    candidates.push(PathBuf::from("configs/dispersion.yaml"));

    for candidate in candidates {
        if candidate.exists() {
            let contents = fs::read_to_string(candidate)?;
            let config = parse_dispersion_config(&contents)?;
            let steps = if let Some(steps) = config.steps {
                let mut dedup: Vec<u32> = steps.into_iter().collect();
                dedup.sort_unstable();
                dedup.dedup();
                dedup
            } else {
                DispersionOptions::default().steps
            };
            let options = DispersionOptions {
                steps,
                tolerance: config.tolerance.unwrap_or(0.03),
            };
            return Ok((config.species, options));
        }
    }

    Ok((
        Vec::new(),
        DispersionOptions {
            tolerance: 0.03,
            ..DispersionOptions::default()
        },
    ))
}

struct ParsedDispersionConfig {
    species: Vec<SpeciesId>,
    steps: Option<Vec<u32>>,
    tolerance: Option<f64>,
}

fn parse_dispersion_config(contents: &str) -> Result<ParsedDispersionConfig, Box<dyn Error>> {
    let file: DispersionConfigFile = serde_yaml::from_str(contents)?;
    let mut species_set = BTreeSet::new();
    for value in file.species {
        species_set.insert(parse_species_value(value)?);
    }
    let species: Vec<SpeciesId> = species_set.into_iter().collect();

    let steps = if file.steps.is_empty() {
        None
    } else {
        let mut set = BTreeSet::new();
        for step in file.steps {
            set.insert(step);
        }
        Some(set.into_iter().collect())
    };

    Ok(ParsedDispersionConfig {
        species,
        steps,
        tolerance: file.tolerance,
    })
}

fn parse_species_value(value: DispersionSpeciesValue) -> Result<SpeciesId, Box<dyn Error>> {
    match value {
        DispersionSpeciesValue::Numeric(raw) => Ok(SpeciesId::from_raw(raw)),
        DispersionSpeciesValue::String(text) => parse_species_string(&text),
    }
}

fn parse_species_string(text: &str) -> Result<SpeciesId, Box<dyn Error>> {
    let trimmed = text.trim();
    let trimmed = trimmed.strip_prefix("species-").unwrap_or(trimmed);
    let (radix, digits) = if let Some(rest) = trimmed.strip_prefix("0x") {
        (16, rest)
    } else if let Some(rest) = trimmed.strip_prefix("0X") {
        (16, rest)
    } else {
        (10, trimmed)
    };
    let value = u64::from_str_radix(digits, radix)?;
    Ok(SpeciesId::from_raw(value))
}

fn write_species_reports(dir: &Path, report: &DispersionReport) -> Result<(), Box<dyn Error>> {
    for entry in &report.per_species {
        let filename = format!("{}.json", species_file_stem(entry.species));
        write_json(dir.join(filename), entry)?;
    }
    Ok(())
}

fn species_file_stem(species: SpeciesId) -> String {
    format!("species-{:#x}", species.as_raw())
}

fn write_common_c(
    out_dir: &Path,
    report: &DispersionReport,
    tolerance: f64,
) -> Result<(), Box<dyn Error>> {
    let payload = serde_json::json!({
        "common_c": report.common_c,
        "residuals": report.residuals,
        "diagnostics": report.diagnostics,
        "residual_max": residual_max(report),
        "tolerance": tolerance,
    });
    write_json(out_dir.join("common_c.json"), &payload)
}

fn write_checkpoint_summary(
    out_dir: &Path,
    checkpoints: &[(String, DispersionReport)],
    final_report: &DispersionReport,
    tolerance: f64,
) -> Result<(), Box<dyn Error>> {
    let path = out_dir.join("common_c_by_checkpoint.csv");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::File::create(path)?;
    writeln!(file, "checkpoint_id,c,residual_max,tolerance")?;
    for (label, report) in checkpoints {
        writeln!(
            file,
            "{},{:.6},{:.6},{:.6}",
            label,
            report.common_c,
            residual_max(report),
            tolerance,
        )?;
    }
    writeln!(
        file,
        "end_state,{:.6},{:.6},{:.6}",
        final_report.common_c,
        residual_max(final_report),
        tolerance,
    )?;
    Ok(())
}

fn residual_max(report: &DispersionReport) -> f64 {
    report
        .residuals
        .iter()
        .map(|res| res.residual.abs())
        .fold(0.0f64, f64::max)
}

fn collect_default_checkpoints(run_dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let checkpoint_dir = run_dir.join("checkpoints");
    if !checkpoint_dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<PathBuf> = fs::read_dir(&checkpoint_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    paths.sort();
    Ok(paths)
}
