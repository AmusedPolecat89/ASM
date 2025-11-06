//! Experiment orchestration utilities for deterministic ASM workflows.

mod ablations;
mod deform;
mod gaps;
mod hash;
mod registry;
mod runbook;
mod serde;
mod sweep;

pub use ablations::{
    run_ablation, AblationJobReport, AblationMode, AblationPlan, AblationReport, ToleranceSpec,
};
pub use deform::{deform, DeformSpec, DeformationReport};
pub use gaps::{estimate_gaps, GapMethod, GapOpts, GapReport};
pub use hash::{canonical_state_hash, stable_hash_string};
pub use registry::{registry_append, registry_query, Query, Registry, Table};
pub use runbook::{build_runbook, RunBook, RunMeta};
pub use sweep::{
    sweep, GridParameter, LhsParameter, Scheduler, SweepJobReport, SweepPlan, SweepReport,
    SweepStrategy,
};

pub use serde::{from_json_slice, to_canonical_json_bytes};
