use asm_core::errors::{AsmError, ErrorInfo};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::schema::{
    load_artifacts, load_metrics, load_submissions, ArtifactRecord, MetricRecord, SubmissionRecord,
};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryParams {
    #[serde(default)]
    pub submitter: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryQuery {
    pub submissions: Vec<SubmissionRecord>,
    pub artifacts: Vec<ArtifactRecord>,
    pub metrics: Vec<MetricRecord>,
}

impl RegistryQuery {
    pub fn load(conn: &Connection) -> Result<Self, AsmError> {
        Self::execute(conn, &QueryParams::default())
    }

    pub fn execute(conn: &Connection, params: &QueryParams) -> Result<Self, AsmError> {
        let submissions = load_submissions(conn)?;
        let mut filtered_submissions = Vec::new();
        for submission in submissions {
            if let Some(filter) = &params.submitter {
                if &submission.submitter != filter {
                    continue;
                }
            }
            filtered_submissions.push(submission);
        }
        let mut artifacts = Vec::new();
        let mut metrics = Vec::new();
        for submission in &filtered_submissions {
            let mut submission_artifacts = load_artifacts(conn, submission.id)?;
            if let Some(kind) = &params.kind {
                submission_artifacts.retain(|artifact| &artifact.kind == kind);
            }
            artifacts.extend(submission_artifacts);
            metrics.extend(load_metrics(conn, submission.id)?);
        }
        Ok(Self {
            submissions: filtered_submissions,
            artifacts,
            metrics,
        })
    }

    pub fn ensure_deterministic(&self) -> Result<(), AsmError> {
        let submission_ids: Vec<_> = self.submissions.iter().map(|s| s.id).collect();
        let mut sorted = submission_ids.clone();
        sorted.sort();
        if submission_ids != sorted {
            return Err(AsmError::Serde(ErrorInfo::new(
                "asm_dsr.ordering",
                "submissions not ordered deterministically",
            )));
        }
        Ok(())
    }
}
