use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use asm_code::dispersion::{DispersionOptions, DispersionReport};
use asm_code::{serde as code_serde, CSSCode, SpeciesId};
use asm_graph::{graph_from_json, HypergraphImpl};
use asm_mcmc::analysis;
use asm_mcmc::config::RunConfig;
use asm_mcmc::manifest::RunManifest;
use asm_mcmc::{run, RunSummary};
use clap::{Args as ClapArgs, Parser, Subcommand};
use serde::Deserialize;

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
    #[arg(long)]
    input: PathBuf,
    /// Output directory for analysis artefacts.
    #[arg(long)]
    out: PathBuf,
    /// Optional dispersion configuration overriding defaults.
    #[arg(long)]
    dispersion_config: Option<PathBuf>,
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
    fs::create_dir_all(&args.out)?;
    let manifest = RunManifest::load(&args.input.join("manifest.json"))?;
    let (code, graph) = analysis::load_end_state(&args.input)?;

    let (mut species, mut options) =
        load_dispersion_job(args.dispersion_config.as_deref(), &args.input)?;
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
        analysis::resolve_checkpoint_paths(&args.input, &manifest.checkpoints)
    } else {
        collect_default_checkpoints(&args.input)?
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
