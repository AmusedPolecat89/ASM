use asm_exp::{
    sweep, to_canonical_json_bytes, GridParameter, SweepPlan, SweepStrategy,
};
use serde_json::json;

#[test]
fn sweep_reports_repeat() {
    let plan = SweepPlan {
        strategy: SweepStrategy::Grid {
            parameters: vec![
                GridParameter {
                    name: "degree_cap".to_string(),
                    values: vec![json!(2), json!(3)],
                },
                GridParameter {
                    name: "worm_weight".to_string(),
                    values: vec![json!(0.1), json!(0.2)],
                },
            ],
        },
        scheduler: Default::default(),
    };
    let report_a = sweep(&plan, 8001).expect("sweep");
    let report_b = sweep(&plan, 8001).expect("sweep");
    assert_eq!(report_a, report_b);
    let json_a = to_canonical_json_bytes(&report_a).expect("json");
    let json_b = to_canonical_json_bytes(&report_b).expect("json");
    assert_eq!(json_a, json_b);
    assert_eq!(report_a.jobs.len(), 4);
}
