use std::fs;
use std::path::PathBuf;
use std::process::Command;

use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn hash_dir(dir: &PathBuf) -> Vec<u8> {
    let mut hasher = Sha256::new();
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_file() {
            hasher.update(path.file_name().unwrap().to_string_lossy().as_bytes());
            hasher.update(fs::read(&path).unwrap());
        }
    }
    hasher.finalize().to_vec()
}

#[test]
fn dashboards_are_deterministic() {
    let tmp = tempdir().unwrap();
    let replication = tmp.path().join("replication");
    fs::create_dir_all(&replication).unwrap();
    let expected = tmp.path().join("expected");
    fs::create_dir_all(&expected).unwrap();

    for name in [
        "metrics_digest.json",
        "gaps_dispersion.json",
        "gaps_spectral.json",
        "rg_couplings.json",
    ] {
        fs::copy(
            PathBuf::from("fixtures/phase10").join(name),
            expected.join(name),
        )
        .unwrap();
    }

    let registry_dir = tmp.path().join("registry");
    fs::create_dir_all(&registry_dir).unwrap();
    fs::copy(
        PathBuf::from("fixtures/phase10/registry.csv"),
        registry_dir.join("asm.csv"),
    )
    .unwrap();

    let out_one = tmp.path().join("dash_one");
    let status = Command::new("python3")
        .arg("scripts/render_dashboards.py")
        .arg("--replication")
        .arg(&replication)
        .arg("--expected")
        .arg(&expected)
        .arg("--registry")
        .arg(&registry_dir)
        .arg("--fixtures")
        .arg("fixtures/phase10")
        .arg("--out")
        .arg(&out_one)
        .status()
        .expect("render dashboards");
    assert!(status.success());

    let out_two = tmp.path().join("dash_two");
    let status = Command::new("python3")
        .arg("scripts/render_dashboards.py")
        .arg("--replication")
        .arg(&replication)
        .arg("--expected")
        .arg(&expected)
        .arg("--registry")
        .arg(&registry_dir)
        .arg("--fixtures")
        .arg("fixtures/phase10")
        .arg("--out")
        .arg(&out_two)
        .status()
        .expect("rerender dashboards");
    assert!(status.success());

    assert_eq!(hash_dir(&out_one), hash_dir(&out_two));
}
