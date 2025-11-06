use std::process::Command;

#[test]
fn doctor_returns_success() {
    let status = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--bin",
            "asm-sim",
            "--",
            "doctor",
            "--quiet",
        ])
        .status()
        .expect("run asm-sim doctor");
    assert!(status.success(), "doctor command failed");
}
