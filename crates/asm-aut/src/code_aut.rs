use asm_code::{hash, CSSCode};
use asm_core::AsmError;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

/// Automorphism information for a CSS code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeAutReport {
    /// Estimated order of the CSS-preserving automorphism group.
    pub order: u64,
    /// Whether enumeration was truncated.
    pub gens_truncated: bool,
    /// Whether all detected automorphisms preserve the CSS decomposition.
    pub css_preserving: bool,
}

impl Default for CodeAutReport {
    fn default() -> Self {
        Self {
            order: 1,
            gens_truncated: false,
            css_preserving: true,
        }
    }
}

/// Computes CSS automorphism statistics for a code.
pub fn analyse_code(code: &CSSCode) -> Result<CodeAutReport, AsmError> {
    let (num_variables, x_checks_raw, z_checks_raw, _schema, _provenance, _rank_x, _rank_z) =
        hash::decompose(code);
    let x_checks = normalise_supports(x_checks_raw);
    let z_checks = normalise_supports(z_checks_raw);

    if num_variables == 0 {
        return Ok(CodeAutReport::default());
    }

    let exhaustive_limit = 6usize;
    if num_variables > exhaustive_limit || (x_checks.len() + z_checks.len()) > 10 {
        return Ok(CodeAutReport {
            order: 1,
            gens_truncated: true,
            css_preserving: true,
        });
    }

    let mut automorphisms = Vec::new();
    for perm in (0..num_variables).permutations(num_variables) {
        if preserves_css(&x_checks, &z_checks, &perm) {
            automorphisms.push(perm);
        }
    }

    if automorphisms.is_empty() {
        // Identity should always be present; fall back to identity entry.
        automorphisms.push((0..num_variables).collect());
    }

    let css_preserving = !detect_non_css_mapping(&x_checks, &z_checks);

    Ok(CodeAutReport {
        order: automorphisms.len() as u64,
        gens_truncated: false,
        css_preserving,
    })
}

fn normalise_supports(checks: Vec<asm_code::Constraint>) -> Vec<Vec<usize>> {
    let mut supports: Vec<Vec<usize>> = checks
        .into_iter()
        .map(|constraint| constraint.variables().to_vec())
        .collect();
    for support in &mut supports {
        support.sort_unstable();
    }
    supports.sort();
    supports
}

fn preserves_css(x_checks: &[Vec<usize>], z_checks: &[Vec<usize>], perm: &[usize]) -> bool {
    permute_checks(x_checks, perm) == x_checks && permute_checks(z_checks, perm) == z_checks
}

fn permute_checks(checks: &[Vec<usize>], perm: &[usize]) -> Vec<Vec<usize>> {
    let mut mapped: Vec<Vec<usize>> = checks
        .iter()
        .map(|support| {
            let mut mapped_support: Vec<usize> = support.iter().map(|&idx| perm[idx]).collect();
            mapped_support.sort_unstable();
            mapped_support
        })
        .collect();
    mapped.sort();
    mapped
}

fn detect_non_css_mapping(x_checks: &[Vec<usize>], z_checks: &[Vec<usize>]) -> bool {
    if x_checks.len() != z_checks.len() {
        return false;
    }
    let num_variables = x_checks
        .iter()
        .chain(z_checks.iter())
        .flat_map(|support| support.iter())
        .copied()
        .max()
        .map(|idx| idx + 1)
        .unwrap_or(0);
    if num_variables == 0 || num_variables > 6 {
        return false;
    }
    for perm in (0..num_variables).permutations(num_variables) {
        let maps_x_to_z = permute_checks(x_checks, &perm);
        let maps_z_to_x = permute_checks(z_checks, &perm);
        if maps_x_to_z == z_checks
            && maps_z_to_x == x_checks
            && (maps_x_to_z != x_checks || maps_z_to_x != z_checks)
        {
            return true;
        }
    }
    false
}
