use std::error::Error;
use std::fs;
use std::path::PathBuf;

use asm_dsr::query::QueryParams;
use asm_web::{build_site, pages::SiteConfig};
use clap::Args;
use rusqlite::Connection;

#[derive(Args, Debug)]
pub struct WebArgs {
    /// SQLite registry path used for dataset submissions
    #[arg(long)]
    pub registry: PathBuf,
    /// Site configuration YAML
    #[arg(long)]
    pub config: PathBuf,
    /// Output directory for the generated static site
    #[arg(long)]
    pub out: PathBuf,
}

pub fn run(args: &WebArgs) -> Result<(), Box<dyn Error>> {
    let conn = Connection::open(&args.registry)?;
    let contents = fs::read_to_string(&args.config)?;
    let config: SiteConfig = serde_yaml::from_str(&contents)?;
    let manifest = build_site(&conn, &config, &args.out, &QueryParams::default())?;
    println!("built site with {} pages", manifest.page_count);
    Ok(())
}
