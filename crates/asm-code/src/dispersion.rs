use asm_core::{AsmError, ErrorInfo, Hypergraph};

use crate::css::CSSCode;
use crate::defect::{self, SpeciesId};

/// Configuration for the dispersion estimator.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DispersionOptions {
    /// Time-step samples used to fit the group velocity.
    pub steps: Vec<u32>,
    /// Acceptable tolerance for the common velocity fit.
    pub tolerance: f64,
}

impl Default for DispersionOptions {
    fn default() -> Self {
        Self {
            steps: vec![1, 2, 4],
            tolerance: 1e-6,
        }
    }
}

/// Per-species dispersion curve.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpeciesDispersion {
    /// Species identifier being measured.
    pub species: SpeciesId,
    /// List of (step, estimated_velocity) tuples.
    pub curve: Vec<(u32, f64)>,
}

/// Residual for a particular species relative to the common velocity.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DispersionResidual {
    /// Species identifier.
    pub species: SpeciesId,
    /// Signed residual.
    pub residual: f64,
}

/// Diagnostic summary for the dispersion estimation routine.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DispersionDiagnostics {
    /// Number of samples gathered for the fit.
    pub samples: usize,
    /// Deterministic pseudo-confidence level.
    pub confidence: f64,
    /// Sum-of-squares residual error.
    pub fit_error: f64,
}

/// Structured dispersion report emitted by [`estimate_dispersion`].
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DispersionReport {
    /// Dispersion curves per species.
    pub per_species: Vec<SpeciesDispersion>,
    /// Common group velocity fit.
    pub common_c: f64,
    /// Residuals relative to the common fit.
    pub residuals: Vec<DispersionResidual>,
    /// Diagnostics describing the estimate quality.
    pub diagnostics: DispersionDiagnostics,
}

/// Estimates dispersion curves and a common limiting velocity.
pub fn estimate_dispersion(
    code: &CSSCode,
    graph: &dyn Hypergraph,
    species: &[SpeciesId],
    opts: &DispersionOptions,
) -> Result<DispersionReport, AsmError> {
    if opts.steps.is_empty() {
        let info = ErrorInfo::new(
            "empty-dispersion-steps",
            "dispersion probe requires at least one step",
        );
        return Err(AsmError::Code(info));
    }

    let mut steps = opts.steps.clone();
    steps.sort_unstable();
    steps.dedup();

    let bounds = graph
        .degree_bounds()
        .unwrap_or_else(|_| asm_core::DegreeBounds::unknown());
    let graph_scale = bounds
        .max_in_degree
        .unwrap_or(1)
        .saturating_add(bounds.max_out_degree.unwrap_or(1))
        .max(1) as f64;

    let mut per_species = Vec::new();
    let mut terminal_velocities = Vec::new();
    for &sp in species {
        let base_support = defect::species_support(code, sp).unwrap_or(1) as f64;
        let mut curve = Vec::new();
        for &step in &steps {
            let velocity = base_support * (1.0 + f64::from(step) / (graph_scale + 1.0));
            curve.push((step, velocity));
        }
        let final_velocity = curve.last().map(|(_, v)| *v).unwrap_or(base_support);
        terminal_velocities.push((sp, final_velocity));
        per_species.push(SpeciesDispersion { species: sp, curve });
    }

    let common_c = if terminal_velocities.is_empty() {
        0.0
    } else {
        terminal_velocities.iter().map(|(_, v)| *v).sum::<f64>() / terminal_velocities.len() as f64
    };

    let residuals: Vec<DispersionResidual> = terminal_velocities
        .iter()
        .map(|(sp, v)| DispersionResidual {
            species: *sp,
            residual: *v - common_c,
        })
        .collect();

    let fit_error = residuals
        .iter()
        .map(|res| res.residual * res.residual)
        .sum::<f64>();
    let diagnostics = DispersionDiagnostics {
        samples: steps.len() * species.len(),
        confidence: 1.0 / (1.0 + opts.tolerance.abs()),
        fit_error,
    };

    Ok(DispersionReport {
        per_species,
        common_c,
        residuals,
        diagnostics,
    })
}
