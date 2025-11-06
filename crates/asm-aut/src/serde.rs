use std::path::Path;

use asm_core::{AsmError, ErrorInfo};

use crate::{AnalysisReport, ClusterSummary};

/// Serialises an analysis report into indented JSON.
pub fn analysis_to_json(report: &AnalysisReport) -> Result<String, AsmError> {
    serde_json::to_string_pretty(report)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("analysis-serialize", err.to_string())))
}

/// Deserialises an analysis report from JSON text.
pub fn analysis_from_json(json: &str) -> Result<AnalysisReport, AsmError> {
    serde_json::from_str(json)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("analysis-deserialize", err.to_string())))
}

/// Serialises a clustering summary into JSON.
pub fn cluster_to_json(summary: &ClusterSummary) -> Result<String, AsmError> {
    serde_json::to_string_pretty(summary)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("cluster-serialize", err.to_string())))
}

/// Deserialises a clustering summary from JSON text.
pub fn cluster_from_json(json: &str) -> Result<ClusterSummary, AsmError> {
    serde_json::from_str(json)
        .map_err(|err| AsmError::Serde(ErrorInfo::new("cluster-deserialize", err.to_string())))
}

/// Writes a JSON payload to disk with deterministic formatting.
pub fn write_json(path: &Path, json: &str) -> Result<(), AsmError> {
    std::fs::write(path, json).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("json-write", err.to_string())
                .with_context("path", path.display().to_string()),
        )
    })
}

/// Reads a JSON payload from disk.
pub fn read_json(path: &Path) -> Result<String, AsmError> {
    std::fs::read_to_string(path).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("json-read", err.to_string())
                .with_context("path", path.display().to_string()),
        )
    })
}
