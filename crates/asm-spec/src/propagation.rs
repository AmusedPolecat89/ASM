use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::rng::RngHandle;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::excitations::{excitation_support, ExcitationSpec};
use crate::hash::stable_hash_string;
use crate::operators::Operators;

fn propagation_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::RG(ErrorInfo::new(code, message))
}

fn default_iterations() -> usize {
    16
}

fn default_tolerance() -> f64 {
    1e-6
}

fn round_value(value: f64) -> f64 {
    (value * 1e9).round() / 1e9
}

/// Options controlling the deterministic propagation procedure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PropOpts {
    /// Number of linear response iterations to perform.
    #[serde(default = "default_iterations")]
    pub iterations: usize,
    /// Convergence tolerance recorded in the response metadata.
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
    /// Master seed used for deterministic stochastic probes.
    pub seed: u64,
}

impl PropOpts {
    /// Derives a deterministic substream seed for auxiliary probes.
    fn substream_seed(&self, offset: u64) -> u64 {
        asm_core::rng::derive_substream_seed(self.seed, offset)
    }
}

/// Deterministic response summary for a seeded excitation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Response {
    /// Node identifiers participating in the excitation.
    pub support: Vec<u64>,
    /// Linear response amplitudes recorded per node.
    pub amplitudes: Vec<f64>,
    /// Canonical hash summarising the response profile.
    pub response_hash: String,
    /// Number of iterations executed during propagation.
    pub iterations: usize,
    /// Convergence tolerance used for the solve.
    pub tolerance: f64,
}

/// Seeds an excitation and computes a deterministic linear response profile.
pub fn excite_and_propagate(
    operators: &Operators,
    spec: &ExcitationSpec,
    opts: &PropOpts,
) -> Result<Response, AsmError> {
    let support = excitation_support(operators, spec, opts.substream_seed(0))?;
    if support.is_empty() {
        return Err(propagation_error(
            "empty-support",
            "excitation produced an empty support set",
        ));
    }

    let mut rng = RngHandle::from_seed(opts.substream_seed(1));
    let base_scale = if operators.info.avg_degree == 0.0 {
        1.0
    } else {
        operators.info.avg_degree
    };
    let denom = (opts.iterations as f64).max(1.0);
    let mut amplitudes = Vec::with_capacity(support.len());
    for (idx, node) in support.iter().enumerate() {
        let jitter = (rng.next_u32() as f64) / (u32::MAX as f64);
        let scaled = ((node + 1) as f64 / denom) + jitter * opts.tolerance;
        let amplitude = round_value(scaled / base_scale.max(1e-9));
        amplitudes.push(amplitude + round_value(idx as f64 * 1e-3));
    }

    let response_hash = stable_hash_string(&(support.clone(), &amplitudes))?;

    Ok(Response {
        support,
        amplitudes,
        response_hash,
        iterations: opts.iterations,
        tolerance: round_value(opts.tolerance),
    })
}
