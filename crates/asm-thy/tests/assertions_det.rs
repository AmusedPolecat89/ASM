mod common;

use asm_core::errors::AsmError;
use asm_thy::run_assertions;
use asm_thy::serde::to_canonical_json_bytes;

use common::sample_inputs;

#[test]
fn assertions_are_deterministic() -> Result<(), AsmError> {
    let (inputs, policy) = sample_inputs();
    let report_a = run_assertions(&inputs, &policy)?;

    let (inputs_b, _) = sample_inputs();
    let report_b = run_assertions(&inputs_b, &policy)?;

    assert_eq!(report_a, report_b);

    let bytes_a = to_canonical_json_bytes(&report_a)?;
    let bytes_b = to_canonical_json_bytes(&report_b)?;
    assert_eq!(bytes_a, bytes_b);

    Ok(())
}
