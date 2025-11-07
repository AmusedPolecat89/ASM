mod common;

use std::fs;

use asm_core::errors::AsmError;
use asm_thy::bundle::{build_manuscript_bundle, BundlePlan, ManuscriptBundle};
use asm_thy::report::AssertionReport;
use asm_thy::run_assertions;
use asm_thy::serde::{from_json_slice, to_canonical_json_bytes};

use common::{sample_inputs, workspace_root};
use tempfile::tempdir;

#[test]
fn assertion_report_roundtrip() -> Result<(), AsmError> {
    let (inputs, policy) = sample_inputs();
    let report = run_assertions(&inputs, &policy)?;
    let bytes = to_canonical_json_bytes(&report)?;
    let decoded: AssertionReport = from_json_slice(&bytes)?;
    assert_eq!(report, decoded);
    Ok(())
}

#[test]
fn manuscript_bundle_roundtrip() -> Result<(), AsmError> {
    let plan = BundlePlan {
        include: vec!["fixtures/phase11/**/spectrum_report.json".to_string()],
        copy_figures: false,
        flatten_paths: true,
    };
    let tmp = tempdir().unwrap();
    let bundle = build_manuscript_bundle(&[workspace_root()], tmp.path(), &plan)?;
    let manifest_bytes = fs::read(tmp.path().join("manifest.json")).unwrap();
    let decoded: ManuscriptBundle = from_json_slice(&manifest_bytes)?;
    assert_eq!(bundle, decoded);
    Ok(())
}
