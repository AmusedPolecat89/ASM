#![deny(missing_docs)]
#![doc = include_str!("../docs/phase4-ensemble-api.md")]

//! Deterministic MCMC ensemble sampler for ASM graphs and codes.

/// Analysis helpers for inspecting run artefacts.
pub mod analysis;
/// Checkpoint serialization helpers and payload structures.
pub mod checkpoint;
/// YAML configuration schema and defaults.
pub mod config;
/// Deterministic seed derivation helpers.
pub mod determinism;
/// Energy proxy implementations.
pub mod energy;
/// Core sampling kernel and public `run`/`resume` entry points.
pub mod kernel;
/// Run manifest serialization helpers.
pub mod manifest;
/// Metrics collection and coverage summaries.
pub mod metrics;
/// Code-level proposal utilities.
pub mod moves_code;
/// Graph-level proposal utilities.
pub mod moves_graph;
/// Logical worm/loop proposal utilities.
pub mod moves_worm;
/// Parallel tempering ladder helpers.
pub mod tempering;

pub use config::{
    CheckpointConfig, LadderConfig, MoveCounts, RunConfig, ScoringWeights, SeedPolicy,
};
pub use energy::{score, EnergyBreakdown};
pub use kernel::{resume, run, ProposalOutcome, RunSummary};
pub use metrics::{CoverageMetrics, MetricSample};
