#![deny(missing_docs)]
#![doc = "Theory assertion, cross-checking, and manuscript bundling utilities for ASM Phase 15."]

/// Assertion registry and execution helpers.
pub mod assertions;
/// Manuscript bundling utilities.
pub mod bundle;
/// Cross-check helpers relating numeric and symbolic artefacts.
pub mod crosscheck;
/// Canonical hashing helpers.
pub mod hash;
/// Policy definitions controlling tolerance discipline.
pub mod policies;
/// Aggregated assertion reports and provenance types.
pub mod report;
/// Canonical JSON helpers.
pub mod serde;
/// Minimal symbolic algebra helpers.
pub mod symbolic;

pub use assertions::{run_assertions, AssertionInputs};
pub use bundle::{build_manuscript_bundle, BundlePlan, ManuscriptBundle};
pub use crosscheck::{crosscheck_numeric, CrosscheckResult};
pub use policies::{Policy, PolicyRange};
pub use report::{AssertionCheck, AssertionProvenance, AssertionReport};
pub use symbolic::{NumMat, SymExpr};
