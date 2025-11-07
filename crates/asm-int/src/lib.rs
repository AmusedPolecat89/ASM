#![deny(missing_docs)]
#![doc = "Deterministic few-body interaction utilities spanning preparation, propagation, measurement and coupling extraction for ASM states."]

/// Coupling extraction utilities.
pub mod fit;
/// Canonical hashing helpers.
pub mod hash;
/// Deterministic evolution kernel primitives.
pub mod kernel;
/// Observable extraction utilities.
pub mod measure;
/// Few-body state preparation helpers.
pub mod prepare;
/// Aggregated interaction report assembly.
pub mod report;
/// Running coupling extraction helpers.
pub mod running;
/// Canonical JSON serde helpers.
pub mod serde;

pub use fit::{fit_couplings, CouplingsFit, FitConfidenceIntervals, FitOpts};
pub use kernel::{evolve, KernelMode, KernelOpts, Trajectory, TrajectoryMeta, TrajectoryStep};
pub use measure::{measure, MeasureOpts, ObsReport};
pub use prepare::{
    prepare_state, ParticipantSpec, PrepSpec, PrepTemplate, PreparedParticipant, PreparedState,
};
pub use report::{interact, interact_full, InteractionProvenance, InteractionReport};
pub use running::{
    fit_running, BetaSummary, RunningOpts, RunningReport, RunningStep, RunningThresholds,
};
