//! Sandboxed plugin host for ASM community extensions.

mod abi;
mod hash;
mod loader;
mod manifest;
mod registry;
mod sandbox;
mod serde;

pub use abi::{AbiString, AsmPluginInfo, AsmPluginVTable, AsmStatus, Capability, ASM_ABI_VERSION};
pub use hash::{compute_manifest_hash, compute_plugin_hash};
pub use loader::{load_plugin_manifest, verify_abi_compat};
pub use manifest::{PluginManifest, PluginMetadata};
pub use registry::{PluginRegistry, RegistryEntry};
pub use sandbox::{SandboxCaps, SandboxDecision, SandboxEvent, SandboxGuard};
pub use serde::{from_json_slice, to_canonical_json_bytes};

