use std::collections::BTreeMap;

use asm_core::errors::{AsmError, ErrorInfo};
use serde::{Deserialize, Serialize};

use crate::hash::stable_hash_string;
use crate::policies::Policy;
use crate::serde::to_canonical_json_bytes;

fn report_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, message.into()))
}

/// Single assertion evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssertionCheck {
    /// Stable identifier for the assertion.
    pub name: String,
    /// Whether the assertion passed under the configured policy.
    pub pass: bool,
    /// Rounded metric captured during evaluation.
    pub metric: f64,
    /// Optional threshold used for scalar assertions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    /// Optional range used for interval assertions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<[f64; 2]>,
    /// Optional note surfaced when the assertion fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Provenance metadata attached to [`AssertionReport`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssertionProvenance {
    /// Policy applied during the assertion run.
    pub policy: Policy,
    /// Stable hashes of the inputs contributing to the report.
    pub input_hashes: BTreeMap<String, String>,
    /// Ordering of executed checks for determinism.
    pub check_order: Vec<String>,
}

/// Aggregated assertion report bundling all executed checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssertionReport {
    /// Content-addressed hash of the report payload.
    pub analysis_hash: String,
    /// Per-assertion results.
    pub checks: Vec<AssertionCheck>,
    /// Provenance describing policy and input hashes.
    pub provenance: AssertionProvenance,
}

impl AssertionReport {
    /// Constructs a report from checks and provenance while computing the stable hash.
    pub fn new(
        checks: Vec<AssertionCheck>,
        provenance: AssertionProvenance,
    ) -> Result<Self, AsmError> {
        let analysis_hash = stable_hash_string(&(&checks, &provenance))?;
        Ok(Self {
            analysis_hash,
            checks,
            provenance,
        })
    }

    /// Persists the report as canonical JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, AsmError> {
        to_canonical_json_bytes(self)
    }
}

impl AssertionProvenance {
    /// Constructs provenance metadata from the policy and input hashes.
    pub fn new(
        policy: Policy,
        input_hashes: BTreeMap<String, String>,
        check_order: Vec<String>,
    ) -> Self {
        Self {
            policy,
            input_hashes,
            check_order,
        }
    }
}

/// Validates that the report contains at least one check.
pub fn validate_checks(checks: &[AssertionCheck]) -> Result<(), AsmError> {
    if checks.is_empty() {
        return Err(report_error(
            "empty-assertions",
            "at least one assertion must be executed",
        ));
    }
    Ok(())
}
