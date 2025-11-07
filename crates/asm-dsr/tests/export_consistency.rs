use std::fs::{self, File};
use std::io::Write;

use asm_dsr::{export::export_json, ingest_bundle, init_schema, IngestOptions};
use rusqlite::Connection;
use serde_json::json;
use sha2::{Digest, Sha256};
use tempfile::{tempdir, NamedTempFile};
use zip::write::FileOptions;

fn bundle(path: &std::path::Path) {
    let file = File::create(path).expect("create bundle");
    let mut zip = zip::ZipWriter::new(file);
    let manifest = json!({
        "submitter": "team",
        "toolchain": "asm 0.16",
        "artifacts": [
            {
                "kind": "SummaryReport",
                "path": "summary.json",
                "sha256": hex::encode(Sha256::digest(b"{\"totals\":1}"))
            }
        ],
        "metrics": [
            {"name": "pass_rate", "value": 0.9}
        ]
    });
    let bytes = serde_json::to_vec(&manifest).expect("json");
    zip.start_file("manifest.json", FileOptions::default())
        .expect("manifest");
    zip.write_all(&bytes).expect("write manifest");
    zip.start_file("summary.json", FileOptions::default())
        .expect("summary");
    zip.write_all(b"{\"totals\":1}").expect("write summary");
    zip.finish().expect("finish");
}

#[test]
fn json_export_is_deterministic() {
    let bundle_path = NamedTempFile::new().expect("bundle");
    bundle(bundle_path.path());
    let db = NamedTempFile::new().expect("db");
    let artifacts = tempdir().expect("artifacts");
    let conn = Connection::open(db.path()).expect("open");
    init_schema(&conn).expect("schema");
    let opts = IngestOptions::new(artifacts.path());
    ingest_bundle(&conn, bundle_path.path(), &opts).expect("ingest");
    let export_a = NamedTempFile::new().expect("export");
    let export_b = NamedTempFile::new().expect("export");
    export_json(&conn, export_a.path()).expect("export a");
    export_json(&conn, export_b.path()).expect("export b");
    let bytes_a = fs::read(export_a.path()).expect("read a");
    let bytes_b = fs::read(export_b.path()).expect("read b");
    assert_eq!(bytes_a, bytes_b);
}
