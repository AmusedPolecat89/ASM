use asm_code::{hash::canonical_code_hash, CSSCode};
use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::rng::derive_substream_seed;
use asm_graph::{canonical_hash as graph_hash, HypergraphImpl};
use serde::{Deserialize, Serialize};

use crate::dispersion::{dispersion_scan, DispersionReport, DispersionSpec};
use crate::hash::stable_hash_string;
use crate::operators::{build_operators, OpOpts, Operators, OpsVariant};
use crate::propagation::{excite_and_propagate, PropOpts};
use crate::{correl::CorrelSpec, correl::CorrelationReport};

fn report_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, message))
}

fn default_fit_tol() -> f64 {
    1e-6
}

/// Aggregated configuration for a spectrum analysis run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpecOpts {
    /// Operator construction options.
    #[serde(default)]
    pub ops: OpOpts,
    /// Excitation description.
    #[serde(default)]
    pub excitation: crate::excitations::ExcitationSpec,
    /// Propagation configuration (including deterministic seed).
    pub propagation: PropOpts,
    /// Dispersion scan configuration.
    #[serde(default)]
    pub dispersion: DispersionSpec,
    /// Correlation scan configuration.
    #[serde(default)]
    pub correlation: CorrelSpec,
    /// Master seed used to derive substreams for dispersion/correlation.
    pub master_seed: u64,
    /// Fit tolerance recorded in the provenance payload.
    #[serde(default = "default_fit_tol")]
    pub fit_tolerance: f64,
}

impl SpecOpts {
    fn dispersion_seed(&self) -> u64 {
        derive_substream_seed(self.master_seed, 1)
    }

    fn correlation_seed(&self) -> u64 {
        derive_substream_seed(self.master_seed, 2)
    }
}

/// Provenance metadata bundled with a [`SpectrumReport`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpectrumProvenance {
    /// Source commit (if available) or package version.
    pub commit: String,
    /// Master deterministic seed for the analysis.
    pub master_seed: u64,
    /// Seed used when seeding excitations and propagating responses.
    pub propagation_seed: u64,
    /// Seed used during the dispersion scan.
    pub dispersion_seed: u64,
    /// Seed used during the correlation scan.
    pub correlation_seed: u64,
    /// Recorded fit tolerance.
    pub fit_tolerance: f64,
    /// Operator variant used for construction.
    pub ops_variant: OpsVariant,
    /// Deterministic hash of the intermediate linear response.
    pub response_hash: String,
}

/// Deterministic spectrum analysis bundle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpectrumReport {
    /// Content addressed hash of the full analysis artefact.
    pub analysis_hash: String,
    /// Canonical hash of the input graph.
    pub graph_hash: String,
    /// Canonical hash of the input code.
    pub code_hash: String,
    /// Operators constructed for the input state.
    pub operators: Operators,
    /// Dispersion diagnostics.
    pub dispersion: DispersionReport,
    /// Correlation diagnostics.
    pub correlation: CorrelationReport,
    /// Provenance information describing deterministic seeds and knobs.
    pub provenance: SpectrumProvenance,
}

fn commit_string() -> String {
    option_env!("GIT_COMMIT_HASH")
        .or_else(|| option_env!("VERGEN_GIT_SHA"))
        .map(|value| value.to_string())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
}

/// Performs deterministic spectral analysis and emits a [`SpectrumReport`].
pub fn analyze_spectrum(
    graph: &HypergraphImpl,
    code: &CSSCode,
    sopts: &SpecOpts,
) -> Result<SpectrumReport, AsmError> {
    if sopts.propagation.seed == 0 {
        return Err(report_error(
            "missing-propagation-seed",
            "propagation seed must be provided in SpecOpts",
        ));
    }
    let operators = build_operators(graph, code, &sopts.ops)?;
    let response = excite_and_propagate(&operators, &sopts.excitation, &sopts.propagation)?;
    let dispersion = dispersion_scan(&operators, &sopts.dispersion, sopts.dispersion_seed())?;
    let correlation =
        crate::correl::correlation_scan(&operators, &sopts.correlation, sopts.correlation_seed())?;

    let graph_hash = graph_hash(graph).map_err(|err| match err {
        AsmError::Graph(info) => AsmError::Graph(info),
        other => other,
    })?;
    let code_hash = canonical_code_hash(code);

    let provenance = SpectrumProvenance {
        commit: commit_string(),
        master_seed: sopts.master_seed,
        propagation_seed: sopts.propagation.seed,
        dispersion_seed: sopts.dispersion_seed(),
        correlation_seed: sopts.correlation_seed(),
        fit_tolerance: sopts.fit_tolerance,
        ops_variant: sopts.ops.variant,
        response_hash: response.response_hash,
    };

    let mut report = SpectrumReport {
        analysis_hash: String::new(),
        graph_hash,
        code_hash,
        operators,
        dispersion,
        correlation,
        provenance,
    };

    report.analysis_hash = stable_hash_string(&(
        &report.graph_hash,
        &report.code_hash,
        &report.operators.info.hash,
        &report.dispersion,
        &report.correlation,
        &report.provenance,
    ))?;

    Ok(report)
}
