use std::path::PathBuf;

use asm_land::{
    filters::load_filters,
    plan::load_plan,
    report::{build_atlas, summarize, AtlasOpts},
    run_plan,
    serde::{from_json_slice, to_canonical_json_bytes},
    RunOpts,
};

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(relative)
}

#[test]
fn landscape_artifacts_roundtrip() {
    let plan_path = fixture_path("landscape/plans/smoke.yaml");
    let plan = load_plan(&plan_path).expect("load plan");
    let temp = tempfile::tempdir().expect("tmp dir");
    let _report = run_plan(&plan, temp.path(), &RunOpts::default()).expect("run plan");

    let path = temp.path().join("landscape_report.json");
    let bytes = std::fs::read(&path).expect("read report");
    let parsed: asm_land::report::LandscapeReport = from_json_slice(&bytes).expect("parse");
    let roundtrip = to_canonical_json_bytes(&parsed).expect("serialize");
    let original_value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    let roundtrip_value: serde_json::Value = serde_json::from_slice(&roundtrip).expect("roundtrip");
    assert_eq!(original_value, roundtrip_value);

    let atlas = build_atlas(temp.path(), &AtlasOpts::default()).expect("build atlas");
    let atlas_bytes = to_canonical_json_bytes(&atlas).expect("atlas serialize");
    let atlas_parsed: asm_land::report::Atlas = from_json_slice(&atlas_bytes).expect("atlas parse");
    let atlas_roundtrip = to_canonical_json_bytes(&atlas_parsed).expect("atlas roundtrip");
    let atlas_value: serde_json::Value = serde_json::from_slice(&atlas_bytes).expect("atlas json");
    let atlas_roundtrip_value: serde_json::Value =
        serde_json::from_slice(&atlas_roundtrip).expect("atlas roundtrip json");
    assert_eq!(atlas_value, atlas_roundtrip_value);

    let filters = load_filters(&plan.filters_path()).expect("filters");
    let summary = summarize(temp.path(), &filters).expect("summarize");
    let summary_bytes = to_canonical_json_bytes(&summary).expect("summary serialize");
    let summary_parsed: asm_land::report::SummaryReport =
        from_json_slice(&summary_bytes).expect("summary parse");
    let summary_roundtrip = to_canonical_json_bytes(&summary_parsed).expect("summary roundtrip");
    let summary_value: serde_json::Value =
        serde_json::from_slice(&summary_bytes).expect("summary json");
    let summary_roundtrip_value: serde_json::Value =
        serde_json::from_slice(&summary_roundtrip).expect("summary roundtrip json");
    assert_eq!(summary_value, summary_roundtrip_value);
}
