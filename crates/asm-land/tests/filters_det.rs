use std::path::PathBuf;

use asm_land::{filters::load_filters, plan::load_plan, run_plan, RunOpts};

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(relative)
}

#[test]
fn filter_decisions_are_stable() {
    let plan_path = fixture_path("landscape/plans/smoke.yaml");
    let plan = load_plan(&plan_path).expect("load plan");
    let filter_spec = load_filters(&plan.filters_path()).expect("filters load");
    let temp = tempfile::tempdir().expect("tmp dir");
    let report_first = run_plan(&plan, temp.path(), &RunOpts::default()).expect("first run");
    let report_second = run_plan(&plan, temp.path(), &RunOpts::default()).expect("second run");

    for (a, b) in report_first.jobs.iter().zip(report_second.jobs.iter()) {
        let expected = filter_spec.evaluate(&a.kpis);
        assert_eq!(expected, a.filters);
        assert_eq!(a.filters, b.filters);
    }
}
