use asm_core::{AsmError, ErrorInfo};

use crate::css::CSSCode;
use crate::defect::ViolationSet;

/// Computes the violated constraints for a given binary state.
pub fn compute_violations(code: &CSSCode, bits: &[u8]) -> Result<ViolationSet, AsmError> {
    if bits.len() != code.num_variables() {
        let info = ErrorInfo::new(
            "state-length-mismatch",
            "state size does not match number of variables",
        )
        .with_context("num_variables", code.num_variables().to_string())
        .with_context("state_len", bits.len().to_string());
        return Err(AsmError::Code(info));
    }

    let mut violated_x = Vec::new();
    for (idx, constraint) in code.x_checks().iter().enumerate() {
        if parity(bits, constraint.variables()) {
            violated_x.push(idx);
        }
    }

    let mut violated_z = Vec::new();
    for (idx, constraint) in code.z_checks().iter().enumerate() {
        if parity(bits, constraint.variables()) {
            violated_z.push(idx);
        }
    }

    Ok(ViolationSet::new(
        violated_x.into_boxed_slice(),
        violated_z.into_boxed_slice(),
    ))
}

fn parity(bits: &[u8], vars: &[usize]) -> bool {
    let mut value = 0u8;
    for &idx in vars {
        value ^= bits[idx] & 1;
    }
    value == 1
}
