use std::path::PathBuf;

use asm_land::{dispatch::RunOpts, plan::load_plan, run_plan};

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(relative)
}

#[test]
fn dispatch_resume_produces_identical_report() {
    let plan_path = fixture_path("landscape/plans/smoke.yaml");
    let plan = load_plan(&plan_path).expect("load plan");
    let temp = tempfile::tempdir().expect("tmp dir");
    let _report_initial = run_plan(&plan, temp.path(), &RunOpts::default()).expect("initial run");
    let initial_path = temp.path().join("landscape_report.json");
    let initial_bytes = std::fs::read(&initial_path).expect("read initial");

    let job_dir = temp.path().join("42_0");
    std::fs::remove_file(job_dir.join("kpi.json")).expect("remove kpi");
    std::fs::remove_file(job_dir.join("hashes.json")).expect("remove hashes");

    let resume_opts = RunOpts {
        resume: true,
        ..RunOpts::default()
    };
    let _report_resumed = run_plan(&plan, temp.path(), &resume_opts).expect("resumed run");
    let resumed_bytes = std::fs::read(&initial_path).expect("read resumed");

    let mut initial_report: asm_land::report::LandscapeReport =
        asm_land::serde::from_json_slice(&initial_bytes).expect("parse initial");
    let mut resumed_report: asm_land::report::LandscapeReport =
        asm_land::serde::from_json_slice(&resumed_bytes).expect("parse resumed");
    initial_report.provenance.created_at.clear();
    resumed_report.provenance.created_at.clear();

    assert_eq!(initial_report.jobs, resumed_report.jobs);
    assert_eq!(initial_report.filters, resumed_report.filters);
    assert_eq!(initial_report.stats, resumed_report.stats);
}
