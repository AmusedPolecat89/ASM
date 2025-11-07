use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use asm_core::errors::{AsmError, ErrorInfo};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::hash::stable_hash_string;
use crate::serde::to_canonical_json_bytes;

fn bundle_error(code: &str, message: impl std::fmt::Display) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, message.to_string()))
}

/// Plan describing how manuscript inputs should be collected.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BundlePlan {
    /// Glob patterns to include relative to each root.
    pub include: Vec<String>,
    /// Whether to copy figure assets alongside JSON/CSV artefacts.
    #[serde(default)]
    pub copy_figures: bool,
    /// Whether to flatten output paths instead of recreating directory trees.
    #[serde(default)]
    pub flatten_paths: bool,
}

impl Default for BundlePlan {
    fn default() -> Self {
        Self {
            include: Vec::new(),
            copy_figures: true,
            flatten_paths: true,
        }
    }
}

/// Manifest describing the assembled manuscript bundle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManuscriptBundle {
    /// Stable hash over the manifest contents.
    pub bundle_hash: String,
    /// Paths of the inputs relative to the bundle root.
    pub inputs: Vec<String>,
    /// Mapping from bundle paths to their original source.
    pub manifest: BTreeMap<String, String>,
}

fn build_globset(patterns: &[String]) -> Result<GlobSet, AsmError> {
    let mut builder = GlobSetBuilder::new();
    if patterns.is_empty() {
        builder.add(Glob::new("**/*").map_err(|err| bundle_error("glob", err))?);
    } else {
        for pattern in patterns {
            builder.add(Glob::new(pattern).map_err(|err| bundle_error("glob", err))?);
        }
    }
    builder
        .build()
        .map_err(|err| bundle_error("glob-build", err))
}

fn is_figure(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        matches!(
            ext.to_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "svg" | "pdf"
        )
    } else {
        false
    }
}

fn normalise(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn destination_path(plan: &BundlePlan, rel: &Path) -> String {
    if !plan.flatten_paths {
        return normalise(rel);
    }
    rel.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("__")
}

fn copy_entry(src: &Path, dest: &Path) -> Result<(), AsmError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|err| bundle_error("bundle-mkdir", err))?;
    }
    fs::copy(src, dest).map_err(|err| bundle_error("bundle-copy", err))?;
    Ok(())
}

/// Builds a deterministic manuscript bundle from the provided roots and plan.
pub fn build_manuscript_bundle(
    src_roots: &[PathBuf],
    out_dir: &Path,
    plan: &BundlePlan,
) -> Result<ManuscriptBundle, AsmError> {
    if src_roots.is_empty() {
        return Err(bundle_error(
            "missing-roots",
            "at least one root must be provided",
        ));
    }
    fs::create_dir_all(out_dir).map_err(|err| bundle_error("bundle-out-dir", err))?;
    let globset = build_globset(&plan.include)?;
    let mut manifest = BTreeMap::new();
    for root in src_roots {
        let root = root.canonicalize().unwrap_or_else(|_| root.clone());
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = match entry.path().strip_prefix(&root) {
                Ok(rel) => rel,
                Err(_) => continue,
            };
            if !globset.is_match(rel) {
                continue;
            }
            if !plan.copy_figures && is_figure(entry.path()) {
                continue;
            }
            let rel_path = PathBuf::from(normalise(rel));
            let dest_name = destination_path(plan, &rel_path);
            let dest_path = out_dir.join(&dest_name);
            copy_entry(entry.path(), &dest_path)?;
            manifest.insert(dest_name, normalise(entry.path()));
        }
    }
    let mut inputs: Vec<String> = manifest.keys().cloned().collect();
    inputs.sort();
    let bundle_hash = stable_hash_string(&manifest)?;
    let bundle = ManuscriptBundle {
        bundle_hash,
        inputs,
        manifest,
    };
    let bytes = to_canonical_json_bytes(&bundle)?;
    fs::write(out_dir.join("manifest.json"), bytes)
        .map_err(|err| bundle_error("bundle-write", err))?;
    Ok(bundle)
}
