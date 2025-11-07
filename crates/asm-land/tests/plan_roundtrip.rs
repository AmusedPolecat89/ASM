use std::path::PathBuf;

use asm_land::plan::load_plan;

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(relative)
}

#[test]
fn plan_roundtrip_matches_fixture() {
    let path = fixture_path("landscape/plans/smoke.yaml");
    let original = std::fs::read_to_string(&path).expect("fixture exists");
    let plan = load_plan(&path).expect("plan loads");
    let serialized = plan.to_yaml_string().expect("serialize");
    let serialized_value: serde_yaml::Value = serde_yaml::from_str(&serialized).expect("value");
    let original_value: serde_yaml::Value = serde_yaml::from_str(&original).expect("fixture value");
    assert_eq!(serialized_value, original_value);

    let reparsed = asm_land::serde::from_yaml_slice::<asm_land::plan::Plan>(serialized.as_bytes())
        .expect("reparse");
    let normalized = reparsed.to_yaml_string().expect("normalized serialize");
    let reparsed_value: serde_yaml::Value =
        serde_yaml::from_str(&normalized).expect("normalized value");
    assert_eq!(serialized_value, reparsed_value);
}
