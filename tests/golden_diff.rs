use std::fs;
use std::process::Command;

fn write_file(path: &std::path::Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create dir");
    }
    fs::write(path, contents).expect("write file");
}

#[test]
fn compare_within_tolerance_succeeds() {
    let temp = std::env::temp_dir().join("asm_golden_diff_ok");
    let plan = temp.join("plan.yaml");
    let report = temp.join("report.json");
    let golden = temp.join("golden.json");

    write_file(
        &plan,
        "name: demo\nmode: grid\nfactors: {}\ntolerances:\n  metric:\n    abs: 1e-9\n    rel: 1e-3\n    max: 1.0\n",
    );
    write_file(
        &report,
        r#"{
  "plan_name": "demo",
  "plan_hash": "abc",
  "jobs": [
    {"params": {}, "seed": 1, "metrics": {"kpis": {"metric": {"value": 0.4}}}}
  ],
  "summary": {"provenance": {"created_at": "1970-01-01T00:00:00Z", "commit": "unknown"}},
  "artifacts": []
}
"#,
    );
    write_file(&golden, fs::read_to_string(&report).unwrap().as_str());

    let status = Command::new("python3")
        .arg("scripts/compare_to_golden.py")
        .arg("--plan")
        .arg(&plan)
        .arg("--report")
        .arg(&report)
        .arg("--golden")
        .arg(&golden)
        .status()
        .expect("run compare script");
    assert!(status.success());
}

#[test]
fn compare_exceeding_tolerance_fails() {
    let temp = std::env::temp_dir().join("asm_golden_diff_fail");
    let plan = temp.join("plan.yaml");
    let report = temp.join("report.json");
    let golden = temp.join("golden.json");

    write_file(
        &plan,
        "name: demo\nmode: grid\nfactors: {}\ntolerances:\n  metric:\n    abs: 1e-9\n    rel: 1e-3\n    max: 0.3\n",
    );
    write_file(
        &report,
        r#"{
  "plan_name": "demo",
  "plan_hash": "abc",
  "jobs": [
    {"params": {}, "seed": 1, "metrics": {"kpis": {"metric": {"value": 0.4}}}}
  ],
  "summary": {"provenance": {"created_at": "1970-01-01T00:00:00Z", "commit": "unknown"}},
  "artifacts": []
}
"#,
    );
    write_file(
        &golden,
        r#"{
  "plan_name": "demo",
  "plan_hash": "abc",
  "jobs": [
    {"params": {}, "seed": 1, "metrics": {"kpis": {"metric": {"value": 0.25}}}}
  ],
  "summary": {"provenance": {"created_at": "1970-01-01T00:00:00Z", "commit": "unknown"}},
  "artifacts": []
}
"#,
    );

    let status = Command::new("python3")
        .arg("scripts/compare_to_golden.py")
        .arg("--plan")
        .arg(&plan)
        .arg("--report")
        .arg(&report)
        .arg("--golden")
        .arg(&golden)
        .status()
        .expect("run compare script");
    assert!(!status.success());
}
