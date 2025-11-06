use std::fs::{self, OpenOptions};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use asm_core::errors::{AsmError, ErrorInfo};
use csv::{ReaderBuilder, WriterBuilder};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ablations::AblationReport;
use crate::serde::to_canonical_json_bytes;

/// Supported registry backends.
#[derive(Debug, Clone, PartialEq)]
pub enum Registry {
    Csv(PathBuf),
    Sqlite(PathBuf),
}

impl Registry {
    /// Construct a registry handle from a filesystem path.
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("sqlite") | Some("db") => Registry::Sqlite(path),
            _ => Registry::Csv(path),
        }
    }
}

/// Query descriptor for registry lookups.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Query {
    #[serde(default)]
    pub plan_name: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Table representation returned from registry queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Append an [`AblationReport`] to the registry backend.
pub fn registry_append(registry: &Registry, report: &AblationReport) -> Result<(), AsmError> {
    match registry {
        Registry::Csv(path) => append_csv(path, report),
        Registry::Sqlite(path) => append_sqlite(path, report),
    }
}

/// Query the registry returning a structured table.
pub fn registry_query(registry: &Registry, query: &Query) -> Result<Table, AsmError> {
    match registry {
        Registry::Csv(path) => query_csv(path, query),
        Registry::Sqlite(path) => query_sqlite(path, query),
    }
}

fn append_csv(path: &Path, report: &AblationReport) -> Result<(), AsmError> {
    ensure_parent(path)?;
    let file_exists = path.exists();
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("registry-open", "failed to open CSV registry")
                    .with_context("path", path.display().to_string())
                    .with_hint(err.to_string()),
            )
        })?;
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .from_writer(BufWriter::new(file));
    if !file_exists {
        writer
            .write_record([
                "date",
                "commit",
                "plan_name",
                "plan_hash",
                "job_id",
                "params",
                "metrics",
            ])
            .map_err(|err| wrap_csv("registry-write-header", err))?;
    }
    for (idx, job) in report.jobs.iter().enumerate() {
        let record = vec![
            provenance_date(&report.summary),
            provenance_commit(&report.summary),
            report.plan_name.clone(),
            report.plan_hash.clone(),
            idx.to_string(),
            canonical_string(&job.params)?,
            canonical_string(&job.metrics)?,
        ];
        writer
            .write_record(&record)
            .map_err(|err| wrap_csv("registry-write-row", err))?;
    }
    writer
        .flush()
        .map_err(|err| wrap_csv("registry-flush", err.into()))?;
    Ok(())
}

fn append_sqlite(path: &Path, report: &AblationReport) -> Result<(), AsmError> {
    ensure_parent(path)?;
    let mut conn = Connection::open(path).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("registry-sqlite-open", "failed to open sqlite registry")
                .with_context("path", path.display().to_string())
                .with_hint(err.to_string()),
        )
    })?;
    conn.execute_batch(
        r#"CREATE TABLE IF NOT EXISTS runs (
            date TEXT NOT NULL,
            "commit" TEXT NOT NULL,
            plan_name TEXT NOT NULL,
            plan_hash TEXT NOT NULL,
            job_id INTEGER NOT NULL,
            params TEXT NOT NULL,
            metrics TEXT NOT NULL
        );"#,
    )
    .map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("registry-sqlite-schema", "failed to ensure registry schema")
                .with_hint(err.to_string()),
        )
    })?;
    let tx = conn.transaction().map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("registry-sqlite-transaction", "failed to start transaction")
                .with_hint(err.to_string()),
        )
    })?;
    for (idx, job) in report.jobs.iter().enumerate() {
        tx.execute(
            r#"INSERT INTO runs (date, "commit", plan_name, plan_hash, job_id, params, metrics)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![
                provenance_date(&report.summary),
                provenance_commit(&report.summary),
                &report.plan_name,
                &report.plan_hash,
                idx as i64,
                canonical_string(&job.params)?,
                canonical_string(&job.metrics)?,
            ],
        )
        .map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("registry-sqlite-insert", "failed to append registry row")
                    .with_hint(err.to_string()),
            )
        })?;
    }
    tx.commit().map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("registry-sqlite-commit", "failed to commit registry rows")
                .with_hint(err.to_string()),
        )
    })?;
    Ok(())
}

fn query_csv(path: &Path, query: &Query) -> Result<Table, AsmError> {
    if !path.exists() {
        return Ok(empty_table());
    }
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .map_err(|err| wrap_csv("registry-read", err))?;
    let mut rows = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|err| wrap_csv("registry-record", err))?;
        if let Some(plan) = &query.plan_name {
            if record.get(2) != Some(plan) {
                continue;
            }
        }
        rows.push(record.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        if let Some(limit) = query.limit {
            if rows.len() >= limit {
                break;
            }
        }
    }
    Ok(Table {
        columns: table_columns(),
        rows,
    })
}

fn query_sqlite(path: &Path, query: &Query) -> Result<Table, AsmError> {
    if !path.exists() {
        return Ok(empty_table());
    }
    let conn = Connection::open(path).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("registry-sqlite-open", "failed to open sqlite registry")
                .with_hint(err.to_string()),
        )
    })?;
    let mut sql =
        r#"SELECT date, "commit", plan_name, plan_hash, job_id, params, metrics FROM runs"#
            .to_string();
    let mut clauses = Vec::new();
    if query.plan_name.is_some() {
        clauses.push("plan_name = ?1".to_string());
    }
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY date, plan_name, job_id");
    if let Some(limit) = query.limit {
        sql.push_str(&format!(" LIMIT {}", limit));
    }
    let mut stmt = conn.prepare(&sql).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new(
                "registry-sqlite-prepare",
                "failed to prepare registry query",
            )
            .with_hint(err.to_string()),
        )
    })?;
    let mut rows_iter = if let Some(plan) = &query.plan_name {
        stmt.query([plan])
    } else {
        stmt.query([])
    }
    .map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("registry-sqlite-query", "failed to execute registry query")
                .with_hint(err.to_string()),
        )
    })?;
    let mut rows = Vec::new();
    while let Some(row) = rows_iter.next().map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("registry-sqlite-row", "failed to fetch registry row")
                .with_hint(err.to_string()),
        )
    })? {
        let mut result = Vec::new();
        for idx in 0..7 {
            let value: String = row.get(idx).map_err(|err| {
                AsmError::Serde(
                    ErrorInfo::new("registry-sqlite-get", "failed to read column")
                        .with_hint(err.to_string()),
                )
            })?;
            result.push(value);
        }
        rows.push(result);
        if let Some(limit) = query.limit {
            if rows.len() >= limit {
                break;
            }
        }
    }
    Ok(Table {
        columns: table_columns(),
        rows,
    })
}

fn canonical_string(value: &Value) -> Result<String, AsmError> {
    let bytes = to_canonical_json_bytes(value)?;
    String::from_utf8(bytes).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("registry-canonical", "failed to encode canonical json")
                .with_hint(err.to_string()),
        )
    })
}

fn provenance_date(summary: &Value) -> String {
    summary
        .get("provenance")
        .and_then(|v| v.get("created_at"))
        .and_then(|v| v.as_str())
        .unwrap_or("1970-01-01T00:00:00Z")
        .to_string()
}

fn provenance_commit(summary: &Value) -> String {
    summary
        .get("provenance")
        .and_then(|v| v.get("commit"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}

fn ensure_parent(path: &Path) -> Result<(), AsmError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("registry-create", "failed to create registry directory")
                    .with_context("path", parent.display().to_string())
                    .with_hint(err.to_string()),
            )
        })?
    }
    Ok(())
}

fn table_columns() -> Vec<String> {
    vec![
        "date".into(),
        "commit".into(),
        "plan_name".into(),
        "plan_hash".into(),
        "job_id".into(),
        "params".into(),
        "metrics".into(),
    ]
}

fn empty_table() -> Table {
    Table {
        columns: table_columns(),
        rows: Vec::new(),
    }
}

fn wrap_csv(code: &str, err: csv::Error) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, "CSV registry failure").with_hint(err.to_string()))
}
