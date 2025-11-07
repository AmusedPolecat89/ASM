use std::fs;
use std::path::Path;

use asm_core::errors::{AsmError, ErrorInfo};
use rusqlite::Connection;

use crate::query::RegistryQuery;
use crate::schema::load_submissions;
use crate::serde::to_canonical_json_bytes;

pub fn export_json(conn: &Connection, out_path: &Path) -> Result<(), AsmError> {
    let query = RegistryQuery::load(conn)?;
    let bytes = to_canonical_json_bytes(&query)?;
    fs::write(out_path, bytes).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("asm_dsr.export", err.to_string())
                .with_context("path", out_path.display().to_string()),
        )
    })
}

pub fn export_csv(conn: &Connection, out_path: &Path) -> Result<(), AsmError> {
    let mut wtr = csv::Writer::from_path(out_path).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("asm_dsr.export", err.to_string())
                .with_context("path", out_path.display().to_string()),
        )
    })?;
    for submission in load_submissions(conn)? {
        wtr.write_record([
            submission.id.to_string(),
            submission.submitter.clone(),
            submission.date.clone(),
            submission.toolchain.clone(),
            submission.notes.clone().unwrap_or_default(),
        ])
        .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.export", err.to_string())))?;
    }
    wtr.flush()
        .map_err(|err| AsmError::Serde(ErrorInfo::new("asm_dsr.export", err.to_string())))
}
