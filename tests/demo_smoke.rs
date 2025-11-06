use std::process::Command;

use serde_json::Value;

#[test]
fn demo_produces_hashes() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "asm-sim",
            "--",
            "demo",
            "--input",
            "replication/seeds/state_seed0",
        ])
        .output()
        .expect("run asm-sim demo");
    assert!(output.status.success());
    let body = String::from_utf8(output.stdout).expect("utf8");
    let value: Value = serde_json::from_str(&body).expect("json");
    assert!(value.get("state_hash").and_then(|v| v.as_str()).is_some());
    let gaps = value.get("gaps").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    assert_eq!(gaps.len(), 2, "expected two gap reports");
}
