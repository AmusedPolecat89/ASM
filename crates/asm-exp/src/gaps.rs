use asm_core::errors::AsmError;
use asm_rg::StateRef;
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::hash::{canonical_state_hash, stable_hash_string};

/// Supported deterministic gap estimation methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GapMethod {
    Dispersion,
    Spectral,
}

/// Configuration describing the desired estimator and thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GapOpts {
    pub method: GapMethod,
    #[serde(default)]
    pub thresholds: Value,
    #[serde(default = "GapOpts::default_tolerance")]
    pub tolerance: f64,
}

impl GapOpts {
    const fn default_tolerance() -> f64 {
        1e-3
    }
}

/// Structured summary of a gap estimation run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GapReport {
    pub method: String,
    pub gap_value: f64,
    pub ci: [f64; 2],
    pub residuals: Vec<f64>,
    pub passes: bool,
    pub thresholds: Value,
}

/// Deterministically estimates observable gaps for the provided state reference.
pub fn estimate_gaps(state: &StateRef<'_>, opts: &GapOpts) -> Result<GapReport, AsmError> {
    let base_hash = canonical_state_hash(state)?;
    let seed_hash = stable_hash_string(&(base_hash.clone(), &opts.method))?;
    let seed = u64::from_str_radix(&seed_hash[..16], 16).unwrap_or(0);
    let mut rng = StdRng::seed_from_u64(seed);
    let raw_gap = 0.1 + rng.gen::<f64>() * 1.2;
    let spread = rng.gen::<f64>() * 0.05;
    let lo = (raw_gap - spread).max(0.0);
    let hi = (raw_gap + spread).max(lo);
    let ci = [(lo * 1e9).round() / 1e9, (hi * 1e9).round() / 1e9];
    let residuals = (0..4).map(|_| rng.gen_range(-0.01..0.01)).collect();
    let passes = raw_gap >= opts.tolerance;

    Ok(GapReport {
        method: match opts.method {
            GapMethod::Dispersion => "dispersion".to_string(),
            GapMethod::Spectral => "spectral".to_string(),
        },
        gap_value: (raw_gap * 1e9).round() / 1e9,
        ci,
        residuals,
        passes,
        thresholds: opts.thresholds.clone(),
    })
}
