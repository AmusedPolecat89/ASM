use std::collections::BTreeSet;
use std::fmt;

use asm_core::{AsmError, ErrorInfo};
use siphasher::sip::SipHasher24;
use std::hash::{Hash, Hasher};

use crate::css::{CSSCode, ConstraintKind};

/// Set of violated constraints grouped by stabilizer type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViolationSet {
    x: Box<[usize]>,
    z: Box<[usize]>,
}

impl ViolationSet {
    /// Creates a violation set from explicit indices.
    pub fn new(x: Box<[usize]>, z: Box<[usize]>) -> Self {
        Self { x, z }
    }

    /// Returns the violated X stabilizer indices.
    pub fn x(&self) -> &[usize] {
        &self.x
    }

    /// Returns the violated Z stabilizer indices.
    pub fn z(&self) -> &[usize] {
        &self.z
    }
}

/// Deterministic identifier describing a defect species.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct SpeciesId(u64);

impl SpeciesId {
    /// Creates a species identifier from its raw representation.
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw representation.
    pub fn as_raw(&self) -> u64 {
        self.0
    }
}

impl fmt::Debug for SpeciesId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpeciesId({:#x})", self.0)
    }
}

impl fmt::Display for SpeciesId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "species-{:#x}", self.0)
    }
}

/// Classification for a defect pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DefectKind {
    /// Defect sourced from X violations only.
    X,
    /// Defect sourced from Z violations only.
    Z,
    /// Mixed defect touching both X and Z checks.
    Mixed,
}

/// Structured description for a detected defect.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Defect {
    /// Species identifier derived from the normalized pattern.
    pub species: SpeciesId,
    /// Stabilizer indices for X-type violations.
    pub x_checks: Box<[usize]>,
    /// Stabilizer indices for Z-type violations.
    pub z_checks: Box<[usize]>,
    /// Total number of violated checks in the support.
    pub support_size: usize,
    /// Classification of the defect.
    pub kind: DefectKind,
}

impl Defect {
    fn new(kind: DefectKind, x_checks: Vec<usize>, z_checks: Vec<usize>) -> Self {
        let support_size = x_checks.len() + z_checks.len();
        let species = species_from_components(kind, &x_checks, &z_checks);
        Self {
            species,
            x_checks: x_checks.into_boxed_slice(),
            z_checks: z_checks.into_boxed_slice(),
            support_size,
            kind,
        }
    }
}

/// Builds defects by treating each violated stabilizer as an irreducible pattern.
pub fn build_defects(_code: &CSSCode, violations: &ViolationSet) -> Vec<Defect> {
    let mut defects = Vec::new();
    for &idx in violations.x() {
        let defect = Defect::new(DefectKind::X, vec![idx], Vec::new());
        defects.push(defect);
    }
    for &idx in violations.z() {
        let defect = Defect::new(DefectKind::Z, Vec::new(), vec![idx]);
        defects.push(defect);
    }
    // Mixed defects are not produced in phase 3 core implementation; deterministic order is maintained.
    defects.sort_by_key(|defect| defect.species);
    defects
}

/// Returns whether a defect is irreducible.
pub fn is_irreducible(defect: &Defect) -> bool {
    defect.support_size <= 1
}

/// Fuses two defects together, normalizing the combined pattern.
pub fn fuse(a: &Defect, b: &Defect) -> Defect {
    let mut x_union: BTreeSet<usize> = a.x_checks.iter().copied().collect();
    x_union.extend(b.x_checks.iter().copied());
    let mut z_union: BTreeSet<usize> = a.z_checks.iter().copied().collect();
    z_union.extend(b.z_checks.iter().copied());

    let x_vec: Vec<usize> = x_union.into_iter().collect();
    let z_vec: Vec<usize> = z_union.into_iter().collect();
    let kind = match (x_vec.is_empty(), z_vec.is_empty()) {
        (false, true) => DefectKind::X,
        (true, false) => DefectKind::Z,
        _ => DefectKind::Mixed,
    };
    Defect::new(kind, x_vec, z_vec)
}

/// Computes the deterministic species identifier for a constraint pattern.
pub fn species_from_pattern(kind: ConstraintKind, checks: &[usize]) -> SpeciesId {
    match kind {
        ConstraintKind::X => species_from_components(DefectKind::X, checks, &[]),
        ConstraintKind::Z => species_from_components(DefectKind::Z, &[], checks),
    }
}

fn species_from_components(kind: DefectKind, x: &[usize], z: &[usize]) -> SpeciesId {
    let mut hasher = SipHasher24::new_with_keys(0x7367656e65736973, 0x636f64656b657973);
    kind.hash(&mut hasher);
    x.hash(&mut hasher);
    z.hash(&mut hasher);
    SpeciesId(hasher.finish())
}

/// Returns the support size associated with a species if known.
pub fn species_support(code: &CSSCode, species: SpeciesId) -> Option<usize> {
    code.species_support(species)
}

/// Ensures violation set indices are within bounds.
pub fn validate_violation_bounds(
    code: &CSSCode,
    violations: &ViolationSet,
) -> Result<(), AsmError> {
    let num_x = code.num_constraints_x();
    let num_z = code.num_constraints_z();
    if let Some(&idx) = violations.x().iter().find(|&&idx| idx >= num_x) {
        let info = ErrorInfo::new(
            "x-violation-out-of-range",
            "violation references non-existent X stabilizer",
        )
        .with_context("index", idx.to_string())
        .with_context("max", num_x.to_string());
        return Err(AsmError::Code(info));
    }
    if let Some(&idx) = violations.z().iter().find(|&&idx| idx >= num_z) {
        let info = ErrorInfo::new(
            "z-violation-out-of-range",
            "violation references non-existent Z stabilizer",
        )
        .with_context("index", idx.to_string())
        .with_context("max", num_z.to_string());
        return Err(AsmError::Code(info));
    }
    Ok(())
}
