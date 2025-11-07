#![deny(missing_docs)]
#![doc = "Phase 14 landscape enumeration utilities for ASM."]

/// Stage orchestration and resume logic.
pub mod dispatch;
/// Anthropic filter helpers.
pub mod filters;
/// Canonical hashing helpers.
pub mod hash;
/// KPI extraction utilities.
pub mod metrics;
/// Deterministic plan loading and schema helpers.
pub mod plan;
/// Report assembly helpers.
pub mod report;
/// Canonical JSON serde helpers.
pub mod serde;
/// Wrappers for existing stage artefacts.
pub mod stages;
/// Statistical aggregation primitives.
pub mod stat;

pub use dispatch::{run_plan, run_plan_from_path, RunOpts};
pub use filters::{load_filters, FilterDecision, FilterSpec};
pub use plan::{
    load_plan, CodeSpec, GraphSpec, InteractSpec, OutputLayout, OutputSpec, Plan, RuleSpec,
    SamplerSpec, SpectrumSpec,
};
pub use report::{
    build_atlas, summarize, Atlas, AtlasEntry, AtlasOpts, JobReport, JobState, JobStatus,
    LandscapeReport, SummaryReport,
};
pub use stat::{Correlations, Histogram, Quantiles, StatsSummary};
