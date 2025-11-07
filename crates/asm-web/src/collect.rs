use asm_core::errors::{AsmError, ErrorInfo};
use asm_dsr::query::{QueryParams, RegistryQuery};
use asm_dsr::schema::{ArtifactRecord, MetricRecord, SubmissionRecord};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SiteData {
    pub submissions: Vec<SubmissionRecord>,
    pub artifacts: Vec<ArtifactRecord>,
    pub metrics: Vec<MetricRecord>,
}

pub fn collect_site_data(conn: &Connection, params: &QueryParams) -> Result<SiteData, AsmError> {
    let query = RegistryQuery::execute(conn, params)?;
    Ok(SiteData {
        submissions: query.submissions,
        artifacts: query.artifacts,
        metrics: query.metrics,
    })
}

pub fn summarize_metric(data: &SiteData, name: &str) -> Result<f64, AsmError> {
    let mut values: Vec<f64> = data
        .metrics
        .iter()
        .filter(|metric| metric.name == name)
        .map(|metric| metric.value)
        .collect();
    if values.is_empty() {
        return Err(AsmError::Serde(ErrorInfo::new(
            "asm_web.metric_missing",
            format!("metric {name} missing"),
        )));
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Ok(values[values.len() / 2])
}
