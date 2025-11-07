use asm_core::errors::{AsmError, ErrorInfo};
use serde::{Deserialize, Serialize};

use crate::abi::Capability;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub abi_version: u32,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub minimum_workspace: Option<String>,
    pub license: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), AsmError> {
        if self.name.trim().is_empty() {
            return Err(AsmError::Serde(ErrorInfo::new(
                "asm_host.manifest_name",
                "plugin manifest missing name",
            )));
        }
        if self.version.trim().is_empty() {
            return Err(AsmError::Serde(ErrorInfo::new(
                "asm_host.manifest_version",
                "plugin manifest missing version",
            )));
        }
        if self.license.trim().is_empty() {
            return Err(AsmError::Serde(ErrorInfo::new(
                "asm_host.manifest_license",
                "plugin manifest missing license",
            )));
        }
        Ok(())
    }

    pub fn capability_flags(&self) -> u32 {
        self.capabilities
            .iter()
            .filter_map(|cap| match cap.as_str() {
                "graph" => Some(Capability::Graph),
                "code" => Some(Capability::Code),
                "spectrum" => Some(Capability::Spectrum),
                "gauge" => Some(Capability::Gauge),
                "interact" => Some(Capability::Interact),
                "rg" => Some(Capability::Rg),
                "exp" => Some(Capability::Exp),
                _ => None,
            })
            .map(|cap| cap.flag())
            .fold(0, |mask, flag| mask | flag)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub abi_version: u32,
    pub capabilities: Vec<String>,
    pub manifest_hash: String,
}

impl PluginMetadata {
    pub fn from_manifest(manifest: &PluginManifest, manifest_hash: String) -> Self {
        Self {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            abi_version: manifest.abi_version,
            capabilities: manifest.capabilities.clone(),
            manifest_hash,
        }
    }
}
