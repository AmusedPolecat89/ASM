use asm_core::errors::AsmError;
use serde::{Deserialize, Serialize};

use crate::hash::stable_hash_string;
use crate::metrics::JobKpi;

/// Lightweight manifest describing the outcome of the MCMC stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McmcManifest {
    /// Seed identifier for the job.
    pub seed: u64,
    /// Rule identifier for the job.
    pub rule_id: u64,
    /// Number of sweeps performed.
    pub sweeps: u32,
    /// Final energy recorded by the sampler.
    pub energy_final: f64,
}

/// Simplified spectrum report summarising the energy landscape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectrumSummary {
    /// Number of modes included in the spectrum.
    pub modes: u32,
    /// Number of k-points sampled.
    pub k_points: u32,
    /// Spectral gap proxy derived from the run.
    pub spectral_gap: f64,
}

/// Simplified gauge report describing closure status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GaugeSummary {
    /// Whether closure checks passed.
    pub closure_pass: bool,
    /// Whether Ward identity checks passed.
    pub ward_pass: bool,
    /// Gauge factor labels detected during the run.
    pub factors: Vec<String>,
}

/// Simplified interaction report containing selected couplings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionSummary {
    /// Gauge coupling vector at the reference scale.
    pub g: Vec<f64>,
    /// Higgs self-coupling estimate.
    pub lambda_h: f64,
    /// Central charge estimate derived from interactions.
    pub c_est: f64,
}

/// Hash summary for all stages in a job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StageHashes {
    /// Hash of the MCMC manifest.
    pub mcmc: String,
    /// Hash of the spectrum summary.
    pub spectrum: String,
    /// Hash of the gauge summary.
    pub gauge: String,
    /// Hash of the interaction summary.
    pub interaction: String,
}

/// Deterministic bundle of stage outputs ready to be persisted to disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageOutputs {
    /// Lightweight MCMC manifest output.
    pub mcmc: McmcManifest,
    /// Spectrum summary output.
    pub spectrum: SpectrumSummary,
    /// Gauge summary output.
    pub gauge: GaugeSummary,
    /// Interaction summary output.
    pub interaction: InteractionSummary,
    /// KPI snapshot computed from the job.
    pub kpi: JobKpi,
    /// Canonical hashes for each stage output.
    pub hashes: StageHashes,
}

impl StageOutputs {
    fn build_hashes(
        mcmc: &McmcManifest,
        spectrum: &SpectrumSummary,
        gauge: &GaugeSummary,
        interaction: &InteractionSummary,
    ) -> Result<StageHashes, AsmError> {
        Ok(StageHashes {
            mcmc: stable_hash_string(mcmc)?,
            spectrum: stable_hash_string(spectrum)?,
            gauge: stable_hash_string(gauge)?,
            interaction: stable_hash_string(interaction)?,
        })
    }
}

/// Synthesises deterministic stage artefacts for the provided identifiers.
pub fn synthesise_stage_outputs(
    seed: u64,
    rule_id: u64,
    sweeps: u32,
    modes: u32,
    k_points: u32,
) -> Result<StageOutputs, AsmError> {
    let base = seed.wrapping_add(rule_id.wrapping_mul(37));
    let energy_final = -1.0 - (base % 100) as f64 / 1000.0;
    let kpi = JobKpi::synthesise(seed, rule_id);
    let mcmc = McmcManifest {
        seed,
        rule_id,
        sweeps,
        energy_final,
    };
    let spectrum = SpectrumSummary {
        modes,
        k_points,
        spectral_gap: kpi.gap_proxy,
    };
    let gauge = GaugeSummary {
        closure_pass: kpi.closure_pass,
        ward_pass: kpi.ward_pass,
        factors: kpi.factors.clone(),
    };
    let interaction = InteractionSummary {
        g: kpi.g.clone(),
        lambda_h: kpi.lambda_h,
        c_est: kpi.c_est,
    };
    let hashes = StageOutputs::build_hashes(&mcmc, &spectrum, &gauge, &interaction)?;
    Ok(StageOutputs {
        mcmc,
        spectrum,
        gauge,
        interaction,
        kpi,
        hashes,
    })
}
