use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use asm_host::{load_plugin_manifest, PluginRegistry};
use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct PluginArgs {
    /// Registry root directory
    #[arg(long, default_value = "registry/plugins")]
    pub registry: PathBuf,
    #[command(subcommand)]
    pub command: PluginCommand,
}

#[derive(Subcommand, Debug)]
pub enum PluginCommand {
    Install { path: PathBuf },
    List,
    Verify { name: String },
    Remove { name: String },
}

pub fn run(args: &PluginArgs) -> Result<(), Box<dyn Error>> {
    let registry = PluginRegistry::new(&args.registry);
    match &args.command {
        PluginCommand::Install { path } => install(&registry, path)?,
        PluginCommand::List => list(&registry)?,
        PluginCommand::Verify { name } => verify(&registry, name)?,
        PluginCommand::Remove { name } => remove(&registry, name)?,
    }
    Ok(())
}

fn install(registry: &PluginRegistry, path: &Path) -> Result<(), Box<dyn Error>> {
    let manifest_path = if path.is_dir() {
        path.join("plugin.toml")
    } else {
        path.to_path_buf()
    };
    let manifest = load_plugin_manifest(&manifest_path)?;
    let binary_path = manifest_path.parent().map(|dir| dir.join("plugin.bin"));
    let plugin_bytes = match binary_path {
        Some(p) if p.exists() => Some(fs::read(p)?),
        _ => None,
    };
    let entry = registry.install(&manifest, plugin_bytes.as_deref())?;
    println!(
        "installed plugin {} {}",
        entry.metadata.name, entry.metadata.version
    );
    Ok(())
}

fn list(registry: &PluginRegistry) -> Result<(), Box<dyn Error>> {
    let entries = registry.list()?;
    for entry in entries {
        println!("{} {}", entry.metadata.name, entry.metadata.version);
    }
    Ok(())
}

fn verify(registry: &PluginRegistry, name: &str) -> Result<(), Box<dyn Error>> {
    let entry = registry.verify(name)?;
    println!("verified {}", entry.metadata.name);
    Ok(())
}

fn remove(registry: &PluginRegistry, name: &str) -> Result<(), Box<dyn Error>> {
    registry.remove(name)?;
    println!("removed {name}");
    Ok(())
}
