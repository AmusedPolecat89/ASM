use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use asm_land::{build_atlas, plan::load_plan, report::AtlasOpts, run_plan, summarize, RunOpts};
use criterion::Criterion;
use serde_json::json;
use tempfile::tempdir;

fn write_baseline(plan: &asm_land::plan::Plan, duration: f64) {
    let jobs = plan.seeds.len() * plan.rules().len();
    let payload = json!({
        "plan_hash": plan.plan_hash().unwrap_or_default(),
        "jobs": jobs,
        "seconds": duration,
    });
    fs::create_dir_all("repro/phase14").expect("baseline dir");
    fs::write(
        "repro/phase14/bench_landscape.json",
        serde_json::to_vec_pretty(&payload).expect("serialize baseline"),
    )
    .expect("write baseline");
}

fn workspace_root() -> PathBuf {
    let candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    candidate.canonicalize().unwrap_or(candidate)
}

fn normalize_plan_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        workspace_root().join(path)
    }
}

fn default_plan_path() -> PathBuf {
    normalize_plan_path(PathBuf::from("landscape/plans/smoke.yaml"))
}

fn plan_from_env() -> Option<PathBuf> {
    ["ASM_LAND_PLAN_PATH", "ASM_PLAN_PATH", "ASM_LAND_PLAN"]
        .into_iter()
        .find_map(|key| env::var_os(key).map(PathBuf::from))
        .map(normalize_plan_path)
}

fn extract_plan_flag(args: Vec<OsString>) -> Option<(PathBuf, Vec<OsString>)> {
    if args.is_empty() {
        return None;
    }

    let mut filtered = Vec::with_capacity(args.len());
    let mut iter = args.into_iter();
    if let Some(first) = iter.next() {
        filtered.push(first);
    }

    let mut plan_override = None;
    while let Some(arg) = iter.next() {
        if arg == "--plan" {
            if let Some(value) = iter.next() {
                plan_override = Some(normalize_plan_path(PathBuf::from(value)));
            }
            continue;
        }

        if let Some(value) = arg
            .to_str()
            .and_then(|raw| raw.strip_prefix("--plan="))
            .map(|raw| normalize_plan_path(PathBuf::from(raw)))
        {
            plan_override = Some(value);
            continue;
        }

        filtered.push(arg);
    }

    plan_override.map(|plan| (plan, filtered))
}

fn reexec_without_plan(plan: &Path, filtered_args: &[OsString]) -> ! {
    let exe = env::current_exe().expect("current exe");
    let mut command = Command::new(exe);

    if filtered_args.len() > 1 {
        command.args(filtered_args.iter().skip(1));
    }

    command.envs(env::vars_os());
    command.env("ASM_PLAN_PATH", plan);
    command.env("ASM_LAND_PLAN_REEXEC", "1");

    let status = command.status().expect("re-exec bench");
    let code = status.code().unwrap_or(1);
    std::process::exit(code);
}

fn resolve_plan_path() -> PathBuf {
    if let Some(path) = plan_from_env() {
        if env::var_os("ASM_LAND_PLAN_REEXEC").is_some() {
            env::remove_var("ASM_LAND_PLAN_REEXEC");
        }
        return path;
    }

    if env::var_os("ASM_LAND_PLAN_REEXEC").is_none() {
        if let Some((plan, filtered_args)) = extract_plan_flag(env::args_os().collect()) {
            reexec_without_plan(&plan, &filtered_args);
        }
    }

    if let Some(path) = plan_from_env() {
        env::remove_var("ASM_LAND_PLAN_REEXEC");
        return path;
    }

    default_plan_path()
}

fn bench_landscape(c: &mut Criterion, plan_path: &Path) {
    let plan = load_plan(plan_path)
        .unwrap_or_else(|err| panic!("load plan {}: {}", plan_path.display(), err));
    let warm_dir = tempdir().expect("warm dir");
    let start = Instant::now();
    let _report = run_plan(&plan, warm_dir.path(), &RunOpts::default()).expect("baseline run");
    let duration = start.elapsed().as_secs_f64();
    let filters = asm_land::filters::load_filters(&plan.filters_path()).expect("filters");
    let _ = summarize(warm_dir.path(), &filters).expect("summary");
    let _ = build_atlas(warm_dir.path(), &AtlasOpts::default()).expect("atlas");
    write_baseline(&plan, duration);

    c.bench_function("landscape_throughput", |b| {
        b.iter(|| {
            let dir = tempdir().expect("bench dir");
            run_plan(&plan, dir.path(), &RunOpts::default()).expect("bench run");
        });
    });
}

fn main() {
    let plan_path = resolve_plan_path();
    let mut criterion = Criterion::default().configure_from_args();
    bench_landscape(&mut criterion, &plan_path);
    criterion.final_summary();
}
