mod common;

use asm_core::errors::AsmError;
use asm_thy::run_assertions;

use common::sample_inputs;

#[test]
fn stricter_policy_triggers_failure() -> Result<(), AsmError> {
    let (inputs, mut policy) = sample_inputs();
    policy.strict = true;
    policy.fit_resid_max = 0.05;

    let report = run_assertions(&inputs, &policy)?;
    let failed = report
        .checks
        .iter()
        .any(|check| check.name == "couplings_fit_resid" && !check.pass);
    assert!(
        failed,
        "strict policy should fail the coupling residual check"
    );
    Ok(())
}
