use std::fs;
use std::path::PathBuf;

use asm_exp::{sweep, to_canonical_json_bytes, GridParameter, SweepPlan, SweepStrategy};
use serde_json::json;

fn main() {
    let plan = SweepPlan {
        strategy: SweepStrategy::Grid {
            parameters: vec![
                GridParameter {
                    name: "degree_cap".into(),
                    values: vec![json!(2), json!(3)],
                },
                GridParameter {
                    name: "worm_weight".into(),
                    values: vec![json!(0.1), json!(0.2)],
                },
            ],
        },
        scheduler: Default::default(),
    };
    let report = sweep(&plan, 4242).expect("sweep");
    let bytes = to_canonical_json_bytes(&report).expect("json");
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("repro/phase7");
    fs::create_dir_all(&out_dir).expect("create baseline dir");
    fs::write(out_dir.join("bench_sweep.json"), bytes).expect("write baseline");
}
