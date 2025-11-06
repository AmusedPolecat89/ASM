use asm_core::errors::{AsmError, ErrorInfo};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::covariance::CovarianceReport;
use crate::dictionary::{CouplingsReport, DictionaryProvenance};
use crate::{RGRunReport, RGStepReport};

fn hash_json<T: Serialize>(value: &T) -> Result<String, AsmError> {
    let json = serde_json::to_vec(value)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("serialize", err.to_string())))?;
    let mut hasher = Sha256::new();
    hasher.update(json);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Computes the canonical hash for an RG step report.
pub fn hash_step(report: &RGStepReport) -> Result<String, AsmError> {
    hash_json(report)
}

/// Computes the canonical hash for an RG run report.
pub fn hash_run(report: &RGRunReport) -> Result<String, AsmError> {
    hash_json(report)
}

/// Computes the canonical hash for an operator dictionary payload.
pub fn hash_couplings(
    c_kin: f64,
    g: &[f64; 3],
    lambda_h: f64,
    yukawa: &[f64],
    provenance: &DictionaryProvenance,
    residuals: f64,
) -> Result<String, AsmError> {
    #[derive(Serialize)]
    struct Payload<'a> {
        c_kin: f64,
        g: &'a [f64; 3],
        lambda_h: f64,
        yukawa: &'a [f64],
        provenance: &'a DictionaryProvenance,
        residuals: f64,
    }

    let payload = Payload {
        c_kin,
        g,
        lambda_h,
        yukawa,
        provenance,
        residuals,
    };
    hash_json(&payload)
}

/// Computes the canonical hash for a covariance report.
pub fn hash_covariance(report: &CovarianceReport) -> Result<String, AsmError> {
    hash_json(report)
}

/// Convenience wrapper for hashing a full couplings report.
pub fn hash_couplings_report(report: &CouplingsReport) -> Result<String, AsmError> {
    hash_json(report)
}
