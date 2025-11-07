use std::fs::File;
use std::io::Write;

use asm_dsr::{
    ingest_bundle, init_schema,
    query::{QueryParams, RegistryQuery},
    IngestOptions,
};
use rusqlite::Connection;
use serde_json::json;
use sha2::{Digest, Sha256};
use tempfile::{tempdir, NamedTempFile};
use zip::write::FileOptions;

fn make_bundle(path: &std::path::Path, submitter: &str, kind: &str) {
    let file = File::create(path).expect("create bundle");
    let mut zip = zip::ZipWriter::new(file);
    let manifest = json!({
        "submitter": submitter,
        "toolchain": "asm 0.16",
        "artifacts": [
            {
                "kind": kind,
                "path": "artifact.json",
                "sha256": hex::encode(Sha256::digest(b"{}"))
            }
        ],
        "metrics": []
    });
    let bytes = serde_json::to_vec(&manifest).expect("json");
    zip.start_file("manifest.json", FileOptions::default())
        .expect("manifest");
    zip.write_all(&bytes).expect("write manifest");
    zip.start_file("artifact.json", FileOptions::default())
        .expect("artifact");
    zip.write_all(b"{}").expect("write artifact");
    zip.finish().expect("finish");
}

#[test]
fn query_filters_by_submitter_and_kind() {
    let bundle_a = NamedTempFile::new().expect("bundle");
    let bundle_b = NamedTempFile::new().expect("bundle");
    make_bundle(bundle_a.path(), "alice", "interaction_report");
    make_bundle(bundle_b.path(), "bob", "LandscapeReport");
    let db = NamedTempFile::new().expect("db");
    let artifacts = tempdir().expect("artifacts");
    let conn = Connection::open(db.path()).expect("open");
    init_schema(&conn).expect("schema");
    let opts = IngestOptions::new(artifacts.path());
    ingest_bundle(&conn, bundle_a.path(), &opts).expect("ingest a");
    ingest_bundle(&conn, bundle_b.path(), &opts).expect("ingest b");
    let params = QueryParams {
        submitter: Some("alice".into()),
        kind: Some("interaction_report".into()),
    };
    let query = RegistryQuery::execute(&conn, &params).expect("query");
    assert_eq!(query.submissions.len(), 1);
    assert_eq!(query.artifacts.len(), 1);
    assert_eq!(query.artifacts[0].kind, "interaction_report");
    query.ensure_deterministic().expect("deterministic");
}
