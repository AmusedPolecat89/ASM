use std::error::Error;
use std::path::{Path, PathBuf};

use asm_exp::to_canonical_json_bytes;
use clap::Args;
use serde::Serialize;

#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Root of the ASM workspace to inspect.
    #[arg(long, default_value = ".")]
    pub root: PathBuf,
    /// Emit only JSON without additional context.
    #[arg(long)]
    pub quiet: bool,
}

#[derive(Debug, Serialize)]
struct DoctorCheck {
    name: String,
    ok: bool,
    detail: String,
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    status: String,
    checks: Vec<DoctorCheck>,
}

pub fn run(args: &DoctorArgs) -> Result<(), Box<dyn Error>> {
    let report = diagnose(&args.root)?;
    let json = to_canonical_json_bytes(&report).map_err(|err| Box::new(err) as Box<dyn Error>)?;
    let rendered = String::from_utf8(json)?;
    if args.quiet {
        println!("{}", rendered);
    } else {
        println!("asm-sim doctor status: {}", report.status);
        println!("{}", rendered);
    }
    if report.status != "ok" {
        return Err("one or more checks failed".into());
    }
    Ok(())
}

fn diagnose(root: &Path) -> Result<DoctorReport, Box<dyn Error>> {
    let root = root.canonicalize()?;
    let mut checks = Vec::new();
    checks.push(check_path(
        "replication/run.sh",
        &root.join("replication/run.sh"),
    ));
    checks.push(check_path(
        "replication/configs",
        &root.join("replication/configs"),
    ));
    checks.push(check_path(
        "replication/expected",
        &root.join("replication/expected"),
    ));
    checks.push(check_path(
        "replication/seeds",
        &root.join("replication/seeds"),
    ));
    checks.push(check_path(
        "scripts/make_figures.py",
        &root.join("scripts/make_figures.py"),
    ));
    checks.push(check_path(
        "scripts/render_dashboards.py",
        &root.join("scripts/render_dashboards.py"),
    ));
    checks.push(check_path(
        "scripts/build_paper.sh",
        &root.join("scripts/build_paper.sh"),
    ));
    checks.push(check_path("paper/main.md", &root.join("paper/main.md")));

    let mut missing_configs = Vec::new();
    let configs_dir = root.join("replication/configs");
    if configs_dir.exists() {
        let count = configs_dir.read_dir()?.count();
        if count == 0 || count > 8 {
            missing_configs.push(format!("expected 1-8 configs, found {}", count));
        }
    }
    if !missing_configs.is_empty() {
        checks.push(DoctorCheck {
            name: "replication/configs sanity".into(),
            ok: false,
            detail: missing_configs.join(", "),
        });
    } else {
        checks.push(DoctorCheck {
            name: "replication/configs sanity".into(),
            ok: true,
            detail: "configuration count within release bounds".into(),
        });
    }

    let mut status = "ok";
    if checks.iter().any(|check| !check.ok) {
        status = "needs-attention";
    }
    Ok(DoctorReport {
        status: status.into(),
        checks,
    })
}

fn check_path(name: &str, path: &Path) -> DoctorCheck {
    if path.exists() {
        DoctorCheck {
            name: name.into(),
            ok: true,
            detail: path.display().to_string(),
        }
    } else {
        DoctorCheck {
            name: name.into(),
            ok: false,
            detail: "missing".into(),
        }
    }
}
