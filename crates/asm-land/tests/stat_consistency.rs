use std::path::PathBuf;

use asm_land::{plan::load_plan, run_plan, stat::StatsSummary, RunOpts};

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(relative)
}

#[test]
fn stats_are_consistent() {
    let plan_path = fixture_path("landscape/plans/smoke.yaml");
    let plan = load_plan(&plan_path).expect("load plan");
    let temp = tempfile::tempdir().expect("tmp dir");
    let report = run_plan(&plan, temp.path(), &RunOpts::default()).expect("run plan");
    let kpis: Vec<_> = report.jobs.iter().map(|job| job.kpis.clone()).collect();
    let stats_again = StatsSummary::from_kpis(&kpis);

    assert_eq!(report.stats.histograms, stats_again.histograms);
    assert_eq!(report.stats.quantiles, stats_again.quantiles);
    assert_eq!(report.stats.correlations, stats_again.correlations);
}
