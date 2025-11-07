use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_land::filters::load_filters;
use asm_land::plan::{
    CodeSpec, GaugeSpec, GraphSpec, InteractSpec, OutputLayout, OutputSpec, RuleSpec, SamplerSpec,
    SpectrumSpec,
};
use asm_land::serde::{to_canonical_json_bytes, to_yaml_string};
use asm_land::{
    build_atlas, load_plan, plan::Plan, report::AtlasOpts, run_plan, summarize, RunOpts,
};
use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum LandscapeSubcommand {
    /// Generate a deterministic landscape plan YAML file.
    Plan(PlanArgs),
    /// Execute a landscape plan and emit canonical JSON artefacts.
    Run(RunArgs),
    /// Summarize metrics for existing runs under the provided root.
    Summarize(SummarizeArgs),
    /// Build a compact atlas manifest aggregating all universes discovered.
    Atlas(AtlasArgs),
}

#[derive(Args, Debug)]
pub struct PlanArgs {
    /// Destination path for the generated plan YAML.
    #[arg(long)]
    pub out: PathBuf,
    /// Number of deterministic seeds to include in the plan.
    #[arg(long)]
    pub seeds: u64,
    /// Starting seed offset (defaults to zero).
    #[arg(long, default_value_t = 0)]
    pub seed_start: u64,
    /// Graph size parameter.
    #[arg(long)]
    pub size: u32,
    /// Maximum degree for generated graphs.
    #[arg(long)]
    pub degree: u32,
    /// Uniformity of the hypergraph.
    #[arg(long)]
    pub k: u32,
    /// Graph generator variant.
    #[arg(long, default_value = "regular")]
    pub generator: String,
    /// Number of sampler sweeps.
    #[arg(long)]
    pub sweeps: u32,
    /// Worm weight parameter for the sampler.
    #[arg(long)]
    pub worm: f64,
    /// Ladder parameter for the sampler.
    #[arg(long)]
    pub ladder: u32,
    /// Number of checkpoints.
    #[arg(long, default_value_t = 1)]
    pub checkpoints: u32,
    /// Number of k-points in the spectrum evaluation.
    #[arg(long)]
    pub kpoints: u32,
    /// Number of modes in the spectrum evaluation.
    #[arg(long)]
    pub modes: u32,
    /// Kernel steps for the interaction stage.
    #[arg(long, default_value_t = 64)]
    pub steps: u32,
    /// Time step for the interaction kernel.
    #[arg(long, default_value_t = 0.02)]
    pub dt: f64,
    /// Path to the filters YAML file.
    #[arg(long, default_value = "landscape/filters/default.yaml")]
    pub filters: PathBuf,
    /// Whether to emit per-seed output directories.
    #[arg(long, default_value_t = false)]
    pub per_seed: bool,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Path to the plan YAML file.
    #[arg(long)]
    pub plan: PathBuf,
    /// Output directory for the run artefacts.
    #[arg(long)]
    pub out: PathBuf,
    /// Resume partially completed runs.
    #[arg(long, default_value_t = false)]
    pub resume: bool,
    /// Advisory concurrency level.
    #[arg(long, default_value_t = 1)]
    pub concurrency: usize,
}

#[derive(Args, Debug)]
pub struct SummarizeArgs {
    /// Root directory containing completed landscape runs.
    #[arg(long)]
    pub root: PathBuf,
    /// Filter specification to apply when summarizing.
    #[arg(long)]
    pub filters: PathBuf,
    /// Output directory for the summary artefacts.
    #[arg(long)]
    pub out: PathBuf,
}

#[derive(Args, Debug)]
pub struct AtlasArgs {
    /// Root directory containing completed landscape runs.
    #[arg(long)]
    pub root: PathBuf,
    /// Output directory for the atlas manifest.
    #[arg(long)]
    pub out: PathBuf,
    /// Include failed jobs in the atlas.
    #[arg(long, default_value_t = false)]
    pub include_failed: bool,
}

pub fn run(cmd: &LandscapeSubcommand) -> Result<(), Box<dyn Error>> {
    match cmd {
        LandscapeSubcommand::Plan(args) => generate_plan(args),
        LandscapeSubcommand::Run(args) => execute_plan(args),
        LandscapeSubcommand::Summarize(args) => summarize_runs(args),
        LandscapeSubcommand::Atlas(args) => build_atlas_manifest(args),
    }
}

fn generate_plan(args: &PlanArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(
        args.out
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".")),
    )?;
    let seeds: Vec<u64> = (0..args.seeds)
        .map(|offset| args.seed_start + offset)
        .collect();
    let plan = Plan {
        seeds,
        graph: GraphSpec {
            degree_cap: args.degree,
            k_uniform: args.k,
            size: args.size,
            generator: args.generator.clone(),
        },
        code: CodeSpec {
            density: 0.1,
            css_variant: "css-plan".to_string(),
            rowop_rate: 0.2,
        },
        sampler: SamplerSpec {
            sweeps: args.sweeps,
            worm_weight: args.worm,
            ladder: args.ladder,
            checkpoints: args.checkpoints,
        },
        spectrum: SpectrumSpec {
            k_points: args.kpoints,
            modes: args.modes,
        },
        gauge: GaugeSpec {
            closure_tol: 0.001,
            ward_tol: 0.001,
        },
        interact: InteractSpec {
            steps: args.steps,
            dt: args.dt,
            measure: "default".to_string(),
            fit: "default".to_string(),
        },
        filters: args.filters.clone(),
        outputs: OutputSpec {
            layout: if args.per_seed {
                OutputLayout::PerSeed
            } else {
                OutputLayout::Flat
            },
            keep_intermediate: true,
        },
        rules: vec![RuleSpec::default()],
        base_dir: PathBuf::new(),
    };
    let yaml = to_yaml_string(&plan)?;
    fs::write(&args.out, yaml)?;
    Ok(())
}

fn execute_plan(args: &RunArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let plan = load_plan(&args.plan)?;
    let opts = RunOpts {
        resume: args.resume,
        concurrency: args.concurrency,
        max_retries: 2,
    };
    run_plan(&plan, &args.out, &opts)?;
    Ok(())
}

fn summarize_runs(args: &SummarizeArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let filters = load_filters(&args.filters)?;
    let summary = summarize(&args.root, &filters)?;
    fs::write(
        args.out.join("summary_report.json"),
        to_canonical_json_bytes(&summary)?,
    )?;
    Ok(())
}

fn build_atlas_manifest(args: &AtlasArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let atlas = build_atlas(
        &args.root,
        &AtlasOpts {
            include_failed: args.include_failed,
        },
    )?;
    fs::write(
        args.out.join("atlas.json"),
        to_canonical_json_bytes(&atlas)?,
    )?;
    Ok(())
}
