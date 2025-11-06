use std::path::Path;
use std::process::Command;

#[test]
fn no_tracked_binary_files() {
    let output = Command::new("git")
        .args(["ls-files"])
        .output()
        .expect("failed to list tracked files");

    assert!(output.status.success(), "git ls-files exited with error");

    let stdout = String::from_utf8(output.stdout).expect("git ls-files output not utf8");
    let mut binary_files = Vec::new();

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let path = Path::new(line);
        // Skip submodules or deleted entries just in case.
        if !path.exists() {
            continue;
        }
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        if bytes.contains(&0) {
            binary_files.push(line.to_owned());
        }
    }

    if !binary_files.is_empty() {
        panic!(
            "tracked binary-like files detected: {}",
            binary_files.join(", ")
        );
    }
}
