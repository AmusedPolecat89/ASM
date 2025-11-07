use asm_core::errors::AsmError;
use asm_thy::{crosscheck_numeric, NumMat, Policy, SymExpr};

#[test]
fn symbolic_numeric_crosscheck_matches() -> Result<(), AsmError> {
    let symbolic = SymExpr::from_diagonal(&[1.0, 2.0]);
    let numeric = NumMat::new(2, vec![1.0, 0.0, 0.0, 2.0]);
    let result = crosscheck_numeric(&symbolic, &numeric, &Policy::default())?;
    assert!(result.pass);
    Ok(())
}
