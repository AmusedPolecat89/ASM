use asm_core::RngHandle;
use rand::RngCore;

use crate::config::{LadderConfig, LadderPolicy};

/// Builds a deterministic temperature ladder following the provided policy.
pub fn build_ladder(config: &LadderConfig) -> Vec<f64> {
    match &config.policy {
        LadderPolicy::Geometric { ratio } => {
            let ratio = (*ratio).max(1.01);
            let mut ladder = Vec::with_capacity(config.replicas.max(1));
            let mut temp = config.base_temperature;
            for _ in 0..config.replicas.max(1) {
                ladder.push(temp.max(1e-6));
                temp *= ratio;
            }
            ladder
        }
        LadderPolicy::Manual { temperatures } => {
            if temperatures.is_empty() {
                vec![config.base_temperature]
            } else {
                temperatures.clone()
            }
        }
    }
}

/// Computes the Metropolis acceptance for exchanging two replicas.
pub fn exchange_acceptance(energy_a: f64, temp_a: f64, energy_b: f64, temp_b: f64) -> f64 {
    let beta_a = 1.0 / temp_a.max(1e-9);
    let beta_b = 1.0 / temp_b.max(1e-9);
    let delta = (beta_a - beta_b) * (energy_b - energy_a);
    (-delta).exp().min(1.0)
}

/// Attempts a replica exchange using the provided RNG handle.
pub fn attempt_exchange(
    energy_a: f64,
    temp_a: f64,
    energy_b: f64,
    temp_b: f64,
    rng: &mut RngHandle,
) -> (bool, f64) {
    let acceptance = exchange_acceptance(energy_a, temp_a, energy_b, temp_b);
    let draw = rng.next_u64() as f64 / u64::MAX as f64;
    (draw < acceptance, acceptance)
}
