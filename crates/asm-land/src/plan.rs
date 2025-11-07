use std::fs;
use std::path::{Path, PathBuf};

use asm_core::errors::{AsmError, ErrorInfo};
use serde::{Deserialize, Serialize};

use crate::hash::stable_hash_string;
use crate::serde::{from_yaml_slice, to_yaml_string};

fn io_error(code: &str, err: impl ToString) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, err.to_string()))
}

/// Layout describing how job outputs are written to disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum OutputLayout {
    /// All artefacts live in a flat directory with seed_rule identifiers.
    #[default]
    Flat,
    /// Each seed owns its own directory tree.
    PerSeed,
}

/// Output configuration for the landscape run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputSpec {
    /// Directory layout to use when emitting artefacts.
    #[serde(default)]
    pub layout: OutputLayout,
    /// Whether intermediate stage artefacts should be preserved.
    #[serde(default)]
    pub keep_intermediate: bool,
}

impl Default for OutputSpec {
    fn default() -> Self {
        Self {
            layout: OutputLayout::Flat,
            keep_intermediate: true,
        }
    }
}

/// Graph ensemble parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphSpec {
    /// Maximum degree allowed when generating graph instances.
    pub degree_cap: u32,
    /// Uniformity of the hypergraph (number of nodes per edge).
    pub k_uniform: u32,
    /// Number of nodes in the graph.
    pub size: u32,
    /// Generator variant used to synthesise the ensemble.
    pub generator: String,
}

/// Code ensemble parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeSpec {
    /// Sparsity (density) parameter for constraint matrices.
    pub density: f64,
    /// CSS code variant label to use when sampling.
    pub css_variant: String,
    /// Row operation rate applied during the sampler.
    pub rowop_rate: f64,
}

/// Sampler configuration used by the MCMC stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SamplerSpec {
    /// Number of sweeps performed per job.
    pub sweeps: u32,
    /// Worm move weight parameter controlling update probabilities.
    pub worm_weight: f64,
    /// Ladder depth used when exploring excited states.
    pub ladder: u32,
    /// Number of checkpoint snapshots to persist.
    pub checkpoints: u32,
}

/// Spectrum evaluation configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectrumSpec {
    /// Number of k-points sampled in the dispersion relation.
    pub k_points: u32,
    /// Number of modes retained in the spectrum.
    pub modes: u32,
}

/// Gauge stage configuration knobs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GaugeSpec {
    /// Closure tolerance applied to gauge constraints.
    pub closure_tol: f64,
    /// Ward identity tolerance applied during validation.
    pub ward_tol: f64,
}

/// Interaction stage configuration shared with Phase 13.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractSpec {
    #[serde(default = "InteractSpec::default_steps")]
    /// Number of integration steps performed.
    pub steps: u32,
    #[serde(default = "InteractSpec::default_dt")]
    /// Time step used during evolution.
    pub dt: f64,
    /// Measurement selector identifier.
    pub measure: String,
    /// Fit configuration identifier.
    pub fit: String,
}

impl InteractSpec {
    fn default_steps() -> u32 {
        64
    }

    fn default_dt() -> f64 {
        0.02
    }
}

/// Rule variant controlling parameter perturbations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleSpec {
    /// Identifier for deterministic hashing and directory naming.
    pub id: u64,
    /// Human readable label describing the rule variant.
    pub label: String,
}

impl Default for RuleSpec {
    fn default() -> Self {
        Self {
            id: 0,
            label: "default".to_string(),
        }
    }
}

/// Deterministic landscape exploration plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    /// Seeds describing the ensemble instances to visit.
    pub seeds: Vec<u64>,
    /// Graph ensemble parameters.
    pub graph: GraphSpec,
    /// Code ensemble parameters.
    pub code: CodeSpec,
    /// MCMC sampler configuration.
    pub sampler: SamplerSpec,
    /// Spectrum evaluation configuration.
    pub spectrum: SpectrumSpec,
    /// Gauge configuration.
    pub gauge: GaugeSpec,
    /// Interaction configuration knobs.
    pub interact: InteractSpec,
    /// Path to the anthropic filter specification.
    pub filters: PathBuf,
    /// Output layout configuration.
    #[serde(default)]
    pub outputs: OutputSpec,
    /// Rule variants to scan.
    #[serde(default)]
    pub rules: Vec<RuleSpec>,
    /// Directory containing the plan on disk (ignored when serializing).
    #[serde(skip)]
    pub base_dir: PathBuf,
}

impl Plan {
    /// Returns the deterministic hash associated with the plan contents.
    pub fn plan_hash(&self) -> Result<String, AsmError> {
        stable_hash_string(self)
    }

    /// Returns an iterator over the rule variants, synthesising the default rule when omitted.
    pub fn rules(&self) -> Vec<RuleSpec> {
        if self.rules.is_empty() {
            vec![RuleSpec::default()]
        } else {
            self.rules.clone()
        }
    }

    /// Produces a canonical YAML representation of the plan.
    pub fn to_yaml_string(&self) -> Result<String, AsmError> {
        to_yaml_string(self)
    }

    /// Returns the resolved path to the anthropic filter specification.
    pub fn filters_path(&self) -> PathBuf {
        if self.filters.is_absolute() {
            self.filters.clone()
        } else {
            self.base_dir.join(&self.filters)
        }
    }
}

/// Loads a plan from disk, ensuring deterministic ordering of seeds and rules.
pub fn load_plan<P: AsRef<Path>>(path: P) -> Result<Plan, AsmError> {
    let plan_path = path.as_ref();
    let bytes = fs::read(plan_path).map_err(|err| io_error("plan_read", err))?;
    let mut plan: Plan = from_yaml_slice(&bytes)?;
    plan.seeds.sort_unstable();
    plan.rules.sort_by_key(|rule| rule.id);
    plan.base_dir = plan_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    Ok(plan)
}
