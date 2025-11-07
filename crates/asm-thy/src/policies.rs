use serde::{Deserialize, Serialize};

/// Inclusive range used for acceptance checks.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PolicyRange {
    /// Minimum accepted value.
    pub min: f64,
    /// Maximum accepted value.
    pub max: f64,
}

impl PolicyRange {
    /// Returns whether the provided value lies within the inclusive range.
    pub fn contains(&self, value: f64) -> bool {
        value >= self.min && value <= self.max
    }
}

impl Default for PolicyRange {
    fn default() -> Self {
        Self { min: 0.0, max: 1.0 }
    }
}

/// Tolerance policy controlling assertion behaviour.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Policy {
    /// Rounding granularity applied to reported metrics.
    #[serde(default = "Policy::default_rounding")]
    pub rounding: f64,
    /// Absolute tolerance for cross-checks.
    #[serde(default = "Policy::default_abs_tol")]
    pub abs_tol: f64,
    /// Relative tolerance for trend checks.
    #[serde(default = "Policy::default_rel_tol")]
    pub rel_tol: f64,
    /// Closure tolerance expected from gauge artefacts.
    #[serde(default = "Policy::default_closure_tol")]
    pub closure_tol: f64,
    /// Ward commutator tolerance expected from gauge artefacts.
    #[serde(default = "Policy::default_ward_tol")]
    pub ward_tol: f64,
    /// Relative tolerance for dispersion linearity checks.
    #[serde(default = "Policy::default_rel_tol_lin")]
    pub rel_tol_lin: f64,
    /// Maximum tolerated coupling fit residual.
    #[serde(default = "Policy::default_fit_resid")]
    pub fit_resid_max: f64,
    /// Accepted anthropic pass-rate range.
    #[serde(default)]
    pub landscape_rate: PolicyRange,
    /// Whether strict mode is enabled (unused but reserved for future tightening).
    #[serde(default)]
    pub strict: bool,
    /// Require closure artefacts to be present.
    #[serde(default = "Policy::default_require_closure")]
    pub require_closure: bool,
    /// Require ward artefacts to be present.
    #[serde(default = "Policy::default_require_ward")]
    pub require_ward: bool,
}

impl Policy {
    const fn default_rounding() -> f64 {
        1e-9
    }

    const fn default_abs_tol() -> f64 {
        1e-9
    }

    const fn default_rel_tol() -> f64 {
        1e-5
    }

    const fn default_closure_tol() -> f64 {
        1e-6
    }

    const fn default_ward_tol() -> f64 {
        1e-5
    }

    const fn default_rel_tol_lin() -> f64 {
        5e-2
    }

    const fn default_fit_resid() -> f64 {
        1.5
    }

    const fn default_require_closure() -> bool {
        true
    }

    const fn default_require_ward() -> bool {
        true
    }

    /// Rounds the provided value according to the policy granularity.
    pub fn round(&self, value: f64) -> f64 {
        if self.rounding <= 0.0 {
            return value;
        }
        (value / self.rounding).round() * self.rounding
    }
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            rounding: Self::default_rounding(),
            abs_tol: Self::default_abs_tol(),
            rel_tol: Self::default_rel_tol(),
            closure_tol: Self::default_closure_tol(),
            ward_tol: Self::default_ward_tol(),
            rel_tol_lin: Self::default_rel_tol_lin(),
            fit_resid_max: Self::default_fit_resid(),
            landscape_rate: PolicyRange { min: 0.4, max: 0.9 },
            strict: false,
            require_closure: Self::default_require_closure(),
            require_ward: Self::default_require_ward(),
        }
    }
}
