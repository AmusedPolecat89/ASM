use asm_core::errors::{AsmError, ErrorInfo};
use asm_gauge::GaugeReport;
use asm_spec::SpectrumReport;
use serde::{Deserialize, Serialize};

use crate::fit::{fit_couplings, CouplingsFit, FitOpts};
use crate::hash::stable_hash_string;
use crate::kernel::{evolve, KernelOpts, Trajectory};
use crate::measure::{measure, MeasureOpts, ObsReport};
use crate::prepare::{prepare_state, PrepSpec, PreparedState};

fn report_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, message.into()))
}

/// Provenance payload recorded in [`InteractionReport`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InteractionProvenance {
    /// Preparation seed provided by the caller.
    pub seed: u64,
    /// Kernel options summary.
    pub kernel: KernelOpts,
    /// Measurement options summary.
    pub measure: MeasureOpts,
    /// Fit options summary.
    pub fit: FitOpts,
}

/// Aggregated interaction report capturing preparation, measurement and fit artefacts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InteractionReport {
    /// Stable hash of the full interaction bundle.
    pub analysis_hash: String,
    /// Graph hash forwarded from the spectrum report.
    pub graph_hash: String,
    /// Code hash forwarded from the spectrum report.
    pub code_hash: String,
    /// Preparation hash identifying the initial state.
    pub prep_hash: String,
    /// Observable hash identifying the measurement record.
    pub obs_hash: String,
    /// Coupling fit payload.
    pub fit: CouplingsFit,
    /// Optional trajectory metadata.
    pub trajectory: Trajectory,
    /// Provenance payload describing deterministic seeds and knobs.
    pub provenance: InteractionProvenance,
}

fn validate_reports(spec: &SpectrumReport, gauge: &GaugeReport) -> Result<(), AsmError> {
    if spec.graph_hash != gauge.graph_hash || spec.code_hash != gauge.code_hash {
        return Err(report_error(
            "hash-mismatch",
            "spectrum and gauge reports describe different states",
        ));
    }
    Ok(())
}

/// Aggregates a full single-shot interaction experiment.
pub fn interact(
    spec: &SpectrumReport,
    gauge: &GaugeReport,
    prep: &PrepSpec,
    kern: &KernelOpts,
    mopts: &MeasureOpts,
    fopts: &FitOpts,
    seed: u64,
) -> Result<InteractionReport, AsmError> {
    validate_reports(spec, gauge)?;
    let prepared = prepare_state(spec, gauge, prep, seed)?;
    let trajectory = evolve(&prepared, kern)?;
    let obs = measure(&trajectory, mopts)?;
    let fit = fit_couplings(&obs, fopts)?;

    let provenance = InteractionProvenance {
        seed,
        kernel: kern.clone(),
        measure: mopts.clone(),
        fit: fopts.clone(),
    };

    let analysis_hash = stable_hash_string(&(
        &spec.graph_hash,
        &spec.code_hash,
        &prepared.prep_hash,
        &obs.obs_hash,
        &fit.fit_hash,
        &provenance.seed,
    ))?;

    Ok(InteractionReport {
        analysis_hash,
        graph_hash: spec.graph_hash.clone(),
        code_hash: spec.code_hash.clone(),
        prep_hash: prepared.prep_hash.clone(),
        obs_hash: obs.obs_hash.clone(),
        fit,
        trajectory,
        provenance,
    })
}

/// Convenience helper returning the full suite of artefacts produced by [`interact`].
pub fn interact_full(
    spec: &SpectrumReport,
    gauge: &GaugeReport,
    prep: &PrepSpec,
    kern: &KernelOpts,
    mopts: &MeasureOpts,
    fopts: &FitOpts,
    seed: u64,
) -> Result<
    (
        PreparedState,
        Trajectory,
        ObsReport,
        CouplingsFit,
        InteractionReport,
    ),
    AsmError,
> {
    validate_reports(spec, gauge)?;
    let prepared = prepare_state(spec, gauge, prep, seed)?;
    let trajectory = evolve(&prepared, kern)?;
    let obs = measure(&trajectory, mopts)?;
    let fit = fit_couplings(&obs, fopts)?;

    let provenance = InteractionProvenance {
        seed,
        kernel: kern.clone(),
        measure: mopts.clone(),
        fit: fopts.clone(),
    };
    let analysis_hash = stable_hash_string(&(
        &spec.graph_hash,
        &spec.code_hash,
        &prepared.prep_hash,
        &obs.obs_hash,
        &fit.fit_hash,
        &provenance.seed,
    ))?;

    let report = InteractionReport {
        analysis_hash,
        graph_hash: spec.graph_hash.clone(),
        code_hash: spec.code_hash.clone(),
        prep_hash: prepared.prep_hash.clone(),
        obs_hash: obs.obs_hash.clone(),
        fit: fit.clone(),
        trajectory: trajectory.clone(),
        provenance,
    };

    Ok((prepared, trajectory, obs, fit, report))
}
