use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use asm_core::{
    AsmError, ConstraintProjector, ConstraintState, ErrorInfo, LogicalAlgebraSummary,
    RunProvenance, SchemaVersion,
};

use crate::analyze;
use crate::defect::{self, SpeciesId, ViolationSet};
use crate::hash;
use crate::state;
use crate::syndrome;

/// Kind of CSS constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstraintKind {
    /// X-type stabilizer.
    X,
    /// Z-type stabilizer.
    Z,
}

/// Sparse mod-2 constraint acting on a subset of variables.
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Constraint {
    variables: Box<[usize]>,
}

impl fmt::Debug for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Constraint").field(&self.variables).finish()
    }
}

impl Constraint {
    /// Creates a new normalized constraint from sorted indices.
    pub(crate) fn new(mut vars: Vec<usize>) -> Self {
        vars.sort_unstable();
        let mut normalized = Vec::with_capacity(vars.len());
        for var in vars {
            if normalized.last() == Some(&var) {
                normalized.pop();
            } else {
                normalized.push(var);
            }
        }
        Self {
            variables: normalized.into_boxed_slice(),
        }
    }

    /// Returns the variables touched by the constraint.
    pub fn variables(&self) -> &[usize] {
        &self.variables
    }
}

impl PartialOrd for Constraint {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Constraint {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.variables.cmp(&other.variables)
    }
}

/// Deterministic CSS code implementing [`ConstraintProjector`].
pub struct CSSCode {
    num_variables: usize,
    x_checks: Vec<Constraint>,
    z_checks: Vec<Constraint>,
    schema_version: SchemaVersion,
    provenance: RunProvenance,
    rank_x: usize,
    rank_z: usize,
    x_adjacency: Vec<Vec<usize>>,
    z_adjacency: Vec<Vec<usize>>,
    species_lookup: BTreeMap<SpeciesId, usize>,
}

impl fmt::Debug for CSSCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CSSCode")
            .field("num_variables", &self.num_variables)
            .field("x_checks", &self.x_checks)
            .field("z_checks", &self.z_checks)
            .field("schema_version", &self.schema_version)
            .field("provenance", &self.provenance)
            .finish_non_exhaustive()
    }
}

impl CSSCode {
    /// Constructs a CSS code from sparse X/Z stabilizer lists.
    pub fn new(
        num_variables: usize,
        x_checks: Vec<Vec<usize>>,
        z_checks: Vec<Vec<usize>>,
        schema_version: SchemaVersion,
        provenance: RunProvenance,
    ) -> Result<Self, AsmError> {
        let normalized_x = Self::normalize_checks(num_variables, ConstraintKind::X, x_checks)?;
        let normalized_z = Self::normalize_checks(num_variables, ConstraintKind::Z, z_checks)?;
        Self::validate_css_orthogonality(&normalized_x, &normalized_z)?;

        let mut x_checks = normalized_x;
        let mut z_checks = normalized_z;
        x_checks.sort();
        z_checks.sort();

        let rank_x = Self::mod2_rank(num_variables, &x_checks);
        let rank_z = Self::mod2_rank(num_variables, &z_checks);

        let x_adjacency = Self::build_adjacency(num_variables, &x_checks);
        let z_adjacency = Self::build_adjacency(num_variables, &z_checks);

        let mut species_lookup = BTreeMap::new();
        for (idx, constraint) in x_checks.iter().enumerate() {
            let species =
                defect::species_from_pattern(ConstraintKind::X, std::slice::from_ref(&idx));
            species_lookup.insert(species, constraint.variables().len());
        }
        for (idx, constraint) in z_checks.iter().enumerate() {
            let species =
                defect::species_from_pattern(ConstraintKind::Z, std::slice::from_ref(&idx));
            species_lookup.insert(species, constraint.variables().len());
        }

        Ok(Self {
            num_variables,
            x_checks,
            z_checks,
            schema_version,
            provenance,
            rank_x,
            rank_z,
            x_adjacency,
            z_adjacency,
            species_lookup,
        })
    }

    fn normalize_checks(
        num_variables: usize,
        kind: ConstraintKind,
        raw_checks: Vec<Vec<usize>>,
    ) -> Result<Vec<Constraint>, AsmError> {
        let mut seen = BTreeSet::new();
        let mut constraints = Vec::with_capacity(raw_checks.len());
        for (idx, raw) in raw_checks.into_iter().enumerate() {
            let constraint = Constraint::new(raw);
            if constraint
                .variables()
                .iter()
                .any(|&var| var >= num_variables)
            {
                let info = ErrorInfo::new(
                    "variable-out-of-range",
                    "constraint references variable outside allowed domain",
                )
                .with_context("constraint_kind", format!("{:?}", kind))
                .with_context("constraint_index", idx.to_string())
                .with_context("num_variables", num_variables.to_string());
                return Err(AsmError::Code(info));
            }
            if !seen.insert(constraint.clone()) {
                let info =
                    ErrorInfo::new("duplicate-constraint", "duplicate CSS constraint detected")
                        .with_context("constraint_kind", format!("{:?}", kind))
                        .with_context("constraint_index", idx.to_string());
                return Err(AsmError::Code(info));
            }
            constraints.push(constraint);
        }
        Ok(constraints)
    }

    fn validate_css_orthogonality(
        x_checks: &[Constraint],
        z_checks: &[Constraint],
    ) -> Result<(), AsmError> {
        for (xi, x) in x_checks.iter().enumerate() {
            for (zi, z) in z_checks.iter().enumerate() {
                let mut parity = false;
                let mut ix = 0;
                let mut iz = 0;
                while ix < x.variables().len() && iz < z.variables().len() {
                    match x.variables()[ix].cmp(&z.variables()[iz]) {
                        std::cmp::Ordering::Less => ix += 1,
                        std::cmp::Ordering::Greater => iz += 1,
                        std::cmp::Ordering::Equal => {
                            parity = !parity;
                            ix += 1;
                            iz += 1;
                        }
                    }
                }
                if parity {
                    let info = ErrorInfo::new(
                        "css-orthogonality-failed",
                        "X/Z constraint pair anticommutes",
                    )
                    .with_context("x_index", xi.to_string())
                    .with_context("z_index", zi.to_string());
                    return Err(AsmError::Code(info));
                }
            }
        }
        Ok(())
    }

    fn build_adjacency(num_variables: usize, checks: &[Constraint]) -> Vec<Vec<usize>> {
        let mut adjacency = vec![Vec::new(); num_variables];
        for (idx, constraint) in checks.iter().enumerate() {
            for &var in constraint.variables() {
                adjacency[var].push(idx);
            }
        }
        for entries in &mut adjacency {
            entries.shrink_to_fit();
        }
        adjacency
    }

    fn mod2_rank(num_variables: usize, checks: &[Constraint]) -> usize {
        let width = num_variables.div_ceil(64);
        let mut rows = Vec::with_capacity(checks.len());
        for constraint in checks {
            let mut row = vec![0u64; width];
            for &var in constraint.variables() {
                let bucket = var / 64;
                let offset = var % 64;
                row[bucket] ^= 1u64 << offset;
            }
            rows.push(row);
        }
        let mut rank = 0;
        let mut col = 0;
        for i in 0..rows.len() {
            while col < num_variables {
                let pivot_bucket = col / 64;
                let pivot_offset = col % 64;
                if let Some((pivot, _)) = rows
                    .iter()
                    .enumerate()
                    .skip(i)
                    .find(|(_, row)| ((row[pivot_bucket] >> pivot_offset) & 1) == 1)
                {
                    rows.swap(i, pivot);
                    for j in 0..rows.len() {
                        if j != i {
                            let bit = (rows[j][pivot_bucket] >> pivot_offset) & 1;
                            if bit == 1 {
                                for k in 0..width {
                                    rows[j][k] ^= rows[i][k];
                                }
                            }
                        }
                    }
                    rank += 1;
                    col += 1;
                    break;
                }
                col += 1;
            }
            if col >= num_variables {
                break;
            }
        }
        rank
    }

    /// Returns the number of variables in the code.
    pub fn num_variables(&self) -> usize {
        self.num_variables
    }

    /// Returns the number of X stabilizers.
    pub fn num_constraints_x(&self) -> usize {
        self.x_checks.len()
    }

    /// Returns the number of Z stabilizers.
    pub fn num_constraints_z(&self) -> usize {
        self.z_checks.len()
    }

    /// Returns the rank of the X stabilizers.
    pub fn rank_x(&self) -> usize {
        self.rank_x
    }

    /// Returns the rank of the Z stabilizers.
    pub fn rank_z(&self) -> usize {
        self.rank_z
    }

    /// Returns whether the stored constraints satisfy CSS orthogonality.
    pub fn is_css_orthogonal(&self) -> bool {
        true
    }

    /// Returns the schema version associated with the code.
    pub fn schema_version(&self) -> SchemaVersion {
        self.schema_version
    }

    /// Returns the provenance payload stored with the code.
    pub fn provenance(&self) -> &RunProvenance {
        &self.provenance
    }

    /// Computes the canonical structural hash for the code.
    pub fn canonical_hash(&self) -> String {
        hash::canonical_code_hash(self)
    }

    /// Computes the violation set for the provided state handle.
    pub fn violations_for_state(
        &self,
        state: &dyn ConstraintState,
    ) -> Result<ViolationSet, AsmError> {
        let bits = state::view_bits(state)?;
        syndrome::compute_violations(self, bits)
    }

    /// Computes violation sets for a batch of states.
    pub fn violations_for_states(
        &self,
        states: &[&dyn ConstraintState],
    ) -> Result<Vec<ViolationSet>, AsmError> {
        let mut results = Vec::with_capacity(states.len());
        for state in states {
            results.push(self.violations_for_state(*state)?);
        }
        Ok(results)
    }

    /// Extracts irreducible defects from a violation set.
    pub fn find_defects(&self, violations: &ViolationSet) -> Vec<defect::Defect> {
        defect::build_defects(self, violations)
    }

    /// Computes a deterministic species identifier for a defect.
    pub fn species(&self, defect: &defect::Defect) -> SpeciesId {
        defect.species
    }

    /// Returns the deterministically ordered catalog of defect species.
    pub fn species_catalog(&self) -> Vec<SpeciesId> {
        self.species_lookup.keys().copied().collect()
    }

    /// Returns the cached degree information for X checks touching a variable.
    pub fn x_adjacency(&self, var: usize) -> &[usize] {
        &self.x_adjacency[var]
    }

    /// Returns the cached degree information for Z checks touching a variable.
    pub fn z_adjacency(&self, var: usize) -> &[usize] {
        &self.z_adjacency[var]
    }

    /// Returns the catalogued support size for a species if known.
    pub(crate) fn species_support(&self, species: SpeciesId) -> Option<usize> {
        self.species_lookup.get(&species).copied()
    }

    /// Returns references to the internal X stabilizers.
    pub(crate) fn x_checks(&self) -> &[Constraint] {
        &self.x_checks
    }

    /// Returns references to the internal Z stabilizers.
    pub(crate) fn z_checks(&self) -> &[Constraint] {
        &self.z_checks
    }
}

impl ConstraintProjector for CSSCode {
    fn num_variables(&self) -> usize {
        self.num_variables()
    }

    fn num_constraints(&self) -> usize {
        self.num_constraints_x() + self.num_constraints_z()
    }

    fn rank(&self) -> Result<usize, AsmError> {
        Ok(self.rank_x + self.rank_z)
    }

    fn check_violations(&self, state: &dyn ConstraintState) -> Result<Box<[usize]>, AsmError> {
        let violations = self.violations_for_state(state)?;
        let mut merged = Vec::with_capacity(violations.x().len() + violations.z().len());
        merged.extend_from_slice(violations.x());
        let offset = self.num_constraints_x();
        merged.extend(violations.z().iter().map(|&idx| idx + offset));
        Ok(merged.into_boxed_slice())
    }

    fn logical_algebra_summary(&self) -> Result<LogicalAlgebraSummary, AsmError> {
        analyze::logical_summary(self)
    }
}

/// Reconstructs a CSS code from serialized components.
pub fn from_parts(
    num_variables: usize,
    x_checks: Vec<Constraint>,
    z_checks: Vec<Constraint>,
    schema_version: SchemaVersion,
    provenance: RunProvenance,
    rank_x: usize,
    rank_z: usize,
) -> CSSCode {
    let x_adjacency = CSSCode::build_adjacency(num_variables, &x_checks);
    let z_adjacency = CSSCode::build_adjacency(num_variables, &z_checks);
    let mut species_lookup = BTreeMap::new();
    for (idx, constraint) in x_checks.iter().enumerate() {
        let species = defect::species_from_pattern(ConstraintKind::X, std::slice::from_ref(&idx));
        species_lookup.insert(species, constraint.variables().len());
    }
    for (idx, constraint) in z_checks.iter().enumerate() {
        let species = defect::species_from_pattern(ConstraintKind::Z, std::slice::from_ref(&idx));
        species_lookup.insert(species, constraint.variables().len());
    }

    CSSCode {
        num_variables,
        x_checks,
        z_checks,
        schema_version,
        provenance,
        rank_x,
        rank_z,
        x_adjacency,
        z_adjacency,
        species_lookup,
    }
}

/// Serializes a CSS code into plain components.
pub fn into_parts(
    code: &CSSCode,
) -> (
    usize,
    Vec<Constraint>,
    Vec<Constraint>,
    SchemaVersion,
    RunProvenance,
    usize,
    usize,
) {
    (
        code.num_variables,
        code.x_checks.clone(),
        code.z_checks.clone(),
        code.schema_version,
        code.provenance.clone(),
        code.rank_x,
        code.rank_z,
    )
}
