use std::error::Error;
use std::path::PathBuf;

use asm_exp::{
    canonical_state_hash, estimate_gaps, to_canonical_json_bytes, GapMethod, GapOpts, GapReport,
};
use asm_rg::StateRef;
use clap::Args;
use serde::Serialize;
use serde_json::json;

use super::deform::load_state;

#[derive(Args, Debug)]
pub struct DemoArgs {
    /// Seed directory containing `code.json` and `graph.json`.
    #[arg(long, default_value = "replication/seeds/state_seed0")]
    pub input: PathBuf,
    /// Seed for deterministic gap estimation.
    #[arg(long, default_value_t = 2024)]
    pub seed: u64,
}

#[derive(Debug, Serialize)]
struct DemoReport {
    provenance: DemoProvenance,
    state_hash: String,
    code_qubits: usize,
    graph_nodes: usize,
    graph_edges: usize,
    gaps: Vec<GapReport>,
}

#[derive(Debug, Serialize)]
struct DemoProvenance {
    input: String,
    seed: u64,
}

pub fn run(args: &DemoArgs) -> Result<(), Box<dyn Error>> {
    let report = build_demo_report(args)?;
    let json = to_canonical_json_bytes(&report).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    println!("{}", String::from_utf8(json)?);
    Ok(())
}

fn build_demo_report(args: &DemoArgs) -> Result<DemoReport, Box<dyn Error>> {
    let loaded = load_state(&args.input)?;
    let state_ref = StateRef {
        graph: &loaded.graph,
        code: &loaded.code,
    };
    let state_hash =
        canonical_state_hash(&state_ref).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    let threshold_meta = json!({"demo_seed": args.seed});
    let dispersion = estimate_gaps(
        &state_ref,
        &GapOpts {
            method: GapMethod::Dispersion,
            thresholds: threshold_meta.clone(),
            tolerance: 0.03,
        },
    )
    .map_err(|err| Box::new(err) as Box<dyn Error>)?;
    let spectral = estimate_gaps(
        &state_ref,
        &GapOpts {
            method: GapMethod::Spectral,
            thresholds: threshold_meta,
            tolerance: 0.0,
        },
    )
    .map_err(|err| Box::new(err) as Box<dyn Error>)?;
    let graph_meta: serde_json::Value = serde_json::from_str(&loaded.graph_json)?;
    let nodes = graph_meta
        .get("nodes")
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .unwrap_or(0);
    let edges = graph_meta
        .get("edges")
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .unwrap_or(0);

    Ok(DemoReport {
        provenance: DemoProvenance {
            input: args.input.display().to_string(),
            seed: args.seed,
        },
        state_hash,
        code_qubits: loaded.code.num_variables(),
        graph_nodes: nodes,
        graph_edges: edges,
        gaps: vec![dispersion, spectral],
    })
}
