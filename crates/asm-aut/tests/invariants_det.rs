use asm_aut::{analyze_state, serde_io, ScanOpts};
use asm_core::AsmError;

mod fixtures;

#[test]
fn repeated_analysis_is_deterministic() -> Result<(), AsmError> {
    let fixture = fixtures::load_fixture("t1_seed0")?;
    let provenance = fixtures::provenance_from_manifest(&fixture.manifest);
    let mut opts = ScanOpts::default();
    opts.provenance = Some(provenance);
    let report_a = analyze_state(&fixture.graph, &fixture.code, &opts)?;
    let report_b = analyze_state(&fixture.graph, &fixture.code, &opts)?;
    assert_eq!(report_a.hashes.analysis_hash, report_b.hashes.analysis_hash);
    let json_a = serde_io::analysis_to_json(&report_a)?;
    let json_b = serde_io::analysis_to_json(&report_b)?;
    assert_eq!(json_a, json_b);
    Ok(())
}
