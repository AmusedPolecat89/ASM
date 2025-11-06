use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use asm_code::serde as code_serde;
use asm_code::CSSCode;
use asm_graph::{graph_from_json, HypergraphImpl};
use asm_mcmc::analysis;
use asm_mcmc::manifest::RunManifest;
use asm_rg::{dictionary::extract_couplings, serde_io, DictOpts};
use clap::Args;

#[derive(Args, Debug)]
pub struct ExtractArgs {
    /// Input directory containing a manifest or raw graph/code JSON files.
    #[arg(long)]
    pub input: PathBuf,
    /// Output directory for couplings artefacts.
    #[arg(long)]
    pub out: PathBuf,
    /// Number of Yukawa couplings to report.
    #[arg(long, default_value_t = 4)]
    pub yukawa: usize,
    /// Deterministic seed for dictionary extraction.
    #[arg(long, default_value_t = 0)]
    pub seed: u64,
    /// Residual tolerance attached to the report.
    #[arg(long, default_value_t = 1e-6)]
    pub residual_tolerance: f64,
}

pub fn run(args: &ExtractArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let (code, graph) = load_state(&args.input)?;
    let opts = DictOpts {
        yukawa_count: args.yukawa.max(1),
        seed: args.seed,
        residual_tolerance: args.residual_tolerance.max(0.0),
    };
    let report =
        extract_couplings(&graph, &code, &opts).map_err(|err| Box::new(err) as Box<dyn Error>)?;

    let json =
        serde_io::couplings_to_json(&report).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    fs::write(args.out.join("couplings.json"), json)?;

    let mut csv = fs::File::create(args.out.join("couplings.csv"))?;
    writeln!(csv, "parameter,value")?;
    writeln!(csv, "c_kin,{:.6}", report.c_kin)?;
    for (idx, value) in report.g.iter().enumerate() {
        writeln!(csv, "g{},{:.6}", idx + 1, value)?;
    }
    writeln!(csv, "lambda_h,{:.6}", report.lambda_h)?;
    for (idx, value) in report.yukawa.iter().enumerate() {
        writeln!(csv, "yukawa{},{:.6}", idx + 1, value)?;
    }

    Ok(())
}

fn load_state(path: &Path) -> Result<(CSSCode, HypergraphImpl), Box<dyn Error>> {
    if path.join("manifest.json").exists() {
        let _manifest = RunManifest::load(&path.join("manifest.json"))
            .map_err(|err| Box::new(err) as Box<dyn Error>)?;
        analysis::load_end_state(path).map_err(|err| Box::new(err) as Box<dyn Error>)
    } else {
        let code_path = path.join("code.json");
        let graph_path = path.join("graph.json");
        if !code_path.exists() || !graph_path.exists() {
            return Err(format!(
                "expected manifest.json or code.json/graph.json in {}",
                path.display()
            )
            .into());
        }
        let code_json = fs::read_to_string(code_path)?;
        let graph_json = fs::read_to_string(graph_path)?;
        let code =
            code_serde::from_json(&code_json).map_err(|err| Box::new(err) as Box<dyn Error>)?;
        let graph = graph_from_json(&graph_json).map_err(|err| Box::new(err) as Box<dyn Error>)?;
        Ok((code, graph))
    }
}
