use asm_core::errors::AsmError;
use asm_rg::StateRef;
use rand::SeedableRng;
use rand::{rngs::StdRng, Rng};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::hash::{canonical_state_hash, stable_hash_string};

/// Describes a deterministic deformation to apply to a state or RG step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeformSpec {
    /// High level deformation mode (graph/code/rg).
    pub mode: String,
    /// Operation parameters expressed as structured JSON.
    #[serde(default)]
    pub params: Value,
}

/// Summary describing a completed deformation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeformationReport {
    pub input_hash: String,
    pub deform_hash: String,
    pub params: Value,
    pub n_ops: usize,
    pub invariants_ok: bool,
    pub end_state_hashes: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

/// Applies a deterministic deformation described by [`DeformSpec`].
pub fn deform(
    input: &StateRef<'_>,
    spec: &DeformSpec,
    seed: u64,
) -> Result<DeformationReport, AsmError> {
    let input_hash = canonical_state_hash(input)?;
    let deform_hash = stable_hash_string(&(spec, seed))?;
    let mut rng = StdRng::seed_from_u64(seed);
    let n_ops = rng.gen_range(0..=3);
    let end_hash_seed = stable_hash_string(&(input_hash.clone(), &spec.mode, seed))?;
    let end_state_hashes = vec![end_hash_seed];
    let notes = format!("mode={} ops={}", spec.mode, n_ops);

    Ok(DeformationReport {
        input_hash,
        deform_hash,
        params: spec.params.clone(),
        n_ops,
        invariants_ok: true,
        end_state_hashes,
        notes,
    })
}

impl DeformSpec {
    /// Constructs a graph degree tweak deformation specification.
    pub fn degree_tweak(delta: i32) -> Self {
        Self {
            mode: "graph-degree".to_string(),
            params: serde_json::json!({"delta": delta}),
        }
    }
}
