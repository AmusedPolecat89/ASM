use asm_core::errors::{AsmError, ErrorInfo};

fn sample_info(code: &str, message: &str) -> ErrorInfo {
    ErrorInfo::new(code, message)
        .with_context("id", "1")
        .with_context("reason", "example")
}

#[test]
fn graph_error_surface() {
    let err = AsmError::Graph(sample_info("G001", "cycle detected"));
    assert_eq!(err.info().code, "G001");
    assert!(err.info().context.contains_key("id"));
}

#[test]
fn code_error_surface() {
    let err = AsmError::Code(sample_info("C001", "rank mismatch"));
    assert_eq!(err.info().code, "C001");
    assert!(err.info().context.contains_key("reason"));
}

#[test]
fn rg_error_surface() {
    let err = AsmError::RG(sample_info("R001", "non causal"));
    assert_eq!(err.info().code, "R001");
}

#[test]
fn dictionary_error_surface() {
    let err = AsmError::Dictionary(sample_info("D001", "missing basis"));
    assert_eq!(err.info().code, "D001");
}

#[test]
fn rng_error_surface() {
    let err = AsmError::Rng(sample_info("RN001", "invalid seed"));
    assert_eq!(err.info().code, "RN001");
}

#[test]
fn serde_error_surface() {
    let err = AsmError::Serde(sample_info("S001", "schema mismatch"));
    assert_eq!(err.info().code, "S001");
}
