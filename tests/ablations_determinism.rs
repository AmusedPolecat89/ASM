use asm_exp::{
    run_ablation, to_canonical_json_bytes, AblationMode, AblationPlan, ToleranceSpec,
};
use serde_json::json;

#[test]
fn ablation_reports_are_deterministic() {
    let mut plan = AblationPlan {
        name: "determinism".to_string(),
        mode: AblationMode::Grid,
        samples: None,
        factors: [
            (
                "graph.degree_cap".to_string(),
                vec![json!(3), json!(4)],
            ),
            (
                "moves.worm_weight".to_string(),
                vec![json!(0.0), json!(0.3)],
            ),
        ]
        .into_iter()
        .collect(),
        fixed: [
            ("sampler.sweeps".to_string(), json!(32)),
            ("code.generator_density".to_string(), json!(0.1)),
        ]
        .into_iter()
        .collect(),
        tolerances: [
            (
                "exchange_acceptance".to_string(),
                ToleranceSpec {
                    min: Some(0.2),
                    max: Some(0.5),
                    abs: Some(1e-6),
                    rel: Some(1e-3),
                },
            ),
            (
                "common_c_residual".to_string(),
                ToleranceSpec {
                    min: None,
                    max: Some(0.03),
                    abs: Some(1e-6),
                    rel: Some(1e-3),
                },
            ),
        ]
        .into_iter()
        .collect(),
    };
    let report_a = run_ablation(&plan, 101).expect("run ablation");
    let report_b = run_ablation(&plan, 101).expect("run ablation");
    assert_eq!(report_a, report_b);
    let bytes_a = to_canonical_json_bytes(&report_a).expect("json");
    let bytes_b = to_canonical_json_bytes(&report_b).expect("json");
    assert_eq!(bytes_a, bytes_b);

    plan.mode = AblationMode::Lhs;
    plan.samples = Some(3);
    plan.factors.insert("moves.worm_weight".to_string(), vec![json!(0.0), json!(0.3)]);
    let lhs_report = run_ablation(&plan, 303).expect("lhs ablation");
    assert_eq!(lhs_report.jobs.len(), 3);
}
