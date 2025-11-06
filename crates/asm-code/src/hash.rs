use sha2::{Digest, Sha256};

use crate::css::{CSSCode, Constraint};

fn update_constraint(hasher: &mut Sha256, constraint: &Constraint) {
    let vars = constraint.variables();
    hasher.update((vars.len() as u64).to_le_bytes());
    for &var in vars {
        hasher.update((var as u64).to_le_bytes());
    }
}

/// Computes the canonical structural hash for a CSS code.
pub fn canonical_code_hash(code: &CSSCode) -> String {
    let mut hasher = Sha256::new();
    let version = code.schema_version();
    hasher.update((version.major as u64).to_le_bytes());
    hasher.update((version.minor as u64).to_le_bytes());
    hasher.update((version.patch as u64).to_le_bytes());
    hasher.update((code.num_variables() as u64).to_le_bytes());
    hasher.update((code.num_constraints_x() as u64).to_le_bytes());
    hasher.update((code.num_constraints_z() as u64).to_le_bytes());
    hasher.update((code.rank_x() as u64).to_le_bytes());
    hasher.update((code.rank_z() as u64).to_le_bytes());

    for constraint in code.x_checks() {
        update_constraint(&mut hasher, constraint);
    }
    for constraint in code.z_checks() {
        update_constraint(&mut hasher, constraint);
    }

    let digest = hasher.finalize();
    digest
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>()
}

/// Restores a CSS code from its serialized components.
pub fn reconstruct(
    num_variables: usize,
    x_checks: Vec<Constraint>,
    z_checks: Vec<Constraint>,
    schema: asm_core::SchemaVersion,
    provenance: asm_core::RunProvenance,
    rank_x: usize,
    rank_z: usize,
) -> crate::css::CSSCode {
    crate::css::from_parts(
        num_variables,
        x_checks,
        z_checks,
        schema,
        provenance,
        rank_x,
        rank_z,
    )
}

/// Decomposes a CSS code into its serialized components.
pub fn decompose(
    code: &crate::css::CSSCode,
) -> (
    usize,
    Vec<Constraint>,
    Vec<Constraint>,
    asm_core::SchemaVersion,
    asm_core::RunProvenance,
    usize,
    usize,
) {
    crate::css::into_parts(code)
}
