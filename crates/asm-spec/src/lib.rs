//! Spectral analysis utilities for ASM states.

pub mod correl;
pub mod dispersion;
pub mod excitations;
pub mod hash;
pub mod operators;
pub mod propagation;
pub mod report;
pub mod serde;

pub use correl::{correlation_scan, CorrelSpec, CorrelationReport};
pub use dispersion::{dispersion_scan, DispersionMode, DispersionReport, DispersionSpec};
pub use excitations::{ExcitationKind, ExcitationSpec};
pub use hash::stable_hash_string;
pub use operators::{build_operators, OpOpts, OperatorEntry, Operators, OperatorsInfo, OpsVariant};
pub use propagation::{excite_and_propagate, PropOpts, Response};
pub use report::{analyze_spectrum, SpecOpts, SpectrumProvenance, SpectrumReport};
pub use serde::{from_json_slice, to_canonical_json_bytes};
