use std::path::PathBuf;

use asm_core::errors::{AsmError, ErrorInfo};
use serde::{Deserialize, Serialize};

use crate::collect::SiteData;
use crate::figures::{render_histogram_svg, FigureConfig};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteConfig {
    pub title: String,
    #[serde(default)]
    pub navbar: Vec<String>,
    #[serde(default)]
    pub featured_runs: Vec<String>,
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            title: "ASM Dashboard".into(),
            navbar: vec!["home".into(), "vacua".into()],
            featured_runs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageDescriptor {
    pub path: PathBuf,
    pub content: String,
}

pub fn render_pages(config: &SiteConfig, data: &SiteData) -> Result<Vec<PageDescriptor>, AsmError> {
    Ok(vec![
        PageDescriptor {
            path: PathBuf::from("index.html"),
            content: render_home(config, data),
        },
        PageDescriptor {
            path: PathBuf::from("vacua.html"),
            content: render_vacua(data),
        },
    ])
}

fn render_home(config: &SiteConfig, data: &SiteData) -> String {
    let total = data.submissions.len();
    let values: Vec<f64> = data.metrics.iter().map(|m| m.value).collect();
    format!(
        "<html><head><title>{title}</title></head><body><h1>{title}</h1><p>Total submissions: {total}</p>{hist}</body></html>",
        title = config.title,
        total = total,
        hist = render_histogram_svg(&values, &FigureConfig::default())
    )
}

fn render_vacua(data: &SiteData) -> String {
    let mut rows = String::new();
    for submission in &data.submissions {
        rows.push_str(&format!(
            "<tr><td>{id}</td><td>{submitter}</td><td>{toolchain}</td></tr>",
            id = submission.id,
            submitter = submission.submitter,
            toolchain = submission.toolchain,
        ));
    }
    format!(
        "<html><head><title>Vacua</title></head><body><h1>Vacua</h1><table>{rows}</table></body></html>",
        rows = rows
    )
}

pub fn validate_config(config: &SiteConfig) -> Result<(), AsmError> {
    if config.title.trim().is_empty() {
        return Err(AsmError::Serde(ErrorInfo::new(
            "asm_web.config_title",
            "site title cannot be empty",
        )));
    }
    Ok(())
}
