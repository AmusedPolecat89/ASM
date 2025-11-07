use std::fs;
use std::path::Path;

use asm_core::errors::{AsmError, ErrorInfo};
use asm_dsr::query::QueryParams;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::collect::collect_site_data;
use crate::pages::{render_pages, validate_config, SiteConfig};
use crate::serde::to_canonical_json_bytes;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildManifest {
    pub page_count: usize,
    pub generated_at: String,
}

pub fn build_site(
    conn: &Connection,
    config: &SiteConfig,
    out_dir: &Path,
    params: &QueryParams,
) -> Result<BuildManifest, AsmError> {
    validate_config(config)?;
    let data = collect_site_data(conn, params)?;
    let pages = render_pages(config, &data)?;
    fs::create_dir_all(out_dir).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("asm_web.output_dir", err.to_string())
                .with_context("path", out_dir.display().to_string()),
        )
    })?;
    for page in &pages {
        let path = out_dir.join(&page.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                AsmError::Serde(
                    ErrorInfo::new("asm_web.output_dir", err.to_string())
                        .with_context("path", parent.display().to_string()),
                )
            })?;
        }
        fs::write(&path, page.content.as_bytes()).map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("asm_web.write", err.to_string())
                    .with_context("path", path.display().to_string()),
            )
        })?;
    }
    let manifest = BuildManifest {
        page_count: pages.len(),
        generated_at: chrono::Utc::now().to_rfc3339(),
    };
    let manifest_bytes = to_canonical_json_bytes(&manifest)?;
    fs::write(out_dir.join("manifest.json"), manifest_bytes).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("asm_web.write", err.to_string())
                .with_context("path", out_dir.join("manifest.json").display().to_string()),
        )
    })?;
    Ok(manifest)
}
