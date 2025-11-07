use std::fs;
use std::path::Path;

use asm_core::errors::{AsmError, ErrorInfo};
use serde::{Deserialize, Serialize};

use crate::metrics::JobKpi;
use crate::serde::from_yaml_slice;

fn io_error(code: &str, err: impl ToString) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, err.to_string()))
}

/// Anthropic filter specification applied to job KPIs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterSpec {
    /// Whether closure checks must pass.
    #[serde(default = "FilterSpec::default_require_closure")]
    pub require_closure: bool,
    /// Whether Ward checks must pass.
    #[serde(default = "FilterSpec::default_require_ward")]
    pub require_ward: bool,
    /// Minimum accepted central charge.
    #[serde(default = "FilterSpec::default_c_min")]
    pub c_min: f64,
    /// Maximum accepted central charge.
    #[serde(default = "FilterSpec::default_c_max")]
    pub c_max: f64,
    /// Minimum accepted spectral gap proxy.
    #[serde(default = "FilterSpec::default_gap_min")]
    pub gap_min: f64,
    /// Required factors that must be present in the gauge summary.
    #[serde(default)]
    pub factor_presence: Vec<String>,
}

impl FilterSpec {
    fn default_require_closure() -> bool {
        true
    }
    fn default_require_ward() -> bool {
        true
    }
    fn default_c_min() -> f64 {
        0.7
    }
    fn default_c_max() -> f64 {
        1.4
    }
    fn default_gap_min() -> f64 {
        0.05
    }

    /// Applies the filter specification to the provided KPI snapshot.
    pub fn evaluate(&self, kpi: &JobKpi) -> FilterDecision {
        let closure = if self.require_closure {
            kpi.closure_pass
        } else {
            true
        };
        let ward = if self.require_ward {
            kpi.ward_pass
        } else {
            true
        };
        let c_range = kpi.c_est >= self.c_min && kpi.c_est <= self.c_max;
        let gap_ok = kpi.gap_proxy >= self.gap_min;
        let factors_ok = self
            .factor_presence
            .iter()
            .all(|factor| kpi.factors.iter().any(|f| f == factor));
        FilterDecision {
            closure,
            ward,
            c_range,
            gap_ok,
            factors: factors_ok,
        }
    }
}

/// Outcome of applying anthropic filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FilterDecision {
    /// Closure predicate result.
    pub closure: bool,
    /// Ward predicate result.
    pub ward: bool,
    /// Central charge window predicate result.
    pub c_range: bool,
    /// Gap predicate result.
    pub gap_ok: bool,
    /// Factor presence predicate result.
    pub factors: bool,
}

impl FilterDecision {
    /// Returns true when all predicates succeed.
    pub fn passes(&self) -> bool {
        self.closure && self.ward && self.c_range && self.gap_ok && self.factors
    }
}

/// Loads a filter specification from the provided YAML path.
pub fn load_filters(path: &Path) -> Result<FilterSpec, AsmError> {
    let bytes = fs::read(path).map_err(|err| io_error("filter_read", err))?;
    from_yaml_slice(&bytes)
}
