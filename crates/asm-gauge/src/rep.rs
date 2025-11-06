use asm_aut::AnalysisReport;
use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::rng::RngHandle;
use asm_spec::SpectrumReport;
use rand::RngCore;
use serde::{Deserialize, Serialize};

fn gauge_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, message))
}

fn round(value: f64) -> f64 {
    (value * 1e9).round() / 1e9
}

fn default_basis() -> String {
    "modes".to_string()
}

fn default_generator_limit() -> usize {
    3
}

/// Options controlling representation construction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepOpts {
    /// Basis descriptor recorded in the output payload.
    #[serde(default = "default_basis")]
    pub basis: String,
    /// Maximum number of generators to synthesise from the automorphism report.
    #[serde(default = "default_generator_limit")]
    pub max_generators: usize,
    /// Optional deterministic seed overriding the provenance derived from hashes.
    #[serde(default)]
    pub seed: Option<u64>,
}

impl Default for RepOpts {
    fn default() -> Self {
        Self {
            basis: default_basis(),
            max_generators: default_generator_limit(),
            seed: None,
        }
    }
}

/// Dense representation of a single generator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepGenerator {
    /// Deterministic identifier assigned to the generator.
    pub id: String,
    /// Row-major dense matrix describing the action on the mode basis.
    pub matrix: Vec<f64>,
    /// Frobenius norm of the generator matrix.
    pub norm: f64,
}

/// Representation matrices attached to the gauge analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepMatrices {
    /// Label describing the basis ordering (defaults to "modes").
    pub basis: String,
    /// Dimensionality of the retained basis.
    pub dim: usize,
    /// Generator matrices recorded in deterministic order.
    pub gens: Vec<RepGenerator>,
}

fn seed_from_hash(hash: &str) -> u64 {
    let trimmed = hash.trim();
    let len = trimmed.len().min(16);
    if len == 0 {
        return 0;
    }
    u64::from_str_radix(&trimmed[..len], 16).unwrap_or(0)
}

fn generator_count(opts: &RepOpts, spectrum: &SpectrumReport, analysis: &AnalysisReport) -> usize {
    let modes = spectrum.dispersion.modes.len().max(1);
    if opts.max_generators == 0 {
        return 0;
    }
    let base = (analysis.graph_aut.order as usize % (modes + 1))
        + (analysis.code_aut.order as usize % (modes + 2))
        + 1;
    base.min(opts.max_generators.max(1))
}

fn diagonal_pattern(dim: usize, rng: &mut RngHandle, scale: f64) -> Vec<f64> {
    let mut diag = Vec::with_capacity(dim);
    for idx in 0..dim {
        let raw = rng.next_u32();
        let centred = (raw as f64 / u32::MAX as f64) - 0.5;
        let value = round(centred * scale + if idx == 0 { 1.0 } else { 0.0 });
        diag.push(value);
    }
    diag
}

fn normalise_trace(diag: &mut [f64], force_zero: bool) {
    if diag.is_empty() {
        return;
    }
    let mean: f64 = diag.iter().copied().sum::<f64>() / diag.len() as f64;
    if force_zero {
        for value in diag.iter_mut() {
            *value = round(*value - mean);
        }
    }
}

/// Builds deterministic representation matrices on the dispersion mode basis.
pub fn build_rep(
    spectrum: &SpectrumReport,
    aut: &AnalysisReport,
    opts: &RepOpts,
) -> Result<RepMatrices, AsmError> {
    let dim = spectrum.dispersion.modes.len();
    if dim == 0 {
        return Err(gauge_error(
            "empty-mode-basis",
            "spectrum report does not provide dispersion modes",
        ));
    }
    let mut seed = opts
        .seed
        .unwrap_or_else(|| seed_from_hash(&aut.hashes.analysis_hash));
    if seed == 0 {
        seed = seed_from_hash(&spectrum.analysis_hash).max(1);
    }
    let mut rng = RngHandle::from_seed(seed);
    let count = generator_count(opts, spectrum, aut);
    if count == 0 {
        return Err(gauge_error(
            "no-generators",
            "representation requested zero generators",
        ));
    }

    let mut generators = Vec::with_capacity(count);
    for gen_idx in 0..count {
        let scale = (aut.graph_aut.order.max(1) as f64 + aut.code_aut.order.max(1) as f64)
            / ((gen_idx + 1) as f64 * dim as f64);
        let mut diag = diagonal_pattern(dim, &mut rng, scale.max(0.05));
        // Alternate between zero-trace (su2-like) and shifted (u1-like) generators.
        let force_zero = gen_idx % 2 == 0;
        normalise_trace(&mut diag, force_zero);
        let mut matrix = vec![0.0; dim * dim];
        for idx in 0..dim {
            matrix[idx * dim + idx] = diag[idx];
        }
        let norm = round(matrix.iter().map(|value| value * value).sum::<f64>().sqrt());
        generators.push(RepGenerator {
            id: format!("G{gen_idx}"),
            matrix,
            norm,
        });
    }

    Ok(RepMatrices {
        basis: opts.basis.clone(),
        dim,
        gens: generators,
    })
}
