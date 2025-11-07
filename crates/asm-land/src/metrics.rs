use serde::{Deserialize, Serialize};

/// Deterministic KPI snapshot extracted from a job's artefacts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobKpi {
    /// Final energy recorded by the sampler.
    pub energy_final: f64,
    /// Central charge estimate derived from the interaction stage.
    pub c_est: f64,
    /// Proxy gap measurement aggregated from the spectrum.
    pub gap_proxy: f64,
    /// Correlation length estimator.
    pub xi: f64,
    /// Whether gauge closure checks passed.
    pub closure_pass: bool,
    /// Whether Ward identities passed.
    pub ward_pass: bool,
    /// Factor labels detected in the gauge stage.
    pub factors: Vec<String>,
    /// Selected gauge couplings at the reference scale.
    pub g: Vec<f64>,
    /// Higgs self coupling estimate.
    pub lambda_h: f64,
}

impl JobKpi {
    /// Synthesises a deterministic KPI snapshot from the provided identifiers.
    pub fn synthesise(seed: u64, rule_id: u64) -> Self {
        let base = seed ^ (rule_id.wrapping_mul(0x9e3779b97f4a7c15));
        let norm = (base % 10_000) as f64 / 10_000.0;
        let energy_final = -1.0 - (norm * 0.1);
        let c_est = 0.8 + norm * 0.4;
        let gap_proxy = 0.05 + norm * 0.2;
        let xi = 1.0 + norm * 0.5;
        let closure_pass = (base & 0b1) == 0;
        let ward_pass = (base & 0b10) == 0;
        let factors = if (base & 0b100) == 0 {
            vec!["u1".to_string(), "su2".to_string()]
        } else {
            vec!["u1".to_string()]
        };
        let g1 = 0.1 + norm * 0.05;
        let g2 = 0.2 + norm * 0.05;
        let g3 = 0.3 + norm * 0.05;
        let lambda_h = 0.01 + norm * 0.02;
        Self {
            energy_final,
            c_est,
            gap_proxy,
            xi,
            closure_pass,
            ward_pass,
            factors,
            g: vec![g1, g2, g3],
            lambda_h,
        }
    }
}

impl Default for JobKpi {
    fn default() -> Self {
        Self {
            energy_final: 0.0,
            c_est: 0.0,
            gap_proxy: 0.0,
            xi: 0.0,
            closure_pass: false,
            ward_pass: false,
            factors: Vec::new(),
            g: Vec::new(),
            lambda_h: 0.0,
        }
    }
}
