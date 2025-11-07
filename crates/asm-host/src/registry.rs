use std::fs;
use std::path::PathBuf;

use asm_core::errors::{AsmError, ErrorInfo};
use serde::{Deserialize, Serialize};

use crate::hash::{compute_manifest_hash, compute_plugin_hash};
use crate::manifest::{PluginManifest, PluginMetadata};
use crate::serde::to_canonical_json_bytes;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub metadata: PluginMetadata,
    pub plugin_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PluginRegistry {
    root: PathBuf,
}

impl PluginRegistry {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn entry_dir(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    pub fn install(
        &self,
        manifest: &PluginManifest,
        plugin_bytes: Option<&[u8]>,
    ) -> Result<RegistryEntry, AsmError> {
        manifest.validate()?;
        let manifest_hash = compute_manifest_hash(manifest)?;
        let metadata = PluginMetadata::from_manifest(manifest, manifest_hash);
        let entry = RegistryEntry {
            metadata,
            plugin_hash: plugin_bytes.map(compute_plugin_hash),
        };
        let dir = self.entry_dir(&manifest.name);
        fs::create_dir_all(&dir).map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("asm_host.registry_io", err.to_string())
                    .with_context("path", dir.display().to_string()),
            )
        })?;
        let manifest_path = dir.join("manifest.toml");
        fs::write(
            &manifest_path,
            toml::to_string_pretty(manifest).map_err(|err| {
                AsmError::Serde(ErrorInfo::new(
                    "asm_host.manifest_serialize",
                    err.to_string(),
                ))
            })?,
        )
        .map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("asm_host.registry_io", err.to_string())
                    .with_context("path", manifest_path.display().to_string()),
            )
        })?;
        if let Some(bytes) = plugin_bytes {
            let bin_path = dir.join("plugin.bin");
            fs::write(&bin_path, bytes).map_err(|err| {
                AsmError::Serde(
                    ErrorInfo::new("asm_host.registry_io", err.to_string())
                        .with_context("path", bin_path.display().to_string()),
                )
            })?;
        }
        let metadata_path = dir.join("metadata.json");
        let entry_bytes = to_canonical_json_bytes(&entry)?;
        fs::write(&metadata_path, entry_bytes).map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("asm_host.registry_io", err.to_string())
                    .with_context("path", metadata_path.display().to_string()),
            )
        })?;
        Ok(entry)
    }

    pub fn remove(&self, name: &str) -> Result<(), AsmError> {
        let dir = self.entry_dir(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|err| {
                AsmError::Serde(
                    ErrorInfo::new("asm_host.registry_io", err.to_string())
                        .with_context("path", dir.display().to_string()),
                )
            })?;
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<RegistryEntry>, AsmError> {
        let mut entries = Vec::new();
        if !self.root.exists() {
            return Ok(entries);
        }
        for entry in fs::read_dir(&self.root).map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("asm_host.registry_io", err.to_string())
                    .with_context("path", self.root.display().to_string()),
            )
        })? {
            let entry = entry.map_err(|err| {
                AsmError::Serde(ErrorInfo::new("asm_host.registry_io", err.to_string()))
            })?;
            let metadata_path = entry.path().join("metadata.json");
            if !metadata_path.exists() {
                continue;
            }
            let bytes = fs::read(&metadata_path).map_err(|err| {
                AsmError::Serde(
                    ErrorInfo::new("asm_host.registry_io", err.to_string())
                        .with_context("path", metadata_path.display().to_string()),
                )
            })?;
            let parsed: RegistryEntry = crate::serde::from_json_slice(&bytes)?;
            entries.push(parsed);
        }
        entries.sort_by(|a, b| a.metadata.name.cmp(&b.metadata.name));
        Ok(entries)
    }

    pub fn verify(&self, name: &str) -> Result<RegistryEntry, AsmError> {
        let dir = self.entry_dir(name);
        let metadata_path = dir.join("metadata.json");
        if !metadata_path.exists() {
            return Err(AsmError::Serde(ErrorInfo::new(
                "asm_host.registry_missing",
                format!("plugin {name} not installed"),
            )));
        }
        let bytes = fs::read(&metadata_path).map_err(|err| {
            AsmError::Serde(
                ErrorInfo::new("asm_host.registry_io", err.to_string())
                    .with_context("path", metadata_path.display().to_string()),
            )
        })?;
        let mut entry: RegistryEntry = crate::serde::from_json_slice(&bytes)?;
        if let Some(ref hash) = entry.plugin_hash {
            let plugin_path = dir.join("plugin.bin");
            let plugin_bytes = fs::read(&plugin_path).map_err(|err| {
                AsmError::Serde(
                    ErrorInfo::new("asm_host.registry_io", err.to_string())
                        .with_context("path", plugin_path.display().to_string()),
                )
            })?;
            let actual = compute_plugin_hash(&plugin_bytes);
            if actual != *hash {
                return Err(AsmError::Serde(
                    ErrorInfo::new("asm_host.registry_hash", "plugin hash mismatch")
                        .with_context("expected", hash.clone())
                        .with_context("actual", actual),
                ));
            }
        }
        let manifest_path = dir.join("manifest.toml");
        let manifest = crate::loader::load_plugin_manifest(&manifest_path)?;
        let manifest_hash = compute_manifest_hash(&manifest)?;
        if manifest_hash != entry.metadata.manifest_hash {
            return Err(AsmError::Serde(
                ErrorInfo::new("asm_host.registry_manifest_hash", "manifest hash mismatch")
                    .with_context("expected", entry.metadata.manifest_hash.clone())
                    .with_context("actual", manifest_hash),
            ));
        }
        entry.metadata = PluginMetadata::from_manifest(&manifest, manifest_hash);
        Ok(entry)
    }
}
