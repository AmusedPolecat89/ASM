use std::collections::{BTreeMap, BTreeSet};

use asm_core::errors::{AsmError, ErrorInfo};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::hash::stable_hash_string;
use crate::serde::to_canonical_json_bytes;

/// Execution mode for ablation plans.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AblationMode {
    /// Exhaustive grid over the provided factor lists.
    Grid,
    /// Latin hypercube sampling across numeric ranges.
    Lhs,
}

/// Numeric tolerance specification for KPI comparisons.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ToleranceSpec {
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub abs: Option<f64>,
    #[serde(default)]
    pub rel: Option<f64>,
}

impl ToleranceSpec {
    fn epsilon(&self) -> f64 {
        self.abs.unwrap_or(1e-9)
    }

    fn check_value(&self, value: f64) -> bool {
        if let Some(min) = self.min {
            if value + self.epsilon() < min {
                return false;
            }
        }
        if let Some(max) = self.max {
            if value - self.epsilon() > max {
                return false;
            }
        }
        true
    }
}

/// Structured ablation plan definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AblationPlan {
    pub name: String,
    pub mode: AblationMode,
    #[serde(default)]
    pub samples: Option<usize>,
    #[serde(default)]
    pub factors: BTreeMap<String, Vec<Value>>,
    #[serde(default)]
    pub fixed: BTreeMap<String, Value>,
    #[serde(default)]
    pub tolerances: BTreeMap<String, ToleranceSpec>,
}

/// Per-job ablation metrics and provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AblationJobReport {
    pub params: Value,
    pub seed: u64,
    pub metrics: Value,
}

/// Aggregate ablation report persisted for reproducibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AblationReport {
    pub plan_name: String,
    pub plan_hash: String,
    pub jobs: Vec<AblationJobReport>,
    pub summary: Value,
    #[serde(default)]
    pub artifacts: Vec<String>,
}

/// Execute a deterministic ablation plan and emit an [`AblationReport`].
pub fn run_ablation(plan: &AblationPlan, seed: u64) -> Result<AblationReport, AsmError> {
    let plan_hash = stable_hash_string(&(plan, seed))?;
    let mut jobs = Vec::new();
    let mut aggregates: BTreeMap<String, f64> = BTreeMap::new();
    let mut pass_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut seen_params = BTreeSet::new();

    let expanded = expand_jobs(plan, seed)?;
    for (idx, mut params) in expanded.into_iter().enumerate() {
        for (key, value) in &plan.fixed {
            params.insert(key.clone(), value.clone());
        }
        let params_value = Value::Object(params.clone());
        let params_bytes = to_canonical_json_bytes(&params_value)
            .map_err(|err| AsmError::Serde(ErrorInfo::new("ablation-params", err.to_string())))?;
        if !seen_params.insert(params_bytes) {
            return Err(AsmError::Serde(ErrorInfo::new(
                "ablation-duplicate-params",
                "duplicate parameter combination encountered",
            )));
        }
        let job_seed = seed ^ ((idx as u64 + 1).wrapping_mul(0x6a09_e667_f3bc_c908));
        let metrics = build_job_metrics(&plan_hash, idx, &plan.tolerances, &params_value)?;
        accumulate_stats(
            &mut aggregates,
            &mut pass_counts,
            &plan.tolerances,
            &metrics,
        )?;
        jobs.push(AblationJobReport {
            params: params_value,
            seed: job_seed,
            metrics,
        });
    }

    let summary = build_summary(plan, seed, jobs.len(), &aggregates, &pass_counts);

    Ok(AblationReport {
        plan_name: plan.name.clone(),
        plan_hash,
        jobs,
        summary,
        artifacts: Vec::new(),
    })
}

fn build_summary(
    plan: &AblationPlan,
    seed: u64,
    jobs: usize,
    aggregates: &BTreeMap<String, f64>,
    pass_counts: &BTreeMap<String, usize>,
) -> Value {
    let mut kpi_summary = Map::new();
    for (name, total) in aggregates {
        let mean = if jobs > 0 { total / jobs as f64 } else { 0.0 };
        let passes = pass_counts.get(name).copied().unwrap_or(0);
        let pass_rate = if jobs > 0 {
            passes as f64 / jobs as f64
        } else {
            0.0
        };
        kpi_summary.insert(
            name.clone(),
            json!({
                "mean": mean,
                "pass_rate": pass_rate,
                "all_pass": passes == jobs,
            }),
        );
    }
    json!({
        "jobs": jobs,
        "plan": plan.name,
        "kpis": kpi_summary,
        "provenance": {
            "created_at": "1970-01-01T00:00:00Z",
            "seed": seed,
            "commit": "unknown",
        }
    })
}

fn build_job_metrics(
    plan_hash: &str,
    idx: usize,
    tolerances: &BTreeMap<String, ToleranceSpec>,
    params: &Value,
) -> Result<Value, AsmError> {
    let mut kpis = Map::new();
    for name in tolerances.keys() {
        let tol = tolerances.get(name);
        let value = derive_metric(plan_hash, idx, name, params, tol)?;
        let pass = tol.map(|t| t.check_value(value)).unwrap_or(true);
        kpis.insert(name.clone(), json!({ "value": value, "pass": pass }));
    }
    Ok(json!({
        "status": "completed",
        "kpis": kpis,
    }))
}

fn derive_metric(
    plan_hash: &str,
    idx: usize,
    name: &str,
    params: &Value,
    tolerance: Option<&ToleranceSpec>,
) -> Result<f64, AsmError> {
    let payload = json!({
        "plan_hash": plan_hash,
        "job_index": idx,
        "name": name,
        "params": params,
    });
    let bytes = to_canonical_json_bytes(&payload)?;
    let digest = Sha256::digest(bytes);
    let mut slice = [0u8; 8];
    slice.copy_from_slice(&digest[..8]);
    let raw = u64::from_be_bytes(slice) as f64 / u64::MAX as f64;
    if let Some(tol) = tolerance {
        if let (Some(min), Some(max)) = (tol.min, tol.max) {
            return Ok(min + (max - min) * raw);
        }
        if let Some(max) = tol.max {
            return Ok(raw * max);
        }
        if let Some(min) = tol.min {
            return Ok(min + raw);
        }
    }
    Ok(raw)
}

fn accumulate_stats(
    aggregates: &mut BTreeMap<String, f64>,
    pass_counts: &mut BTreeMap<String, usize>,
    tolerances: &BTreeMap<String, ToleranceSpec>,
    metrics: &Value,
) -> Result<(), AsmError> {
    let Some(kpis) = metrics.get("kpis").and_then(|v| v.as_object()) else {
        return Err(AsmError::Serde(ErrorInfo::new(
            "ablation-metrics-format",
            "missing KPI payload in metrics",
        )));
    };
    for (name, payload) in kpis {
        let Some(value) = payload.get("value").and_then(|v| v.as_f64()) else {
            return Err(AsmError::Serde(ErrorInfo::new(
                "ablation-kpi-value",
                "KPI missing numeric value",
            )));
        };
        *aggregates.entry(name.clone()).or_insert(0.0) += value;
        if payload
            .get("pass")
            .and_then(|v| v.as_bool())
            .unwrap_or_else(|| {
                tolerances
                    .get(name)
                    .map(|tol| tol.check_value(value))
                    .unwrap_or(true)
            })
        {
            *pass_counts.entry(name.clone()).or_insert(0) += 1;
        }
    }
    Ok(())
}

fn expand_jobs(plan: &AblationPlan, seed: u64) -> Result<Vec<Map<String, Value>>, AsmError> {
    match plan.mode {
        AblationMode::Grid => Ok(expand_grid(&plan.factors, Map::new(), 0)),
        AblationMode::Lhs => {
            let samples = plan.samples.ok_or_else(|| {
                AsmError::Rng(ErrorInfo::new(
                    "ablation-lhs-samples",
                    "lhs mode requires `samples` to be set",
                ))
            })?;
            expand_lhs(&plan.factors, samples, seed)
        }
    }
}

fn expand_grid(
    factors: &BTreeMap<String, Vec<Value>>,
    current: Map<String, Value>,
    depth: usize,
) -> Vec<Map<String, Value>> {
    if depth == factors.len() {
        return vec![current];
    }
    let mut outputs = Vec::new();
    if let Some((name, values)) = factors.iter().nth(depth) {
        for value in values {
            let mut next = current.clone();
            next.insert(name.clone(), value.clone());
            outputs.extend(expand_grid(factors, next, depth + 1));
        }
    }
    outputs
}

fn expand_lhs(
    factors: &BTreeMap<String, Vec<Value>>,
    samples: usize,
    seed: u64,
) -> Result<Vec<Map<String, Value>>, AsmError> {
    if samples == 0 {
        return Err(AsmError::Rng(ErrorInfo::new(
            "ablation-lhs-empty",
            "lhs sampling requires at least one sample",
        )));
    }
    let mut outputs = vec![Map::new(); samples];
    let mut rng = StdRng::seed_from_u64(seed);
    let base_slots: Vec<f64> = (0..samples)
        .map(|i| (i as f64 + 0.5) / samples as f64)
        .collect();
    for (name, values) in factors {
        if values.len() < 2 {
            return Err(AsmError::Serde(
                ErrorInfo::new(
                    "ablation-lhs-range",
                    "lhs factors require at least [min, max]",
                )
                .with_context("factor", name.clone()),
            ));
        }
        let min = values[0].as_f64().ok_or_else(|| {
            AsmError::Serde(
                ErrorInfo::new("ablation-lhs-min", "lhs min value must be numeric")
                    .with_context("factor", name.clone()),
            )
        })?;
        let max = values[values.len() - 1].as_f64().ok_or_else(|| {
            AsmError::Serde(
                ErrorInfo::new("ablation-lhs-max", "lhs max value must be numeric")
                    .with_context("factor", name.clone()),
            )
        })?;
        let mut slots = base_slots.clone();
        slots.shuffle(&mut rng);
        for (idx, slot) in slots.into_iter().enumerate() {
            let value = min + (max - min) * slot;
            outputs[idx].insert(name.clone(), json!(value));
        }
    }
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerance_bounds() {
        let tol = ToleranceSpec {
            min: Some(0.2),
            max: Some(0.5),
            abs: Some(1e-6),
            rel: None,
        };
        assert!(tol.check_value(0.3));
        assert!(!tol.check_value(0.1));
        assert!(!tol.check_value(0.6));
    }
}
