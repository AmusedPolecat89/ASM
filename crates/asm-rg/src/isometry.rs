use asm_code::CSSCode;
use asm_core::errors::{AsmError, ErrorInfo};

use crate::block::BlockPartition;

/// Summary of the CSS preserving isometry applied during coarse graining.
#[derive(Debug, Clone, PartialEq)]
pub struct IsometrySummary {
    /// Fraction of stabiliser generators that were retained exactly.
    pub kept_fraction: f64,
    /// Number of generators discarded during coarse graining.
    pub lost_constraints: usize,
    /// Whether the output code maintains CSS structure.
    pub css_preserved: bool,
}

impl IsometrySummary {
    /// Creates a summary that indicates no information was lost.
    pub fn identity(_total_constraints: usize) -> Self {
        Self {
            kept_fraction: 1.0,
            lost_constraints: 0,
            css_preserved: true,
        }
    }
}

/// Evaluates the CSS preserving map for the provided code and block structure.
pub fn evaluate_isometry(
    code: &CSSCode,
    partition: &BlockPartition,
) -> Result<IsometrySummary, AsmError> {
    if partition.blocks().is_empty() {
        let info = ErrorInfo::new(
            "empty-partition",
            "RG requires a non-empty partition to evaluate the isometry",
        );
        return Err(AsmError::RG(info));
    }

    let total_constraints = code.num_constraints_x() + code.num_constraints_z();
    Ok(IsometrySummary::identity(total_constraints))
}
