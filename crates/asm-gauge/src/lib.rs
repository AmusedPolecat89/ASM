#![deny(missing_docs)]
#![doc = "Gauge algebra extraction utilities for ASM Phase 12 workflows."]

mod closure;
mod decomp;
mod hash;
mod invariants;
mod rep;
mod report;
mod serde;
mod ward;

pub use closure::{check_closure, ClosureOpts, ClosureReport, StructureTensorEntry};
pub use decomp::{decompose, DecompOpts, DecompReport, FactorInfo};
pub use hash::stable_hash_string;
pub use rep::{build_rep, RepGenerator, RepMatrices, RepOpts};
pub use report::{analyze_gauge, GaugeOpts, GaugeProvenance, GaugeReport};
pub use serde::{from_json_slice, to_canonical_json_bytes};
pub use ward::{ward_check, WardOpts, WardReport, WardThresholds};

pub use invariants::GeneratorInvariants;
