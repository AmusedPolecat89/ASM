use std::fs::File;
use std::io::Write;
use std::path::Path;

use asm_dsr::{ingest_bundle, init_schema, IngestOptions};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::{tempdir, NamedTempFile};
use zip::write::FileOptions;

fn build_bundle(path: &Path) {
    let file = File::create(path).expect("create bundle");
    let mut zip = zip::ZipWriter::new(file);
    let manifest = serde_json::json!({
        "submitter": "tester",
        "toolchain": "asm 0.16",
        "notes": "demo",
        "artifacts": [
            {
                "kind": "interaction_report",
                "path": "interaction.json",
                "sha256": hex::encode(Sha256::digest(b"{}")),
                "analysis_hash": null
            }
        ],
        "metrics": [
            {"name": "energy_final", "value": 1.0, "unit": "arb"}
        ]
    });
    let manifest_bytes = serde_json::to_vec(&manifest).expect("json");
    zip.start_file("manifest.json", FileOptions::default())
        .expect("start manifest");
    zip.write_all(&manifest_bytes).expect("write manifest");
    zip.start_file("interaction.json", FileOptions::default())
        .expect("start artifact");
    zip.write_all(b"{}").expect("write artifact");
    zip.finish().expect("finish zip");
}

#[test]
fn ingest_creates_records() {
    let bundle = NamedTempFile::new().expect("bundle temp");
    build_bundle(bundle.path());
    let registry_db = NamedTempFile::new().expect("db temp");
    let artifact_root = tempdir().expect("artifact root");
    let conn = Connection::open(registry_db.path()).expect("open db");
    init_schema(&conn).expect("schema");
    let opts = IngestOptions::new(artifact_root.path());
    let submission = ingest_bundle(&conn, bundle.path(), &opts).expect("ingest");
    assert_eq!(submission.submitter, "tester");
    let stored_artifact = artifact_root
        .path()
        .join(format!("submission_{}/interaction.json", submission.id));
    assert!(stored_artifact.exists());
}
