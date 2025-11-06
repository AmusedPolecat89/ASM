use asm_code::css::CSSCode;
use asm_code::state::StateHandle;
use asm_core::errors::ErrorInfo;
use asm_core::{AsmError, RngHandle};
use asm_graph::HypergraphImpl;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Result of a logical worm/loop proposal.
#[derive(Debug, Clone)]
pub struct WormProposal {
    /// Deterministic hash of the sampled logical pattern.
    pub sample_hash: String,
    /// Human readable description for logs.
    pub description: String,
    /// Number of violated checks touched by the worm sample.
    pub support_size: usize,
}

/// Generates a deterministic worm/loop sample used for coverage diagnostics.
pub fn propose_worm(
    code: &CSSCode,
    _graph: &HypergraphImpl,
    rng: &mut RngHandle,
) -> Result<WormProposal, AsmError> {
    let num_variables = code.num_variables();
    if num_variables == 0 {
        return Err(AsmError::Code(ErrorInfo::new(
            "empty-code",
            "cannot generate worm sample for empty code",
        )));
    }
    let mut bits = vec![0u8; num_variables];
    let head = (rng.next_u64() as usize) % num_variables;
    bits[head] = 1;
    let state = StateHandle::from_bits(bits)?;
    let violations = code.violations_for_state(&state)?;
    let defects = code.find_defects(&violations);
    let mut hasher = Sha256::new();
    let mut support_size = 0usize;
    let mut species_strings = Vec::new();
    for defect in &defects {
        support_size += defect.support_size;
        hasher.update(defect.species.as_raw().to_le_bytes());
        species_strings.push(defect.species.to_string());
    }
    if species_strings.is_empty() {
        species_strings.push("trivial".to_string());
    }
    let digest = hasher.finalize();
    let sample_hash = format!("worm-{:x}", digest);
    Ok(WormProposal {
        sample_hash,
        support_size,
        description: format!("worm:var{}:{}", head, species_strings.join("+")),
    })
}
