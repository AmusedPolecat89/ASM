use std::fs;
use std::path::Path;

use crate::abi::{AsmPluginInfo, ASM_ABI_VERSION};
use crate::manifest::PluginManifest;
use asm_core::errors::{AsmError, ErrorInfo};

pub fn load_plugin_manifest(path: &Path) -> Result<PluginManifest, AsmError> {
    let contents = fs::read_to_string(path).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new(
                "asm_host.read_manifest",
                format!("failed to read manifest: {err}"),
            )
            .with_context("path", path.display().to_string()),
        )
    })?;
    toml::from_str(&contents).map_err(|err| {
        AsmError::Serde(
            ErrorInfo::new("asm_host.parse_manifest", err.to_string())
                .with_context("path", path.display().to_string()),
        )
    })
}

pub fn verify_abi_compat(info: &AsmPluginInfo) -> Result<(), AsmError> {
    if info.abi_version != ASM_ABI_VERSION {
        return Err(AsmError::Serde(ErrorInfo::new(
            "asm_host.abi_mismatch",
            format!(
                "plugin ABI {} is incompatible with host ABI {}",
                info.abi_version, ASM_ABI_VERSION
            ),
        )));
    }
    Ok(())
}

