use std::collections::BTreeSet;

use asm_code::css::CSSCode;
use asm_core::errors::ErrorInfo;
use asm_core::{AsmError, RngHandle};
use rand::RngCore;

/// Result of a CSS code proposal.
#[derive(Debug)]
pub struct CodeMoveProposal {
    /// Candidate code produced by the move.
    pub candidate: CSSCode,
    /// Forward proposal probability used for MH acceptance.
    pub forward_prob: f64,
    /// Reverse proposal probability used for MH acceptance.
    pub reverse_prob: f64,
    /// Indices of generators touched by the move.
    pub touched_generators: Vec<usize>,
    /// Human readable description of the move.
    pub description: String,
}

/// Attempts to toggle the support of a randomly chosen generator.
pub fn propose_generator_flip(
    code: &CSSCode,
    rng: &mut RngHandle,
) -> Result<CodeMoveProposal, AsmError> {
    let num_variables = code.num_variables();
    let (_, x_parts, z_parts, _, _, _, _) = asm_code::css::into_parts(code);
    let mut x_checks: Vec<Vec<usize>> = x_parts
        .iter()
        .map(|constraint| constraint.variables().to_vec())
        .collect();
    let mut z_checks: Vec<Vec<usize>> = z_parts
        .iter()
        .map(|constraint| constraint.variables().to_vec())
        .collect();

    if x_checks.is_empty() && z_checks.is_empty() {
        return Err(AsmError::Code(ErrorInfo::new(
            "no-generators",
            "cannot flip generator in empty code",
        )));
    }

    let total = x_checks.len() + z_checks.len();
    let choice = (rng.next_u64() as usize) % total;
    let var_choice = if num_variables == 0 {
        0
    } else {
        (rng.next_u64() as usize) % num_variables
    };

    let (target_vec, description_prefix) = if choice < x_checks.len() {
        (&mut x_checks[choice], "x")
    } else {
        (&mut z_checks[choice - x_checks.len()], "z")
    };

    if let Some(pos) = target_vec.iter().position(|&var| var == var_choice) {
        target_vec.remove(pos);
    } else {
        target_vec.push(var_choice);
        target_vec.sort_unstable();
        target_vec.dedup();
    }

    let candidate = CSSCode::new(
        num_variables,
        x_checks.clone(),
        z_checks.clone(),
        code.schema_version(),
        code.provenance().clone(),
    )?;

    Ok(CodeMoveProposal {
        candidate,
        forward_prob: 1.0 / total.max(1) as f64,
        reverse_prob: 1.0 / total.max(1) as f64,
        touched_generators: vec![choice],
        description: format!("generator-flip:{description_prefix}{choice}:var{var_choice}"),
    })
}

/// Proposes a row operation by XORing two generators from the same family.
pub fn propose_row_operation(
    code: &CSSCode,
    rng: &mut RngHandle,
) -> Result<CodeMoveProposal, AsmError> {
    let num_variables = code.num_variables();
    let (_, x_parts, z_parts, _, _, _, _) = asm_code::css::into_parts(code);
    let mut x_checks: Vec<Vec<usize>> = x_parts
        .iter()
        .map(|constraint| constraint.variables().to_vec())
        .collect();
    let mut z_checks: Vec<Vec<usize>> = z_parts
        .iter()
        .map(|constraint| constraint.variables().to_vec())
        .collect();

    let choose_x_family = if x_checks.len() < 2 {
        false
    } else if z_checks.len() < 2 {
        true
    } else {
        (rng.next_u64() & 1) == 0
    };

    let (family_len, idx_a, idx_b) = {
        let family = if choose_x_family {
            &mut x_checks
        } else {
            &mut z_checks
        };

        if family.len() < 2 {
            return Err(AsmError::Code(ErrorInfo::new(
                "insufficient-generators",
                "not enough generators for row op",
            )));
        }

        let family_len = family.len();
        let idx_a = (rng.next_u64() as usize) % family_len;
        let mut idx_b = (rng.next_u64() as usize) % family_len;
        if idx_b == idx_a {
            idx_b = (idx_b + 1) % family_len;
        }

        let mut set: BTreeSet<usize> = family[idx_a].iter().copied().collect();
        for var in &family[idx_b] {
            if !set.insert(*var) {
                set.remove(var);
            }
        }
        family[idx_a] = set.iter().copied().collect();
        (family_len, idx_a, idx_b)
    };

    let candidate = CSSCode::new(
        num_variables,
        x_checks.clone(),
        z_checks.clone(),
        code.schema_version(),
        code.provenance().clone(),
    )?;

    let family_label = if choose_x_family { "x" } else { "z" };
    Ok(CodeMoveProposal {
        candidate,
        forward_prob: 1.0 / family_len.max(1) as f64,
        reverse_prob: 1.0 / family_len.max(1) as f64,
        touched_generators: vec![idx_a],
        description: format!("row-op:{family_label}{idx_a}^{family_label}{idx_b}"),
    })
}
