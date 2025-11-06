use asm_aut::{analyze_state, serde_io, ClusterOpts, ScanOpts};
use asm_core::AsmError;

mod fixtures;

#[test]
fn analysis_roundtrip_preserves_payload() -> Result<(), AsmError> {
    let fixture = fixtures::load_fixture("t1_seed0")?;
    let provenance = fixtures::provenance_from_manifest(&fixture.manifest);
    let mut opts = ScanOpts::default();
    opts.provenance = Some(provenance);
    let report = analyze_state(&fixture.graph, &fixture.code, &opts)?;
    let json = serde_io::analysis_to_json(&report)?;
    let restored = serde_io::analysis_from_json(&json)?;
    assert_eq!(report.hashes, restored.hashes);
    Ok(())
}

#[test]
fn cluster_roundtrip_preserves_payload() -> Result<(), AsmError> {
    let names = ["t1_seed0", "t1_seed1"];
    let mut reports = Vec::new();
    for name in names {
        let fixture = fixtures::load_fixture(name)?;
        let provenance = fixtures::provenance_from_manifest(&fixture.manifest);
        let mut opts = ScanOpts::default();
        opts.provenance = Some(provenance);
        reports.push(analyze_state(&fixture.graph, &fixture.code, &opts)?);
    }
    let summary = asm_aut::cluster(&reports, &ClusterOpts::default());
    let json = serde_io::cluster_to_json(&summary)?;
    let restored = serde_io::cluster_from_json(&json)?;
    assert_eq!(summary.clusters, restored.clusters);
    Ok(())
}
