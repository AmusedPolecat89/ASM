use asm_aut::code_aut::analyse_code;
use asm_core::{AsmError, RunProvenance, SchemaVersion};

fn symmetric_code() -> Result<asm_code::CSSCode, AsmError> {
    asm_code::CSSCode::new(
        2,
        vec![vec![0, 1]],
        vec![vec![0, 1]],
        SchemaVersion::new(1, 0, 0),
        RunProvenance::default(),
    )
}

fn swapping_code() -> Result<asm_code::CSSCode, AsmError> {
    asm_code::CSSCode::new(
        4,
        vec![vec![0, 1]],
        vec![vec![2, 3]],
        SchemaVersion::new(1, 0, 0),
        RunProvenance::default(),
    )
}

#[test]
fn css_preserving_automorphisms_detected() -> Result<(), AsmError> {
    let code = symmetric_code()?;
    let report = analyse_code(&code)?;
    assert!(report.order > 1);
    assert!(report.css_preserving);
    Ok(())
}

#[test]
fn non_css_mappings_flagged() -> Result<(), AsmError> {
    let code = swapping_code()?;
    let report = analyse_code(&code)?;
    assert!(!report.css_preserving);
    Ok(())
}
