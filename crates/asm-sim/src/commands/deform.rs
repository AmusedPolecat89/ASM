use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use asm_code::{serde as code_serde, CSSCode};
use asm_exp::{deform, to_canonical_json_bytes, DeformSpec};
use asm_graph::{graph_from_json, graph_to_json, HypergraphImpl};
use asm_mcmc::{analysis, manifest::RunManifest};
use asm_rg::StateRef;
use clap::Args;
use serde_yaml::from_str;

#[derive(Args, Debug)]
pub struct DeformArgs {
    #[arg(long)]
    pub input: PathBuf,
    #[arg(long)]
    pub spec: PathBuf,
    #[arg(long)]
    pub seed: u64,
    #[arg(long)]
    pub out: PathBuf,
}

pub fn run(args: &DeformArgs) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&args.out)?;
    let spec_text = fs::read_to_string(&args.spec)?;
    let spec: DeformSpec = from_str(&spec_text)?;

    let loaded = load_state(&args.input)?;
    let state_ref = StateRef {
        graph: &loaded.graph,
        code: &loaded.code,
    };
    let report =
        deform(&state_ref, &spec, args.seed).map_err(|err| Box::new(err) as Box<dyn Error>)?;

    let json = to_canonical_json_bytes(&report).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    fs::write(args.out.join("deformation.json"), json)?;

    let end_state_dir = args.out.join("end_state");
    fs::create_dir_all(&end_state_dir)?;
    fs::write(end_state_dir.join("graph.json"), loaded.graph_json)?;
    fs::write(end_state_dir.join("code.json"), loaded.code_json)?;

    Ok(())
}

pub(crate) struct LoadedState {
    pub code: CSSCode,
    pub graph: HypergraphImpl,
    pub code_json: String,
    pub graph_json: String,
}

pub(crate) fn load_state(path: &Path) -> Result<LoadedState, Box<dyn Error>> {
    if path.join("manifest.json").exists() {
        RunManifest::load(&path.join("manifest.json"))
            .map_err(|err| Box::new(err) as Box<dyn Error>)?;
        let (code, graph) =
            analysis::load_end_state(path).map_err(|err| Box::new(err) as Box<dyn Error>)?;
        let code_json =
            code_serde::to_json(&code).map_err(|err| Box::new(err) as Box<dyn Error>)?;
        let graph_json = graph_to_json(&graph).map_err(|err| Box::new(err) as Box<dyn Error>)?;
        return Ok(LoadedState {
            code,
            graph,
            code_json,
            graph_json,
        });
    }
    let code_path = path.join("code.json");
    let graph_path = path.join("graph.json");
    let code_json = fs::read_to_string(code_path)?;
    let graph_json = fs::read_to_string(graph_path)?;
    let code = code_serde::from_json(&code_json).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    let graph = graph_from_json(&graph_json).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    Ok(LoadedState {
        code,
        graph,
        code_json,
        graph_json,
    })
}
