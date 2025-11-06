use std::fs;
use std::path::PathBuf;
use std::process::Command;

use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn figure_bytes(path: &PathBuf) -> Vec<u8> {
    fs::read(path).expect("figure bytes")
}

fn figure_hash(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}

#[test]
fn figures_are_deterministic() {
    let temp = tempdir().unwrap();
    let rep_dir = temp.path().join("replication");
    fs::create_dir_all(&rep_dir).unwrap();
    let fixtures = PathBuf::from("fixtures/phase10");

    let figs_one = temp.path().join("figs_one");
    fs::create_dir_all(&figs_one).unwrap();
    let status = Command::new("python3")
        .arg("scripts/make_figures.py")
        .arg("--replication")
        .arg(&rep_dir)
        .arg("--fixtures")
        .arg(&fixtures)
        .arg("--figures")
        .arg(&figs_one)
        .status()
        .expect("run make_figures");
    assert!(status.success());

    let energy_path = figs_one.join("energy_vs_sweep_seed0.pdf");
    assert!(energy_path.exists());
    let hash_one = figure_hash(&figure_bytes(&energy_path));

    let figs_two = temp.path().join("figs_two");
    fs::create_dir_all(&figs_two).unwrap();
    let status = Command::new("python3")
        .arg("scripts/make_figures.py")
        .arg("--replication")
        .arg(&rep_dir)
        .arg("--fixtures")
        .arg(&fixtures)
        .arg("--figures")
        .arg(&figs_two)
        .status()
        .expect("rerun make_figures");
    assert!(status.success());
    let hash_two = figure_hash(&figure_bytes(&figs_two.join("energy_vs_sweep_seed0.pdf")));

    assert_eq!(hash_one, hash_two, "figure hashes diverged");
}
