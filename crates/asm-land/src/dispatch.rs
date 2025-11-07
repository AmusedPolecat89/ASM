use std::fs;
use std::path::{Path, PathBuf};

use asm_core::errors::{AsmError, ErrorInfo};

use std::sync::Arc;

use rayon::prelude::*;

use crate::filters::FilterDecision;
use crate::filters::{load_filters, FilterSpec};
use crate::plan::{load_plan, OutputLayout, Plan, RuleSpec};
use crate::report::{JobReport, JobStatus, LandscapeReport};
use crate::serde::{from_json_slice, to_canonical_json_bytes};
use crate::stages::{synthesise_stage_outputs, StageHashes, StageOutputs};
use crate::stat::StatsSummary;

fn io_error(code: &str, err: impl ToString) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, err.to_string()))
}

/// Options governing landscape execution.
#[derive(Debug, Clone)]
pub struct RunOpts {
    /// Resume partially completed runs when true.
    pub resume: bool,
    /// Number of jobs to execute in parallel (currently advisory).
    pub concurrency: usize,
    /// Maximum number of deterministic retries per job.
    pub max_retries: u32,
}

impl Default for RunOpts {
    fn default() -> Self {
        Self {
            resume: false,
            concurrency: 1,
            max_retries: 2,
        }
    }
}

/// Executes a landscape plan, emitting deterministic artefacts on disk.
pub fn run_plan(plan: &Plan, out: &Path, opts: &RunOpts) -> Result<LandscapeReport, AsmError> {
    fs::create_dir_all(out).map_err(|err| io_error("plan_out_dir", err))?;
    let filter_spec = Arc::new(load_filters(&plan.filters_path())?);
    let jobs = enumerate_jobs(plan, out);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(opts.concurrency.max(1))
        .build()
        .map_err(|err| io_error("thread_pool", err))?;

    let results: Result<Vec<_>, AsmError> = pool.install(|| {
        jobs.par_iter()
            .enumerate()
            .map(|(index, job)| -> Result<(usize, JobResult), AsmError> {
                let result = process_job(
                    plan,
                    filter_spec.as_ref(),
                    &job.dir,
                    job.seed,
                    &job.rule,
                    opts,
                )?;
                Ok((index, result))
            })
            .collect()
    });

    let mut ordered = results?;
    ordered.sort_by_key(|(index, _)| *index);

    let mut job_reports = Vec::with_capacity(ordered.len());
    let mut stats_kpis = Vec::new();
    for (_, result) in ordered {
        if let Some(kpi) = result.stats_kpi {
            stats_kpis.push(kpi);
        }
        job_reports.push(result.report);
    }

    job_reports.sort_by(|a, b| a.seed.cmp(&b.seed).then(a.rule_id.cmp(&b.rule_id)));
    let stats = StatsSummary::from_kpis(&stats_kpis);
    let report = LandscapeReport::new(plan, job_reports, stats, (*filter_spec).clone());
    let report_bytes = to_canonical_json_bytes(&report)?;
    fs::write(out.join("landscape_report.json"), report_bytes)
        .map_err(|err| io_error("landscape_report_write", err))?;
    Ok(report)
}

/// Loads a plan from disk and executes it.
pub fn run_plan_from_path(
    plan_path: &Path,
    out: &Path,
    opts: &RunOpts,
) -> Result<LandscapeReport, AsmError> {
    let plan = load_plan(plan_path)?;
    run_plan(&plan, out, opts)
}

fn process_job(
    plan: &Plan,
    filter_spec: &FilterSpec,
    job_dir: &Path,
    seed: u64,
    rule: &RuleSpec,
    opts: &RunOpts,
) -> Result<JobResult, AsmError> {
    if opts.resume && job_complete(job_dir)? {
        let existing = load_existing_job(job_dir)?;
        let filters = filter_spec.evaluate(&existing.kpi);
        return Ok(JobResult {
            stats_kpi: Some(existing.kpi.clone()),
            report: JobReport {
                seed,
                rule_id: rule.id,
                status: existing.status,
                hashes: existing.hashes,
                kpis: existing.kpi,
                filters,
            },
        });
    }

    fs::create_dir_all(job_dir).map_err(|err| io_error("job_dir", err))?;
    match execute_with_retries(plan, job_dir, seed, rule, opts.max_retries) {
        Ok((outputs, attempts)) => {
            let filters = filter_spec.evaluate(&outputs.kpi);
            let status = JobStatus::success(attempts);
            let kpi_for_stats = outputs.kpi.clone();
            persist_stage_outputs(plan, job_dir, &outputs, &status, &filters)?;
            Ok(JobResult {
                stats_kpi: Some(kpi_for_stats),
                report: JobReport {
                    seed,
                    rule_id: rule.id,
                    status,
                    hashes: outputs.hashes,
                    kpis: outputs.kpi,
                    filters,
                },
            })
        }
        Err(failure) => {
            let status = JobStatus::failed(failure.attempts, failure.error);
            persist_failure(job_dir, &status)?;
            Ok(JobResult {
                stats_kpi: None,
                report: JobReport {
                    seed,
                    rule_id: rule.id,
                    status,
                    hashes: StageHashes::default(),
                    kpis: crate::metrics::JobKpi::default(),
                    filters: FilterDecision::default(),
                },
            })
        }
    }
}

fn execute_with_retries(
    plan: &Plan,
    job_dir: &Path,
    seed: u64,
    rule: &RuleSpec,
    max_retries: u32,
) -> Result<(StageOutputs, u32), JobFailure> {
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        let result = synthesise_stage_outputs(
            derive_seed(seed, attempt),
            rule.id,
            plan.sampler.sweeps,
            plan.spectrum.modes,
            plan.spectrum.k_points,
        );
        match result {
            Ok(outputs) => {
                cleanup_incomplete(job_dir);
                return Ok((outputs, attempt));
            }
            Err(_err) if attempt < max_retries.max(1) => {
                cleanup_incomplete(job_dir);
                continue;
            }
            Err(err) => {
                cleanup_incomplete(job_dir);
                return Err(JobFailure {
                    attempts: attempt,
                    error: err.to_string(),
                });
            }
        }
    }
}

fn persist_stage_outputs(
    plan: &Plan,
    job_dir: &Path,
    outputs: &StageOutputs,
    status: &JobStatus,
    filters: &FilterDecision,
) -> Result<(), AsmError> {
    if plan.outputs.keep_intermediate {
        write_json(job_dir.join("mcmc/manifest.json"), &outputs.mcmc)?;
        write_json(
            job_dir.join("spectrum/spectrum_report.json"),
            &outputs.spectrum,
        )?;
        write_json(job_dir.join("gauge/gauge_report.json"), &outputs.gauge)?;
        write_json(
            job_dir.join("interact/interaction_report.json"),
            &outputs.interaction,
        )?;
    } else {
        fs::create_dir_all(job_dir).map_err(|err| io_error("job_dir", err))?;
    }
    write_json(job_dir.join("kpi.json"), &outputs.kpi)?;
    write_json(job_dir.join("hashes.json"), &outputs.hashes)?;
    write_json(job_dir.join("filters.json"), filters)?;
    write_json(job_dir.join("status.json"), status)?;
    Ok(())
}

fn write_json<T: serde::Serialize>(path: PathBuf, value: &T) -> Result<(), AsmError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| io_error("stage_dir", err))?;
    }
    let bytes = to_canonical_json_bytes(value)?;
    fs::write(path, bytes).map_err(|err| io_error("stage_write", err))
}

fn job_dir(base: &Path, layout: OutputLayout, seed: u64, rule_id: u64) -> PathBuf {
    match layout {
        OutputLayout::Flat => base.join(format!("{}_{}", seed, rule_id)),
        OutputLayout::PerSeed => base.join(seed.to_string()).join(rule_id.to_string()),
    }
}

fn job_complete(job_dir: &Path) -> Result<bool, AsmError> {
    let status_path = job_dir.join("status.json");
    if !status_path.exists() {
        return Ok(false);
    }
    let bytes = fs::read(status_path).map_err(|err| io_error("status_read", err))?;
    let status: JobStatus = from_json_slice(&bytes)?;
    if status.state != crate::report::JobState::Complete {
        return Ok(false);
    }
    Ok(job_dir.join("kpi.json").exists() && job_dir.join("hashes.json").exists())
}

fn load_existing_job(job_dir: &Path) -> Result<ExistingJob, AsmError> {
    let kpi_bytes = fs::read(job_dir.join("kpi.json")).map_err(|err| io_error("kpi_read", err))?;
    let hashes_bytes =
        fs::read(job_dir.join("hashes.json")).map_err(|err| io_error("hashes_read", err))?;
    let status_bytes =
        fs::read(job_dir.join("status.json")).map_err(|err| io_error("status_read", err))?;
    let kpi = from_json_slice(&kpi_bytes)?;
    let hashes: StageHashes = from_json_slice(&hashes_bytes)?;
    let status: JobStatus = from_json_slice(&status_bytes)?;
    Ok(ExistingJob {
        kpi,
        hashes,
        status,
    })
}

fn persist_failure(job_dir: &Path, status: &JobStatus) -> Result<(), AsmError> {
    fs::create_dir_all(job_dir).map_err(|err| io_error("job_dir", err))?;
    cleanup_incomplete(job_dir);
    write_json(job_dir.join("status.json"), status)?;
    Ok(())
}

fn cleanup_incomplete(job_dir: &Path) {
    let _ = fs::remove_file(job_dir.join("kpi.json"));
    let _ = fs::remove_file(job_dir.join("hashes.json"));
    let _ = fs::remove_file(job_dir.join("filters.json"));
}

fn derive_seed(seed: u64, attempt: u32) -> u64 {
    if attempt <= 1 {
        seed
    } else {
        seed ^ ((attempt - 1) as u64).wrapping_mul(0x9e3779b97f4a7c15)
    }
}

fn enumerate_jobs(plan: &Plan, out: &Path) -> Vec<JobSpec> {
    let mut jobs = Vec::new();
    for rule in plan.rules() {
        for &seed in &plan.seeds {
            jobs.push(JobSpec {
                seed,
                rule: rule.clone(),
                dir: job_dir(out, plan.outputs.layout, seed, rule.id),
            });
        }
    }
    jobs
}

struct JobSpec {
    seed: u64,
    rule: RuleSpec,
    dir: PathBuf,
}

struct JobResult {
    report: JobReport,
    stats_kpi: Option<crate::metrics::JobKpi>,
}

struct JobFailure {
    attempts: u32,
    error: String,
}

struct ExistingJob {
    kpi: crate::metrics::JobKpi,
    hashes: StageHashes,
    status: JobStatus,
}
