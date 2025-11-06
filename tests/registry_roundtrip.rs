use std::env;
use std::path::PathBuf;

use asm_exp::{
    registry_append, registry_query, run_ablation, AblationMode, AblationPlan, Query, Registry,
    ToleranceSpec,
};
use serde_json::json;

fn temp_path(name: &str) -> PathBuf {
    let mut base = env::temp_dir();
    base.push(format!("asm_registry_test_{}_{}", name, std::process::id()));
    base
}

#[test]
fn registry_roundtrip_csv_and_sqlite() {
    let plan = AblationPlan {
        name: "registry".to_string(),
        mode: AblationMode::Grid,
        samples: None,
        factors: [
            (
                "rg.scale".to_string(),
                vec![json!(2)],
            ),
            (
                "rg.steps".to_string(),
                vec![json!(1), json!(2)],
            ),
        ]
        .into_iter()
        .collect(),
        fixed: [
            ("dict.variant".to_string(), json!("A")),
        ]
        .into_iter()
        .collect(),
        tolerances: [
            (
                "delta_c".to_string(),
                ToleranceSpec {
                    min: None,
                    max: Some(0.05),
                    abs: Some(1e-6),
                    rel: Some(1e-3),
                },
            ),
        ]
        .into_iter()
        .collect(),
    };
    let report = run_ablation(&plan, 77).expect("ablation");

    let csv_path = temp_path("csv");
    let _ = std::fs::remove_file(&csv_path);
    let registry_csv = Registry::from_path(&csv_path);
    registry_append(&registry_csv, &report).expect("append csv");
    let table_csv = registry_query(&registry_csv, &Query::default()).expect("query csv");
    assert_eq!(table_csv.rows.len(), report.jobs.len());

    let sqlite_path = csv_path.with_extension("sqlite");
    let _ = std::fs::remove_file(&sqlite_path);
    let registry_sqlite = Registry::from_path(&sqlite_path);
    registry_append(&registry_sqlite, &report).expect("append sqlite");
    let table_sqlite = registry_query(&registry_sqlite, &Query::default()).expect("query sqlite");
    assert_eq!(table_sqlite.rows.len(), report.jobs.len());
}
