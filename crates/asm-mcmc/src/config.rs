use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// YAML-configurable parameters governing an ensemble run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    /// Number of full sweeps to execute (post burn-in).
    pub sweeps: usize,
    /// Number of initial sweeps to discard when computing metrics.
    #[serde(default)]
    pub burn_in: usize,
    /// Interval at which to record metrics samples.
    #[serde(default = "default_thinning")]
    pub thinning: usize,
    /// Replica ladder specification.
    #[serde(default)]
    pub ladder: LadderConfig,
    /// Number of proposals of each move type per sweep.
    #[serde(default)]
    pub move_counts: MoveCounts,
    /// Checkpointing behaviour.
    #[serde(default)]
    pub checkpoint: CheckpointConfig,
    /// Weights for the scoring proxies.
    #[serde(default)]
    pub scoring: ScoringWeights,
    /// Master seed and substream policy.
    #[serde(default)]
    pub seed_policy: SeedPolicy,
    /// Output directory configuration.
    #[serde(default)]
    pub output: OutputConfig,
}

fn default_thinning() -> usize {
    1
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            sweeps: 32,
            burn_in: 0,
            thinning: 1,
            ladder: LadderConfig::default(),
            move_counts: MoveCounts::default(),
            checkpoint: CheckpointConfig::default(),
            scoring: ScoringWeights::default(),
            seed_policy: SeedPolicy::default(),
            output: OutputConfig::default(),
        }
    }
}

/// Replica ladder construction settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LadderConfig {
    /// Number of replicas in the ladder.
    #[serde(default = "default_replicas")]
    pub replicas: usize,
    /// Base temperature used for the coldest replica.
    #[serde(default = "default_base_temperature")]
    pub base_temperature: f64,
    /// Policy used to generate higher temperatures.
    #[serde(default)]
    pub policy: LadderPolicy,
}

fn default_replicas() -> usize {
    3
}

fn default_base_temperature() -> f64 {
    1.0
}

impl Default for LadderConfig {
    fn default() -> Self {
        Self {
            replicas: default_replicas(),
            base_temperature: default_base_temperature(),
            policy: LadderPolicy::default(),
        }
    }
}

/// Supported ladder construction strategies.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum LadderPolicy {
    /// Geometric progression with a fixed ratio between neighbouring replicas.
    Geometric {
        /// Multiplicative spacing ratio between adjacent replicas.
        #[serde(default = "default_ratio")]
        ratio: f64,
    },
    /// Explicit list of temperatures supplied by the user (overrides `replicas`).
    Manual {
        /// Ordered list of temperatures assigned to replicas.
        temperatures: Vec<f64>,
    },
}

fn default_ratio() -> f64 {
    1.5
}

impl Default for LadderPolicy {
    fn default() -> Self {
        LadderPolicy::Geometric {
            ratio: default_ratio(),
        }
    }
}

/// Number of proposals per move type performed within a sweep.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveCounts {
    /// Generator flip proposals.
    #[serde(default = "default_move_weight")]
    pub generator_flips: usize,
    /// Row operation proposals.
    #[serde(default = "default_move_weight")]
    pub row_ops: usize,
    /// Graph rewiring proposals.
    #[serde(default = "default_move_weight")]
    pub graph_rewires: usize,
    /// Worm / logical loop proposals.
    #[serde(default = "default_move_weight")]
    pub worm_moves: usize,
}

fn default_move_weight() -> usize {
    1
}

impl Default for MoveCounts {
    fn default() -> Self {
        Self {
            generator_flips: default_move_weight(),
            row_ops: default_move_weight(),
            graph_rewires: default_move_weight(),
            worm_moves: default_move_weight(),
        }
    }
}

/// Checkpointing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointConfig {
    /// Interval in sweeps between checkpoint writes (0 disables checkpoints).
    #[serde(default)]
    pub interval: usize,
    /// Directory where checkpoints are stored. Relative paths resolved from CLI working dir.
    #[serde(default)]
    pub directory: Option<PathBuf>,
    /// Maximum number of checkpoints to retain.
    #[serde(default = "default_checkpoint_retention")]
    pub max_to_keep: usize,
}

fn default_checkpoint_retention() -> usize {
    4
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            interval: 0,
            directory: None,
            max_to_keep: default_checkpoint_retention(),
        }
    }
}

/// Weights applied to the scoring proxies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    /// Weight for the cMDL proxy term.
    #[serde(default = "default_cmdl_weight")]
    pub cmdl: f64,
    /// Weight for the spectrum regularity proxy.
    #[serde(default = "default_specreg_weight")]
    pub spec: f64,
    /// Weight for the curvature variance proxy.
    #[serde(default = "default_curv_weight")]
    pub curv: f64,
}

fn default_cmdl_weight() -> f64 {
    1.0
}

fn default_specreg_weight() -> f64 {
    1.0
}

fn default_curv_weight() -> f64 {
    1.0
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            cmdl: default_cmdl_weight(),
            spec: default_specreg_weight(),
            curv: default_curv_weight(),
        }
    }
}

/// Deterministic seeding configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedPolicy {
    /// Master seed used for the run.
    #[serde(default = "default_master_seed")]
    pub master_seed: u64,
    /// Optional label used when deriving substream seeds (documented in manifests).
    #[serde(default)]
    pub label: Option<String>,
}

fn default_master_seed() -> u64 {
    0x05EE_D5EE_DD15_5EED_u64
}

impl Default for SeedPolicy {
    fn default() -> Self {
        Self {
            master_seed: default_master_seed(),
            label: None,
        }
    }
}

/// Output directory layout configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Root directory for run artefacts. Created if it does not exist.
    #[serde(default)]
    pub run_directory: Option<PathBuf>,
    /// Metrics filename relative to `run_directory`.
    #[serde(default = "default_metrics_filename")]
    pub metrics_file: PathBuf,
    /// Manifest filename relative to `run_directory`.
    #[serde(default = "default_manifest_filename")]
    pub manifest_file: PathBuf,
    /// Subdirectory used for checkpoint files.
    #[serde(default = "default_checkpoint_dir")]
    pub checkpoint_dir: PathBuf,
    /// Directory for final end-state exports.
    #[serde(default = "default_end_state_dir")]
    pub end_state_dir: PathBuf,
}

fn default_metrics_filename() -> PathBuf {
    PathBuf::from("metrics.csv")
}

fn default_manifest_filename() -> PathBuf {
    PathBuf::from("manifest.json")
}

fn default_checkpoint_dir() -> PathBuf {
    PathBuf::from("checkpoints")
}

fn default_end_state_dir() -> PathBuf {
    PathBuf::from("end_state")
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            run_directory: None,
            metrics_file: default_metrics_filename(),
            manifest_file: default_manifest_filename(),
            checkpoint_dir: default_checkpoint_dir(),
            end_state_dir: default_end_state_dir(),
        }
    }
}
