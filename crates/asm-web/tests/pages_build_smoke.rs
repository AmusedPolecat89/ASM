use std::fs;

use asm_dsr::query::QueryParams;
use asm_dsr::schema::{init_schema, insert_artifact, insert_metric, insert_submission};
use asm_web::{build_site, pages::SiteConfig};
use rusqlite::Connection;
use tempfile::tempdir;

#[test]
fn build_site_emits_pages() {
    let conn = Connection::open_in_memory().expect("mem db");
    init_schema(&conn).expect("schema");
    let submission_id = insert_submission(&conn, "alice", "asm 0.16", None).expect("submission");
    insert_artifact(
        &conn,
        submission_id,
        "interaction_report",
        "interaction.json",
        "abc",
        None,
    )
    .expect("artifact");
    insert_metric(&conn, submission_id, "energy_final", 1.0, Some("arb")).expect("metric");
    let out = tempdir().expect("out");
    let config = SiteConfig::default();
    let manifest = build_site(&conn, &config, out.path(), &QueryParams::default()).expect("build");
    assert_eq!(manifest.page_count, 2);
    let index = fs::read(out.path().join("index.html")).expect("index");
    assert!(std::str::from_utf8(&index)
        .unwrap()
        .contains("Total submissions"));
    let manifest_bytes = fs::read(out.path().join("manifest.json")).expect("manifest json");
    let manifest_again = fs::read(out.path().join("manifest.json")).expect("manifest json");
    assert_eq!(manifest_bytes, manifest_again);
}
