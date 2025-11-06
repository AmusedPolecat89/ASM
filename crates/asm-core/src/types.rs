use serde::{Deserialize, Serialize};

use crate::provenance::{RunProvenance, SchemaVersion};

/// Physical coupling constants extracted from an ASM run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Couplings {
    /// Schema version for the coupling payload.
    pub schema_version: SchemaVersion,
    /// Provenance information tying the couplings to source artifacts.
    pub provenance: RunProvenance,
    /// Kinetic coupling constant.
    pub c_kin: f64,
    /// Gauge coupling vector ordered as (g1, g2, g3).
    pub gauge: [f64; 3],
    /// Yukawa couplings in canonical ordering.
    pub yukawa: Vec<f64>,
    /// Higgs self-coupling constant.
    pub lambda_h: f64,
    /// Optional notes and diagnostics emitted by the extractor.
    pub notes: Option<String>,
}
