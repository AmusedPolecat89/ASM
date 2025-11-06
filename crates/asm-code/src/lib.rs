#![deny(missing_docs)]
#![doc = "Constraint projector implementations for ASM codes. See docs/phase3-code-api.md for detailed contracts."]

/// Logical algebra helpers and summary reporting.
pub mod analyze;
/// CSS code construction and `ConstraintProjector` implementation.
pub mod css;
/// Defect extraction, fusion, and species helpers.
pub mod defect;
/// Deterministic dispersion probe utilities.
pub mod dispersion;
/// Canonical hashing helpers for CSS codes.
pub mod hash;
/// Serialization routines for JSON and binary round-trips.
pub mod serde;
/// Opaque state handle utilities.
pub mod state;
/// Syndrome computation helpers.
pub mod syndrome;

pub use analyze::LogicalSummary;
pub use css::{CSSCode, Constraint, ConstraintKind};
pub use defect::{Defect, DefectKind, SpeciesId, ViolationSet};
pub use dispersion::{
    DispersionDiagnostics, DispersionOptions, DispersionReport, SpeciesDispersion,
};
pub use hash::canonical_code_hash;
pub use serde::{from_bytes, from_json, to_bytes, to_json};
pub use state::StateHandle;
