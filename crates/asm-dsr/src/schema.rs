use asm_core::errors::{AsmError, ErrorInfo};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmissionRecord {
    pub id: i64,
    pub submitter: String,
    pub date: String,
    pub toolchain: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub id: i64,
    pub submission_id: i64,
    pub kind: String,
    pub path: String,
    pub sha256: String,
    pub analysis_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricRecord {
    pub submission_id: i64,
    pub name: String,
    pub value: f64,
    pub unit: Option<String>,
}

pub fn init_schema(conn: &Connection) -> Result<(), AsmError> {
    conn.execute_batch(
        "BEGIN;
        CREATE TABLE IF NOT EXISTS meta(version INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS submissions(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            submitter TEXT NOT NULL,
            date TEXT NOT NULL,
            toolchain TEXT NOT NULL,
            notes TEXT
        );
        CREATE TABLE IF NOT EXISTS artifacts(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            submission_id INTEGER NOT NULL,
            kind TEXT NOT NULL,
            path TEXT NOT NULL,
            sha256 TEXT NOT NULL,
            analysis_hash TEXT,
            FOREIGN KEY(submission_id) REFERENCES submissions(id)
        );
        CREATE TABLE IF NOT EXISTS metrics(
            submission_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            value REAL NOT NULL,
            unit TEXT,
            FOREIGN KEY(submission_id) REFERENCES submissions(id)
        );
        COMMIT;",
    )
    .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.schema", err.to_string())))?;
    set_version(conn, SCHEMA_VERSION)?;
    Ok(())
}

fn set_version(conn: &Connection, version: i64) -> Result<(), AsmError> {
    let existing: Option<i64> = conn
        .query_row("SELECT version FROM meta LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.schema", err.to_string())))?;
    match existing {
        Some(current) if current == version => Ok(()),
        Some(current) => Err(AsmError::Serde(ErrorInfo::new(
            "asm_dsr.schema_version",
            format!("registry schema {current} incompatible with expected {version}"),
        ))),
        None => {
            conn.execute("INSERT INTO meta(version) VALUES (?)", params![version])
                .map_err(|err| {
                    AsmError::Serde(ErrorInfo::new("asm_dsr.schema", err.to_string()))
                })?;
            Ok(())
        }
    }
}

pub fn insert_submission(
    conn: &Connection,
    submitter: &str,
    toolchain: &str,
    notes: Option<&str>,
) -> Result<i64, AsmError> {
    let date = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO submissions(submitter, date, toolchain, notes) VALUES (?, ?, ?, ?)",
        params![submitter, date, toolchain, notes],
    )
    .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.insert_submission", err.to_string())))?;
    Ok(conn.last_insert_rowid())
}

pub fn insert_artifact(
    conn: &Connection,
    submission_id: i64,
    kind: &str,
    path: &str,
    sha256: &str,
    analysis_hash: Option<&str>,
) -> Result<i64, AsmError> {
    conn.execute(
        "INSERT INTO artifacts(submission_id, kind, path, sha256, analysis_hash) VALUES (?, ?, ?, ?, ?)",
        params![submission_id, kind, path, sha256, analysis_hash],
    )
    .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.insert_artifact", err.to_string())))?;
    Ok(conn.last_insert_rowid())
}

pub fn insert_metric(
    conn: &Connection,
    submission_id: i64,
    name: &str,
    value: f64,
    unit: Option<&str>,
) -> Result<(), AsmError> {
    conn.execute(
        "INSERT INTO metrics(submission_id, name, value, unit) VALUES (?, ?, ?, ?)",
        params![submission_id, name, value, unit],
    )
    .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.insert_metric", err.to_string())))?;
    Ok(())
}

pub fn load_submissions(conn: &Connection) -> Result<Vec<SubmissionRecord>, AsmError> {
    let mut stmt = conn
        .prepare("SELECT id, submitter, date, toolchain, notes FROM submissions ORDER BY id")
        .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.query", err.to_string())))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(SubmissionRecord {
                id: row.get(0)?,
                submitter: row.get(1)?,
                date: row.get(2)?,
                toolchain: row.get(3)?,
                notes: row.get(4)?,
            })
        })
        .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.query", err.to_string())))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.query", err.to_string())))
}

pub fn load_artifacts(
    conn: &Connection,
    submission_id: i64,
) -> Result<Vec<ArtifactRecord>, AsmError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, submission_id, kind, path, sha256, analysis_hash FROM artifacts WHERE submission_id = ? ORDER BY id",
        )
        .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.query", err.to_string())))?;
    let rows = stmt
        .query_map([submission_id], |row| {
            Ok(ArtifactRecord {
                id: row.get(0)?,
                submission_id: row.get(1)?,
                kind: row.get(2)?,
                path: row.get(3)?,
                sha256: row.get(4)?,
                analysis_hash: row.get(5)?,
            })
        })
        .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.query", err.to_string())))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.query", err.to_string())))
}

pub fn load_metrics(conn: &Connection, submission_id: i64) -> Result<Vec<MetricRecord>, AsmError> {
    let mut stmt = conn
        .prepare(
            "SELECT submission_id, name, value, unit FROM metrics WHERE submission_id = ? ORDER BY name",
        )
        .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.query", err.to_string())))?;
    let rows = stmt
        .query_map([submission_id], |row| {
            Ok(MetricRecord {
                submission_id: row.get(0)?,
                name: row.get(1)?,
                value: row.get(2)?,
                unit: row.get(3)?,
            })
        })
        .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.query", err.to_string())))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.query", err.to_string())))
}
