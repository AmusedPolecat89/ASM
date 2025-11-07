use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::rng::{derive_substream_seed, RngHandle};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::hash::{round_f64, seed_from_hash, stable_hash_string};
use crate::prepare::PreparedState;

fn kernel_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Code(ErrorInfo::new(code, message.into()))
}

fn default_steps() -> usize {
    256
}

fn default_dt() -> f64 {
    0.01
}

fn default_tolerance() -> f64 {
    1e-6
}

/// Kernel execution mode used for determinism guidance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum KernelMode {
    /// Light mode intended for CI.
    #[default]
    Light,
    /// Full fidelity mode used during HPC runs.
    Full,
    /// Fast exploratory mode.
    Fast,
}

/// Discrete trajectory step recorded during propagation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrajectoryStep {
    /// Step index.
    pub step: usize,
    /// Simulation time associated with the step.
    pub time: f64,
    /// Normalisation after applying the kernel.
    pub norm: f64,
    /// Phase accumulator at this step.
    pub phase: f64,
}

/// Metadata summarising a trajectory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrajectoryMeta {
    /// Total number of steps executed.
    pub steps: usize,
    /// Total simulated time.
    pub total_time: f64,
    /// Final norm recorded for the state.
    pub final_norm: f64,
    /// Stable hash of the trajectory contents.
    pub traj_hash: String,
}

/// Propagation trajectory produced by the kernel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Trajectory {
    /// Metadata summarising the trajectory.
    pub meta: TrajectoryMeta,
    /// Optional per-step samples.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<TrajectoryStep>,
}

/// Deterministic kernel configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KernelOpts {
    /// Number of discrete propagation steps.
    #[serde(default = "default_steps")]
    pub steps: usize,
    /// Time increment applied at each step.
    #[serde(default = "default_dt")]
    pub dt: f64,
    /// Error tolerance guiding stability heuristics.
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
    /// Whether to retain the full trajectory.
    #[serde(default)]
    pub save_trajectory: bool,
    /// Execution mode used for provenance.
    #[serde(default)]
    pub mode: KernelMode,
}

impl Default for KernelOpts {
    fn default() -> Self {
        Self {
            steps: default_steps(),
            dt: default_dt(),
            tolerance: default_tolerance(),
            save_trajectory: true,
            mode: KernelMode::Light,
        }
    }
}

fn effective_steps(opts: &KernelOpts) -> usize {
    match opts.mode {
        KernelMode::Light => opts.steps.min(128),
        KernelMode::Fast => opts.steps.min(64),
        KernelMode::Full => opts.steps,
    }
}

fn integrate_phase(rng: &mut RngHandle, tolerance: f64) -> f64 {
    let jitter = (rng.gen::<f64>() - 0.5) * tolerance.sqrt();
    round_f64(jitter)
}

/// Applies the deterministic interaction kernel producing a trajectory.
pub fn evolve(state: &PreparedState, opts: &KernelOpts) -> Result<Trajectory, AsmError> {
    if opts.steps == 0 {
        return Err(kernel_error(
            "zero-steps",
            "kernel must run for at least one step",
        ));
    }
    if !opts.dt.is_finite() || opts.dt <= 0.0 {
        return Err(kernel_error(
            "invalid-dt",
            "time step must be positive and finite",
        ));
    }

    let steps = effective_steps(opts);
    let seed = seed_from_hash(&state.prep_hash);
    let mut rng = RngHandle::from_seed(derive_substream_seed(seed, 2));
    let mut norm = state.norm;
    let decay = 1.0 / (steps as f64 + 1.0);
    let mut history = Vec::new();
    let mut time = 0.0;
    for step in 0..steps {
        time += opts.dt;
        let phase = integrate_phase(&mut rng, opts.tolerance);
        norm = round_f64((norm * (1.0 - decay)).max(0.0));
        if opts.save_trajectory {
            history.push(TrajectoryStep {
                step,
                time: round_f64(time),
                norm,
                phase,
            });
        }
    }

    let meta = TrajectoryMeta {
        steps,
        total_time: round_f64(time),
        final_norm: norm,
        traj_hash: stable_hash_string(&(&state.prep_hash, steps, round_f64(time), norm, &history))?,
    };

    Ok(Trajectory {
        meta,
        steps: history,
    })
}
