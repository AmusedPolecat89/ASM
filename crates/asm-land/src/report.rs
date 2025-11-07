use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::provenance::RunProvenance;

use crate::filters::{FilterDecision, FilterSpec};
use crate::hash::stable_hash_string;
use crate::metrics::JobKpi;
use crate::plan::{GraphSpec, Plan};
use crate::serde::from_json_slice;
use crate::stages::StageHashes;
use crate::stat::{Correlations, Histogram, Quantiles, StatsSummary};

/// Status of an individual job within a landscape run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobStatus {
    /// State of the job (pending, complete, failed).
    pub state: JobState,
    /// Number of deterministic attempts used to complete the job.
    pub attempts: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional error message captured when the job fails.
    pub error: Option<String>,
}

impl JobStatus {
    /// Constructs a successful status descriptor for a completed job.
    pub fn success(attempts: u32) -> Self {
        Self {
            state: JobState::Complete,
            attempts,
            error: None,
        }
    }

    /// Constructs a failed status descriptor capturing the error string.
    pub fn failed(attempts: u32, error: impl Into<String>) -> Self {
        Self {
            state: JobState::Failed,
            attempts,
            error: Some(error.into()),
        }
    }
}

/// State enumeration for a landscape job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobState {
    /// Job has not been executed yet.
    Pending,
    /// Job completed successfully.
    Complete,
    /// Job failed after exhausting retries.
    Failed,
}

/// Canonical report entry for a single job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobReport {
    /// Seed associated with the job.
    pub seed: u64,
    /// Rule variant identifier associated with the job.
    pub rule_id: u64,
    /// Execution status metadata.
    pub status: JobStatus,
    /// Canonical stage hashes recorded for the job.
    pub hashes: StageHashes,
    /// Key performance indicators extracted from the job.
    pub kpis: JobKpi,
    /// Anthropic filter decisions recorded for the job.
    pub filters: FilterDecision,
}

/// Aggregated filter summary across all jobs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandscapeFilters {
    /// Filter specification evaluated for each job.
    pub spec: FilterSpec,
    /// Number of jobs passing all filters.
    pub pass_count: usize,
    /// Total number of jobs analysed.
    pub total: usize,
}

/// Deterministic landscape exploration report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandscapeReport {
    /// Canonical hash of the plan driving the run.
    pub plan_hash: String,
    /// Per-job results captured during the run.
    pub jobs: Vec<JobReport>,
    /// Aggregate statistics across all jobs.
    pub stats: StatsSummary,
    /// Anthropic filter specification and counts.
    pub filters: LandscapeFilters,
    /// Provenance metadata describing the run.
    pub provenance: RunProvenance,
}

impl LandscapeReport {
    /// Constructs a new report from its constituent parts.
    pub fn new(
        plan: &Plan,
        jobs: Vec<JobReport>,
        stats: StatsSummary,
        filters: FilterSpec,
    ) -> Self {
        let pass_count = jobs.iter().filter(|job| job.filters.passes()).count();
        let total = jobs.len();
        let plan_hash = plan.plan_hash().unwrap_or_else(|_| "".to_string());
        Self {
            plan_hash,
            jobs,
            stats,
            filters: LandscapeFilters {
                spec: filters,
                pass_count,
                total,
            },
            provenance: provenance(plan),
        }
    }
}

fn provenance(plan: &Plan) -> RunProvenance {
    let mut versions = BTreeMap::new();
    versions.insert(
        "asm-land".to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );
    let plan_hash = plan.plan_hash().unwrap_or_else(|_| "".to_string());
    RunProvenance {
        input_hash: plan_hash.clone(),
        graph_hash: hash_graph(&plan.graph),
        code_hash: plan_hash,
        seed: plan.seeds.first().copied().unwrap_or_default(),
        created_at: Utc::now().to_rfc3339(),
        tool_versions: versions,
    }
}

fn hash_graph(graph: &GraphSpec) -> String {
    format!(
        "{:08x}{:08x}{:08x}",
        graph.degree_cap, graph.k_uniform, graph.size
    )
}

/// Atlas entry summarising a single universe discovered during the run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasEntry {
    /// Stable identifier derived from the seed and rule identifier.
    pub id: String,
    /// Hash summarising the graph ensemble.
    pub graph_hash: String,
    /// Hash summarising the interaction artefact.
    pub code_hash: String,
    /// Central charge estimate for the universe.
    pub c_est: f64,
    /// Gap proxy metric for the universe.
    pub gap: f64,
    /// Gauge factors detected during the run.
    pub factors: Vec<String>,
    /// Coupling vector extracted from the interaction stage.
    pub couplings: Vec<f64>,
}

/// Compact atlas manifest enumerating all universes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Atlas {
    /// Entries contained in the atlas manifest.
    pub entries: Vec<AtlasEntry>,
    /// Hash of the atlas entries, enabling change detection.
    pub index_hash: String,
    /// Deterministic ordering of entry identifiers.
    pub manifest: Vec<String>,
}

/// Options controlling atlas construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AtlasOpts {
    /// Include failed jobs when building the atlas.
    pub include_failed: bool,
}

/// Summary report aggregating statistics across multiple runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummaryReport {
    /// Total job counts included in the summary.
    pub totals: SummaryTotals,
    /// Pass-rate information for anthropic filters.
    pub pass_rates: PassRates,
    /// Histogram distributions per metric.
    pub distributions: BTreeMap<String, Histogram>,
    /// Quantile summaries per metric.
    pub quantiles: BTreeMap<String, Quantiles>,
    /// Correlation summaries per metric pair.
    pub correlations: BTreeMap<String, Correlations>,
    /// Free-form notes attached to the summary.
    pub notes: Vec<String>,
}

/// Total counts used in the summary report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummaryTotals {
    /// Total number of jobs analysed.
    pub jobs: usize,
    /// Number of jobs passing all filters.
    pub passing: usize,
}

/// Pass rate descriptor for the anthropic filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PassRates {
    /// Fraction of jobs passing the full anthropic filter suite.
    pub anthropic: f64,
}

impl SummaryReport {
    /// Constructs a new summary from the provided jobs.
    pub fn from_jobs(jobs: &[JobReport], stats: StatsSummary) -> Self {
        let passing = jobs.iter().filter(|job| job.filters.passes()).count();
        let jobs_total = jobs.len();
        let rate = if jobs_total == 0 {
            0.0
        } else {
            passing as f64 / jobs_total as f64
        };
        Self {
            totals: SummaryTotals {
                jobs: jobs_total,
                passing,
            },
            pass_rates: PassRates { anthropic: rate },
            distributions: stats.histograms.clone(),
            quantiles: stats.quantiles.clone(),
            correlations: stats.correlations.clone(),
            notes: vec![],
        }
    }
}

fn io_error(code: &str, err: impl ToString) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, err.to_string()))
}

fn load_report(root: &Path) -> Result<LandscapeReport, AsmError> {
    let path = root.join("landscape_report.json");
    let bytes = fs::read(&path).map_err(|err| io_error("landscape_report_read", err))?;
    from_json_slice(&bytes)
}

/// Constructs an atlas manifest from the runs stored under the provided root.
pub fn build_atlas(root: &Path, opts: &AtlasOpts) -> Result<Atlas, AsmError> {
    let report = load_report(root)?;
    let mut entries = Vec::new();
    for job in report.jobs.iter() {
        if !opts.include_failed && job.status.state != JobState::Complete {
            continue;
        }
        let id = format!("{}_{}", job.seed, job.rule_id);
        entries.push(AtlasEntry {
            id,
            graph_hash: job.hashes.mcmc.clone(),
            code_hash: job.hashes.interaction.clone(),
            c_est: job.kpis.c_est,
            gap: job.kpis.gap_proxy,
            factors: job.kpis.factors.clone(),
            couplings: job.kpis.g.clone(),
        });
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    let index_hash = stable_hash_string(&entries)?;
    let manifest = entries.iter().map(|entry| entry.id.clone()).collect();
    Ok(Atlas {
        entries,
        index_hash,
        manifest,
    })
}

/// Summarises metrics across the runs stored under the provided root.
pub fn summarize(root: &Path, filt: &FilterSpec) -> Result<SummaryReport, AsmError> {
    let report = load_report(root)?;
    let mut jobs = Vec::new();
    for mut job in report.jobs.into_iter() {
        job.filters = filt.evaluate(&job.kpis);
        jobs.push(job);
    }
    let kpis: Vec<JobKpi> = jobs.iter().map(|job| job.kpis.clone()).collect();
    let stats = StatsSummary::from_kpis(&kpis);
    Ok(SummaryReport::from_jobs(&jobs, stats))
}
