use serde::{Deserialize, Serialize};

/// Options controlling RG coarse graining.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RGOpts {
    /// Deterministic scaling factor applied at each step.
    pub scale_factor: usize,
    /// Maximum number of fine nodes merged into a single block.
    pub max_block_size: usize,
    /// Deterministic seed influencing block ordering.
    pub seed: u64,
}

impl Default for RGOpts {
    fn default() -> Self {
        Self {
            scale_factor: 2,
            max_block_size: 2,
            seed: 0xC0FFEE_u64,
        }
    }
}

impl RGOpts {
    /// Ensures the configuration is well-formed and returns a sanitised copy.
    pub fn sanitised(&self) -> Self {
        let scale_factor = self.scale_factor.max(1);
        let max_block_size = self.max_block_size.max(1);
        Self {
            scale_factor,
            max_block_size,
            seed: self.seed,
        }
    }
}

/// Options controlling the operator dictionary extraction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictOpts {
    /// Number of synthetic Yukawa couplings to emit.
    pub yukawa_count: usize,
    /// Deterministic seed used when generating auxiliary probes.
    pub seed: u64,
    /// Maximum tolerated residual when reporting convergence diagnostics.
    pub residual_tolerance: f64,
}

impl Default for DictOpts {
    fn default() -> Self {
        Self {
            yukawa_count: 4,
            seed: 0xA55EED5EED,
            residual_tolerance: 1e-6,
        }
    }
}

impl DictOpts {
    /// Returns a sanitised configuration with positive counts.
    pub fn sanitised(&self) -> Self {
        let yukawa_count = self.yukawa_count.max(1);
        let residual_tolerance = self.residual_tolerance.max(0.0);
        Self {
            yukawa_count,
            seed: self.seed,
            residual_tolerance,
        }
    }
}

/// Thresholds used when performing the covariance check between RG and dictionary flows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CovarianceThresholds {
    /// Relative tolerance applied to the kinetic term.
    pub c_kin_relative: f64,
    /// Absolute tolerance applied to gauge couplings.
    pub g_absolute: f64,
    /// Absolute tolerance applied to the Higgs self coupling.
    pub lambda_absolute: f64,
    /// Absolute tolerance applied to Yukawa couplings.
    pub yukawa_absolute: f64,
}

impl Default for CovarianceThresholds {
    fn default() -> Self {
        Self {
            c_kin_relative: 0.05,
            g_absolute: 0.1,
            lambda_absolute: 0.1,
            yukawa_absolute: 0.1,
        }
    }
}
