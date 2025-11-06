use asm_core::errors::{AsmError, ErrorInfo};

use crate::covariance::CovarianceReport;
use crate::dictionary::CouplingsReport;
use crate::{RGRunReport, RGStepReport};

fn map_err(err: serde_json::Error, code: &str) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, err.to_string()))
}

/// Serialises an RG step report to JSON.
pub fn step_to_json(report: &RGStepReport) -> Result<String, AsmError> {
    serde_json::to_string_pretty(report).map_err(|err| map_err(err, "rg-step-serialize"))
}

/// Restores an RG step report from JSON.
pub fn step_from_json(json: &str) -> Result<RGStepReport, AsmError> {
    serde_json::from_str(json).map_err(|err| map_err(err, "rg-step-deserialize"))
}

/// Serialises an RG run report to JSON.
pub fn run_to_json(report: &RGRunReport) -> Result<String, AsmError> {
    serde_json::to_string_pretty(report).map_err(|err| map_err(err, "rg-run-serialize"))
}

/// Restores an RG run report from JSON.
pub fn run_from_json(json: &str) -> Result<RGRunReport, AsmError> {
    serde_json::from_str(json).map_err(|err| map_err(err, "rg-run-deserialize"))
}

/// Serialises a couplings report to JSON.
pub fn couplings_to_json(report: &CouplingsReport) -> Result<String, AsmError> {
    serde_json::to_string_pretty(report).map_err(|err| map_err(err, "rg-couplings-serialize"))
}

/// Restores a couplings report from JSON.
pub fn couplings_from_json(json: &str) -> Result<CouplingsReport, AsmError> {
    serde_json::from_str(json).map_err(|err| map_err(err, "rg-couplings-deserialize"))
}

/// Serialises a covariance report to JSON.
pub fn covariance_to_json(report: &CovarianceReport) -> Result<String, AsmError> {
    serde_json::to_string_pretty(report).map_err(|err| map_err(err, "rg-covariance-serialize"))
}

/// Restores a covariance report from JSON.
pub fn covariance_from_json(json: &str) -> Result<CovarianceReport, AsmError> {
    serde_json::from_str(json).map_err(|err| map_err(err, "rg-covariance-deserialize"))
}
