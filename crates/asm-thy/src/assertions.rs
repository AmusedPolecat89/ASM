use std::collections::BTreeMap;

use asm_core::errors::{AsmError, ErrorInfo};
use asm_gauge::GaugeReport;
use asm_int::{InteractionReport, RunningReport};
use asm_land::metrics::JobKpi;
use asm_land::report::SummaryReport;
use asm_spec::SpectrumReport;

use crate::hash::stable_hash_string;
use crate::policies::Policy;
use crate::report::{validate_checks, AssertionCheck, AssertionProvenance, AssertionReport};

fn assertion_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, message.into()))
}

/// Input bundle passed to [`run_assertions`].
#[derive(Debug, Default, Clone)]
pub struct AssertionInputs {
    /// Phase 11 spectrum report.
    pub spectrum: Option<SpectrumReport>,
    /// Phase 12 gauge report.
    pub gauge: Option<GaugeReport>,
    /// Phase 13 interaction report.
    pub interaction: Option<InteractionReport>,
    /// Phase 13 running report.
    pub running: Option<RunningReport>,
    /// Landscape summary report from Phase 14.
    pub summary: Option<SummaryReport>,
    /// Landscape KPIs collected across universes.
    pub kpis: Vec<JobKpi>,
}

impl AssertionInputs {
    /// Records a KPI snapshot for later aggregate checks.
    pub fn add_kpi(&mut self, kpi: JobKpi) {
        self.kpis.push(kpi);
    }
}

fn missing_input(name: &str) -> AsmError {
    assertion_error(
        "missing-input",
        format!("required assertion input `{name}` was not provided"),
    )
}

fn ward_commutator_bound(gauge: &GaugeReport, policy: &Policy) -> AssertionCheck {
    let metric = policy.round(gauge.ward.max_comm_norm.abs());
    let pass = metric <= policy.ward_tol;
    AssertionCheck {
        name: "ward_commutator_bound".to_string(),
        pass,
        metric,
        threshold: Some(policy.ward_tol),
        range: None,
        note: if pass {
            None
        } else {
            Some("ward commutator exceeds tolerance".to_string())
        },
    }
}

fn closure_residual(gauge: &GaugeReport, policy: &Policy) -> AssertionCheck {
    let metric = policy.round(gauge.closure.max_dev.abs());
    let pass = metric <= policy.closure_tol;
    AssertionCheck {
        name: "closure_residual".to_string(),
        pass,
        metric,
        threshold: Some(policy.closure_tol),
        range: None,
        note: if pass {
            None
        } else {
            Some("closure residual above configured tolerance".to_string())
        },
    }
}

fn dispersion_linear_limit(spec: &SpectrumReport, policy: &Policy) -> AssertionCheck {
    let metric = if spec.dispersion.k_grid.len() >= 2 && !spec.dispersion.modes.is_empty() {
        let k0 = spec.dispersion.k_grid[0];
        let k1 = spec.dispersion.k_grid[1];
        let omega0 = spec.dispersion.modes[0].omega;
        let omega1 = omega0 + spec.dispersion.c_est * (k1 - k0);
        let slope = if (k1 - k0).abs() <= 1e-9 {
            0.0
        } else {
            (omega1 - omega0) / (k1 - k0)
        };
        let denom = spec.dispersion.c_est.abs().max(1e-12);
        policy.round(((slope - spec.dispersion.c_est) / denom).abs())
    } else {
        0.0
    };
    let pass = metric <= policy.rel_tol_lin;
    AssertionCheck {
        name: "dispersion_linear_limit".to_string(),
        pass,
        metric,
        threshold: Some(policy.rel_tol_lin),
        range: None,
        note: if pass {
            None
        } else {
            Some("low-k dispersion deviates from linear limit".to_string())
        },
    }
}

fn correlation_gap_relation(spec: &SpectrumReport, policy: &Policy) -> AssertionCheck {
    let expected = if spec.dispersion.gap_proxy.abs() <= 1e-9 {
        spec.correlation.xi
    } else {
        1.0 / spec.dispersion.gap_proxy.abs()
    };
    let metric = policy.round((spec.correlation.xi - expected).abs());
    let pass = metric <= policy.abs_tol;
    AssertionCheck {
        name: "correlation_gap_relation".to_string(),
        pass,
        metric,
        threshold: Some(policy.abs_tol),
        range: None,
        note: if pass {
            None
        } else {
            Some("correlation length and gap proxy out of alignment".to_string())
        },
    }
}

fn couplings_fit_resid(interaction: &InteractionReport, policy: &Policy) -> AssertionCheck {
    let metric = policy.round(interaction.fit.fit_resid.abs());
    let pass = metric <= policy.fit_resid_max;
    AssertionCheck {
        name: "couplings_fit_resid".to_string(),
        pass,
        metric,
        threshold: Some(policy.fit_resid_max),
        range: None,
        note: if pass {
            None
        } else {
            Some("coupling fit residual exceeds configured maximum".to_string())
        },
    }
}

fn running_beta_sanity(running: &RunningReport, policy: &Policy) -> AssertionCheck {
    let mut max_beta = running
        .beta_summary
        .dg_dlog_mu
        .iter()
        .copied()
        .map(f64::abs)
        .fold(0.0, f64::max);
    max_beta = max_beta.max(running.beta_summary.dlambda_dlog_mu.abs());
    let mut drift: f64 = 0.0;
    for window in running.steps.windows(2) {
        if let [first, second] = window {
            for idx in 0..3 {
                let diff = (second.fit.g[idx] - first.fit.g[idx]).abs();
                drift = drift.max(diff);
            }
            drift = drift.max((second.fit.lambda_h - first.fit.lambda_h).abs());
        }
    }
    let metric = policy.round(max_beta.max(drift));
    let threshold = running.thresholds.beta_tolerance;
    let pass = metric <= threshold;
    AssertionCheck {
        name: "running_beta_sanity".to_string(),
        pass,
        metric,
        threshold: Some(threshold),
        range: None,
        note: if pass {
            None
        } else {
            Some("running beta summary exceeds configured tolerance".to_string())
        },
    }
}

fn landscape_filter_rate(summary: &SummaryReport, policy: &Policy) -> AssertionCheck {
    let metric = policy.round(summary.pass_rates.anthropic);
    let range = [policy.landscape_rate.min, policy.landscape_rate.max];
    let pass = policy.landscape_rate.contains(metric);
    AssertionCheck {
        name: "landscape_filter_rate".to_string(),
        pass,
        metric,
        threshold: None,
        range: Some(range),
        note: if pass {
            None
        } else {
            Some("anthropic pass rate outside configured interval".to_string())
        },
    }
}

fn collect_hashes(inputs: &AssertionInputs) -> Result<BTreeMap<String, String>, AsmError> {
    let mut hashes = BTreeMap::new();
    if let Some(spec) = &inputs.spectrum {
        hashes.insert("spectrum".to_string(), spec.analysis_hash.clone());
    }
    if let Some(gauge) = &inputs.gauge {
        hashes.insert("gauge".to_string(), gauge.analysis_hash.clone());
    }
    if let Some(interaction) = &inputs.interaction {
        hashes.insert("interaction".to_string(), interaction.analysis_hash.clone());
    }
    if let Some(running) = &inputs.running {
        hashes.insert("running".to_string(), running.running_hash.clone());
    }
    if !inputs.kpis.is_empty() {
        hashes.insert("kpis".to_string(), stable_hash_string(&inputs.kpis)?);
    }
    if let Some(summary) = &inputs.summary {
        hashes.insert("summary".to_string(), stable_hash_string(summary)?);
    }
    Ok(hashes)
}

/// Executes the configured assertions and returns a deterministic report.
pub fn run_assertions(
    inputs: &AssertionInputs,
    policy: &Policy,
) -> Result<AssertionReport, AsmError> {
    let mut checks = Vec::new();
    if policy.require_ward {
        let gauge = inputs
            .gauge
            .as_ref()
            .ok_or_else(|| missing_input("gauge"))?;
        checks.push(ward_commutator_bound(gauge, policy));
    } else if let Some(gauge) = &inputs.gauge {
        checks.push(ward_commutator_bound(gauge, policy));
    }

    if policy.require_closure {
        let gauge = inputs
            .gauge
            .as_ref()
            .ok_or_else(|| missing_input("gauge"))?;
        checks.push(closure_residual(gauge, policy));
    } else if let Some(gauge) = &inputs.gauge {
        checks.push(closure_residual(gauge, policy));
    }

    if let Some(spec) = &inputs.spectrum {
        checks.push(dispersion_linear_limit(spec, policy));
        checks.push(correlation_gap_relation(spec, policy));
    } else if policy.strict {
        return Err(missing_input("spectrum"));
    }

    if let Some(interaction) = &inputs.interaction {
        checks.push(couplings_fit_resid(interaction, policy));
    } else if policy.strict {
        return Err(missing_input("interaction"));
    }

    if let Some(running) = &inputs.running {
        checks.push(running_beta_sanity(running, policy));
    } else if policy.strict {
        return Err(missing_input("running"));
    }

    if let Some(summary) = &inputs.summary {
        checks.push(landscape_filter_rate(summary, policy));
    } else if policy.strict {
        return Err(missing_input("summary"));
    }

    validate_checks(&checks)?;
    let check_order = checks.iter().map(|check| check.name.clone()).collect();
    let provenance = AssertionProvenance::new(policy.clone(), collect_hashes(inputs)?, check_order);
    AssertionReport::new(checks, provenance)
}
