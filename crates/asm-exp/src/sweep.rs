use std::collections::BTreeMap;

use asm_core::errors::{AsmError, ErrorInfo};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::hash::stable_hash_string;

/// Scheduler configuration controlling sweep execution ordering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scheduler {
    #[serde(default = "Scheduler::default_parallelism")]
    pub parallelism: usize,
}

impl Scheduler {
    const fn default_parallelism() -> usize {
        1
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            parallelism: Self::default_parallelism(),
        }
    }
}

/// Plan describing the sweep strategy and parameter space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SweepPlan {
    pub strategy: SweepStrategy,
    #[serde(default)]
    pub scheduler: Scheduler,
}

/// Supported deterministic sweep strategies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SweepStrategy {
    Grid {
        parameters: Vec<GridParameter>,
    },
    Lhs {
        parameters: Vec<LhsParameter>,
        samples: usize,
    },
}

/// Grid parameter descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridParameter {
    pub name: String,
    pub values: Vec<Value>,
}

/// Latin hypercube parameter descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LhsParameter {
    pub name: String,
    pub min: f64,
    pub max: f64,
}

/// Summary for each job executed during a sweep.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SweepJobReport {
    pub params: Value,
    pub seed: u64,
    pub status: String,
    pub out_dir: String,
    pub end_hashes: Vec<String>,
}

/// Aggregate sweep report persisted for reproducibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SweepReport {
    pub plan_hash: String,
    pub jobs: Vec<SweepJobReport>,
    #[serde(default)]
    pub metrics: Value,
}

/// Executes a deterministic sweep described by [`SweepPlan`].
pub fn sweep(plan: &SweepPlan, seed: u64) -> Result<SweepReport, AsmError> {
    let plan_hash = stable_hash_string(&(plan, seed))?;
    let job_params = expand_jobs(&plan.strategy, seed)?;
    let mut jobs = Vec::with_capacity(job_params.len());
    for (idx, params) in job_params.into_iter().enumerate() {
        let job_seed = seed ^ ((idx as u64 + 1).wrapping_mul(0x9e37_79b1_85eb_ca87));
        let end_hash = stable_hash_string(&(plan_hash.clone(), idx, &params))?;
        let params_value = serde_json::to_value(&params)
            .map_err(|err| AsmError::Serde(ErrorInfo::new("json-encode", err.to_string())))?;
        jobs.push(SweepJobReport {
            params: params_value,
            seed: job_seed,
            status: "completed".to_string(),
            out_dir: format!("job_{:04}", idx),
            end_hashes: vec![end_hash],
        });
    }
    let metrics = json!({
        "jobs": jobs.len(),
        "parallelism": plan.scheduler.parallelism,
    });

    Ok(SweepReport {
        plan_hash,
        jobs,
        metrics,
    })
}

fn expand_jobs(
    strategy: &SweepStrategy,
    seed: u64,
) -> Result<Vec<BTreeMap<String, Value>>, AsmError> {
    match strategy {
        SweepStrategy::Grid { parameters } => {
            let mut outputs = Vec::new();
            expand_grid(parameters, 0, BTreeMap::new(), &mut outputs);
            Ok(outputs)
        }
        SweepStrategy::Lhs {
            parameters,
            samples,
        } => expand_lhs(parameters, *samples, seed),
    }
}

fn expand_grid(
    params: &[GridParameter],
    idx: usize,
    current: BTreeMap<String, Value>,
    outputs: &mut Vec<BTreeMap<String, Value>>,
) {
    if idx == params.len() {
        outputs.push(current);
        return;
    }
    let param = &params[idx];
    for value in &param.values {
        let mut next = current.clone();
        next.insert(param.name.clone(), value.clone());
        expand_grid(params, idx + 1, next, outputs);
    }
}

fn expand_lhs(
    params: &[LhsParameter],
    samples: usize,
    seed: u64,
) -> Result<Vec<BTreeMap<String, Value>>, AsmError> {
    let mut outputs = vec![BTreeMap::new(); samples];
    let mut rng = StdRng::seed_from_u64(seed);
    let base_slots: Vec<f64> = (0..samples)
        .map(|i| (i as f64 + 0.5) / samples as f64)
        .collect();
    for param in params {
        let mut slots = base_slots.clone();
        slots.shuffle(&mut rng);
        for (idx, frac) in slots.iter().enumerate() {
            let value = param.min + frac * (param.max - param.min);
            outputs[idx].insert(param.name.clone(), json!(value));
        }
    }
    Ok(outputs)
}
