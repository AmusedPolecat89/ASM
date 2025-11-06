use std::collections::BTreeMap;

use asm_core::{AsmError, ErrorInfo, LogicalAlgebraSummary};

use crate::css::CSSCode;

/// Lightweight logical algebra summary helper.
pub type LogicalSummary = LogicalAlgebraSummary;

/// Computes a logical algebra summary for the provided CSS code.
pub fn logical_summary(code: &CSSCode) -> Result<LogicalAlgebraSummary, AsmError> {
    if !code.is_css_orthogonal() {
        let info = ErrorInfo::new(
            "non-orthogonal-code",
            "logical summary requested for non-orthogonal CSS code",
        );
        return Err(AsmError::Code(info));
    }

    let mut summary = LogicalAlgebraSummary::default();
    let total_rank = code.rank_x() + code.rank_z();
    let num_logical = code.num_variables().saturating_sub(total_rank);
    summary.num_logical = num_logical;
    summary.labels = (0..num_logical)
        .map(|idx| format!("logical-{idx}"))
        .collect();
    let mut metadata = BTreeMap::new();
    metadata.insert("rank_x".to_string(), code.rank_x().to_string());
    metadata.insert("rank_z".to_string(), code.rank_z().to_string());
    metadata.insert(
        "num_variables".to_string(),
        code.num_variables().to_string(),
    );
    summary.metadata = metadata;
    Ok(summary)
}
