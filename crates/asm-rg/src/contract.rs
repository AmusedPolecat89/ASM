use asm_code::css::{from_parts, into_parts};
use asm_code::CSSCode;
use asm_core::errors::{AsmError, ErrorInfo};

use crate::block::BlockPartition;
use crate::isometry::{evaluate_isometry, IsometrySummary};

/// Result of contracting a CSS code under the RG map.
#[derive(Debug)]
pub struct ContractResult {
    /// The coarse grained CSS code.
    pub code: CSSCode,
    /// Summary statistics describing the transformation.
    pub summary: IsometrySummary,
}

/// Applies a CSS-preserving contraction according to the provided partition.
pub fn apply_contract(
    code: &CSSCode,
    partition: &BlockPartition,
) -> Result<ContractResult, AsmError> {
    let summary = evaluate_isometry(code, partition)?;
    if !code.is_css_orthogonal() {
        let info = ErrorInfo::new(
            "non-css-input",
            "input code does not satisfy CSS orthogonality",
        );
        return Err(AsmError::RG(info));
    }

    let (num_variables, x_checks, z_checks, schema, provenance, rank_x, rank_z) = into_parts(code);
    let coarse_code = from_parts(
        num_variables,
        x_checks,
        z_checks,
        schema,
        provenance,
        rank_x,
        rank_z,
    );

    Ok(ContractResult {
        code: coarse_code,
        summary,
    })
}
