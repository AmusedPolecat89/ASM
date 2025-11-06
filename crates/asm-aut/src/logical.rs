use asm_code::CSSCode;
use asm_core::{AsmError, ConstraintProjector, LogicalAlgebraSummary};
use serde::{Deserialize, Serialize};

/// Logical rank and commutation profile extracted from a CSS code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LogicalReport {
    /// Rank of the X logical operators.
    pub rank_x: u32,
    /// Rank of the Z logical operators.
    pub rank_z: u32,
    /// Deterministic commutation signature derived from `LogicalAlgebraSummary`.
    pub comm_signature: String,
}

/// Extracts logical invariants for the provided CSS code.
pub fn analyse_logical(code: &CSSCode) -> Result<LogicalReport, AsmError> {
    let summary = code.logical_algebra_summary()?;
    let rank_x = code.rank_x() as u32;
    let rank_z = code.rank_z() as u32;
    let comm_signature = format_summary(&summary);
    Ok(LogicalReport {
        rank_x,
        rank_z,
        comm_signature,
    })
}

fn format_summary(summary: &LogicalAlgebraSummary) -> String {
    let mut metadata: Vec<(String, String)> = summary
        .metadata
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    metadata.sort();
    let labels = if summary.labels.is_empty() {
        String::from("-")
    } else {
        let mut labels = summary.labels.clone();
        labels.sort();
        labels.join("|")
    };
    let metadata_str = metadata
        .into_iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "logical:{}|labels:{}|meta:{}",
        summary.num_logical, labels, metadata_str
    )
}
